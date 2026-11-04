import { defineStore } from 'pinia'
import { useAppStore } from './app'
import { api } from '@/api'

/**
 * 登录态 store（服务端鉴权：每人密码 + JWT）
 *
 * - 进入任何页面前都要过路由守卫，未登录一律先去 /login
 * - 密码由服务端 argon2 哈希保存；未设密码的成员可免密登录（渐进迁移）
 * - 登录成功后令牌存本地：「记住登录状态」→ localStorage（关浏览器再开还在）；
 *   不勾选 → sessionStorage（关掉标签页即失效）
 * - 401 时 http.js 会清登录态并跳登录页
 */

const AUTH_KEY = 'fd_auth'

function readAuth() {
  try {
    const raw = localStorage.getItem(AUTH_KEY) || sessionStorage.getItem(AUTH_KEY)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export const useAuthStore = defineStore('auth', {
  state: () => ({
    memberId: null,
    token: '',
    remember: true,
    hydrated: false
  }),

  getters: {
    loggedIn: (s) => !!s.memberId && !!s.token
  },

  actions: {
    /** 从存储恢复登录态：localStorage（记住我）优先，其次 sessionStorage */
    hydrate() {
      const data = readAuth()
      if (data) {
        this.memberId = Number(data.memberId) || null
        this.token = String(data.token || '')
        this.remember = !!localStorage.getItem(AUTH_KEY)
      }
      this.hydrated = true
    },

    /**
     * 登录：POST /api/auth/login
     * @returns {Promise<{ok: boolean, error?: string, passwordSet?: boolean}>}
     */
    async login(memberId, password, remember = true) {
      try {
        const res = await api.auth.login({
          member_id: Number(memberId),
          password: String(password || '')
        })
        this.memberId = res.member?.id ?? Number(memberId)
        this.token = res.token || ''
        this.remember = remember
        const keep = remember ? localStorage : sessionStorage
        const drop = remember ? sessionStorage : localStorage
        keep.setItem(
          AUTH_KEY,
          JSON.stringify({ memberId: this.memberId, token: this.token, ts: Date.now() })
        )
        drop.removeItem(AUTH_KEY)
        // 同步「当前操作者」，点餐/收藏才能记到名下
        useAppStore().setCurrentMember(this.memberId)
        return { ok: true, passwordSet: !!res.password_set }
      } catch (e) {
        return { ok: false, error: e?.message || '登录失败，请重试' }
      }
    },

    logout() {
      this.memberId = null
      this.token = ''
      localStorage.removeItem(AUTH_KEY)
      sessionStorage.removeItem(AUTH_KEY)
      useAppStore().setCurrentMember(null)
    },

    /**
     * 修改自己的密码（PUT /api/auth/password）
     * @returns {Promise<{ok: boolean, error?: string}>}
     */
    async changePassword(_memberId, oldPwd, newPwd) {
      if (!newPwd || String(newPwd).length < 4) {
        return { ok: false, error: '新密码至少 4 位' }
      }
      try {
        await api.auth.changePassword({
          old_password: String(oldPwd || ''),
          new_password: String(newPwd)
        })
        return { ok: true }
      } catch (e) {
        return { ok: false, error: e?.message || '修改失败，请重试' }
      }
    }
  }
})
