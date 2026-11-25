import { defineStore } from 'pinia'
import { api } from '@/api'

/**
 * 自定义加载动画（全局设置）
 *
 * 数据落点：后端 shared_data 表（家庭共享一份，data_key = 'app_loader_gif'），
 * 通过 /api/sync/shared/app_loader_gif 存取。
 *
 * payload 结构：{ url: '/api/uploads/xxx.gif', set_by: memberId, set_at: '...' }
 * 未设置 / 已清除时 payload 为 null，customGif 回落到空字符串（用默认「妍」字动画）。
 *
 * 注意：主页(HomeView)的加载动画不替换——该处 YanLoader 显式传 custom-gif="false"。
 */

const KEY = 'app_loader_gif'

export const useLoaderStore = defineStore('loader', {
  state: () => ({
    /** 自定义 GIF 地址（空 = 未设置，用默认动画） */
    customGif: '',
    loaded: false
  }),

  actions: {
    /** 拉取全局自定义加载动画设置（失败静默，回落默认动画） */
    async load() {
      if (this.loaded) return this.customGif
      try {
        const res = await api.sync.getShared(KEY)
        const url = res?.payload?.url
        this.customGif = typeof url === 'string' && url ? url : ''
      } catch (e) {
        this.customGif = ''
      } finally {
        this.loaded = true
      }
      return this.customGif
    },

    /** 保存自定义加载动画 GIF（url 为 /api/uploads/xxx.gif） */
    async save(url) {
      const res = await api.sync.putShared(KEY, {
        url,
        set_by: null,
        set_at: new Date().toISOString()
      })
      this.customGif = url || ''
      this.loaded = true
      return res
    },

    /** 清除自定义加载动画，恢复默认「妍」字动画 */
    async clear() {
      await api.sync.putShared(KEY, null)
      this.customGif = ''
      this.loaded = true
    }
  }
})
