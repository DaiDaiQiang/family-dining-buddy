import { defineStore } from 'pinia'
import { api } from '@/api'

/**
 * 耗材维护与消耗统计 store（阶段 3）
 *
 * - 档案列表：list() 拉启用项，inactive() 拉停用项
 * - 消耗登记：consume(id, qty) 扣减库存并记日志
 * - 低库存：low() 供首页滚动栏提示
 * - 统计：stats() 供统计面板
 */
export const useConsumableStore = defineStore('consumable', {
  state: () => ({
    items: [],
    inactive: [],
    lowStock: [],
    stats: null,
    loading: false,
    logs: {}, // consumable_id -> 消耗记录数组
    logLoading: {}
  }),

  getters: {
    /** 低库存数量（滚动栏徽标用） */
    lowCount: (s) => s.lowStock.length,
    /** 按分类分组 */
    grouped: (s) => {
      const out = {}
      for (const c of s.items) {
        ;(out[c.category] = out[c.category] || []).push(c)
      }
      return out
    }
  },

  actions: {
    async load() {
      this.loading = true
      try {
        const [items, low] = await Promise.all([
          api.consumables.list(),
          api.consumables.low().catch(() => [])
        ])
        this.items = items || []
        this.lowStock = low || []
        return this.items
      } catch (e) {
        console.warn('加载耗材失败', e)
        throw e
      } finally {
        this.loading = false
      }
    },

    async loadLow() {
      try {
        this.lowStock = (await api.consumables.low()) || []
      } catch (e) {
        console.warn('加载低库存失败', e)
      }
      return this.lowStock
    },

    async loadStats() {
      try {
        this.stats = await api.consumables.stats()
      } catch (e) {
        console.warn('加载耗材统计失败', e)
      }
      return this.stats
    },

    async create(data) {
      const row = await api.consumables.create(data)
      await this.load()
      return row
    },

    async update(id, data) {
      const row = await api.consumables.update(id, data)
      const idx = this.items.findIndex((c) => c.id === id)
      if (idx >= 0 && row) this.items[idx] = row
      await this.loadLow()
      return row
    },

    async remove(id) {
      await api.consumables.remove(id)
      await this.load()
      return { ok: true }
    },

    /** 登记消耗：扣库存并记日志，返回最新库存 */
    async consume(id, data) {
      const res = await api.consumables.addConsumption(id, data)
      // 本地同步最新库存
      const idx = this.items.findIndex((c) => c.id === id)
      if (idx >= 0 && res?.stock !== undefined) {
        this.items[idx] = { ...this.items[idx], stock: res.stock }
      }
      await this.loadLow()
      return res
    },

    /** 拉某耗材的消耗记录（带缓存） */
    async loadLogs(id) {
      if (this.logLoading[id]) return this.logs[id] || []
      this.logLoading[id] = true
      try {
        const logs = (await api.consumables.consumption(id)) || []
        this.logs = { ...this.logs, [id]: logs }
        return logs
      } catch (e) {
        console.warn('加载消耗记录失败', e)
        return []
      } finally {
        this.logLoading[id] = false
      }
    }
  }
})
