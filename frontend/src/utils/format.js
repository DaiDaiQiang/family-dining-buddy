/**
 * 通用格式化工具
 */

/** 难度中文 */
const DIFFICULTY = { easy: '简单', medium: '中等', hard: '挑战' }
const DIFFICULTY_CLASS = { easy: 'chip--success', medium: 'chip--warning', hard: 'chip--danger' }
const MEAL = { breakfast: '早餐', lunch: '午餐', dinner: '晚餐', snack: '加餐' }
const MEAL_EMOJI = { breakfast: '🌅', lunch: '☀️', dinner: '🌙', snack: '🍪' }
const STATUS = { draft: '草稿', confirmed: '已确认', cooking: '烹饪中', served: '已开饭', cancelled: '已取消' }
const STATUS_CLASS = {
  draft: 'chip--info', confirmed: 'chip--brand', cooking: 'chip--warning',
  served: 'chip--success', cancelled: 'chip--danger'
}

export const difficultyLabel = (v) => DIFFICULTY[v] || v || ''
export const difficultyClass = (v) => DIFFICULTY_CLASS[v] || ''
export const mealLabel = (v) => MEAL[v] || v || ''
export const mealEmoji = (v) => MEAL_EMOJI[v] || '🍽️'
export const statusLabel = (v) => STATUS[v] || v || ''
export const statusClass = (v) => STATUS_CLASS[v] || ''

/** 时长 → 「45 分钟」/「1 小时 20 分」 */
export function durationText(min) {
  const m = Number(min) || 0
  if (m <= 0) return '免烹饪'
  if (m < 60) return `${m} 分钟`
  const h = Math.floor(m / 60)
  const r = m % 60
  return r ? `${h} 小时 ${r} 分` : `${h} 小时`
}

/** 日期格式化：支持 'YYYY-MM-DD' 或 ISO 串 */
export function dateText(v, withWeek = true) {
  if (!v) return ''
  const s = String(v).slice(0, 10)
  const [y, m, d] = s.split('-')
  if (!y) return s
  const week = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
  const dt = new Date(`${s}T00:00:00`)
  const wd = week[dt.getDay()]
  return withWeek ? `${Number(m)}月${Number(d)}日 ${wd}` : `${Number(m)}月${Number(d)}日`
}

/** 相对今天：今天 / 明天 / 昨天 / N月N日 */
export function relativeDay(v) {
  if (!v) return ''
  const s = String(v).slice(0, 10)
  const today = new Date()
  const target = new Date(`${s}T00:00:00`)
  const diff = Math.round((target - new Date(today.getFullYear(), today.getMonth(), today.getDate())) / 86400000)
  if (diff === 0) return '今天'
  if (diff === 1) return '明天'
  if (diff === -1) return '昨天'
  return dateText(v)
}

/** 评分保留一位小数 */
export function ratingText(v) {
  const n = Number(v)
  if (!n || Number.isNaN(n)) return '—'
  return n.toFixed(1)
}

/** 标签名串 → 数组 */
export function splitTags(v) {
  if (!v) return []
  if (Array.isArray(v)) return v
  return String(v).split(/[,，]/).map((s) => s.trim()).filter(Boolean)
}

/** 采购用量美化：去掉多余的 0 */
export function quantityText(v) {
  const n = Number(v)
  if (Number.isNaN(n)) return v
  return Number.isInteger(n) ? String(n) : n.toFixed(2).replace(/\.?0+$/, '')
}

/** 金额/数字千分位 */
export function num(n) {
  const v = Number(n) || 0
  return v.toLocaleString('zh-CN')
}
