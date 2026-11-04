/**
 * 一键建库建表 + 导入示例数据
 *
 * 用法：npm run apply
 * 环境变量可覆盖连接信息：DB_HOST / DB_PORT / DB_USER / DB_PASSWORD / DB_NAME
 */
import path from 'node:path'
import fs from 'node:fs'
import mysql from 'mysql2/promise'
import {
  DB, connect, runSqlFile, repoRoot, TABLES, VIEWS, PROCEDURES
} from './sql-runner.mjs'

const root = repoRoot()
const SCHEMA = path.join(root, '01_schema.sql')
const SEED = path.join(root, '02_seed_data.sql')
const PHASE1 = path.join(root, '03_phase1_family_hub.sql')

function mask(s) {
  return s ? '*'.repeat(Math.min(8, s.length)) : '(空)'
}

async function main() {
  console.log('='.repeat(64))
  console.log('  家庭点餐系统 · 数据库初始化')
  console.log('='.repeat(64))
  console.log(`目标：${DB.user}@${DB.host}:${DB.port}`)
  console.log(`密码：${mask(DB.password)}`)
  console.log(`数据库：${DB.database}`)
  console.log('')

  // 1) 先不指定库连接，确认服务器可达并读取版本
  let conn
  try {
    conn = await connect(false)
  } catch (e) {
    console.error(`[失败] 无法连接数据库：${e.code || ''} ${e.message}`)
    process.exit(1)
  }

  const [[ver]] = await conn.query('SELECT VERSION() AS v, @@character_set_server AS cs, @@collation_server AS co')
  console.log(`✓ 已连接  MySQL ${ver.v}`)
  console.log(`  服务器字符集：${ver.cs} / ${ver.co}`)

  const major = Number(String(ver.v).split('.')[0])
  const minor = Number(String(ver.v).split('.')[1])
  if (major < 8) {
    console.warn(`[警告] 脚本针对 MySQL 8.0 设计（用到 CHECK 约束、生成列、窗口函数），当前为 ${ver.v}，可能不兼容。`)
  } else if (major === 8 && minor === 0) {
    const patch = Number(String(ver.v).split('.')[2]) || 0
    if (patch < 16) {
      console.warn(`[警告] CHECK 约束需 8.0.16+，当前 ${ver.v}，约束可能被忽略。`)
    } else {
      console.log(`  ✓ 版本满足要求（CHECK 约束需 8.0.16+）`)
    }
  }
  console.log('')

  // 2) 执行建表脚本（脚本内含 CREATE DATABASE 与 USE）
  console.log('[1/4] 执行 01_schema.sql …')
  const r1 = await runSqlFile(conn, SCHEMA, { verbose: true })
  console.log(`  语句 ${r1.ok}/${r1.total} 成功`)
  if (r1.errors.length) {
    console.error(`  ✗ 有 ${r1.errors.length} 条语句失败：`)
    for (const e of r1.errors.slice(0, 10)) {
      console.error(`     [${e.index}] ${e.statement}\n        → ${e.code || e.errno}: ${e.message}`)
    }
  }
  console.log('')

  // 建表失败就不再继续，避免在残缺结构上导数据
  if (r1.errors.length) {
    console.error('[中止] 建表阶段存在错误，请先解决后再导入数据。')
    await conn.end()
    process.exit(1)
  }

  // 重新连接到目标库
  await conn.end()
  conn = await connect(true)

  // 3) 执行示例数据脚本
  console.log('[2/4] 执行 02_seed_data.sql …')
  const r2 = await runSqlFile(conn, SEED, { verbose: true })
  console.log(`  语句 ${r2.ok}/${r2.total} 成功`)
  if (r2.errors.length) {
    console.error(`  ✗ 有 ${r2.errors.length} 条语句失败：`)
    for (const e of r2.errors.slice(0, 10)) {
      console.error(`     [${e.index}] ${e.statement}\n        → ${e.code || e.errno}: ${e.message}`)
    }
    await conn.end()
    process.exit(1)
  }
  console.log('')

  // 3.5) 阶段 1 迁移（账本/快递/健康/同步/提醒/密码列，幂等）
  if (fs.existsSync(PHASE1)) {
    console.log('[3/4] 执行 03_phase1_family_hub.sql …')
    const r3 = await runSqlFile(conn, PHASE1, { verbose: true })
    console.log(`  语句 ${r3.ok}/${r3.total} 成功`)
    if (r3.errors.length) {
      console.error(`  ✗ 有 ${r3.errors.length} 条语句失败：`)
      for (const e of r3.errors.slice(0, 10)) {
        console.error(`     [${e.index}] ${e.statement}\n        → ${e.code || e.errno}: ${e.message}`)
      }
      await conn.end()
      process.exit(1)
    }
    console.log('')
  } else {
    console.log('[3/4] 未找到 03_phase1_family_hub.sql，跳过阶段 1 迁移')
    console.log('')
  }

  // 4) 核验结构与实际行数
  console.log('[4/4] 核验结果 …')
  const [[tbl]] = await conn.query(
    "SELECT COUNT(*) AS n FROM information_schema.TABLES WHERE TABLE_SCHEMA=? AND TABLE_TYPE='BASE TABLE'",
    [DB.database]
  )
  const [[vw]] = await conn.query(
    "SELECT COUNT(*) AS n FROM information_schema.VIEWS WHERE TABLE_SCHEMA=?",
    [DB.database]
  )
  const [[pr]] = await conn.query(
    "SELECT COUNT(*) AS n FROM information_schema.ROUTINES WHERE ROUTINE_SCHEMA=? AND ROUTINE_TYPE='PROCEDURE'",
    [DB.database]
  )

  console.log(`  表：${tbl.n}/17   视图：${vw.n}/5   存储过程：${pr.n}/2`)

  console.log('\n  各表行数（以数据库实际统计为准）：')
  const counts = {}
  for (const t of TABLES) {
    const [[row]] = await conn.query(`SELECT COUNT(*) AS n FROM \`${t}\``)
    counts[t] = row.n
    console.log(`    ${t.padEnd(20)} ${String(row.n).padStart(5)}`)
  }

  // 关键业务校验：采购清单是否被存储过程正确生成
  const [[shop]] = await conn.query('SELECT COUNT(*) AS n FROM shopping_item')
  console.log(`\n  采购清单项：${shop.n}（由存储过程在导入时生成）`)

  const [[preview]] = await conn.query(
    'SELECT COUNT(*) AS n FROM v_order_shopping_preview WHERE order_id = 1'
  )
  console.log(`  订单1 视图预览项：${preview.n}`)

  // 抽查一条换算：红烧肉×2（3人份，五花肉600g）应为 400g
  const [[pork]] = await conn.query(
    `SELECT total_quantity, unit, recipe_names FROM shopping_item
      WHERE order_id = 1 AND name_snapshot = '五花肉'`
  )
  if (pork) {
    const expected = 400
    const actual = Number(pork.total_quantity)
    const good = Math.abs(actual - expected) < 0.01
    console.log(`  换算抽查：五花肉 = ${actual}${pork.unit}（期望 ${expected}）${good ? ' ✓' : ' ✗'}`)
  }

  // 生成列校验
  const [[gen]] = await conn.query(
    "SELECT total_minutes FROM recipe WHERE id = 1"
  )
  console.log(`  生成列抽查：红烧肉 total_minutes = ${gen?.total_minutes}（期望 75 = 20+55）`)

  // CHECK 约束是否真的生效（MySQL 8.0.16+）
  const [[chk]] = await conn.query(
    `SELECT COUNT(*) AS n FROM information_schema.TABLE_CONSTRAINTS
      WHERE TABLE_SCHEMA=? AND CONSTRAINT_TYPE='CHECK'`,
    [DB.database]
  )
  console.log(`  CHECK 约束数量：${chk.n}`)

  console.log('\n' + '='.repeat(64))
  const allGood = tbl.n === 17 && vw.n === 5 && pr.n === 2 && counts.recipe === 30
  console.log(allGood ? '  ✓ 数据库初始化完成' : '  ⚠ 初始化完成但数量与预期不符，请检查上方输出')
  console.log('='.repeat(64))

  await conn.end()
  process.exit(allGood ? 0 : 2)
}

main().catch((e) => {
  console.error('未预期错误：', e.message)
  process.exit(1)
})
