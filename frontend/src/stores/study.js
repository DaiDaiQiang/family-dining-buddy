import { defineStore } from 'pinia'
import { api } from '@/api'

/**
 * 学习板块
 *
 * 覆盖三块内容：刷题、英语学习、知识点笔记。
 * 数据存服务端 member_data（JSON blob 整包同步），localStorage 做缓存与降级展示。
 *
 * 全部内容都可以通过一份**固定格式的 JSON** 批量导入 / 导出，
 * 导出的文件可以直接再导回来（往返一致）。
 * 格式说明见 docs/study-json-format.md，模板见 frontend/public/study-template.json。
 */

/** 防抖推送定时器（模块级，避免放进响应式 state） */
let pushTimer = null
const STORAGE_KEY = 'fd_study'
const SEQ_KEY = 'fd_study_seq'

/** 导入包的类型标识与版本号，用于后期兼容升级 */
export const STUDY_SCHEMA = 'family-dining.study'
export const STUDY_VERSION = 1

/** 未指定题库时的默认归类 */
export const DEFAULT_BANK = '默认题库'

/* ------------------------------------------------------------------ *
 *  字典
 * ------------------------------------------------------------------ */

/** 题型 */
export const QUESTION_TYPES = [
  { value: 'single', label: '单选题', emoji: '🅰️' },
  { value: 'multiple', label: '多选题', emoji: '🔠' },
  { value: 'judge', label: '判断题', emoji: '✅' },
  { value: 'fill', label: '填空题', emoji: '✏️' },
  { value: 'answer', label: '问答题', emoji: '💬' }
]

/** 难度 */
export const DIFFICULTIES = [
  { value: 'easy', label: '基础', tone: 'success' },
  { value: 'medium', label: '提高', tone: 'warning' },
  { value: 'hard', label: '拔高', tone: 'danger' }
]

/** 单词学习状态 */
export const WORD_STATUSES = [
  { value: 'new', label: '未学', emoji: '🆕', tone: '' },
  { value: 'learning', label: '学习中', emoji: '📖', tone: 'warning' },
  { value: 'mastered', label: '已掌握', emoji: '✅', tone: 'success' }
]

/** 常用学科预设（导入时可自带 subjects 覆盖） */
export const SUBJECT_PRESETS = [
  { value: 'chinese', name: '语文', emoji: '📖' },
  { value: 'math', name: '数学', emoji: '🔢' },
  { value: 'english', name: '英语', emoji: '🔤' },
  { value: 'physics', name: '物理', emoji: '⚛️' },
  { value: 'chemistry', name: '化学', emoji: '🧪' },
  { value: 'biology', name: '生物', emoji: '🧬' },
  { value: 'history', name: '历史', emoji: '🏛️' },
  { value: 'geography', name: '地理', emoji: '🌏' },
  { value: 'politics', name: '道德与法治', emoji: '⚖️' },
  { value: 'other', name: '其他', emoji: '📘' }
]

const JUDGE_OPTIONS = [
  { key: 'T', text: '正确' },
  { key: 'F', text: '错误' }
]

export function questionTypeMeta(v) {
  return QUESTION_TYPES.find((t) => t.value === v) || QUESTION_TYPES[0]
}
export function difficultyMeta(v) {
  return DIFFICULTIES.find((d) => d.value === v) || DIFFICULTIES[0]
}
export function wordStatusMeta(v) {
  return WORD_STATUSES.find((s) => s.value === v) || WORD_STATUSES[0]
}

/* ------------------------------------------------------------------ *
 *  小工具
 * ------------------------------------------------------------------ */

function toArray(v) {
  if (v == null || v === '') return []
  return Array.isArray(v) ? v : [v]
}

/** 逗号 / 顿号 / 分号分隔的字符串 → 数组 */
function splitTags(v) {
  if (Array.isArray(v)) return v.map((x) => String(x).trim()).filter(Boolean)
  return String(v || '')
    .split(/[,，、;；|]/)
    .map((x) => x.trim())
    .filter(Boolean)
}

function nowStr() {
  return new Date().toISOString().slice(0, 19).replace('T', ' ')
}

/** 判分用：忽略大小写与首尾空格；填空按 | 分隔多个可接受答案 */
function looseEq(a, b) {
  return String(a).trim().toLowerCase().replace(/\s+/g, '') === String(b).trim().toLowerCase().replace(/\s+/g, '')
}

/* ------------------------------------------------------------------ *
 *  记录规范化（导入与手工录入共用，保证库里字段永远齐全）
 * ------------------------------------------------------------------ */

function normalizeSubject(raw) {
  const value = String(raw.value ?? raw.id ?? raw.key ?? '').trim()
  const name = String(raw.name ?? raw.label ?? '').trim()
  if (!value && !name) return null
  return {
    value: value || name,
    name: name || value,
    emoji: raw.emoji || '📘'
  }
}

function normalizeOptions(type, rawOptions) {
  const list = (Array.isArray(rawOptions) ? rawOptions : [])
    .map((o, i) => {
      if (typeof o === 'string') return { key: String.fromCharCode(65 + i), text: o.trim() }
      const key = String(o.key ?? o.value ?? String.fromCharCode(65 + i)).trim().toUpperCase()
      const text = String(o.text ?? o.label ?? o.content ?? '').trim()
      return { key, text }
    })
    .filter((o) => o.key && o.text)
  if (type === 'judge' && !list.length) return JUDGE_OPTIONS.map((o) => ({ ...o }))
  if (type === 'judge') return list.slice(0, 2)
  if (type === 'fill' || type === 'answer') return []
  return list
}

/** 统一答案为字符串数组；判断题兼容 true/false、对/错、T/F 等写法 */
function normalizeAnswer(type, raw, options) {
  let list = toArray(raw)
    .map((x) => String(x).trim())
    .filter((x) => x !== '')

  if (type === 'judge') {
    list = list.map((v) => {
      const s = v.toLowerCase()
      if (['true', 't', 'a', 'y', 'yes', '1', '对', '正确', '√', '是'].includes(s)) return 'T'
      if (['false', 'f', 'b', 'n', 'no', '0', '错', '错误', '×', '否'].includes(s)) return 'F'
      return v.toUpperCase()
    })
    if (!list.length) list = ['T']
    return [list[0]]
  }

  if (type === 'single' || type === 'multiple') {
    list = list.map((v) => v.toUpperCase())
    if (options.length) {
      const keys = new Set(options.map((o) => o.key))
      list = list.filter((v) => keys.has(v))
    }
    list = [...new Set(list)]
    if (type === 'single') return list.slice(0, 1)
    return list.sort()
  }

  // 填空 / 问答：保留原文，可用 | 表示多个可接受答案
  return list
}

function normalizeQuestion(raw, fallbackId) {
  const stem = String(raw.stem ?? raw.question ?? raw.title ?? '').trim()
  if (!stem) return { error: '缺少题干（stem）' }

  const type = QUESTION_TYPES.some((t) => t.value === raw.type) ? raw.type : 'single'
  const options = normalizeOptions(type, raw.options)
  if ((type === 'single' || type === 'multiple') && options.length < 2) {
    return { error: `${questionTypeMeta(type).label}至少需要 2 个选项（options）` }
  }

  const answer = normalizeAnswer(type, raw.answer ?? raw.answers ?? raw.correct ?? raw.correct_answer, options)
  if ((type === 'single' || type === 'multiple' || type === 'judge') && !answer.length) {
    return { error: '缺少正确答案（answer）' }
  }
  if (type === 'multiple' && answer.length < 2) {
    return { error: '多选题的正确答案至少 2 个' }
  }

  return {
    id: String(raw.id ?? fallbackId).trim(),
    bank: String(raw.bank ?? raw.bank_name ?? '').trim() || DEFAULT_BANK,
    subject: String(raw.subject ?? raw.subject_value ?? '').trim(),
    type,
    difficulty: DIFFICULTIES.some((d) => d.value === raw.difficulty) ? raw.difficulty : 'easy',
    stem,
    options,
    answer,
    analysis: String(raw.analysis ?? raw.explanation ?? raw.parse ?? '').trim(),
    tags: splitTags(raw.tags ?? raw.tag_names),
    source: String(raw.source ?? raw.origin ?? '').trim(),
    score: Number(raw.score) > 0 ? Number(raw.score) : 1
  }
}

function normalizeWord(raw, fallbackId) {
  const word = String(raw.word ?? raw.en ?? raw.english ?? '').trim()
  if (!word) return { error: '缺少单词（word）' }
  return {
    id: String(raw.id ?? fallbackId).trim(),
    word,
    phonetic: String(raw.phonetic ?? raw.pronunciation ?? '').trim(),
    meaning: String(raw.meaning ?? raw.cn ?? raw.translation ?? '').trim(),
    example: String(raw.example ?? raw.sentence ?? '').trim(),
    example_cn: String(raw.example_cn ?? raw.sentence_cn ?? '').trim(),
    level: String(raw.level ?? raw.grade ?? '').trim(),
    unit: String(raw.unit ?? raw.lesson ?? '').trim(),
    tags: splitTags(raw.tags)
  }
}

function normalizeNote(raw, fallbackId) {
  const title = String(raw.title ?? raw.name ?? '').trim()
  const content = String(raw.content ?? raw.body ?? raw.text ?? '').trim()
  if (!title && !content) return { error: '标题（title）与内容（content）不能同时为空' }
  return {
    id: String(raw.id ?? fallbackId).trim(),
    subject: String(raw.subject ?? '').trim(),
    title: title || content.slice(0, 20),
    content,
    key_points: toArray(raw.key_points ?? raw.points ?? raw.keywords).map((x) => String(x).trim()).filter(Boolean),
    tags: splitTags(raw.tags),
    is_exam_point: !!(raw.is_exam_point ?? raw.exam_point ?? raw.important),
    updated_at: String(raw.updated_at ?? '').trim() || nowStr()
  }
}

function normalizeProgress(raw) {
  const p = raw && typeof raw === 'object' ? raw : {}
  const dict = (src) => {
    const out = {}
    if (src && typeof src === 'object') {
      Object.keys(src).forEach((k) => {
        if (src[k] && typeof src[k] === 'object') out[k] = { ...src[k] }
      })
    }
    return out
  }
  return {
    questions: dict(p.questions ?? p.quiz),
    words: dict(p.words ?? p.english),
    notes: dict(p.notes)
  }
}

/** 题库元数据规范化：名称唯一，附带展示图片与说明 */
function normalizeBank(raw) {
  const name = String(raw.name ?? raw.bank ?? raw.title ?? '').trim()
  if (!name) return null
  return {
    name,
    cover: String(raw.cover ?? raw.image ?? raw.cover_url ?? '').trim(),
    desc: String(raw.desc ?? raw.description ?? raw.remark ?? '').trim()
  }
}

function emptyState() {
  return {
    subjects: [],
    banks: [],
    questions: [],
    english: [],
    notes: [],
    progress: { questions: {}, words: {}, notes: {} }
  }
}

function readStore() {
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE_KEY))
    if (!raw || typeof raw !== 'object') return emptyState()
    const s = emptyState()
    if (Array.isArray(raw.subjects)) s.subjects = raw.subjects.map(normalizeSubject).filter(Boolean)
    if (Array.isArray(raw.banks)) {
      s.banks = raw.banks.map(normalizeBank).filter(Boolean)
    }
    if (Array.isArray(raw.questions)) {
      s.questions = raw.questions
        .filter((q) => q && q.id && q.stem)
        // 旧数据没有 bank 字段，统一归到默认题库
        .map((q) => ({ ...q, bank: String(q.bank || '').trim() || DEFAULT_BANK }))
    }
    if (Array.isArray(raw.english)) s.english = raw.english.filter((w) => w && w.id && w.word)
    if (Array.isArray(raw.notes)) s.notes = raw.notes.filter((n) => n && n.id)
    s.progress = normalizeProgress(raw.progress)
    return s
  } catch {
    return emptyState()
  }
}

/* ------------------------------------------------------------------ *
 *  导入包解析
 * ------------------------------------------------------------------ */

/**
 * 解析导入文本。
 * 接受三种写法：
 *   1. 完整包：{ version, type, data: { subjects, questions, english, notes } }
 *   2. 裸 data：{ subjects, questions, english, notes }
 *   3. 单类数组：[ ... ]（按字段特征猜是题目 / 单词 / 知识点）
 */
export function parseStudyPayload(text) {
  let json
  try {
    json = JSON.parse(text)
  } catch (e) {
    return { ok: false, error: `JSON 解析失败：${e.message}` }
  }
  if (!json || typeof json !== 'object') {
    return { ok: false, error: 'JSON 顶层必须是对象或数组' }
  }

  if (json.version != null && Number(json.version) > STUDY_VERSION) {
    return {
      ok: false,
      error: `模板版本 v${json.version} 高于当前程序支持的 v${STUDY_VERSION}，请先升级程序`
    }
  }

  const body = Array.isArray(json) ? guessByArray(json) : json.data && typeof json.data === 'object' ? json.data : json
  return {
    ok: true,
    payload: {
      subjects: Array.isArray(body.subjects) ? body.subjects : [],
      banks: Array.isArray(body.banks) ? body.banks : [],
      questions: Array.isArray(body.questions) ? body.questions : [],
      english: Array.isArray(body.english ?? body.words) ? body.english ?? body.words : [],
      notes: Array.isArray(body.notes ?? body.knowledge) ? body.notes ?? body.knowledge : [],
      progress: body.progress && typeof body.progress === 'object' ? body.progress : null
    },
    meta: {
      version: Number(json.version) || STUDY_VERSION,
      type: json.type || '',
      exported_at: json.exported_at || ''
    }
  }
}

/** 裸数组时按字段特征归类 */
function guessByArray(list) {
  const out = { subjects: [], questions: [], english: [], notes: [] }
  list.forEach((item) => {
    if (!item || typeof item !== 'object') return
    if (item.word || item.en) out.english.push(item)
    else if (item.stem || item.question) out.questions.push(item)
    else if (item.title || item.content) out.notes.push(item)
    else if (item.name && (item.value || item.key)) out.subjects.push(item)
  })
  return out
}

/* ------------------------------------------------------------------ *
 *  Store
 * ------------------------------------------------------------------ */

export const useStudyStore = defineStore('study', {
  state: () => ({
    ...readStore(),
    seq: Number(localStorage.getItem(SEQ_KEY)) || 1,
    /** 最近一次导入结果，供界面展示 */
    lastImport: null
  }),

  getters: {
    /** 学科字典：预设 + 数据里出现过的学科，去重 */
    allSubjects(s) {
      const map = new Map()
      SUBJECT_PRESETS.forEach((x) => map.set(x.value, { ...x }))
      s.subjects.forEach((x) => map.set(x.value, { ...x }))
      // 数据里引用了但字典没有的学科，用 value 兜底显示
      const seen = new Set(map.keys())
      ;[...s.questions, ...s.notes].forEach((r) => {
        const v = String(r.subject || '').trim()
        if (v && !seen.has(v)) map.set(v, { value: v, name: v, emoji: '📘' })
      })
      return [...map.values()]
    },

    /** 学科 value → 展示信息（含 emoji）；字典没有时用 value 兜底 */
    subjectMeta() {
      return (v) =>
        this.allSubjects.find((x) => x.value === v) || { value: v || '', name: v || '未分类', emoji: '📘' }
    },

    subjectName() {
      return (v) => this.subjectMeta(v).name
    },

    /** 数据里真实用到的学科（含题量），用于筛选栏 */
    usedSubjects(s) {
      const tally = new Map()
      ;[...s.questions, ...s.notes].forEach((r) => {
        const v = String(r.subject || '').trim()
        if (!v) return
        tally.set(v, (tally.get(v) || 0) + 1)
      })
      return [...tally.entries()]
        .map(([value, count]) => ({ ...this.subjectMeta(value), count }))
        .sort((a, b) => b.count - a.count)
    },

    /** 错题：最近一次做错且尚未订正 */
    wrongQuestions(s) {
      return s.questions.filter((q) => s.progress.questions[q.id]?.last_result === 'wrong')
    },

    /** 题库列表（含题量与元数据），刷题页选库用；按名称排序 */
    questionBanks(s) {
      // 先按名称统计题量，并合并元数据（banks 里的图片/说明）
      const map = new Map()
      s.banks.forEach((bk) => {
        map.set(bk.name, { name: bk.name, cover: bk.cover, desc: bk.desc, count: 0 })
      })
      s.questions.forEach((q) => {
        const b = String(q.bank || '').trim() || DEFAULT_BANK
        if (!map.has(b)) map.set(b, { name: b, cover: '', desc: '', count: 0 })
        map.get(b).count += 1
      })
      // 只保留仍有题目或带元数据的题库；纯元数据（0 题）也展示，便于轮盘切换
      return [...map.values()].sort((a, b) => a.name.localeCompare(b.name, 'zh-Hans-CN'))
    },

    /** 还没做过的题 */
    undoneQuestions(s) {
      return s.questions.filter((q) => !s.progress.questions[q.id])
    },

    /** 单词等级 / 单元字典 */
    wordLevels(s) {
      const levels = new Map()
      const units = new Map()
      s.english.forEach((w) => {
        if (w.level) levels.set(w.level, (levels.get(w.level) || 0) + 1)
        if (w.unit) units.set(w.unit, (units.get(w.unit) || 0) + 1)
      })
      return {
        levels: [...levels.entries()].map(([name, count]) => ({ name, count })),
        units: [...units.entries()].map(([name, count]) => ({ name, count }))
      }
    },

    /** 知识点标签 */
    noteTags(s) {
      const tally = new Map()
      s.notes.forEach((n) => n.tags.forEach((t) => tally.set(t, (tally.get(t) || 0) + 1)))
      return [...tally.entries()].map(([name, count]) => ({ name, count })).sort((a, b) => b.count - a.count)
    },

    /** 首页概览统计 */
    stats(s) {
      const answered = s.questions.filter((q) => s.progress.questions[q.id])
      const attempts = Object.values(s.progress.questions).reduce((n, p) => n + (Number(p.attempts) || 0), 0)
      const correct = Object.values(s.progress.questions).reduce((n, p) => n + (Number(p.correct) || 0), 0)
      const words = s.english
      const masteredWords = words.filter((w) => s.progress.words[w.id]?.status === 'mastered').length
      const learningWords = words.filter((w) => s.progress.words[w.id]?.status === 'learning').length
      const masteredNotes = s.notes.filter((n) => s.progress.notes[n.id]?.mastered).length
      return {
        questions: {
          total: s.questions.length,
          done: answered.length,
          wrong: this.wrongQuestions.length,
          attempts,
          accuracy: attempts ? Math.round((correct / attempts) * 100) : 0
        },
        english: {
          total: words.length,
          learning: learningWords,
          mastered: masteredWords,
          fresh: words.length - learningWords - masteredWords
        },
        notes: {
          total: s.notes.length,
          mastered: masteredNotes,
          examPoints: s.notes.filter((n) => n.is_exam_point).length
        }
      }
    },

    /** 数据总量，用于判断是否空库 */
    isEmpty(s) {
      return !s.questions.length && !s.english.length && !s.notes.length
    }
  },

  actions: {
    persist() {
      try {
        const s = this.$state
        localStorage.setItem(
          STORAGE_KEY,
          JSON.stringify({
            subjects: s.subjects,
            banks: s.banks,
            questions: s.questions,
            english: s.english,
            notes: s.notes,
            progress: s.progress
          })
        )
        localStorage.setItem(SEQ_KEY, String(s.seq))
      } catch (e) {
        console.warn('学习数据本地缓存失败', e)
      }
      this.schedulePush()
    },

    /** 整包载荷（含 seq，跨设备不撞 id） */
    bundle() {
      const s = this.$state
      return {
        subjects: s.subjects,
        banks: s.banks,
        questions: s.questions,
        english: s.english,
        notes: s.notes,
        progress: s.progress,
        seq: s.seq
      }
    },

    /** 防抖推送：连续多次变更只发一次请求 */
    schedulePush() {
      if (pushTimer) clearTimeout(pushTimer)
      pushTimer = setTimeout(() => {
        pushTimer = null
        this.push()
      }, 800)
    },

    /** 推送整包到服务端（同步失败仅告警，本地数据不丢） */
    async push() {
      try {
        await api.sync.put('study', this.bundle())
      } catch (e) {
        console.warn('学习数据同步失败', e)
      }
    },

    /**
     * 从服务端拉整包并覆盖本地。
     * 服务端为空时把本地数据推上去（首次启用的迁移路径）。
     */
    async pull() {
      try {
        const res = await api.sync.get('study')
        const p = res && res.payload
        if (p) {
          localStorage.setItem(
            STORAGE_KEY,
            JSON.stringify({
              subjects: p.subjects,
              banks: p.banks,
              questions: p.questions,
              english: p.english,
              notes: p.notes,
              progress: p.progress
            })
          )
          if (p.seq) localStorage.setItem(SEQ_KEY, String(p.seq))
          const s = readStore()
          this.subjects = s.subjects
          this.banks = s.banks
          this.questions = s.questions
          this.english = s.english
          this.notes = s.notes
          this.progress = s.progress
          if (p.seq) this.seq = Number(p.seq) || this.seq
        } else {
          await this.push()
        }
      } catch (e) {
        console.warn('学习数据拉取失败', e)
      }
    },

    nextId(prefix) {
      this.seq += 1
      return `${prefix}-${this.seq}`
    },

    /* ---------------- 学科 ---------------- */

    upsertSubject(raw) {
      const s = normalizeSubject(raw)
      if (!s) return null
      const i = this.subjects.findIndex((x) => x.value === s.value)
      if (i >= 0) this.subjects[i] = { ...this.subjects[i], ...s }
      else this.subjects.push(s)
      return s
    },

    /* ---------------- 刷题 ---------------- */

    /**
     * 记录一次作答
     * @param {string} id 题目 id
     * @param {boolean} isCorrect 是否答对
     */
    recordAnswer(id, isCorrect) {
      const key = String(id)
      const prev = this.progress.questions[key] || { attempts: 0, correct: 0, wrong: 0 }
      this.progress.questions[key] = {
        attempts: (Number(prev.attempts) || 0) + 1,
        correct: (Number(prev.correct) || 0) + (isCorrect ? 1 : 0),
        wrong: (Number(prev.wrong) || 0) + (isCorrect ? 0 : 1),
        last_result: isCorrect ? 'right' : 'wrong',
        last_at: nowStr()
      }
      this.persist()
      return this.progress.questions[key]
    },

    /** 从错题本移除（订正后） */
    clearWrong(id) {
      const p = this.progress.questions[String(id)]
      if (p) {
        p.last_result = 'right'
        p.last_at = nowStr()
      }
      this.persist()
    },

    saveQuestion(raw) {
      const q = normalizeQuestion(raw, raw.id || this.nextId('q'))
      if (q.error) throw new Error(q.error)
      if (raw.subject) this.upsertSubject({ value: raw.subject, name: raw.subject_name || raw.subject })
      const i = this.questions.findIndex((x) => x.id === q.id)
      if (i >= 0) this.questions[i] = { ...this.questions[i], ...q }
      else this.questions.push(q)
      this.persist()
      return q
    },

    removeQuestion(id) {
      const i = this.questions.findIndex((x) => x.id === String(id))
      if (i < 0) throw new Error('题目不存在')
      this.questions.splice(i, 1)
      delete this.progress.questions[String(id)]
      this.persist()
    },

    /* ---------------- 英语 ---------------- */

    setWordStatus(id, status) {
      const value = WORD_STATUSES.some((x) => x.value === status) ? status : 'new'
      const prev = this.progress.words[String(id)] || {}
      this.progress.words[String(id)] = {
        ...prev,
        status: value,
        reviews: (Number(prev.reviews) || 0) + 1,
        last_at: nowStr()
      }
      this.persist()
      return value
    },

    /** 复习一次：未学 → 学习中；已掌握保持不变；累计复习次数 */
    reviewWord(id) {
      const key = String(id)
      const prev = this.progress.words[key] || {}
      this.progress.words[key] = {
        ...prev,
        status: prev.status === 'mastered' ? 'mastered' : 'learning',
        reviews: (Number(prev.reviews) || 0) + 1,
        last_at: nowStr()
      }
      this.persist()
    },

    wordStatus(id) {
      return this.progress.words[String(id)]?.status || 'new'
    },

    /**
     * 记录一次单词练习结果（与刷题进度分开统计）
     * @param {string} id 单词 id
     * @param {boolean} isCorrect 是否答对
     */
    recordWordAnswer(id, isCorrect) {
      const key = String(id)
      const prev = this.progress.words[key] || { status: 'new', reviews: 0, attempts: 0, correct: 0 }
      this.progress.words[key] = {
        ...prev,
        attempts: (Number(prev.attempts) || 0) + 1,
        correct: (Number(prev.correct) || 0) + (isCorrect ? 1 : 0),
        last_result: isCorrect ? 'right' : 'wrong',
        last_at: nowStr()
      }
      this.persist()
      return this.progress.words[key]
    },

    saveWord(raw) {
      const w = normalizeWord(raw, raw.id || this.nextId('w'))
      if (w.error) throw new Error(w.error)
      const i = this.english.findIndex((x) => x.id === w.id)
      if (i >= 0) this.english[i] = { ...this.english[i], ...w }
      else this.english.push(w)
      this.persist()
      return w
    },

    removeWord(id) {
      const i = this.english.findIndex((x) => x.id === String(id))
      if (i < 0) throw new Error('单词不存在')
      this.english.splice(i, 1)
      delete this.progress.words[String(id)]
      this.persist()
    },

    /* ---------------- 知识点 ---------------- */

    toggleNoteMastered(id) {
      const key = String(id)
      const prev = this.progress.notes[key] || { views: 0, mastered: false }
      this.progress.notes[key] = {
        views: Number(prev.views) || 0,
        mastered: !prev.mastered,
        last_at: nowStr()
      }
      this.persist()
      return this.progress.notes[key].mastered
    },

    viewNote(id) {
      const key = String(id)
      const prev = this.progress.notes[key] || { views: 0, mastered: false }
      this.progress.notes[key] = { ...prev, views: (Number(prev.views) || 0) + 1, last_at: nowStr() }
      this.persist()
    },

    saveNote(raw) {
      const n = normalizeNote(raw, raw.id || this.nextId('n'))
      if (n.error) throw new Error(n.error)
      if (raw.subject) this.upsertSubject({ value: raw.subject, name: raw.subject_name || raw.subject })
      const i = this.notes.findIndex((x) => x.id === n.id)
      if (i >= 0) this.notes[i] = { ...this.notes[i], ...n, updated_at: nowStr() }
      else this.notes.push(n)
      this.persist()
      return n
    },

    removeNote(id) {
      const i = this.notes.findIndex((x) => x.id === String(id))
      if (i < 0) throw new Error('知识点不存在')
      this.notes.splice(i, 1)
      delete this.progress.notes[String(id)]
      this.persist()
    },

    /* ---------------- 导入 / 导出 ---------------- */

    /**
     * 查询指定题库当前题量（0 = 不存在）
     * @param {string} bankName
     */
    bankQuestionCount(bankName) {
      const name = String(bankName || '').trim() || DEFAULT_BANK
      return this.questions.filter((q) => (String(q.bank || '').trim() || DEFAULT_BANK) === name).length
    },

    /** 题库元数据是否存在（按名称精确匹配，名称唯一） */
    bankExists(bankName) {
      const name = String(bankName || '').trim()
      if (!name) return false
      return this.banks.some((b) => b.name === name)
    },

    /**
     * 写入/更新题库元数据（名称唯一；不存在则新增，存在则覆盖图片与说明）
     * @param {{name:string, cover?:string, desc?:string}} raw
     */
    saveBank(raw) {
      const bk = normalizeBank(raw)
      if (!bk) throw new Error('题库名称不能为空')
      const i = this.banks.findIndex((b) => b.name === bk.name)
      if (i >= 0) this.banks[i] = { ...this.banks[i], ...bk }
      else this.banks.push(bk)
      this.persist()
      return bk
    },

    /**
     * 导入固定格式 JSON（以题库为单位）
     * @param {string} text JSON 文本
     * @param {object} opt
     *   bank — 本次导入的题库名（必填，作为题库唯一标识）
     *   bankAction — 题库已存在时的处理方式：'replace' 替换原题库 / 'merge' 合并进原题库
     *   bankMeta — 题库元数据 { cover, desc }（可选，随导入写入）
     *   withProgress — 是否同时导入练习进度
     * @returns {{ok:boolean, error?:string, summary?:object}}
     */
    importJSON(text, opt = {}) {
      const bankOverride = String(opt.bank || '').trim() || DEFAULT_BANK
      const bankAction = opt.bankAction === 'replace' ? 'replace' : 'merge'
      const parsed = parseStudyPayload(text)
      if (!parsed.ok) {
        this.lastImport = { ok: false, error: parsed.error, at: nowStr() }
        return this.lastImport
      }

      const skipped = []
      const nextSubjects = []
      const nextQuestions = []
      const nextWords = []
      const nextNotes = []

      parsed.payload.subjects.forEach((raw, i) => {
        const s = normalizeSubject(raw)
        if (!s) skipped.push({ kind: '学科', at: i + 1, reason: '缺少 value 与 name' })
        else nextSubjects.push(s)
      })

      parsed.payload.questions.forEach((raw, i) => {
        const q = normalizeQuestion(raw, raw.id || this.nextId('q'))
        if (q.error) skipped.push({ kind: '题目', at: i + 1, id: raw?.id || '', reason: q.error })
        else {
          q.bank = bankOverride
          nextQuestions.push(q)
        }
      })

      parsed.payload.english.forEach((raw, i) => {
        const w = normalizeWord(raw, raw.id || this.nextId('w'))
        if (w.error) skipped.push({ kind: '单词', at: i + 1, id: raw?.id || '', reason: w.error })
        else nextWords.push(w)
      })

      parsed.payload.notes.forEach((raw, i) => {
        const n = normalizeNote(raw, raw.id || this.nextId('n'))
        if (n.error) skipped.push({ kind: '知识点', at: i + 1, id: raw?.id || '', reason: n.error })
        else nextNotes.push(n)
      })

      const total =
        parsed.payload.subjects.length +
        parsed.payload.questions.length +
        parsed.payload.english.length +
        parsed.payload.notes.length
      if (!total) {
        const error = '文件里没有可导入的数据（questions / english / notes 都是空的）'
        this.lastImport = { ok: false, error, at: nowStr() }
        return this.lastImport
      }

      const counter = { added: 0, updated: 0, removed: 0 }

      /* --- 题目按题库为单位写入 --- */
      const bankName = bankOverride
      if (bankAction === 'replace') {
        // 替换：先清掉该题库的旧题和对应进度
        const removedIds = this.questions
          .filter((q) => (String(q.bank || '').trim() || DEFAULT_BANK) === bankName)
          .map((q) => q.id)
        if (removedIds.length) {
          this.questions = this.questions.filter(
            (q) => (String(q.bank || '').trim() || DEFAULT_BANK) !== bankName
          )
          removedIds.forEach((id) => delete this.progress.questions[id])
          counter.removed = removedIds.length
        }
      }

      // 该题库内已有题目的 id → index 映射（合并用）
      const bankIndex = new Map()
      this.questions.forEach((q, i) => {
        if ((String(q.bank || '').trim() || DEFAULT_BANK) === bankName) bankIndex.set(q.id, i)
      })
      // 全局 id 集合（避免跨题库 id 冲突）
      const globalIds = new Set(this.questions.map((q) => q.id))

      nextQuestions.forEach((q) => {
        if (bankAction === 'merge' && bankIndex.has(q.id)) {
          // 合并：同题库内同 id → 覆盖字段
          const i = bankIndex.get(q.id)
          this.questions[i] = { ...this.questions[i], ...q }
          counter.updated += 1
        } else {
          // 新增：若 id 与其他题库撞了，换一个唯一 id
          if (globalIds.has(q.id)) {
            let nid
            do { nid = this.nextId('q') } while (globalIds.has(nid))
            q.id = nid
          }
          globalIds.add(q.id)
          bankIndex.set(q.id, this.questions.length)
          this.questions.push(q)
          counter.added += 1
        }
      })

      /* --- 单词 / 知识点 / 学科：按 id 全局合并 --- */
      this.mergeList('subjects', nextSubjects, counter)
      this.mergeList('english', nextWords, counter)
      this.mergeList('notes', nextNotes, counter)

      /* --- 题库元数据：名称唯一，导入时带上图片与说明 --- */
      // 优先用显式传入的 bankMeta；否则尝试从导入 JSON 的 banks 列表里按名称匹配
      let bankMeta = opt.bankMeta || null
      if (!bankMeta) {
        const fromPayload = (parsed.payload.banks || []).find((b) => {
          const nm = normalizeBank(b)
          return nm && nm.name === bankName
        })
        if (fromPayload) bankMeta = { cover: fromPayload.cover, desc: fromPayload.desc }
      }
      if (bankMeta) {
        const bk = normalizeBank({ name: bankName, ...bankMeta })
        if (bk) {
          const i = this.banks.findIndex((b) => b.name === bk.name)
          if (i >= 0) this.banks[i] = { ...this.banks[i], ...bk }
          else this.banks.push(bk)
        }
      }

      // 导入包里自带的学习进度（可选）
      if (opt.withProgress && parsed.payload.progress) {
        const p = normalizeProgress(parsed.payload.progress)
        Object.assign(this.progress.questions, p.questions)
        Object.assign(this.progress.words, p.words)
        Object.assign(this.progress.notes, p.notes)
      }

      this.persist()

      const summary = {
        bank: bankName,
        bankAction,
        added: counter.added,
        updated: counter.updated,
        removed: counter.removed,
        subjects: nextSubjects.length,
        questions: nextQuestions.length,
        english: nextWords.length,
        notes: nextNotes.length,
        skipped
      }
      this.lastImport = { ok: true, summary, at: nowStr(), meta: parsed.meta }
      return this.lastImport
    },

    /**
     * 按 id（学科按 value）合并：已存在则覆盖字段，不存在则追加
     * @param {string} key state 里的列表名
     * @param {Array} list 规范化后的记录
     * @param {{added:number,updated:number}} counter 统计容器
     */
    mergeList(key, list, counter) {
      const idOf = key === 'subjects' ? (x) => x.value : (x) => x.id
      const index = new Map(this[key].map((x, i) => [idOf(x), i]))
      list.forEach((item) => {
        const k = idOf(item)
        if (index.has(k)) {
          const i = index.get(k)
          this[key][i] = { ...this[key][i], ...item }
          counter.updated += 1
        } else {
          index.set(k, this[key].length)
          this[key].push(item)
          counter.added += 1
        }
      })
    },

    /**
     * 导出为固定格式 JSON（可直接再导入）
     * @param {object} opt { withProgress: boolean, pretty: boolean }
     */
    exportJSON(opt = {}) {
      const { withProgress = true, pretty = true } = opt
      const payload = {
        version: STUDY_VERSION,
        type: STUDY_SCHEMA,
        exported_at: nowStr(),
        data: {
          subjects: this.subjects,
          banks: this.banks,
          questions: this.questions,
          english: this.english,
          notes: this.notes
        }
      }
      if (withProgress) payload.data.progress = JSON.parse(JSON.stringify(this.progress))
      return pretty ? JSON.stringify(payload, null, 2) : JSON.stringify(payload)
    },

    /** 空白模板：只有结构与字段示例，方便照格式填写 */
    templateJSON() {
      return JSON.stringify(
        {
          version: STUDY_VERSION,
          type: STUDY_SCHEMA,
          exported_at: nowStr(),
          data: {
            subjects: [{ value: 'math', name: '数学', emoji: '🔢' }],
            banks: [{ name: '默认题库', cover: '', desc: '题库说明（可选）' }],
            questions: [
              {
                id: 'q-001',
                bank: '默认题库',
                subject: 'math',
                type: 'single',
                difficulty: 'easy',
                stem: '题干内容',
                options: [
                  { key: 'A', text: '选项一' },
                  { key: 'B', text: '选项二' }
                ],
                answer: ['A'],
                analysis: '解析内容',
                tags: ['示例'],
                source: '',
                score: 1
              }
            ],
            english: [
              {
                id: 'w-001',
                word: 'apple',
                phonetic: '/ˈæp.əl/',
                meaning: '苹果',
                example: 'I eat an apple every day.',
                example_cn: '我每天吃一个苹果。',
                level: '小学',
                unit: 'Unit 1',
                tags: ['食物']
              }
            ],
            notes: [
              {
                id: 'n-001',
                subject: 'math',
                title: '知识点标题',
                content: '知识点正文，支持换行。',
                key_points: ['要点一', '要点二'],
                tags: ['示例'],
                is_exam_point: false
              }
            ]
          }
        },
        null,
        2
      )
    },

    /** 清空全部学习数据（含进度） */
    clearAll() {
      const s = emptyState()
      this.subjects = s.subjects
      this.questions = s.questions
      this.english = s.english
      this.notes = s.notes
      this.progress = s.progress
      this.lastImport = null
      this.persist()
    }
  }
})

export { looseEq, nowStr }
