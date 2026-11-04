/**
 * 阶段迁移执行器：按序运行 database/0*.sql（幂等，可重复执行）
 *
 * 用法：npm run migrate
 * 环境变量可覆盖连接信息：DB_HOST / DB_PORT / DB_USER / DB_PASSWORD / DB_NAME
 */
import path from 'node:path'
import fs from 'node:fs'
import { DB, connect, runSqlFile, repoRoot } from './sql-runner.mjs'

/** 按阶段顺序排列的迁移脚本（全部幂等） */
const MIGRATIONS = [
  { label: '阶段 1 · 家庭管理底座', file: '03_phase1_family_hub.sql' },
  { label: '阶段 2 · 家庭档案扩展', file: '04_phase2_profile_reminder.sql' },
  { label: '阶段 5 · 成员头像图片', file: '05_member_avatar_url.sql' },
  { label: '系统版本 · 更新记录', file: '06_system_version.sql' }
]

async function main() {
  console.log('='.repeat(64))
  console.log('  家庭管理系统 · 阶段迁移')
  console.log('='.repeat(64))
  console.log(`目标：${DB.user}@${DB.host}:${DB.port}/${DB.database}`)
  console.log('')

  let conn
  try {
    conn = await connect(true)
  } catch (e) {
    console.error(`[失败] 无法连接数据库：${e.code || ''} ${e.message}`)
    process.exit(1)
  }

  for (const m of MIGRATIONS) {
    const file = path.join(repoRoot(), m.file)
    if (!fs.existsSync(file)) {
      console.error(`[失败] 找不到迁移脚本：${file}`)
      await conn.end()
      process.exit(1)
    }
    console.log('-'.repeat(64))
    console.log(`  ${m.label}`)
    console.log(`  脚本：${file}`)
    console.log('')
    const result = await runSqlFile(conn, file, { verbose: true })
    console.log(`语句 ${result.ok}/${result.total} 成功`)
    if (result.errors.length) {
      console.error(`✗ 有 ${result.errors.length} 条语句失败：`)
      for (const e of result.errors.slice(0, 10)) {
        console.error(`   [${e.index}] ${e.statement}\n      → ${e.code || e.errno}: ${e.message}`)
      }
      await conn.end()
      process.exit(1)
    }
    console.log('')
  }

  // 核验对象
  const checks = [
    ["member.password_hash", 'password_hash'],
    ["member.birthday", 'birthday'],
    ["member.gender", 'gender'],
    ["member.dislikes", 'dislikes'],
    ["member.avatar_url", 'avatar_url'],
  ]
  console.log('-'.repeat(64))
  for (const [label, col] of checks) {
    const [[row]] = await conn.query(
      `SELECT COUNT(*) AS n FROM information_schema.COLUMNS
       WHERE TABLE_SCHEMA=? AND TABLE_NAME='member' AND COLUMN_NAME=?`,
      [DB.database, col]
    )
    console.log(`  ${label.padEnd(24)} ${row.n > 0 ? '✓ 已就绪' : '✗ 缺失'}`)
  }
  for (const t of ['ledger_entry', 'parcel', 'health_profile', 'member_data', 'app_setting', 'reminder', 'system_version']) {
    const [[row]] = await conn.query(
      "SELECT COUNT(*) AS n FROM information_schema.TABLES WHERE TABLE_SCHEMA=? AND TABLE_NAME=?",
      [DB.database, t]
    )
    console.log(`  ${t.padEnd(24)} ${row.n > 0 ? '✓ 已就绪' : '✗ 缺失'}`)
  }

  await conn.end()
  console.log('\n' + '='.repeat(64))
  console.log('  ✓ 迁移完成')
  console.log('='.repeat(64))
  process.exit(0)
}

main().catch((e) => {
  console.error('未预期错误：', e.message)
  process.exit(1)
})
