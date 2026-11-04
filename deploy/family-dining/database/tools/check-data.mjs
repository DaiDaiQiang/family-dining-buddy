/**
 * 数据库终态核验（只读）
 *
 * 确认：
 *  1) 结构完整（表/视图/存储过程/索引/外键/CHECK 数量）
 *  2) 数据与 02_seed_data.sql 的标准预期一致（验证过程的临时改动已还原）
 *  3) 无验证残留（临时库、测试成员）
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
console.log('  family_dining 数据库终态核验')
console.log('='.repeat(62))

const [[v]] = await c.query('SELECT VERSION() AS v')
console.log(`  MySQL ${v.v}  @  ${DB.host}:${DB.port}/${DB.database}\n`)

// ---------- 1) 结构 ----------
console.log('[1] 结构完整性')
const [[t]] = await c.query(`SELECT COUNT(*) n FROM information_schema.TABLES WHERE TABLE_SCHEMA=? AND TABLE_TYPE='BASE TABLE'`, [DB.database])
const [[vw]] = await c.query(`SELECT COUNT(*) n FROM information_schema.VIEWS WHERE TABLE_SCHEMA=?`, [DB.database])
const [[pr]] = await c.query(`SELECT COUNT(*) n FROM information_schema.ROUTINES WHERE ROUTINE_SCHEMA=? AND ROUTINE_TYPE='PROCEDURE'`, [DB.database])
const [[fk]] = await c.query(`SELECT COUNT(*) n FROM information_schema.TABLE_CONSTRAINTS WHERE TABLE_SCHEMA=? AND CONSTRAINT_TYPE='FOREIGN KEY'`, [DB.database])
const [[ck]] = await c.query(`SELECT COUNT(*) n FROM information_schema.TABLE_CONSTRAINTS WHERE TABLE_SCHEMA=? AND CONSTRAINT_TYPE='CHECK'`, [DB.database])
const [[ix]] = await c.query(`SELECT COUNT(DISTINCT INDEX_NAME) n FROM information_schema.STATISTICS WHERE TABLE_SCHEMA=?`, [DB.database])
const [[cs]] = await c.query(`SELECT DEFAULT_CHARACTER_SET_NAME cs, DEFAULT_COLLATION_NAME co FROM information_schema.SCHEMATA WHERE SCHEMA_NAME=?`, [DB.database])

ok(t.n === 11, `表 = ${t.n}/11`)
ok(vw.n === 5, `视图 = ${vw.n}/5`)
ok(pr.n === 2, `存储过程 = ${pr.n}/2`)
ok(fk.n > 0, `外键约束 = ${fk.n}`)
ok(ck.n === 15, `CHECK 约束 = ${ck.n}/15`)
ok(ix.n > 0, `索引 = ${ix.n}`)
ok(cs.cs === 'utf8mb4', `字符集 = ${cs.cs} / ${cs.co}`)

// ---------- 2) 数据 ----------
console.log('\n[2] 数据行数（与标准种子数据预期比对）')
const expect = {
  member: 5, recipe: 30, recipe_step: 67, ingredient: 77, recipe_ingredient: 224,
  tag: 10, recipe_tag: 81, member_favorite: 20, meal_order: 4, order_item: 18, shopping_item: 60
}
let dataOk = true
for (const [tab, exp] of Object.entries(expect)) {
  const [[r]] = await c.query(`SELECT COUNT(*) n FROM \`${tab}\``)
  const same = r.n === exp
  if (!same) dataOk = false
  console.log(`    ${tab.padEnd(20)} ${String(r.n).padStart(4)} / 预期 ${String(exp).padStart(4)}  ${same ? '✓' : '✗'}`)
}
ok(dataOk, '全部表行数与标准预期一致')

console.log('\n[3] 验证过程无残留')
const [[tmpDb]] = await c.query(`SELECT COUNT(*) n FROM information_schema.SCHEMATA WHERE SCHEMA_NAME='family_dining_verify_tmp'`)
ok(tmpDb.n === 0, '临时验证库已删除')
const [[ghost]] = await c.query(`SELECT COUNT(*) n FROM member WHERE nickname IN ('__chktest__','测试非法色')`)
ok(ghost.n === 0, '无测试成员残留')

// order_item.id=1 验证时改过 rating / portions，确认已还原
const [[it1]] = await c.query(`SELECT rating, portions, item_status FROM order_item WHERE id=1`)
ok(it1.rating === 5 && it1.portions === 2 && it1.item_status === 'done',
  `order_item#1 已还原（rating=${it1.rating}, portions=${it1.portions}, status=${it1.item_status}）`)

// ---------- 4) 业务链路 ----------
console.log('\n[4] 业务链路抽查')
const [[pork]] = await c.query(
  `SELECT total_quantity, unit FROM shopping_item WHERE order_id=1 AND name_snapshot='猪五花肉'`)
ok(pork && Math.abs(Number(pork.total_quantity) - 400) < 0.01,
  `份数换算：猪五花肉 = ${pork?.total_quantity}${pork?.unit}（红烧肉600g×2份÷3人份 = 400g）`)

const [[gen]] = await c.query(`SELECT prep_minutes, cook_minutes, total_minutes FROM recipe WHERE id=1`)
ok(gen.total_minutes === gen.prep_minutes + gen.cook_minutes,
  `生成列：total_minutes = ${gen.total_minutes} = ${gen.prep_minutes}+${gen.cook_minutes}`)

const [[prev]] = await c.query(`SELECT COUNT(*) n FROM v_order_shopping_preview WHERE order_id=1`)
ok(prev.n > 0, `采购清单视图可查询（订单1 = ${prev.n} 项）`)

const [[taste]] = await c.query(`SELECT COUNT(*) n FROM v_member_taste_stat`)
ok(taste.n > 0, `口味统计视图可查询（${taste.n} 行）`)

const [[rank]] = await c.query(`SELECT COUNT(*) n FROM v_category_stat WHERE rank_in_category=1`)
ok(rank.n > 0, `排行视图窗口函数正常（各分类第一名共 ${rank.n} 条）`)

// 全文索引（ngram）
const [[ft]] = await c.query(
  `SELECT COUNT(*) n FROM recipe WHERE MATCH(name, description) AGAINST ('红烧' IN BOOLEAN MODE)`)
ok(ft.n >= 1, `全文索引可检索（"红烧" 命中 ${ft.n} 条）`)

// 中文与 emoji 往返无损
const [[emo]] = await c.query(`SELECT cover_emoji, LENGTH(cover_emoji) len, CHAR_LENGTH(cover_emoji) clen FROM recipe WHERE id=1`)
ok(emo.cover_emoji === '🍖' && emo.clen === 1, `emoji 存取无损（${emo.cover_emoji}，字符数 ${emo.clen}）`)

const [[zh]] = await c.query(`SELECT name FROM recipe WHERE id=1`)
ok(zh.name === '红烧肉', `中文存取无损（${zh.name}）`)

console.log('\n' + '='.repeat(62))
console.log(fail === 0 ? '  ✓ 数据库状态标准、干净、可用' : `  ⚠ 有 ${fail} 项未通过，请检查上方输出`)
console.log('='.repeat(62))

await c.end()
process.exit(fail > 0 ? 1 : 0)
