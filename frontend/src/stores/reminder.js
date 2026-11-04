import { defineStore } from 'pinia'
import { api } from '@/api'

/**
 * 站内提醒 store（阶段 2）
 *
 * - 服务端为数据源：load() 先触发 sync（经期/快递两类自动生成）再拉列表
 * - 操作：done（完成）/ dismiss（忽略）/ postpone（推迟 N 天）
 * - localStorage 仅缓存最近一次列表，离线不阻断展示
 */
const CACHE_KEY = 'fd_reminders'

function readCache() {
  try {
    const raw = localStorage.getItem(CACHE_KEY)
    return raw ? JSON.parse(raw) : []
  } catch {
    return []
  }
}

function persist(list) {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(list))
  } catch { /* 缓存失败不影响主流程 */ }
}

export const useReminderStore = defineStore('reminder', {
  state: () => ({
    items: readCache(),
    loading: false,
    lastSync: 0
  }),

  getters: {
    pending: (s) => s.items.filter((r) => r.status === 'pending'),
    done: (s) => s.items.filter((r) => r.status === 'done'),
    dismissed: (s) => s.items.filter((r) => r.status === 'dismissed'),
    pendingCount() { return this.pending.length },
    /** 首页摘要：前 4 条待办 */
    brief() { return this.pending.slice(0, 4) },
    /** 按类型分组的待办数（徽标用） */
    countByType: (s) => {
      const out = {}
      for (const r of s.items.filter((x) => x.status === 'pending')) {
        out[r.type] = (out[r.type] || 0) + 1
      }
      return out
    }
  },

  actions: {
    /** 同步生成 + 拉列表。sync=true 时先跑生成引擎 */
    async load({ sync = true } = {}) {
      this.loading = true
      try {
        if (sync) await api.reminders.sync()
        this.items = (await api.reminders.list({ status: 'pending' })) || []
        // 附带已完成/已忽略，供「历史」页签浏览（量小，一次拉完）
        const [done, dismissed] = await Promise.all([
          api.reminders.list({ status: 'done' }),
          api.reminders.list({ status: 'dismissed' })
        ])
        this.items = [...this.items, ...(done || []), ...(dismissed || [])]
        this.lastSync = Date.now()
        persist(this.items)
      } catch (e) {
        console.warn('加载提醒失败', e)
        // 失败时保留缓存
        this.items = readCache()
      } finally {
        this.loading = false
      }
    },

    /** 完成 / 忽略 / 重新打开 */
    async setStatus(id, status) {
      const row = this.items.find((r) => r.id === id)
      if (row) row.status = status
      try {
        const updated = await api.reminders.update(id, { status })
        if (row && updated) Object.assign(row, updated)
        persist(this.items)
        return { ok: true }
      } catch (e) {
        if (row) row.status = 'pending' // 失败回滚
        return { ok: false, error: e?.message || '操作失败' }
      }
    },

    /** 推迟：移到 N 天后的当天 09:00 */
    async postpone(id, days = 1) {
      const row = this.items.find((r) => r.id === id)
      const base = row?.due_at ? new Date(row.due_at) : new Date()
      const next = new Date(base.getTime() + days * 86400000)
      next.setHours(9, 0, 0, 0)
      const dueAt = next.toISOString().slice(0, 19) // YYYY-MM-DDTHH:mm:ss
      try {
        const updated = await api.reminders.update(id, { due_at: dueAt })
        if (row && updated) Object.assign(row, updated)
        persist(this.items)
        return { ok: true }
      } catch (e) {
        return { ok: false, error: e?.message || '推迟失败' }
      }
    }
  }
})
