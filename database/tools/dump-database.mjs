/**
 * 标准 SQL 备份导出器
 *
 * 输出格式对齐 mysqldump 惯例，可用 mysql 客户端或任意 GUI 工具直接导入：
 *   - 头部会话设置（与 mysqldump 一致）
 *   - CREATE DATABASE / USE
 *   - 每张表：DROP TABLE IF EXISTS + 真实 SHOW CREATE TABLE + 批量 INSERT
 *   - 每个视图：DROP VIEW IF EXISTS + SHOW CREATE VIEW
 *   - 每个存储过程：DELIMITER $$ 包裹的 SHOW CREATE PROCEDURE
 *
 * 用法：npm run dump
 * 输出：../backup/family_dining_backup_<时间戳>.sql 及同名 latest 副本
 */
import fs from 'node:fs'
import path from 'node:path'
import mysql from 'mysql2/promise'
import { DB, repoRoot, TABLES } from './sql-runner.mjs'

const root = repoRoot()
const BACKUP_DIR = path.join(root, 'backup')

/** SQL 字符串字面量转义 */
function esc(v) {
  if (v === null || v === undefined) return 'NULL'
  if (v instanceof Date) {
    const p = (n, w = 2) => String(n).padStart(w, '0')
    return `'${v.getFullYear()}-${p(v.getMonth() + 1)}-${p(v.getDate())} ${p(v.getHours())}:${p(v.getMinutes())}:${p(v.getSeconds())}'`
  }
  if (typeof v === 'number') return Number.isFinite(v) ? String(v) : 'NULL'
  if (typeof v === 'bigint') return v.toString()
  if (Buffer.isBuffer(v)) return `0x${v.toString('hex')}`
  return "'" + String(v)
    .replace(/\\/g, '\\\\')
    .replace(/'/g, "''")
    .replace(/\r/g, '\\r')
    .replace(/\n/g, '\\n')
    .replace(/\u0000/g, '\\0')
    .replace(/\u001a/g, '\\Z') + "'"
}

/** 把一行按列顺序转成 (v1,v2,...) */
function rowToValues(row, cols) {
  return '(' + cols.map((c) => esc(row[c])).join(',') + ')'
}

async function main() {
  fs.mkdirSync(BACKUP_DIR, { recursive: true })

  const conn = await mysql.createConnection({
    host: DB.host, port: DB.port, user: DB.user, password: DB.password,
    database: DB.database, charset: 'utf8mb4', connectTimeout: 15000,
    // 让 DECIMAL 以字符串返回，避免精度丢失
    decimalNumbers: false,
    dateStrings: ['DATE', 'DATETIME', 'TIMESTAMP']
  })

  const [[meta]] = await conn.query('SELECT VERSION() AS v, NOW() AS t')
  const stamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
  const outFile = path.join(BACKUP_DIR, `family_dining_backup_${stamp}.sql`)
  const latestFile = path.join(BACKUP_DIR, 'family_dining_backup_latest.sql')

  const out = []
  const push = (s = '') => out.push(s)

  // ---------- 头部 ----------
  push('-- MySQL dump (family-dining 标准备份)')
  push(`-- 源服务器版本 : ${meta.v}`)
  push(`-- 导出时间     : ${meta.t}`)
  push(`-- 数据库       : ${DB.database}`)
  push(`-- 源实例       : ${DB.host}:${DB.port}`)
  push('--')
  push('-- 导入方式：')
  push('--   mysql -u root -p < family_dining_backup_latest.sql')
  push('-- 本文件包含建库、建表、视图、存储过程与全部数据，可重复导入（先 DROP 再建）。')
  push('')
  push('/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;')
  push('/*!40101 SET @OLD_CHARACTER_SET_RESULTS=@@CHARACTER_SET_RESULTS */;')
  push('/*!40101 SET @OLD_COLLATION_CONNECTION=@@COLLATION_CONNECTION */;')
  push('/*!50503 SET NAMES utf8mb4 */;')
  push('/*!40103 SET @OLD_TIME_ZONE=@@TIME_ZONE */;')
  push("/*!40103 SET TIME_ZONE='+00:00' */;")
  push('/*!40014 SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0 */;')
  push('/*!40014 SET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0 */;')
  push('/*!40101 SET @OLD_SQL_MODE=@@SQL_MODE, SQL_MODE=\'NO_AUTO_VALUE_ON_ZERO\' */;')
  push('/*!40111 SET @OLD_SQL_NOTES=@@SQL_NOTES, SQL_NOTES=0 */;')
  push('')
  push(`CREATE DATABASE IF NOT EXISTS \`${DB.database}\` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;`)
  push(`USE \`${DB.database}\`;`)
  push('')

  // ---------- 表结构 + 数据 ----------
  const [tblRows] = await conn.query(
    `SELECT TABLE_NAME FROM information_schema.TABLES
      WHERE TABLE_SCHEMA=? AND TABLE_TYPE='BASE TABLE' ORDER BY TABLE_NAME`,
    [DB.database]
  )
  // 按依赖顺序导出：先父表后子表（与 TABLES 常量顺序一致），其余按字母序补在后面
  const dbTables = tblRows.map((r) => r.TABLE_NAME)
  const ordered = [
    ...TABLES.filter((t) => dbTables.includes(t)),
    ...dbTables.filter((t) => !TABLES.includes(t))
  ]

  const stats = []
  for (const t of ordered) {
    push(`-- ----------------------------`)
    push(`-- Table structure for \`${t}\``)
    push(`-- ----------------------------`)
    push(`DROP TABLE IF EXISTS \`${t}\`;`)

    const [[ddl]] = await conn.query(`SHOW CREATE TABLE \`${t}\``)
    const createSql = ddl['Create Table']
    push(createSql.endsWith(';') ? createSql : createSql + ';')
    push('')

    const [rows] = await conn.query(`SELECT * FROM \`${t}\``)
    if (rows.length === 0) {
      push(`-- (空表，无数据)`)
      push('')
      stats.push({ table: t, rows: 0 })
      continue
    }

    // 生成列（GENERATED ALWAYS AS）不能出现在 INSERT 中，
    // 否则导入时报 ER_NON_DEFAULT_VALUE_FOR_GENERATED_COLUMN，导致整份备份还原失败。
    // 与 mysqldump 行为一致：只导出真实存储的列。
    //
    // 判定必须用 GENERATION_EXPRESSION 非空，而不是 EXTRA LIKE '%GENERATED%'：
    // 后者会把 `DEFAULT CURRENT_TIMESTAMP`（EXTRA=DEFAULT_GENERATED）
    // 这类普通列误判为生成列，导致 created_at/updated_at 数据丢失。
    const [genRows] = await conn.query(
      `SELECT COLUMN_NAME FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA=? AND TABLE_NAME=?
          AND GENERATION_EXPRESSION IS NOT NULL
          AND GENERATION_EXPRESSION <> ''`,
      [DB.database, t]
    )
    const generated = new Set(genRows.map((g) => g.COLUMN_NAME))
    const cols = Object.keys(rows[0]).filter((c) => !generated.has(c))
    if (generated.size) {
      push(`-- 注：\`${t}\` 的生成列 ${[...generated].map((c) => `\`${c}\``).join(', ')} 由数据库自动计算，未在 INSERT 中导出`)
    }

    push(`-- Dumping data for table \`${t}\`  (${rows.length} rows)`)
    push(`LOCK TABLES \`${t}\` WRITE;`)

    // 分批 INSERT，单条语句不超过 ~200 行，避免超长
    const BATCH = 100
    const colList = cols.map((c) => `\`${c}\``).join(',')
    for (let i = 0; i < rows.length; i += BATCH) {
      const slice = rows.slice(i, i + BATCH)
      push(`INSERT INTO \`${t}\` (${colList}) VALUES`)
      push(slice.map((r) => rowToValues(r, cols)).join(',\n') + ';')
    }

    push('UNLOCK TABLES;')
    push('')
    stats.push({ table: t, rows: rows.length })
  }

  // ---------- 视图 ----------
  const [vwRows] = await conn.query(
    `SELECT TABLE_NAME FROM information_schema.VIEWS WHERE TABLE_SCHEMA=? ORDER BY TABLE_NAME`,
    [DB.database]
  )
  if (vwRows.length) {
    push(`-- ----------------------------`)
    push(`-- View structures (${vwRows.length})`)
    push(`-- ----------------------------`)
    for (const v of vwRows) {
      const name = v.TABLE_NAME
      push(`DROP VIEW IF EXISTS \`${name}\`;`)
      const [[vddl]] = await conn.query(`SHOW CREATE VIEW \`${name}\``)
      const sql = vddl['Create View']
      push(sql.endsWith(';') ? sql : sql + ';')
      push('')
    }
  }

  // ---------- 存储过程 ----------
  const [prRows] = await conn.query(
    `SELECT ROUTINE_NAME FROM information_schema.ROUTINES
      WHERE ROUTINE_SCHEMA=? AND ROUTINE_TYPE='PROCEDURE' ORDER BY ROUTINE_NAME`,
    [DB.database]
  )
  if (prRows.length) {
    push(`-- ----------------------------`)
    push(`-- Stored procedures (${prRows.length})`)
    push(`-- ----------------------------`)
    push('DELIMITER $$')
    push('')
    for (const p of prRows) {
      const name = p.ROUTINE_NAME
      push(`DROP PROCEDURE IF EXISTS \`${name}\`$$`)
      const [[pddl]] = await conn.query(`SHOW CREATE PROCEDURE \`${name}\``)
      const sql = pddl['Create Procedure']
      push((sql.endsWith(';') ? sql : sql + ';') + '$$')
      push('')
    }
    push('DELIMITER ;')
    push('')
  }

  // ---------- 收尾 ----------
  push('/*!40103 SET TIME_ZONE=@OLD_TIME_ZONE */;')
  push('/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;')
  push('/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;')
  push('/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;')
  push('/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;')
  push('/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;')
  push('/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;')
  push('/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;')
  push('')
  push(`-- Dump completed on ${new Date().toISOString()}`)

  const content = out.join('\n')
  fs.writeFileSync(outFile, content, 'utf8')
  fs.writeFileSync(latestFile, content, 'utf8')

  await conn.end()

  console.log('='.repeat(64))
  console.log('  标准备份已生成')
  console.log('='.repeat(64))
  console.log(`  时间戳版本：${outFile}`)
  console.log(`  最新副本  ：${latestFile}`)
  const kb = (Buffer.byteLength(content, 'utf8') / 1024).toFixed(1)
  console.log(`  文件大小  ：${kb} KB   语句行数：${out.length}`)
  console.log('')
  console.log('  导出内容：')
  const totalRows = stats.reduce((s, x) => s + x.rows, 0)
  for (const s of stats) {
    console.log(`    ${s.table.padEnd(22)} ${String(s.rows).padStart(5)} 行`)
  }
  console.log(`    ${'合计'.padEnd(20)} ${String(totalRows).padStart(5)} 行`)
  console.log(`    视图 ${vwRows.length} 个，存储过程 ${prRows.length} 个`)
  console.log('='.repeat(64))

  return { outFile, latestFile, stats, totalRows }
}

main().catch((e) => { console.error('备份失败：', e.message); process.exit(1) })
