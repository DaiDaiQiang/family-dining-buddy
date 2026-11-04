import { defineStore } from 'pinia'
import { toDateStr } from '@/utils/cycle'
import { api } from '@/api'

/**
 * 家庭账本（服务端存储，跨设备互通）
 *
 * 记录日常收支：购物、买菜、生活开销等。
 * 数据存 MySQL（ledger_entry 表），localStorage 只做最近一次的缓存，
 * 便于首屏秒开与后端短暂不可用时的降级展示。
 */
const STORAGE_KEY = 'fd_ledger'

/** 收支类型 */
export const LEDGER_TYPES = [
  { value: 'expense', label: '支出', emoji: '💸', tone: 'danger' },
  { value: 'income', label: '收入', emoji: '💰', tone: 'success' }
]

/** 支出分类（覆盖购物、买菜、生活等日常场景）——取值与后端 LEDGER_CATEGORIES 一致 */
export const LEDGER_CATEGORIES = [
  { value: 'grocery', label: '买菜', emoji: '🥬' },
  { value: 'shopping', label: '购物', emoji: '🛍️' },
  { value: 'food_out', label: '外食', emoji: '🍜' },
  { value: 'daily', label: '日用', emoji: '🧻' },
  { value: 'transport', label: '交通', emoji: '🚌' },
  { value: 'medical', label: '医疗', emoji: '💊' },
  { value: 'child', label: '育儿', emoji: '🧸' },
  { value: 'housing', label: '居住', emoji: '🏠' },
  { value: 'entertain', label: '娱乐', emoji: '🎬' },
  { value: 'salary', label: '工资', emoji: '🧾' },
  { value: 'bonus', label: '奖金红包', emoji: '🧧' },
  { value: 'other', label: '其他', emoji: '📎' }
]

function readList() {
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE_KEY))
    return Array.isArray(raw) ? raw : []
  } catch {
    return []
  }
}

/** 服务端行 → 视图记录（date 字段与视图保持一致） */
function mapRow(row) {
  return {
    id: Number(row.id),
    type: row.type === 'income' ? 'income' : 'expense',
    amount: Number(row.amount) || 0,
    category: row.category,
    note: row.note || '',
    date: String(row.entry_date || '').slice(0, 10),
    member_id: row.member_id ? Number(row.member_id) : null,
    created_at: String(row.created_at || '').slice(0, 19).replace('T', ' ')
  }
}

export function ledgerTypeMeta(v) {
  return LEDGER_TYPES.find((t) => t.value === v) || LEDGER_TYPES[0]
}

export function ledgerCategoryMeta(v) {
  return LEDGER_CATEGORIES.find((c) => c.value === v) || { value: 'other', label: v || '其他', emoji: '📎' }
}

/** 'YYYY-MM' 前缀 */
function monthKey(d) {
  const dt = typeof d === 'string' ? new Date(`${d.slice(0, 10)}T00:00:00`) : d
  return `${dt.getFullYear()}-${String(dt.getMonth() + 1).padStart(2, '0')}`
}

export const useLedgerStore = defineStore('ledger', {
  state: () => ({
    records: readList(),
    loaded: false
  }),

  getters: {
    /** 按日期倒序（同日按创建时间倒序） */
    sorted: (s) =>
      [...s.records].sort(
        (a, b) => b.date.localeCompare(a.date) || (b.created_at || '').localeCompare(a.created_at || '') || b.id - a.id
      ),

    /** 本月汇总 */
    monthSummary() {
      const mk = monthKey(new Date())
      const list = this.records.filter((r) => monthKey(r.date) === mk)
      return summarize(list)
    },

    /** 按月份分组汇总，最近 12 个月 */
    byMonth() {
      const map = new Map()
      this.records.forEach((r) => {
        const k = monthKey(r.date)
        if (!map.has(k)) map.set(k, [])
        map.get(k).push(r)
      })
      return [...map.entries()]
        .map(([month, list]) => ({ month, ...summarize(list) }))
        .sort((a, b) => b.month.localeCompare(a.month))
        .slice(0, 12)
    },

    /** 本月各分类支出占比，用于横向条形展示 */
    categoryBreakdown() {
      const mk = monthKey(new Date())
      const tally = new Map()
      this.records
        .filter((r) => r.type === 'expense' && monthKey(r.date) === mk)
        .forEach((r) => {
          const meta = ledgerCategoryMeta(r.category)
          if (!tally.has(meta.value)) tally.set(meta.value, { ...meta, amount: 0 })
          tally.get(meta.value).amount += Number(r.amount) || 0
        })
      const total = [...tally.values()].reduce((s, v) => s + v.amount, 0)
      return [...tally.values()]
        .map((v) => ({
          ...v,
          amount: Math.round(v.amount * 100) / 100,
          percent: total ? Math.round((v.amount / total) * 100) : 0
        }))
        .sort((a, b) => b.amount - a.amount)
    },

    /** 最近 N 条记录 */
    recent() {
      return (n = 8) => this.sorted.slice(0, n)
    }
  },

  actions: {
    /** 写本地缓存（仅供首屏秒开/降级展示） */
    persist() {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(this.records))
      } catch (e) {
        console.warn('账本本地缓存失败', e)
      }
    },

    /** 从服务端拉全量账目 */
    async load() {
      const rows = await api.ledger.list()
      this.records = (rows || []).map(mapRow)
      this.persist()
      this.loaded = true
    },

    /**
     * 新增一笔
     * @param {object} data { type, amount, category, note, date, memberId }
     */
    async add(data) {
      const amount = Number(data.amount)
      if (!Number.isFinite(amount) || amount <= 0) throw new Error('请填写大于 0 的金额')

      const row = await api.ledger.create({
        member_id: data.memberId ? Number(data.memberId) : undefined,
        type: data.type === 'income' ? 'income' : 'expense',
        amount: Math.round(amount * 100) / 100,
        category: data.category || (data.type === 'income' ? 'salary' : 'other'),
        note: (data.note || '').trim(),
        entry_date: data.date || toDateStr(new Date())
      })
      const record = mapRow(row)
      this.records.push(record)
      this.persist()
      return record
    },

    async update(id, patch) {
      const payload = {}
      if (patch.amount !== undefined) {
        const amount = Number(patch.amount)
        if (!Number.isFinite(amount) || amount <= 0) throw new Error('请填写大于 0 的金额')
        payload.amount = Math.round(amount * 100) / 100
      }
      if (patch.type) payload.type = patch.type === 'income' ? 'income' : 'expense'
      if (patch.category) payload.category = patch.category
      if (patch.note !== undefined) payload.note = String(patch.note).trim()
      if (patch.date) payload.entry_date = patch.date
      if (patch.memberId !== undefined) payload.member_id = patch.memberId || null

      const row = await api.ledger.update(id, payload)
      const record = mapRow(row)
      const i = this.records.findIndex((x) => x.id === Number(id))
      if (i >= 0) this.records.splice(i, 1, record)
      else this.records.push(record)
      this.persist()
      return record
    },

    async remove(id) {
      await api.ledger.remove(id)
      const i = this.records.findIndex((x) => x.id === Number(id))
      if (i >= 0) this.records.splice(i, 1)
      this.persist()
    },

    /** 指定月份汇总 */
    summaryOf(month) {
      return summarize(this.records.filter((r) => monthKey(r.date) === month))
    },

    /** 指定月份的记录（按日期倒序） */
    recordsOf(month) {
      return this.records
        .filter((r) => monthKey(r.date) === month)
        .sort((a, b) => b.date.localeCompare(a.date) || b.id - a.id)
    },

    /** 按日期分组的记录，用于列表渲染 */
    groupedByDate(month) {
      const map = new Map()
      this.recordsOf(month).forEach((r) => {
        if (!map.has(r.date)) map.set(r.date, [])
        map.get(r.date).push(r)
      })
      return [...map.entries()].map(([date, list]) => ({
        date,
        list,
        income: list.filter((x) => x.type === 'income').reduce((s, x) => s + x.amount, 0),
        expense: list.filter((x) => x.type === 'expense').reduce((s, x) => s + x.amount, 0)
      }))
    }
  }
})

/** 汇总一组记录 */
function summarize(list) {
  const income = list.filter((r) => r.type === 'income').reduce((s, r) => s + (Number(r.amount) || 0), 0)
  const expense = list.filter((r) => r.type === 'expense').reduce((s, r) => s + (Number(r.amount) || 0), 0)
  return {
    count: list.length,
    income: Math.round(income * 100) / 100,
    expense: Math.round(expense * 100) / 100,
    balance: Math.round((income - expense) * 100) / 100
  }
}

export { monthKey }
