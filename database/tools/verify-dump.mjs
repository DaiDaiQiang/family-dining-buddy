/**
 * 备份文件验证
 *
 * 1) 结构校验：用项目自带的 SQL 解析器解析备份，确认能被正确切分执行
 *    （尤其 DELIMITER 包裹的存储过程必须是完整一条语句）
 * 2) 回环校验：把备份导入一个临时库，与源库逐表比对行数与内容校验和，
 *    确认数据无损；验证完删除临时库。
 *
 * 用法：npm run verify-dump
 */
import fs from 'node:fs'
import path from 'node:path'
import mysql from 'mysql2/promise'
import { DB, splitStatements, repoRoot } from './sql-runner.mjs'

const root = repoRoot()
const LATEST = path.join(root, 'backup', 'family_dining_backup_latest.sql')
const TMP_DB = 'family_dining_verify_tmp'

let pass = 0, fail = 0
const ok = (c, m) => { c ? (pass++, console.log('  ✓', m)) : (fail++, console.error('  ✗', m)) }

// ---------- 1) 结构校验（离线） ----------
console.log('=== 1) 备份文件结构校验 ===')
const raw = fs.readFileSync(LATEST, 'utf8')
const stmts = splitStatements(raw)
console.log(`  文件大小 ${(Buffer.byteLength(raw, 'utf8') / 1024).toFixed(1)} KB，解析出 ${stmts.length} 条语句`)

const creates = stmts.filter((s) => /^CREATE TABLE/i.test(s))
ok(creates.length === 11, `CREATE TABLE = ${creates.length}（期望 11）`)

const inserts = stmts.filter((s) => /^INSERT INTO/i.test(s))
ok(inserts.length > 0, `INSERT 语句 = ${inserts.length}`)

const views = stmts.filter((s) => /^CREATE\s+(?:ALGORITHM=\S+\s+)?(?:DEFINER=\S+\s+)?(?:SQL SECURITY \S+\s+)?VIEW/i.test(s))
ok(views.length === 5, `CREATE VIEW = ${views.length}（期望 5）`)

const procs = stmts.filter((s) => /^CREATE\s+(?:DEFINER=\S+\s+)?PROCEDURE/i.test(s))
ok(procs.length === 2, `CREATE PROCEDURE = ${procs.length}（期望 2）`)
for (const p of procs) {
  const name = (p.match(/PROCEDURE\s+`?(\w+)`?/i) || [])[1] || '?'
  const body = p.trim().replace(/;\s*$/, '')
  ok(/\bBEGIN\b/i.test(body) && /\bEND\b\s*$/i.test(body), `存储过程 ${name} 完整（BEGIN…END）`)
}
ok(!stmts.some((s) => /^\s*DELIMITER/i.test(s)), '无残留 DELIMITER 指令')

// 中文与 emoji 是否完好
ok(raw.includes('红烧肉') && raw.includes('🍖'), '中文与 emoji 编码完好')
ok(!raw.includes('â–') && !raw.includes('ï¿½'), '无乱码特征')

// ---------- 2) 回环校验（导入临时库比对） ----------
console.log('\n=== 2) 回环导入校验 ===')

let conn = await mysql.createConnection({
  host: DB.host, port: DB.port, user: DB.user, password: DB.password,
  charset: 'utf8mb4', connectTimeout: 15000, multipleStatements: false
})

try {
  // 清理可能残留的临时库
  await conn.query(`DROP DATABASE IF EXISTS \`${TMP_DB}\``)
  console.log(`  创建临时库 ${TMP_DB}`)

  // 把备份里的目标库名替换成临时库名后逐条执行
  const patched = raw
    .replace(/`family_dining`/g, `\`${TMP_DB}\``)
    .replace(/^USE `family_dining`;/gim, `USE \`${TMP_DB}\`;`)

  const patchedStmts = splitStatements(patched)
  let done = 0
  const errs = []
  for (const s of patchedStmts) {
    try { await conn.query(s); done++ }
    catch (e) { errs.push({ s: s.replace(/\s+/g, ' ').slice(0, 70), msg: `${e.code || e.errno}: ${e.message}` }) }
  }
  ok(errs.length === 0, `临时库导入 ${done}/${patchedStmts.length} 条语句成功${errs.length ? `（失败 ${errs.length}）` : ''}`)
  if (errs.length) {
    errs.slice(0, 5).forEach((e) => console.error(`     ✗ ${e.s}\n        → ${e.msg}`))
  }

  // 切到临时库比对
  await conn.query(`USE \`${TMP_DB}\``)

  const tables = ['member', 'recipe', 'recipe_step', 'ingredient', 'recipe_ingredient',
    'tag', 'recipe_tag', 'member_favorite', 'meal_order', 'order_item', 'shopping_item']

  // MySQL 不支持 CONCAT_WS('|', tbl.*)，需显式列出列名；
  // 生成列可以参与读取校验（只是不能出现在 INSERT 中）。
  async function checksum(schema, table) {
    const [cols] = await conn.query(
      `SELECT COLUMN_NAME FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA=? AND TABLE_NAME=? ORDER BY ORDINAL_POSITION`,
      [schema, table]
    )
    const list = cols.map((c) => `IFNULL(\`${c.COLUMN_NAME}\`,'∅')`).join(',')
    const [[r]] = await conn.query(
      `SELECT COUNT(*) AS n, COALESCE(SUM(CRC32(CONCAT_WS('|', ${list}))),0) AS chk
         FROM \`${schema}\`.\`${table}\``
    )
    return { n: r.n, chk: String(r.chk) }
  }

  console.log('\n  逐表比对（源库 vs 备份还原库）：')
  let allSame = true
  for (const t of tables) {
    const a = await checksum(DB.database, t)
    const b = await checksum(TMP_DB, t)
    const same = a.n === b.n && a.chk === b.chk
    if (!same) allSame = false
    console.log(
      `    ${t.padEnd(20)} 源 ${String(a.n).padStart(4)} 行 / chk ${a.chk.padStart(10)}` +
      `   vs   还原 ${String(b.n).padStart(4)} 行 / chk ${b.chk.padStart(10)}` +
      `   ${same ? '✓ 一致' : '✗ 不一致!'}`
    )
  }
  ok(allSame, '全部表行数与内容校验和一致（数据无损）')

  // 视图与存储过程是否成功还原
  const [[vc]] = await conn.query(
    `SELECT COUNT(*) AS n FROM information_schema.VIEWS WHERE TABLE_SCHEMA=?`, [TMP_DB])
  ok(vc.n === 5, `还原库视图数 = ${vc.n}（期望 5）`)
  const [[pc]] = await conn.query(
    `SELECT COUNT(*) AS n FROM information_schema.ROUTINES WHERE ROUTINE_SCHEMA=? AND ROUTINE_TYPE='PROCEDURE'`, [TMP_DB])
  ok(pc.n === 2, `还原库存储过程数 = ${pc.n}（期望 2）`)

  // 还原库能否正常执行业务查询（视图 + 存储过程可用）
  const [[pv]] = await conn.query(
    `SELECT COUNT(*) AS n FROM \`${TMP_DB}\`.v_order_shopping_preview WHERE order_id=1`)
  ok(pv.n > 0, `还原库视图可查询（订单1 预览 ${pv.n} 项）`)

  await conn.query(`CALL \`${TMP_DB}\`.sp_generate_shopping_list(4)`)
  const [[sc]] = await conn.query(
    `SELECT COUNT(*) AS n FROM \`${TMP_DB}\`.shopping_item WHERE order_id=4`)
  ok(sc.n > 0, `还原库存储过程可调用（为订单4 生成 ${sc.n} 项采购清单）`)

  // CHECK 约束在还原库同样生效
  const [[ck]] = await conn.query(
    `SELECT COUNT(*) AS n FROM information_schema.TABLE_CONSTRAINTS
      WHERE TABLE_SCHEMA=? AND CONSTRAINT_TYPE='CHECK'`, [TMP_DB])
  ok(ck.n === 15, `还原库 CHECK 约束数 = ${ck.n}（期望 15）`)

  // 约束真的拦得住非法值
  // 注意：theme_color 是 CHAR(7)，用 11 字符的 'not-a-color' 会先触发
  // 数据过长错误（errno 1406）而非 CHECK，因此必须用「长度合规但格式非法」的值，
  // 才能精准验证 CHECK 约束本身（errno 3819 = ER_CHECK_CONSTRAINT_VIOLATED）。
  let blocked = false
  try {
    await conn.query(`INSERT INTO \`${TMP_DB}\`.member (nickname, avatar_emoji, theme_color, role)
                      VALUES ('测试非法色', '🙂', '#GGGGGG', 'member')`)
  } catch (e) { blocked = e.errno === 3819 || /CHECK/i.test(e.message) }
  ok(blocked, 'CHECK 约束拦截非法主题色 #GGGGGG（errno 3819）')

  let enumBlocked = false
  try {
    await conn.query(`UPDATE \`${TMP_DB}\`.member SET role = 'ghost' WHERE id = 1`)
  } catch (e) { enumBlocked = true }
  ok(enumBlocked, 'role 枚举约束拦截非法角色')

} finally {
  // 无论成败都清理临时库
  try {
    const c2 = await mysql.createConnection({
      host: DB.host, port: DB.port, user: DB.user, password: DB.password, charset: 'utf8mb4'
    })
    await c2.query(`DROP DATABASE IF EXISTS \`${TMP_DB}\``)
    await c2.end()
    console.log(`\n  已清理临时库 ${TMP_DB}`)
  } catch (e) {
    console.error(`\n  ⚠ 临时库清理失败，请手动执行 DROP DATABASE ${TMP_DB}：${e.message}`)
  }
  await conn.end()
}

console.log(`\n========== 备份验证：${pass} 通过 / ${fail} 失败 ==========`)
process.exit(fail > 0 ? 1 : 0)
