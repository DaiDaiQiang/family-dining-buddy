/**
 * 解析器自检（不连数据库）：验证 splitStatements 能正确切分脚本，
 * 尤其是 DELIMITER 包裹的存储过程必须作为完整一条语句。
 */
import fs from 'node:fs'
import path from 'node:path'
import { splitStatements, repoRoot } from './sql-runner.mjs'

const root = repoRoot()
const schema = path.join(root, '01_schema.sql')
const seed = path.join(root, '02_seed_data.sql')

let pass = 0, fail = 0
const ok = (c, m) => { c ? (pass++, console.log('  ✓', m)) : (fail++, console.error('  ✗', m)) }

console.log('=== 解析 01_schema.sql ===')
const s1 = splitStatements(fs.readFileSync(schema, 'utf8'))
console.log(`  切分出 ${s1.length} 条语句`)

const creates = s1.filter((s) => /^CREATE TABLE/i.test(s))
ok(creates.length === 11, `CREATE TABLE 数量 = ${creates.length}（期望 11）`)

const views = s1.filter((s) => /^CREATE OR REPLACE VIEW/i.test(s))
ok(views.length === 5, `CREATE VIEW 数量 = ${views.length}（期望 5）`)

const procs = s1.filter((s) => /^CREATE PROCEDURE/i.test(s))
ok(procs.length === 2, `CREATE PROCEDURE 数量 = ${procs.length}（期望 2）`)

// 存储过程必须完整包含 BEGIN...END，不能被内部的分号切断
for (const p of procs) {
  const name = (p.match(/PROCEDURE\s+`?(\w+)`?/i) || [])[1] || '?'
  const hasBegin = /\bBEGIN\b/i.test(p)
  const hasEnd = /\bEND\b\s*$/i.test(p.trim())
  ok(hasBegin && hasEnd, `存储过程 ${name} 完整（含 BEGIN…END）`)
  ok(!/^DELIMITER/i.test(p), `存储过程 ${name} 未混入 DELIMITER 指令`)
}

// DROP 语句也应被正确识别
const drops = s1.filter((s) => /^DROP (TABLE|VIEW|PROCEDURE)/i.test(s))
ok(drops.length >= 16, `DROP 语句数量 = ${drops.length}（期望 ≥16）`)

// 不应有残留的 DELIMITER 行
ok(!s1.some((s) => /DELIMITER/i.test(s)), '所有语句中均无残留 DELIMITER')

console.log('\n=== 解析 02_seed_data.sql ===')
const s2 = splitStatements(fs.readFileSync(seed, 'utf8'))
console.log(`  切分出 ${s2.length} 条语句`)

const inserts = s2.filter((s) => /^INSERT INTO/i.test(s))
ok(inserts.length >= 9, `INSERT 数量 = ${inserts.length}（期望 ≥9）`)

const calls = s2.filter((s) => /^CALL /i.test(s))
ok(calls.length === 3, `CALL 存储过程数量 = ${calls.length}（期望 3）`)

const recipeInsert = inserts.find((s) => /INSERT INTO `recipe`\s*\n?\s*\(/.test(s) || /INSERT INTO `recipe` /i.test(s))
ok(!!recipeInsert, '找到菜谱 INSERT 语句')
if (recipeInsert) {
  // 30 道菜：统计行数以 "(数字," 开头的粗略校验
  const rowCount = (recipeInsert.match(/\n\(\d+\s*,/g) || []).length
  ok(rowCount === 30, `菜谱数据行数 = ${rowCount}（期望 30）`)
}

const ingInsert = inserts.find((s) => /INSERT INTO `ingredient`/i.test(s))
if (ingInsert) {
  const rowCount = (ingInsert.match(/\n\(\d+\s*,/g) || []).length
  ok(rowCount === 88, `食材数据行数 = ${rowCount}（期望 88）`)
}

console.log(`\n========== 解析自检：${pass} 通过 / ${fail} 失败 ==========`)
process.exit(fail > 0 ? 1 : 0)
