import { defineStore } from 'pinia'
import { useAppStore } from './app'
import { api } from '@/api'
import {
  computeCycleStatus,
  reminderText,
  parseDate,
  addDays,
  toDateStr,
  PREMENSTRUAL_ALERT_DAYS
} from '@/utils/cycle'

/**
 * 家庭健康档案（服务端存储，跨设备互通）
 *
 * 保存每位成员的性别与生理周期设置，并据此生成提醒。
 * 数据存 MySQL（health_profile 表，一成员一行），localStorage 只做缓存。
 */
const GENDER_KEY = 'fd_member_gender'
const CYCLE_KEY = 'fd_member_cycle'

function readJSON(key, fallback) {
  try {
    return JSON.parse(localStorage.getItem(key)) || fallback
  } catch {
    return fallback
  }
}

/** 未显式设置性别时，按头像 emoji 猜测（仅用于默认勾选，可随时修改） */
const FEMALE_HINT = ['👩', '👧', '👵', '👶', '🧕', '👩‍🦰', '👩‍🦱', '👩‍🦳', '👧🏻']
function guessGender(member) {
  if (!member) return 'unknown'
  return FEMALE_HINT.includes(member.avatar_emoji) ? 'female' : 'unknown'
}

export const useHealthStore = defineStore('health', {
  state: () => ({
    /** { [memberId]: 'female' | 'male' | 'unknown' } */
    genders: readJSON(GENDER_KEY, {}),
    /** { [memberId]: { lastPeriodStart, cycleLength, periodLength, enabled } } */
    cycles: readJSON(CYCLE_KEY, {}),
    loaded: false
  }),

  getters: {
    /** 取某成员的性别；未设置时回落到头像猜测 */
    genderOf: (s) => (memberId) =>
      s.genders[memberId] || guessGender(useAppStore().members.find((m) => m.id === Number(memberId))),

    /** 所有女性成员（用于经期提醒列表） */
    femaleMembers() {
      const app = useAppStore()
      return app.activeMembers.filter((m) => this.genderOf(m.id) === 'female')
    },

    /** 已开启经期提醒的成员及其周期状态 */
    trackedStatuses() {
      return this.femaleMembers
        .map((m) => {
          const profile = this.cycles[m.id]
          if (!profile?.enabled || !profile.lastPeriodStart) return null
          const status = computeCycleStatus(profile)
          return status ? { member: m, status } : null
        })
        .filter(Boolean)
    },

    /**
     * 需要提醒的条目，供首页滚动栏与提醒页共用。
     * 规则：经期第 1 天、经前 3 天内、排卵日当天给出提醒。
     */
    alerts() {
      const out = []
      this.trackedStatuses.forEach(({ member, status }) => {
        const name = member.nickname
        if (status.periodDay === 1) {
          out.push({
            id: `period-${member.id}`,
            level: 'danger',
            emoji: '🩸',
            title: `${name} 月经来潮`,
            text: reminderText(status, name),
            memberId: member.id,
            phase: status.phase
          })
        } else if (status.isPremenstrualAlert) {
          out.push({
            id: `pre-${member.id}`,
            level: 'warning',
            emoji: '⏰',
            title: `${name} 经期将至`,
            text: reminderText(status, name),
            memberId: member.id,
            phase: status.phase
          })
        }
        if (status.phase === 'fertile') {
          const isOvulation = toDateStr(new Date()) === status.ovulationDate
          out.push({
            id: `fertile-${member.id}`,
            level: isOvulation ? 'success' : 'info',
            emoji: isOvulation ? '🥚' : '🌱',
            title: isOvulation ? `${name} 排卵日` : `${name} 排卵期`,
            text: reminderText(status, name),
            memberId: member.id,
            phase: status.phase
          })
        }
      })
      // 紧急程度高的排前面
      const order = { danger: 0, warning: 1, success: 2, info: 3 }
      return out.sort((a, b) => (order[a.level] ?? 9) - (order[b.level] ?? 9))
    },

    /** 未来 N 天的经期预测，用于日历式展示 */
    upcomingPeriods() {
      const map = new Map()
      this.trackedStatuses.forEach(({ member, status }) => {
        const start = parseDate(status.nextStart)
        for (let i = 0; i < 3; i++) {
          const d = addDays(start, i * status.cycleLength)
          for (let k = 0; k < status.periodLength; k++) {
            const key = toDateStr(addDays(d, k))
            if (!map.has(key)) map.set(key, [])
            const list = map.get(key)
            if (!list.some((x) => x.memberId === member.id)) {
              list.push({ memberId: member.id, name: member.nickname, emoji: member.avatar_emoji })
            }
          }
        }
      })
      return [...map.entries()]
        .map(([date, members]) => ({ date, members }))
        .sort((a, b) => a.date.localeCompare(b.date))
    }
  },

  actions: {
    persist() {
      try {
        localStorage.setItem(GENDER_KEY, JSON.stringify(this.genders))
        localStorage.setItem(CYCLE_KEY, JSON.stringify(this.cycles))
      } catch (e) {
        console.warn('健康档案本地缓存失败', e)
      }
    },

    /** 从服务端拉全家健康档案 */
    async load() {
      const rows = await api.health.profiles()
      const genders = {}
      const cycles = {}
      for (const row of rows || []) {
        const mid = Number(row.member_id)
        genders[mid] = row.gender || 'unknown'
        const hasStart = !!row.last_period_start
        cycles[mid] = {
          lastPeriodStart: hasStart ? String(row.last_period_start).slice(0, 10) : '',
          cycleLength: Number(row.cycle_length) || 28,
          periodLength: Number(row.period_length) || 5,
          enabled: hasStart
        }
      }
      this.genders = genders
      this.cycles = cycles
      this.persist()
      this.loaded = true
    },

    /** 把某成员的本地档案推到服务端（整包替换） */
    async pushMember(memberId) {
      const mid = Number(memberId)
      const g = this.genders[mid] || 'unknown'
      const c = this.cycles[mid] || {}
      await api.health.saveProfile(mid, {
        gender: g,
        last_period_start: c.lastPeriodStart || null,
        cycle_length: Number(c.cycleLength) || 28,
        period_length: Number(c.periodLength) || 5
      })
    },

    async setGender(memberId, gender) {
      const mid = Number(memberId)
      this.genders[mid] = ['female', 'male', 'unknown'].includes(gender) ? gender : 'unknown'
      // 改为非女性时关闭经期提醒，避免残留无效提醒
      if (this.genders[mid] !== 'female' && this.cycles[mid]) {
        this.cycles[mid] = { ...this.cycles[mid], enabled: false }
      }
      this.persist()
      await this.pushMember(mid)
    },

    /** 保存周期设置；lastPeriodStart 为空则视为未启用 */
    async saveCycle(memberId, patch = {}) {
      const mid = Number(memberId)
      const prev = this.cycles[mid] || {}
      const next = {
        enabled: patch.enabled ?? prev.enabled ?? true,
        lastPeriodStart: patch.lastPeriodStart !== undefined ? patch.lastPeriodStart : prev.lastPeriodStart || '',
        cycleLength: Number(patch.cycleLength ?? prev.cycleLength ?? 28) || 28,
        periodLength: Number(patch.periodLength ?? prev.periodLength ?? 5) || 5
      }
      if (!parseDate(next.lastPeriodStart)) {
        next.enabled = false
        next.lastPeriodStart = ''
      }
      this.cycles[mid] = next
      this.persist()
      await this.pushMember(mid)
      return next
    },

    /** 记录「这次月经来了」：把开始日设为指定日期 */
    async markPeriodStart(memberId, dateStr) {
      return this.saveCycle(memberId, {
        lastPeriodStart: dateStr || toDateStr(new Date()),
        enabled: true
      })
    },

    async clearCycle(memberId) {
      const mid = Number(memberId)
      delete this.cycles[mid]
      this.persist()
      await this.pushMember(mid)
    },

    /** 某成员的当前状态；未设置时返回 null */
    statusOf(memberId) {
      const profile = this.cycles[memberId]
      if (!profile?.lastPeriodStart) return null
      return computeCycleStatus(profile)
    }
  }
})

/** 提前提醒天数（供界面说明用） */
export const ALERT_LEAD_DAYS = PREMENSTRUAL_ALERT_DAYS
