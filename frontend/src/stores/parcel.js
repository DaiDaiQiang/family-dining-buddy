import { defineStore } from 'pinia'
import { toDateStr } from '@/utils/cycle'
import { api } from '@/api'

/**
 * 快递取件码（服务端存储，跨设备互通）
 *
 * 记录驿站 / 快递柜的取件码，省得翻短信找码。
 * 数据存 MySQL（parcel 表），localStorage 只做最近一次的缓存。
 *
 * 后期接快递100：COURIERS[].code 即快递100的 `com` 编码，
 * 配合记录的 tracking_no 可直接调物流轨迹接口，无需再补数据。
 */
const STORAGE_KEY = 'fd_parcel'

/** 包裹状态 */
export const PARCEL_STATUS = [
  { value: 'pending', label: '待取', emoji: '📮', tone: 'warn' },
  { value: 'picked', label: '已取', emoji: '✅', tone: 'muted' },
  { value: 'expired', label: '已超时', emoji: '⏰', tone: 'danger' }
]

/**
 * 常见快递公司
 * code 采用快递100的 `com` 编码，将来对接轨迹查询可直接使用
 */
export const COURIERS = [
  { code: 'shunfeng', label: '顺丰速运', emoji: '🖤' },
  { code: 'jd', label: '京东物流', emoji: '🐶' },
  { code: 'zhongtong', label: '中通快递', emoji: '🔵' },
  { code: 'yuantong', label: '圆通速递', emoji: '🟡' },
  { code: 'yunda', label: '韵达快递', emoji: '🟠' },
  { code: 'shentong', label: '申通快递', emoji: '🟢' },
  { code: 'jtexpress', label: '极兔速递', emoji: '🐰' },
  { code: 'youzhengguonei', label: '邮政快递包裹', emoji: '📮' },
  { code: 'ems', label: 'EMS', emoji: '✉️' },
  { code: 'debangkuaidi', label: '德邦快递', emoji: '🚚' },
  { code: 'huitongkuaidi', label: '百世快递', emoji: '📦' },
  { code: 'tiantian', label: '天天快递', emoji: '🗓️' },
  { code: 'other', label: '其他', emoji: '📦' }
]

/** 驿站 / 自提点类型 */
export const STATION_TYPES = [
  { value: 'cainiao', label: '菜鸟驿站', emoji: '🏪' },
  { value: 'locker', label: '快递柜', emoji: '🗄️' },
  { value: 'post', label: '邮政网点', emoji: '🏤' },
  { value: 'shop', label: '代收点', emoji: '🏬' },
  { value: 'door', label: '送货上门', emoji: '🚪' },
  { value: 'other', label: '其他', emoji: '📍' }
]

function readList() {
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE_KEY))
    return Array.isArray(raw) ? raw : []
  } catch {
    return []
  }
}

/** 服务端行 → 视图记录 */
function mapRow(row) {
  return {
    id: Number(row.id),
    pickup_code: row.pickup_code,
    courier_code: row.courier_code || 'other',
    station: row.station || '',
    station_type: row.station_type || 'cainiao',
    shelf_no: row.shelf_no || '',
    tracking_no: row.tracking_no || '',
    note: row.note || '',
    member_id: row.recipient_member_id ? Number(row.recipient_member_id) : null,
    status: row.status === 'picked' ? 'picked' : 'pending',
    received_at: String(row.arrived_date || '').slice(0, 10),
    picked_at: row.picked_at ? String(row.picked_at).slice(0, 19).replace('T', ' ') : null,
    created_at: String(row.created_at || '').slice(0, 19).replace('T', ' ')
  }
}

export function parcelStatusMeta(v) {
  return PARCEL_STATUS.find((s) => s.value === v) || PARCEL_STATUS[0]
}

export function courierMeta(code) {
  return COURIERS.find((c) => c.code === code) || COURIERS[COURIERS.length - 1]
}

export function stationMeta(v) {
  return STATION_TYPES.find((s) => s.value === v) || STATION_TYPES[STATION_TYPES.length - 1]
}

/** 状态排序权重：待取最前，其次超时，已取沉底 */
const STATUS_ORDER = { pending: 0, expired: 1, picked: 2 }

/** 取件码展示分组：4 位一切，便于照着念 */
export function formatCode(code) {
  const s = String(code || '').trim()
  if (!s) return ''
  return s.replace(/(.{4})(?=.)/g, '$1 ')
}

export const useParcelStore = defineStore('parcel', {
  state: () => ({
    records: readList(),
    loaded: false
  }),

  getters: {
    /** 待取优先，同状态按到件日期倒序 */
    sorted: (s) =>
      [...s.records].sort((a, b) => {
        const w = (STATUS_ORDER[a.status] ?? 9) - (STATUS_ORDER[b.status] ?? 9)
        if (w !== 0) return w
        return (
          String(b.received_at || '').localeCompare(String(a.received_at || '')) ||
          String(b.created_at || '').localeCompare(String(a.created_at || '')) ||
          b.id - a.id
        )
      }),

    /** 待取包裹（含已超时），去驿站时看这一份就够 */
    pending: (s) => s.records.filter((r) => r.status !== 'picked'),

    pendingCount: (s) => s.records.filter((r) => r.status === 'pending').length,
    expiredCount: (s) => s.records.filter((r) => r.status === 'expired').length,
    pickedCount: (s) => s.records.filter((r) => r.status === 'picked').length,

    /** 按状态筛选；'all' 返回全部 */
    filtered() {
      return (status = 'all') =>
        status === 'all' ? this.sorted : this.sorted.filter((r) => r.status === status)
    },

    /** 待取清单纯文本，便于直接转发给家人代取 */
    pendingText() {
      const list = this.pending
      if (!list.length) return ''
      const lines = list.map((r) => {
        const c = courierMeta(r.courier_code)
        const bits = [`${c.label}${r.station ? ` · ${r.station}` : ''}`, `取件码 ${r.pickup_code}`]
        if (r.shelf_no) bits.push(`货架 ${r.shelf_no}`)
        return bits.join(' ')
      })
      return `待取快递 ${list.length} 件\n${lines.join('\n')}`
    },

    recent() {
      return (n = 6) => this.sorted.slice(0, n)
    },

    /**
     * 按收件人分组的待取件（含已超时），供首页「快递代取」滚动卡使用。
     * 返回 [{ memberId, items: [record...] }]，组内按 sorted 的次序（超时在前）。
     * 未指定收件人的归到 memberId=null 组，展示为「家人」。
     */
    pendingByMember() {
      const groups = new Map()
      for (const r of this.pending) {
        const key = r.member_id ? Number(r.member_id) : null
        if (!groups.has(key)) groups.set(key, [])
        groups.get(key).push(r)
      }
      // 有指定收件人的排前面，未指定的兜底放最后
      return [...groups.entries()]
        .map(([memberId, items]) => ({
          memberId,
          items: items.sort((a, b) => (STATUS_ORDER[a.status] ?? 9) - (STATUS_ORDER[b.status] ?? 9))
        }))
        .sort((a, b) => (a.memberId === null) - (b.memberId === null))
    }
  },

  actions: {
    persist() {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(this.records))
      } catch (e) {
        console.warn('快递取件码本地缓存失败', e)
      }
    },

    /** 从服务端拉全量记录 */
    async load() {
      const rows = await api.parcels.list()
      this.records = (rows || []).map(mapRow)
      this.persist()
      this.loaded = true
    },

    /**
     * 新增一条取件码
     * @param {object} data { pickupCode, courierCode, station, stationType, shelfNo,
     *                        trackingNo, note, memberId, receivedAt }
     */
    async add(data) {
      const pickupCode = String(data.pickupCode || '').trim()
      if (!pickupCode) throw new Error('请填写取件码')
      if (pickupCode.length > 32) throw new Error('取件码过长，最多 32 个字符')

      const row = await api.parcels.create({
        pickup_code: pickupCode,
        courier_code: data.courierCode || 'other',
        station: String(data.station || '').trim(),
        station_type: data.stationType || 'cainiao',
        shelf_no: String(data.shelfNo || '').trim(),
        tracking_no: String(data.trackingNo || '').trim(),
        note: String(data.note || '').trim(),
        recipient_member_id: data.memberId ? Number(data.memberId) : undefined,
        arrived_date: data.receivedAt || toDateStr(new Date())
      })
      const record = mapRow(row)
      this.records.push(record)
      this.persist()
      return record
    },

    async update(id, patch) {
      const payload = {}
      if (patch.pickupCode !== undefined) {
        const code = String(patch.pickupCode).trim()
        if (!code) throw new Error('取件码不能为空')
        if (code.length > 32) throw new Error('取件码过长，最多 32 个字符')
        payload.pickup_code = code
      }
      if (patch.courierCode) payload.courier_code = patch.courierCode
      if (patch.station !== undefined) payload.station = String(patch.station).trim()
      if (patch.stationType) payload.station_type = patch.stationType
      if (patch.shelfNo !== undefined) payload.shelf_no = String(patch.shelfNo).trim()
      if (patch.trackingNo !== undefined) payload.tracking_no = String(patch.trackingNo).trim()
      if (patch.note !== undefined) payload.note = String(patch.note).trim()
      if (patch.memberId !== undefined) payload.recipient_member_id = patch.memberId || null
      if (patch.receivedAt) payload.arrived_date = patch.receivedAt
      if (patch.status && ['pending', 'picked'].includes(patch.status)) payload.status = patch.status

      const row = await api.parcels.update(id, payload)
      const record = mapRow(row)
      const i = this.records.findIndex((x) => x.id === Number(id))
      if (i >= 0) this.records.splice(i, 1, record)
      else this.records.push(record)
      this.persist()
      return record
    },

    /** 标记已取，记录取件时间 */
    async markPicked(id) {
      return this.update(id, { status: 'picked' })
    },

    /** 退回待取（点错已取时用） */
    async reopen(id) {
      return this.update(id, { status: 'pending' })
    },

    async remove(id) {
      await api.parcels.remove(id)
      const i = this.records.findIndex((x) => x.id === Number(id))
      if (i >= 0) this.records.splice(i, 1)
      this.persist()
    },

    /** 批量清除已取记录，列表不至于越积越长 */
    async clearPicked() {
      const before = this.records.length
      await api.parcels.clearPicked()
      this.records = this.records.filter((r) => r.status !== 'picked')
      this.persist()
      return before - this.records.length
    }
  }
})
