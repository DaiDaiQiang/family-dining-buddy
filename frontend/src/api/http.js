import axios from 'axios'

/**
 * HTTP 客户端
 *
 * 统一约定后端响应结构：{ code, message, data }
 * - code === 0        成功，resolve(data)
 * - 其它              reject({ code, message })
 *
 * 鉴权：请求自动携带 `Authorization: Bearer <token>`（登录态见 stores/auth.js）；
 * 401/4010 时清登录态并跳转 /login。
 *
 * 开发期通过 vite proxy 走 /api，生产同源部署同样走 /api，无需改代码。
 */

const AUTH_KEY = 'fd_auth'

/** 读取登录令牌（localStorage 优先，其次 sessionStorage） */
export function getToken() {
  try {
    const raw = localStorage.getItem(AUTH_KEY) || sessionStorage.getItem(AUTH_KEY)
    return raw ? (JSON.parse(raw).token || '') : ''
  } catch {
    return ''
  }
}

/** 清登录态（401 时调用） */
function clearAuth() {
  localStorage.removeItem(AUTH_KEY)
  sessionStorage.removeItem(AUTH_KEY)
}

const http = axios.create({
  baseURL: import.meta.env.VITE_API_BASE || '/api',
  timeout: Number(import.meta.env.VITE_API_TIMEOUT) || 15000,
  headers: { 'Content-Type': 'application/json' }
})

http.interceptors.request.use((config) => {
  const token = getToken()
  if (token) config.headers.Authorization = `Bearer ${token}`
  return config
})

/** 401 → 清登录态跳登录页（登录接口自身除外，避免死循环） */
function handleUnauthorized(config) {
  clearAuth()
  const path = window.location.pathname
  if (!path.startsWith('/login')) {
    const redirect = encodeURIComponent(window.location.pathname + window.location.search)
    window.location.assign(`/login?redirect=${redirect}`)
  }
}

http.interceptors.response.use(
  (res) => {
    const body = res.data
    if (body && typeof body === 'object' && 'code' in body) {
      if (body.code === 0) return body.data
      const err = new Error(body.message || '请求失败')
      err.code = body.code
      err.payload = body
      if (body.code === 4010 && !String(res.config?.url || '').includes('/auth/')) {
        handleUnauthorized(res.config)
      }
      return Promise.reject(err)
    }
    return body
  },
  (error) => {
    const status = error.response?.status
    const data = error.response?.data
    const message =
      data?.message ||
      (status === 503 ? '数据库未就绪，请先导入建表脚本并检查 MySQL 服务'
        : status === 404 ? '接口不存在，请确认后端已启动且版本匹配'
        : status >= 500 ? '服务器开小差了，请稍后再试'
        : '网络异常，请检查后端服务是否已启动')

    const err = new Error(message)
    err.code = data?.code ?? status ?? -1
    err.httpStatus = status
    err.isNetwork = !error.response

    if (status === 401 && !String(error.config?.url || '').includes('/auth/')) {
      handleUnauthorized(error.config)
    }
    return Promise.reject(err)
  }
)

export default http
