import { defineStore } from 'pinia'
import { api, probeBackend, getMode, onModeChange } from '@/api'
import { getToken } from '@/api/http'
import { useLedgerStore } from './ledger'
import { useParcelStore } from './parcel'
import { useHealthStore } from './health'
import { useStudyStore } from './study'
import { useReminderStore } from './reminder'

/**
 * 应用级全局状态：连接模式、成员、元数据字典
 */
export const useAppStore = defineStore('app', {
  state: () => ({
    online: null,            // null 未探测 / true 后端可用 / false 后端不可达
    offlineReason: '',
    ready: false,
    modeBound: false,        // onModeChange 监听只注册一次
    members: [],
    meta: null,
    /** 当前操作者（免密码的轻量身份），存 localStorage */
    currentMemberId: Number(localStorage.getItem('fd_current_member')) || null
  }),

  getters: {
    // 后端不可达（本前端无离线模式，不可用时直接阻断）
    backendDown: (s) => s.online === false,
    currentMember: (s) => s.members.find((m) => m.id === s.currentMemberId) || null,
    activeMembers: (s) => s.members.filter((m) => m.is_active !== false),
    mealTypeLabel: (s) => (v) => s.meta?.meal_types?.find((m) => m.value === v)?.label || v,
    mealTypeEmoji: (s) => (v) => s.meta?.meal_types?.find((m) => m.value === v)?.emoji || '🍽️',
    difficultyLabel: (s) => (v) => s.meta?.difficulties?.find((m) => m.value === v)?.label || v,
    statusLabel: (s) => (v) => s.meta?.order_status?.find((m) => m.value === v)?.label || v,
    categories: (s) => s.meta?.categories || [],
    cuisines: (s) => s.meta?.cuisines || [],
    coverPresets: (s) => s.meta?.cover_presets || []
  },

  actions: {
    /** 启动初始化：探测后端 → 拉成员与字典；force = true 用于阻断页的「重新连接」 */
    async init(force = false) {
      if (this.ready && !force) return
      if (!this.modeBound) {
        this.modeBound = true
        onModeChange(({ online, offlineReason }) => {
          this.online = online
          this.offlineReason = offlineReason
        })
      }
      await probeBackend()
      await this.loadMeta()
      // 成员/四模块接口都要令牌；未登录时跳过（登录成功后 init(true) 重拉）
      if (getToken()) {
        await this.loadMembers()
        // 四个家庭模块 + 提醒中心从服务端拉数据，失败不阻断启动
        await Promise.allSettled([
          useLedgerStore().load(),
          useParcelStore().load(),
          useHealthStore().load(),
          useStudyStore().pull(),
          useReminderStore().load()
        ])
      }
      // 没有记录过当前成员时，默认选第一位
      if (!this.currentMember && this.activeMembers.length) {
        this.setCurrentMember(this.activeMembers[0].id)
      }
      this.ready = true
    },

    async loadMembers() {
      try {
        this.members = (await api.members.list()) || []
      } catch (e) {
        console.warn('加载成员失败', e)
        this.members = []
      }
    },

    async loadMeta() {
      try {
        this.meta = await api.meta()
      } catch (e) {
        console.warn('加载字典失败', e)
        this.meta = null
      }
    },

    setCurrentMember(id) {
      this.currentMemberId = Number(id) || null
      if (this.currentMemberId) localStorage.setItem('fd_current_member', String(this.currentMemberId))
      else localStorage.removeItem('fd_current_member')
    }
  }
})
