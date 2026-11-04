/**
 * 生理周期计算
 *
 * 以「上次月经开始日」为锚点，按周期长度推算各阶段。
 * 黄体期长度固定取 14 天（医学常规假设），据此反推排卵日。
 *
 * 阶段优先级：月经期 > 经前期 > 排卵期 > 黄体期 > 卵泡期
 * 其中「经前期」= 预计月经来潮前 3 天，用于提前提醒。
 */

/** 默认黄体期长度（天），排卵日 = 周期长度 - 该值 */
export const LUTEAL_PHASE_DAYS = 14
/** 经前提醒提前天数 */
export const PREMENSTRUAL_ALERT_DAYS = 3
/** 排卵期窗口：排卵日前 5 天 ~ 后 4 天 */
export const FERTILE_BEFORE = 5
export const FERTILE_AFTER = 4

const DAY_MS = 86400000

/** 'YYYY-MM-DD' → 本地零点 Date；非法输入返回 null */
export function parseDate(v) {
  if (!v) return null
  const s = String(v).slice(0, 10)
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s)
  if (!m) return null
  const d = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]))
  return Number.isNaN(d.getTime()) ? null : d
}

/** Date → 'YYYY-MM-DD'（本地时区，避免 toISOString 的 UTC 偏移） */
export function toDateStr(d) {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

/** 当天零点 */
export function startOfDay(d = new Date()) {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate())
}

/** 在日期上加减天数，返回新 Date */
export function addDays(d, n) {
  const r = new Date(d.getFullYear(), d.getMonth(), d.getDate())
  r.setDate(r.getDate() + Number(n))
  return r
}

/** b - a 的整天数（a、b 均按零点计） */
export function daysBetween(a, b) {
  return Math.round((startOfDay(b) - startOfDay(a)) / DAY_MS)
}

/** 阶段元信息：文案、颜色类、emoji */
export const PHASES = {
  period:       { label: '月经期', emoji: '🩸', tone: 'danger' },
  premenstrual: { label: '经前期', emoji: '⏰', tone: 'warning' },
  fertile:      { label: '排卵期', emoji: '🌱', tone: 'success' },
  luteal:       { label: '黄体期', emoji: '🌙', tone: 'info' },
  follicular:   { label: '卵泡期', emoji: '🌤️', tone: 'brand' }
}

export function phaseMeta(phase) {
  return PHASES[phase] || PHASES.follicular
}

/** 夹到合理区间，防止非法配置导致计算错乱 */
function clampCycle(n, fallback, min, max) {
  const v = Number(n)
  if (!Number.isFinite(v) || v <= 0) return fallback
  return Math.min(max, Math.max(min, Math.round(v)))
}

/**
 * 计算某位成员在当前日期的周期状态
 *
 * @param {object} profile { lastPeriodStart, cycleLength, periodLength }
 * @param {Date}   [today] 基准日（默认当天）
 * @returns {object|null}  未记录月经开始日时返回 null
 */
export function computeCycleStatus(profile, today = new Date()) {
  const anchor = parseDate(profile?.lastPeriodStart)
  if (!anchor) return null

  const cycleLength = clampCycle(profile?.cycleLength, 28, 15, 60)
  const periodLength = Math.min(
    clampCycle(profile?.periodLength, 5, 1, 15),
    cycleLength - 1
  )

  const base = startOfDay(today)
  const diff = daysBetween(anchor, base)

  // 今天早于锚点时，向前回退整数个周期，保证 dayInCycle 落在 1..cycleLength
  const passed = Math.floor(diff / cycleLength)
  const dayInCycle = diff - passed * cycleLength + 1

  const currentStart = addDays(anchor, passed * cycleLength)
  const currentEnd = addDays(currentStart, periodLength - 1)
  const nextStart = addDays(currentStart, cycleLength)

  // 排卵日：下次月经前 14 天（周期第 cycleLength-14+1 天）
  const ovulationDay = Math.max(1, cycleLength - LUTEAL_PHASE_DAYS + 1)
  const ovulationDate = addDays(currentStart, ovulationDay - 1)
  const fertileStart = addDays(ovulationDate, -FERTILE_BEFORE)
  const fertileEnd = addDays(ovulationDate, FERTILE_AFTER)
  const lutealStart = addDays(fertileEnd, 1)

  const daysToNext = daysBetween(base, nextStart)
  const periodDay = dayInCycle <= periodLength ? dayInCycle : 0

  let phase = 'follicular'
  if (periodDay > 0) phase = 'period'
  else if (daysToNext <= PREMENSTRUAL_ALERT_DAYS) phase = 'premenstrual'
  else if (base >= fertileStart && base <= fertileEnd) phase = 'fertile'
  else if (base >= lutealStart) phase = 'luteal'

  return {
    phase,
    phaseLabel: phaseMeta(phase).label,
    phaseEmoji: phaseMeta(phase).emoji,
    tone: phaseMeta(phase).tone,
    cycleLength,
    periodLength,
    /** 当前是本周期的第几天（1 起） */
    dayInCycle,
    /** 经期第几天；非经期为 0 */
    periodDay,
    /** 距下次月经天数；已到来时为 0 */
    daysToNext: Math.max(0, daysToNext),
    /** 是否处于「经前 3 天」提醒窗口 */
    isPremenstrualAlert: daysToNext <= PREMENSTRUAL_ALERT_DAYS,
    cycleNo: passed + 1,
    currentStart: toDateStr(currentStart),
    currentEnd: toDateStr(currentEnd),
    nextStart: toDateStr(nextStart),
    ovulationDay,
    ovulationDate: toDateStr(ovulationDate),
    fertileStart: toDateStr(fertileStart),
    fertileEnd: toDateStr(fertileEnd),
    lutealStart: toDateStr(lutealStart),
    /** 预测可信度：周期越偏离常规越低，仅用于文案提示 */
    predictNote:
      cycleLength < 21 || cycleLength > 35
        ? '周期长度偏离常规范围，预测仅供参考'
        : ''
  }
}

/**
 * 生成给用户的提醒文案
 * @param {object} status computeCycleStatus 的返回值
 * @param {string} [name] 成员昵称
 */
export function reminderText(status, name = '') {
  if (!status) return ''
  const who = name ? `${name} · ` : ''
  switch (status.phase) {
    case 'period':
      return `${who}月经期第 ${status.periodDay} 天，注意保暖、避免生冷`
    case 'premenstrual':
      return status.daysToNext <= 0
        ? `${who}预计今天月经来潮，提前备好用品`
        : `${who}还有 ${status.daysToNext} 天来月经，提前备好用品`
    case 'fertile':
      return `${who}排卵期（${status.fertileStart.slice(5)} ~ ${status.fertileEnd.slice(5)}），易孕期请注意`
    case 'luteal':
      return `${who}黄体期，情绪与食欲易波动，规律作息`
    default:
      return `${who}卵泡期，状态较好，适合安排运动`
  }
}
