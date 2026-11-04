/**
 * 只读预检：目标实例上是否已存在 family_dining 库及数据
 * 不做任何写入或删除，仅报告现状。
 */
import mysql from 'mysql2/promise'
import { DB } from './sql-runner.mjs'

const conn = await mysql.createConnection({
  host: DB.host, port: DB.port, user: DB.user, password: DB.password,
  connectTimeout: 15000, charset: 'utf8mb4'
})

const [[v]] = await conn.query('SELECT VERSION() AS v')
console.log(`MySQL 版本：${v.v}`)

const [dbs] = await conn.query('SHOW DATABASES')
const names = dbs.map((d) => Object.values(d)[0])
console.log(`\n实例上现有数据库（${names.length} 个）：`)
names.forEach((n) => console.log('  - ' + n))

const exists = names.includes(DB.database)
console.log(`\n目标库 ${DB.database} 是否已存在：${exists ? '是' : '否'}`)

if (exists) {
  const [tbls] = await conn.query(
    `SELECT TABLE_NAME, TABLE_ROWS, TABLE_TYPE FROM information_schema.TABLES
      WHERE TABLE_SCHEMA = ? ORDER BY TABLE_NAME`, [DB.database]
  )
  console.log(`其中对象 ${tbls.length} 个：`)
  for (const t of tbls) {
    let real = t.TABLE_ROWS
    if (t.TABLE_TYPE === 'BASE TABLE') {
      const [[c]] = await conn.query(`SELECT COUNT(*) AS n FROM \`${t.TABLE_NAME}\``)
      real = c.n
    }
    console.log(`  ${t.TABLE_TYPE === 'VIEW' ? '[视图]' : '[表]  '} ${t.TABLE_NAME.padEnd(28)} ${real} 行`)
  }
  const withData = tbls.filter((t) => t.TABLE_TYPE === 'BASE TABLE')
  if (withData.length) {
    console.log('\n⚠ 目标库已存在表，导入脚本会先 DROP 再重建（现有数据将被替换）。')
  }
}

await conn.end()
