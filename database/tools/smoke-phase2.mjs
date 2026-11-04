// 阶段 2 冒烟：member 档案字段 + 提醒生成/状态流转
const BASE = 'http://localhost:8080'

async function call(method, path, { token, body } = {}) {
  const res = await fetch(BASE + path, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {})
    },
    body: body ? JSON.stringify(body) : undefined
  })
  const json = await res.json().catch(() => ({}))
  return { status: res.status, json }
}

let pass = 0
function ok(name) { pass++; console.log(`✓ ${name}`) }
function fail(name, extra) { console.error(`✗ ${name}`, extra || ''); process.exit(1) }

// ---- 登录（妍姐 id=2 免密登录，避免密码出现在命令行）----
const login = await call('POST', '/api/auth/login', {
  body: { member_id: 2, password: '' }
})
if (login.json.code !== 0) fail('登录失败', JSON.stringify(login.json))
const token = login.json.data.token
ok('1. 登录（免密）')

// ---- 0. 重置上次冒烟遗留的生成提醒（保证可重复执行）----
for (const st of ['done', 'dismissed']) {
  const old = await call('GET', `/api/reminders?status=${st}`, { token })
  for (const r of old.json.data || []) {
    if (r.payload?.source) {
      await call('PUT', `/api/reminders/${r.id}`, { token, body: { status: 'pending' } })
    }
  }
}

// ---- 2. member 档案字段写入 ----
const upd = await call('PUT', '/api/members/2', {
  token,
  body: { birthday: '1992-06-15', gender: 'female', dislikes: '香菜,内脏' }
})
if (upd.json.code !== 0) fail('member 字段写入', JSON.stringify(upd.json))
const m2 = upd.json.data
if (m2.birthday !== '1992-06-15' || m2.gender !== 'female' || m2.dislikes !== '香菜,内脏') {
  fail('member 字段回读不一致', JSON.stringify(m2))
}
ok('2. member 生日/性别/忌口 写入+回读')

// ---- 3. 显式清空 ----
const clr = await call('PUT', '/api/members/2', {
  token,
  body: { dislikes: null }
})
if (clr.json.code !== 0) fail('member 字段清空', JSON.stringify(clr.json))
if (clr.json.data.dislikes !== null) fail('忌口应清空为 null', JSON.stringify(clr.json.data))
ok('3. member 忌口显式清空（null 语义）')

// ---- 4. gender 校验拒绝非法值 ----
const bad = await call('PUT', '/api/members/2', { token, body: { gender: 'robot' } })
if (bad.json.code === 0) fail('非法 gender 应被拒绝', JSON.stringify(bad.json))
ok('4. 非法 gender 被拒')

// ---- 5. 健康档案 → 经期提醒生成 ----
const hp = await call('PUT', '/api/health/profiles/2', {
  token,
  body: {
    gender: 'female',
    last_period_start: '2026-10-01',
    cycle_length: 28,
    period_length: 5
  }
})
if (hp.json.code !== 0) fail('健康档案保存', JSON.stringify(hp.json))

// ---- 6. 快递超时 → 快递提醒生成（造一条 3 天前到件的待取）----
const pz = await call('POST', '/api/parcels', {
  token,
  body: {
    pickup_code: '8899',
    courier_code: 'sf',
    recipient_member_id: 1,
    arrived_date: (() => {
      const d = new Date(Date.now() - 3 * 86400000)
      return d.toISOString().slice(0, 10)
    })()
  }
})
if (pz.json.code !== 0) fail('快递创建', JSON.stringify(pz.json))
const parcelId = pz.json.data.id

// ---- 7. 同步生成 ----
const sync = await call('POST', '/api/reminders/sync', { token })
if (sync.json.code !== 0) fail('提醒同步', JSON.stringify(sync.json))
ok(`5. 提醒同步 — ${sync.json.message}`)

// ---- 8. 列表校验：应有经期 + 快递两类 ----
const list = await call('GET', '/api/reminders?status=pending', { token })
if (list.json.code !== 0) fail('提醒列表', JSON.stringify(list.json))
const items = list.json.data
const hasHealth = items.some((r) => r.type === 'health')
const hasParcel = items.some((r) => r.type === 'parcel')
if (!hasHealth) fail('缺少经期提醒', JSON.stringify(items))
if (!hasParcel) fail('缺少快递超时提醒', JSON.stringify(items))
ok(`6. 提醒列表含经期+快递（共 ${items.length} 条）`)

// ---- 9. 幂等：再同步不应翻倍 ----
const sync2 = await call('POST', '/api/reminders/sync', { token })
if (sync2.json.code !== 0) fail('二次同步应成功', JSON.stringify(sync2.json))
const list2 = await call('GET', '/api/reminders?status=pending', { token })
if (list2.json.data.length !== items.length) {
  fail('重复同步后数量应不变', `${items.length} -> ${list2.json.data.length}`)
}
ok(`7. 幂等同步（${sync2.json.message}，数量不变）`)

// ---- 10. 状态流转：完成 / 推迟 / 忽略 ----
const target = items.find((r) => r.type === 'health')
const done = await call('PUT', `/api/reminders/${target.id}`, { token, body: { status: 'done' } })
if (done.json.code !== 0 || done.json.data.status !== 'done') fail('完成操作', JSON.stringify(done.json))
ok('8. 完成操作')

const target2 = items.find((r) => r.type === 'parcel')
const nextDay = new Date(Date.now() + 86400000)
const dueAt = nextDay.toISOString().slice(0, 19)
const post = await call('PUT', `/api/reminders/${target2.id}`, { token, body: { due_at: dueAt } })
if (post.json.code !== 0) fail('推迟操作', JSON.stringify(post.json))
if (!String(post.json.data.due_at).startsWith(dueAt.slice(0, 10))) {
  fail('推迟后日期不对', JSON.stringify(post.json.data))
}
ok('9. 推迟操作（改 due_at）')

const dis = await call('PUT', `/api/reminders/${target2.id}`, { token, body: { status: 'dismissed' } })
if (dis.json.code !== 0 || dis.json.data.status !== 'dismissed') fail('忽略操作', JSON.stringify(dis.json))
ok('10. 忽略操作')

// ---- 11. 条件消失自动清理：取件后重同步，快递提醒应被清 ----
const pick = await call('PUT', `/api/parcels/${parcelId}`, { token, body: { status: 'picked' } })
if (pick.json.code !== 0) fail('快递标记已取', JSON.stringify(pick.json))
await call('POST', '/api/reminders/sync', { token })
const list3 = await call('GET', '/api/reminders?status=pending', { token })
const parcelStill = list3.json.data.some((r) => r.type === 'parcel')
if (parcelStill) fail('取件后快递提醒应自动清理')
ok('11. 取件后快递提醒自动清理')

// ---- 12. 清理测试数据（还原妍姐档案）----
await call('PUT', '/api/members/2', {
  token,
  body: { birthday: null, gender: null, dislikes: null }
})
await call('PUT', '/api/health/profiles/2', {
  token,
  body: { gender: 'unknown', last_period_start: '', cycle_length: 28, period_length: 5 }
})
// 档案清空后孤儿化生成提醒，再同步一把自动忽略掉
await call('POST', '/api/reminders/sync', { token })
ok('12. 测试数据清理')

console.log(`\nALL PASS (${pass} 项)`)
