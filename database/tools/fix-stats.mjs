/**
 * 数据库修复：
 *  1) 用修正后的定义替换存储过程 sp_confirm_order（原版内部是 cook_count + 1 增量）
 *  2) 按明细全量重算所有菜谱的 cook_count / avg_rating，消除已产生的漂移
 *  3) 清理端到端测试残留（测试食材、测试标签）
 *  4) 修复后核验：统计与明细完全一致、无残留
 */
import mysql from 'mysql2/promise'
import { DB } from './sql-runner.mjs'

const c = await mysql.createConnection({
  host: DB.host, port: DB.port, user: DB.user, password: DB.password,
  database: DB.database, charset: 'utf8mb4', connectTimeout: 15000
})

let pass = 0, fail = 0
const ok = (cond, msg) => { cond ? (pass++, console.log('  ✓', msg)) : (fail++, console.error('  ✗', msg)) }

console.log('='.repeat(62))
console.log('  数据库修复：统计口径统一 + 测试残留清理')
console.log('='.repeat(62))

// ---------- 修复前快照 ----------
console.log('\n[0] 修复前状态')
const [before] = await c.query(`
  SELECT r.id, r.name, r.cook_count, r.avg_rating,
    COALESCE((SELECT CAST(SUM(oi.portions) AS SIGNED) FROM order_item oi JOIN meal_order o ON o.id=oi.order_id
               WHERE oi.recipe_id=r.id AND o.status NOT IN ('cancelled','draft') AND oi.item_status<>'skipped'),0) AS real_cook,
    (SELECT ROUND(AVG(oi.rating),2) FROM order_item oi JOIN meal_order o ON o.id=oi.order_id
       WHERE oi.recipe_id=r.id AND oi.rating IS NOT NULL AND o.status NOT IN ('cancelled','draft')) AS real_rating
  FROM recipe r ORDER BY r.id`)
const drifted = before.filter(r => r.cook_count != r.real_cook || String(r.avg_rating) !== String(r.real_rating ?? '0.00'))
console.log(`  统计漂移的菜谱：${drifted.length} 道`)
drifted.forEach(r => console.log(`    id=${r.id} ${r.name}: cook_count ${r.cook_count}→${r.real_cook}, avg_rating ${r.avg_rating}→${r.real_rating ?? '0.00'}`))

const [[residue]] = await c.query(`
  SELECT (SELECT COUNT(*) FROM ingredient WHERE name LIKE '%勿动%') AS ing,
         (SELECT COUNT(*) FROM tag WHERE name LIKE '%勿动%') AS tag,
         (SELECT COUNT(*) FROM recipe WHERE name LIKE '%勿动%') AS rec,
         (SELECT COUNT(*) FROM meal_order WHERE note LIKE '%自动化测试%') AS ord`)
console.log(`  测试残留：食材 ${residue.ing}、标签 ${residue.tag}、菜谱 ${residue.rec}、订单 ${residue.ord}`)

// ---------- 1) 替换存储过程 ----------
console.log('\n[1] 替换存储过程 sp_confirm_order（增量 → 重算）')
await c.query('DROP PROCEDURE IF EXISTS `sp_confirm_order`')
await c.query(`
CREATE PROCEDURE \`sp_confirm_order\`(IN p_order_id BIGINT UNSIGNED)
BEGIN
  DECLARE v_status VARCHAR(16);

  SELECT \`status\` INTO v_status FROM \`meal_order\` WHERE id = p_order_id FOR UPDATE;
  IF v_status IS NULL THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = '订单不存在';
  END IF;
  IF v_status = 'cancelled' THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = '已取消的订单无法确认';
  END IF;

  UPDATE \`meal_order\` SET \`status\` = 'confirmed' WHERE id = p_order_id;

  UPDATE \`recipe\` r
     SET r.cook_count = COALESCE((
           SELECT CAST(SUM(oi.portions) AS SIGNED)
             FROM \`order_item\` oi JOIN \`meal_order\` o ON o.id = oi.order_id
            WHERE oi.recipe_id = r.id
              AND o.status NOT IN ('cancelled','draft')
              AND oi.item_status <> 'skipped'
         ), 0),
         r.avg_rating = COALESCE((
           SELECT ROUND(AVG(oi.rating),2)
             FROM \`order_item\` oi JOIN \`meal_order\` o ON o.id = oi.order_id
            WHERE oi.recipe_id = r.id AND oi.rating IS NOT NULL
              AND o.status NOT IN ('cancelled','draft')
         ), 0)
   WHERE r.id IN (SELECT DISTINCT oi2.recipe_id FROM \`order_item\` oi2
                   WHERE oi2.order_id = p_order_id AND oi2.recipe_id IS NOT NULL);

  CALL \`sp_generate_shopping_list\`(p_order_id);
END`)

const [[proc]] = await c.query(`
  SELECT ROUTINE_DEFINITION AS def FROM information_schema.ROUTINES
   WHERE ROUTINE_SCHEMA=? AND ROUTINE_NAME='sp_confirm_order'`, [DB.database])
ok(proc && !/cook_count\s*\+\s*1/i.test(proc.def), '存储过程已替换为按明细重算（不再含 +1 增量）')
ok(proc && /NOT IN \('cancelled','draft'\)/i.test(proc.def), '统计口径已排除 draft 与 cancelled')

// ---------- 2) 全量重算统计 ----------
console.log('\n[2] 全量重算菜谱统计')
const [res] = await c.query(`
  UPDATE recipe r SET
    r.cook_count = COALESCE((
      SELECT CAST(SUM(oi.portions) AS SIGNED)
        FROM order_item oi JOIN meal_order o ON o.id = oi.order_id
       WHERE oi.recipe_id = r.id
         AND o.status NOT IN ('cancelled','draft')
         AND oi.item_status <> 'skipped'), 0),
    r.avg_rating = COALESCE((
      SELECT ROUND(AVG(oi.rating),2)
        FROM order_item oi JOIN meal_order o ON o.id = oi.order_id
       WHERE oi.recipe_id = r.id AND oi.rating IS NOT NULL
         AND o.status NOT IN ('cancelled','draft')), 0.00)`)
console.log(`  已重算，影响 ${res.affectedRows} 行`)

// ---------- 3) 清理测试残留 ----------
console.log('\n[3] 清理端到端测试残留')
// 先确认这些残留没有被任何真实数据引用，避免误删
const [[ref]] = await c.query(`
  SELECT
    (SELECT COUNT(*) FROM recipe_ingredient ri JOIN ingredient i ON i.id=ri.ingredient_id WHERE i.name LIKE '%勿动%') AS ing_ref,
    (SELECT COUNT(*) FROM recipe_tag rt JOIN tag t ON t.id=rt.tag_id WHERE t.name LIKE '%勿动%') AS tag_ref`)
if (ref.ing_ref === 0 && ref.tag_ref === 0) {
  const [d1] = await c.query(`DELETE FROM ingredient WHERE name LIKE '%勿动%'`)
  const [d2] = await c.query(`DELETE FROM tag WHERE name LIKE '%勿动%'`)
  console.log(`  已删除测试食材 ${d1.affectedRows} 条、测试标签 ${d2.affectedRows} 条`)
} else {
  console.log(`  ⚠ 残留仍被引用（食材 ${ref.ing_ref} / 标签 ${ref.tag_ref}），跳过删除以免破坏关联`)
  fail++
}

// ---------- 4) 核验 ----------
console.log('\n[4] 修复后核验')
const [after] = await c.query(`
  SELECT r.id, r.name, r.cook_count, r.avg_rating,
    COALESCE((SELECT CAST(SUM(oi.portions) AS SIGNED) FROM order_item oi JOIN meal_order o ON o.id=oi.order_id
               WHERE oi.recipe_id=r.id AND o.status NOT IN ('cancelled','draft') AND oi.item_status<>'skipped'),0) AS real_cook,
    (SELECT ROUND(AVG(oi.rating),2) FROM order_item oi JOIN meal_order o ON o.id=oi.order_id
       WHERE oi.recipe_id=r.id AND oi.rating IS NOT NULL AND o.status NOT IN ('cancelled','draft')) AS real_rating
  FROM recipe r ORDER BY r.id`)
const stillDrifted = after.filter(r => r.cook_count != r.real_cook || String(r.avg_rating) !== String(r.real_rating ?? '0.00'))
ok(stillDrifted.length === 0, `全部菜谱统计与明细一致（漂移 ${stillDrifted.length} 道）`)
if (stillDrifted.length) stillDrifted.forEach(r => console.error(`    仍漂移 id=${r.id} ${r.name}`))

const [[r1]] = await c.query(`SELECT cook_count, avg_rating FROM recipe WHERE id=1`)
ok(r1.cook_count === 2 && String(r1.avg_rating) === '5.00', `红烧肉已还原：cook_count=${r1.cook_count}(期望2) avg_rating=${r1.avg_rating}(期望5.00)`)

const [[r2]] = await c.query(`SELECT cook_count FROM recipe WHERE id=2`)
ok(r2.cook_count === 1, `番茄炒蛋已还原：cook_count=${r2.cook_count}(期望1)`)

const [[res2]] = await c.query(`
  SELECT (SELECT COUNT(*) FROM ingredient WHERE name LIKE '%勿动%') AS ing,
         (SELECT COUNT(*) FROM tag WHERE name LIKE '%勿动%') AS tag,
         (SELECT COUNT(*) FROM recipe WHERE name LIKE '%勿动%') AS rec,
         (SELECT COUNT(*) FROM meal_order WHERE note LIKE '%自动化测试%') AS ord`)
ok(res2.ing === 0 && res2.tag === 0 && res2.rec === 0 && res2.ord === 0, '测试残留已清空')

const [[cnt]] = await c.query(`
  SELECT (SELECT COUNT(*) FROM ingredient) AS ing, (SELECT COUNT(*) FROM tag) AS tag,
         (SELECT COUNT(*) FROM recipe) AS rec, (SELECT COUNT(*) FROM meal_order) AS ord,
         (SELECT COUNT(*) FROM order_item) AS item, (SELECT COUNT(*) FROM shopping_item) AS shop`)
ok(cnt.ing === 77 && cnt.tag === 10 && cnt.rec === 30 && cnt.ord === 4,
  `核心表行数复原：食材${cnt.ing}/77 标签${cnt.tag}/10 菜谱${cnt.rec}/30 订单${cnt.ord}/4`)

// AUTO_INCREMENT 会被测试推高，重置回合理值（不影响数据，仅避免 ID 跳号）
await c.query('ALTER TABLE meal_order AUTO_INCREMENT = 5')
await c.query('ALTER TABLE recipe AUTO_INCREMENT = 31')
console.log('  已重置 meal_order / recipe 的 AUTO_INCREMENT，避免 ID 跳号')

console.log('\n' + '='.repeat(62))
console.log(fail === 0 ? '  ✓ 数据库修复完成' : `  ⚠ 有 ${fail} 项未通过`)
console.log('='.repeat(62))

await c.end()
process.exit(fail > 0 ? 1 : 0)
