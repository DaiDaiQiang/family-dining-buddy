/**
 * SQL 脚本导入器
 *
 * mysql2 驱动不认识 mysql 客户端的 DELIMITER 指令，
 * 这里自行解析：按 DELIMITER 切换分隔符，再切分并逐条执行，
 * 因此 01_schema.sql 里的存储过程可以正确创建。
 */
import fs from 'node:fs'
import path from 'node:path'
import mysql from 'mysql2/promise'

export const DB = {
  host: process.env.DB_HOST || '169.254.104.35',
  port: Number(process.env.DB_PORT || 3306),
  user: process.env.DB_USER || 'root',
  password: process.env.DB_PASSWORD || 'root',
  database: process.env.DB_NAME || 'family_dining'
}

export async function connect(withDb = true) {
  return mysql.createConnection({
    host: DB.host,
    port: DB.port,
    user: DB.user,
    password: DB.password,
    ...(withDb ? { database: DB.database } : {}),
    multipleStatements: false,
    charset: 'utf8mb4',
    // 建表/导数据可能较慢
    connectTimeout: 15000
  })
}

/** 去掉 SQL 注释（保留字符串字面量内的内容不被误伤，这里脚本无此场景） */
function stripComments(sql) {
  const out = []
  let i = 0
  const n = sql.length
  let inSingle = false, inDouble = false, inBack = false, inLine = false, inBlock = false

  while (i < n) {
    const c = sql[i]
    const c2 = sql.slice(i, i + 2)

    if (inLine) {
      if (c === '\n') { inLine = false; out.push(c) }
      i++
      continue
    }
    if (inBlock) {
      if (c2 === '*/') { inBlock = false; i += 2 } else { i++ }
      continue
    }
    if (!inSingle && !inDouble && !inBack) {
      if (c2 === '--' && (sql[i + 2] === ' ' || sql[i + 2] === '\n' || sql[i + 2] === '\t')) { inLine = true; i += 2; continue }
      if (c === '#') { inLine = true; i++; continue }
      if (c2 === '/*') { inBlock = true; i += 2; continue }
    }
    if (c === "'" && !inDouble && !inBack) {
      if (inSingle && sql[i + 1] === "'") { out.push("''"); i += 2; continue }
      inSingle = !inSingle
    } else if (c === '"' && !inSingle && !inBack) {
      if (inDouble && sql[i + 1] === '"') { out.push('""'); i += 2; continue }
      inDouble = !inDouble
    } else if (c === '`' && !inSingle && !inDouble) {
      inBack = !inBack
    }
    out.push(c)
    i++
  }
  return out.join('')
}

/**
 * 把脚本切成可执行语句数组，正确处理 DELIMITER 切换。
 */
export function splitStatements(rawSql) {
  const sql = stripComments(rawSql)
  const statements = []
  let delimiter = ';'
  let buf = ''
  const lines = sql.split(/\r?\n/)

  for (const line of lines) {
    const m = line.trim().match(/^DELIMITER\s+(\S+)\s*$/i)
    if (m) {
      // 切换前把缓冲里的内容作为一条语句收掉（DELIMITER 前的语句通常已以旧分隔符结束）
      const pending = buf.trim()
      if (pending) statements.push(pending)
      buf = ''
      delimiter = m[1]
      continue
    }

    buf += line + '\n'

    // 当前缓冲是否以分隔符结尾（忽略行尾空白）
    const trimmed = buf.trimEnd()
    if (trimmed.endsWith(delimiter)) {
      const stmt = trimmed.slice(0, trimmed.length - delimiter.length).trim()
      if (stmt) statements.push(stmt)
      buf = ''
    }
  }

  const tail = buf.trim()
  if (tail) statements.push(tail)

  // 过滤掉纯 USE / SET NAMES 之类也照常执行，只丢弃空串
  return statements.filter((s) => s.length > 0)
}

/** 执行一个 SQL 文件，返回执行统计 */
export async function runSqlFile(conn, filePath, { verbose = true } = {}) {
  const raw = fs.readFileSync(filePath, 'utf8')
  const stmts = splitStatements(raw)
  let ok = 0
  const errors = []

  for (const [idx, stmt] of stmts.entries()) {
    const brief = stmt.replace(/\s+/g, ' ').slice(0, 78)
    try {
      await conn.query(stmt)
      ok++
      if (verbose && /CREATE (TABLE|VIEW|PROCEDURE|DATABASE)/i.test(stmt)) {
        console.log(`  ✓ ${brief}…`)
      }
    } catch (e) {
      errors.push({ index: idx + 1, statement: brief, code: e.code, errno: e.errno, message: e.message })
      if (verbose) console.error(`  ✗ [${idx + 1}] ${brief}…\n     → ${e.code || e.errno}: ${e.message}`)
    }
  }
  return { total: stmts.length, ok, errors }
}

/** 统计信息用的表名清单（与 01_schema.sql + 03_phase1_family_hub.sql 保持一致） */
export const TABLES = [
  'member', 'recipe', 'recipe_step', 'ingredient', 'recipe_ingredient',
  'tag', 'recipe_tag', 'member_favorite', 'meal_order', 'order_item', 'shopping_item',
  'ledger_entry', 'parcel', 'health_profile', 'member_data', 'app_setting', 'reminder'
]

export const VIEWS = [
  'v_recipe_full', 'v_order_detail', 'v_order_shopping_preview',
  'v_member_taste_stat', 'v_category_stat'
]

export const PROCEDURES = ['sp_generate_shopping_list', 'sp_confirm_order']

export function repoRoot() {
  return path.resolve(path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1')), '..')
}
