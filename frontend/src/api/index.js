/**
 * API 层（纯在线模式）
 *
 * 本前端不允许离线使用：
 * - 所有请求直连后端，后端不可达时直接向上抛出错误；
 * - 绝不降级到内存 Mock / 离线演示数据（相关 Mock 实现已从运行时链路移除）；
 * - `state.online === false` 仅用于驱动 App.vue 的「后端不可达」阻断页。
 */
import http from './http'

const state = {
  online: null,      // null = 未探测，true = 后端可用，false = 后端不可达
  offlineReason: '',
  listeners: new Set()
}

function emit() {
  state.listeners.forEach((fn) => fn({ online: state.online, offlineReason: state.offlineReason }))
}

export function onModeChange(fn) {
  state.listeners.add(fn)
  return () => state.listeners.delete(fn)
}

export function getMode() {
  return { online: state.online, offlineReason: state.offlineReason }
}

/** 探测后端健康状态（仅用于界面状态展示，不决定是否降级） */
export async function probeBackend() {
  try {
    const res = await http.get('/health')
    const dbOk = res?.database === 'connected'
    state.online = true
    state.offlineReason = dbOk ? '' : '后端已启动，但数据库尚未连通'
  } catch (e) {
    state.online = false
    state.offlineReason = e?.message || '无法连接后端服务'
  }
  emit()
  return state.online
}

/** 统一执行：只走真实接口；失败时标记离线状态并原样抛出，绝不使用演示数据 */
async function call(path, config = {}) {
  try {
    return config.method === 'get' || !config.method
      ? await http.get(path, { params: config.params })
      : await http.request({ url: path, method: config.method, data: config.data })
  } catch (e) {
    if (e.isNetwork || e.httpStatus === 503 || e.code === 5030) {
      state.online = false
      state.offlineReason = e.message || '无法连接后端服务'
      emit()
    }
    throw e
  }
}

// =====================================================================
//  元数据
// =====================================================================
export const api = {
  meta: () => call('/meta'),

  // ---------- 登录鉴权 ----------
  auth: {
    /** 登录页成员卡片（含 has_password，公开接口） */
    members: () => call('/auth/members'),
    login: (data) => call('/auth/login', { method: 'post', data }),
    me: () => call('/auth/me'),
    changePassword: (data) => call('/auth/password', { method: 'put', data }),
    resetPassword: (data) => call('/auth/reset-password', { method: 'post', data })
  },

  // ---------- 家庭账本 ----------
  ledger: {
    list: (params) => call('/ledger', { params }),
    summary: (params) => call('/ledger/summary', { params }),
    create: (data) => call('/ledger', { method: 'post', data }),
    update: (id, data) => call(`/ledger/${id}`, { method: 'put', data }),
    remove: (id) => call(`/ledger/${id}`, { method: 'delete' })
  },

  // ---------- 快递取件码 ----------
  parcels: {
    list: (params) => call('/parcels', { params }),
    create: (data) => call('/parcels', { method: 'post', data }),
    update: (id, data) => call(`/parcels/${id}`, { method: 'put', data }),
    remove: (id) => call(`/parcels/${id}`, { method: 'delete' }),
    clearPicked: () => call('/parcels/picked', { method: 'delete' })
  },

  // ---------- 健康档案（家庭共享） ----------
  health: {
    profiles: () => call('/health/profiles'),
    saveProfile: (memberId, data) =>
      call(`/health/profiles/${memberId}`, { method: 'put', data })
  },

  // ---------- 成员同步数据（JSON blob，学习模块等） ----------
  sync: {
    get: (key) => call(`/sync/${key}`),
    put: (key, payload, expectedVersion) =>
      call(`/sync/${key}`, {
        method: 'put',
        data: expectedVersion === undefined
          ? { payload }
          : { payload, expected_version: expectedVersion }
      })
  },

  // ---------- 站内提醒（阶段 2） ----------
  reminders: {
    list: (params) => call('/reminders', { params }),
    sync: () => call('/reminders/sync', { method: 'post' }),
    update: (id, data) => call(`/reminders/${id}`, { method: 'put', data })
  },

  // ---------- 系统版本更新记录 ----------
  versions: {
    list: () => call('/versions'),
    create: (data) => call('/versions', { method: 'post', data })
  },

  // ---------- 耗材维护与消耗统计（阶段 3） ----------
  consumables: {
    list: (params) => call('/consumables', { params }),
    low: () => call('/consumables/low'),
    stats: () => call('/consumables/stats'),
    create: (data) => call('/consumables', { method: 'post', data }),
    update: (id, data) => call(`/consumables/${id}`, { method: 'put', data }),
    remove: (id) => call(`/consumables/${id}`, { method: 'delete' }),
    addConsumption: (id, data) =>
      call(`/consumables/${id}/consumption`, { method: 'post', data }),
    consumption: (id) => call(`/consumables/${id}/consumption`)
  },


  // ---------- 统计 ----------
  stats: {
    overview: () => call('/stats/overview'),
    popular: (params) => call('/stats/popular', { params }),
    categories: () => call('/stats/categories'),
    members: () => call('/stats/members'),
    memberTaste: (id) => call(`/stats/members/${id}/taste`),
    calendar: (params) => call('/stats/calendar', { params }),
    suggestion: (params) => call('/stats/suggestion', { params })
  },

  // ---------- 菜谱 ----------
  recipes: {
    list: (params) => call('/recipes', { params }),
    facets: () => call('/recipes/facets'),
    random: (params) => call('/recipes/random', { params }),
    detail: (id) => call(`/recipes/${id}`),
    create: (data) => call('/recipes', { method: 'post', data }),
    update: (id, data) => call(`/recipes/${id}`, { method: 'put', data }),
    remove: (id, permanent = false) =>
      call(`/recipes/${id}`, { method: 'delete', params: { permanent } }),
    toggleFavorite: (id, memberId) =>
      call(`/recipes/${id}/favorite/${memberId}`, { method: 'post' }),
    /** 下厨房导入：抓取并解析链接，返回预览（不落库） */
    importPreview: (url) => call('/recipes/import/preview', { method: 'post', data: { url } }),
    /** 下厨房导入：确认后落库建档 */
    importCreate: (data) => call('/recipes/import', { method: 'post', data }),
    /** 上传封面图：dataURL → 后端落盘，返回 { url }（仅在线可用） */
    uploadCover: (dataUrl) => http.post('/uploads', { data: dataUrl }, { timeout: 30000 })
  },

  // ---------- 成员 ----------
  members: {
    list: (params) => call('/members', { params }),
    get: (id) => call(`/members/${id}`),
    create: (data) => call('/members', { method: 'post', data }),
    update: (id, data) => call(`/members/${id}`, { method: 'put', data }),
    remove: (id) => call(`/members/${id}`, { method: 'delete' })
  },

  // ---------- 图片上传 ----------
  uploads: {
    /** dataURL → 后端落盘，返回 { url }（头像/封面共用） */
    create: (dataUrl) => http.post('/uploads', { data: dataUrl }, { timeout: 30000 })
  },

  // ---------- 食材 ----------
  ingredients: {
    list: (params) => call('/ingredients', { params }),
    categories: () => call('/ingredients/categories')
  },

  // ---------- 标签 ----------
  tags: {
    list: () => call('/tags')
  },

  // ---------- 订单 ----------
  orders: {
    list: (params) => call('/orders', { params }),
    today: () => call('/orders/today'),
    detail: (id) => call(`/orders/${id}`),
    create: (data) => call('/orders', { method: 'post', data }),
    update: (id, data) => call(`/orders/${id}`, { method: 'put', data }),
    confirm: (id) => call(`/orders/${id}/confirm`, { method: 'post' }),
    cancel: (id) => call(`/orders/${id}/cancel`, { method: 'post' }),
    remove: (id) => call(`/orders/${id}`, { method: 'delete' }),
    addItem: (id, data) => call(`/orders/${id}/items`, { method: 'post', data }),
    updateItem: (itemId, data) => call(`/orders/items/${itemId}`, { method: 'put', data }),
    removeItem: (itemId) => call(`/orders/items/${itemId}`, { method: 'delete' }),
    rateItem: (itemId, rating) => call(`/orders/items/${itemId}/rating`, { method: 'post', data: { rating } }),
    menu: (id) => call(`/orders/${id}/menu`),
    recipes: (id) => call(`/orders/${id}/recipes`),
    shoppingList: (id) => call(`/orders/${id}/shopping-list`),
    toggleShopping: (itemId, purchased) =>
      call(`/orders/shopping/${itemId}`, { method: 'put', data: { is_purchased: purchased } }),
    reorder: (id) => call(`/orders/${id}/reorder`, { method: 'post' })
  }
}

export default api
