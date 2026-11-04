<script setup>
/**
 * 学习 · 刷题
 *
 * 支持单选 / 多选 / 判断 / 填空 / 问答，按学科、题型、难度、标签筛选。
 * 三种练习模式：顺序练习、随机练习、错题本。作答结果写入本地进度。
 */
import { computed, ref, onMounted } from 'vue'
import {
  useStudyStore,
  QUESTION_TYPES,
  DIFFICULTIES,
  DEFAULT_BANK,
  questionTypeMeta,
  difficultyMeta,
  looseEq
} from '@/stores/study'
import { toast } from '@/composables/useToast'

defineOptions({ name: 'StudyQuizView' })

const study = useStudyStore()

/* ================= 试卷轮盘（题库切换） ================= */

/** 轮盘展示的试卷列表（按名称排序，带元数据） */
const wheelBanks = computed(() => study.questionBanks)

/** 当前选中的试卷索引 */
const wheelIndex = ref(0)
/** 选中试卷（与 fBank 联动） */
const activeBank = computed(() => wheelBanks.value[wheelIndex.value] || null)

/** 是否在练习中（练习时隐藏轮盘，专注答题） */
const showWheel = computed(() => !practicing.value && wheelBanks.value.length > 1)

/** 选中某份试卷（同时联动题库筛选） */
function selectBank(i) {
  const n = wheelBanks.value.length
  if (!n) return
  const next = ((i % n) + n) % n
  if (next === wheelIndex.value) {
    // 点击已居中的试卷 → 进入刷题
    enterBank()
    return
  }
  wheelIndex.value = next
  fBank.value = wheelBanks.value[next].name
}

/** 进入当前试卷刷题 */
function enterBank() {
  if (!activeBank.value) return
  fBank.value = activeBank.value.name
  resetFilterOthers()
  startPractice('order')
}

/** 无封面时的降级背景色（按名称哈希取一组渐变色） */
const HUE_GRADIENTS = [
  'linear-gradient(135deg,#1F2A44,#3a4a70)',
  'linear-gradient(135deg,#3b2f5f,#6a4a8f)',
  'linear-gradient(135deg,#2f4f4f,#4a7a6f)',
  'linear-gradient(135deg,#5f3a2f,#8f5a3a)',
  'linear-gradient(135deg,#2f3f5f,#3a5f8f)',
  'linear-gradient(135deg,#4a3a5f,#7a5a8f)'
]
function bankHue(i) {
  let h = 0
  const name = wheelBanks.value[i]?.name || ''
  for (let c = 0; c < name.length; c++) h = (h * 31 + name.charCodeAt(c)) >>> 0
  return HUE_GRADIENTS[h % HUE_GRADIENTS.length]
}

/** 只重置除题库外的筛选，保留选中题库 */
function resetFilterOthers() {
  fSubject.value = ''
  fType.value = ''
  fDifficulty.value = ''
  fWrongOnly.value = false
  keyword.value = ''
}

/** 上一份 / 下一份试卷 */
function prevBank() { selectBank(wheelIndex.value - 1) }
function nextBank() { selectBank(wheelIndex.value + 1) }

/* ---------------- 轮盘触摸滑动（手机左右滑切试卷） ---------------- */

const touchStartX = ref(0)
const touchStartY = ref(0)
const touchDeltaX = ref(0)
const touchActive = ref(false)

function onTouchStart(e) {
  const t = e.touches[0]
  touchStartX.value = t.clientX
  touchStartY.value = t.clientY
  touchDeltaX.value = 0
  touchActive.value = true
}

function onTouchMove(e) {
  if (!touchActive.value) return
  const t = e.touches[0]
  const dx = t.clientX - touchStartX.value
  const dy = t.clientY - touchStartY.value
  // 横向位移明显大于纵向时，判定为切试卷手势，阻止页面滚动
  if (Math.abs(dx) > Math.abs(dy) && Math.abs(dx) > 8) {
    e.preventDefault()
  }
  touchDeltaX.value = dx
}

function onTouchEnd() {
  if (!touchActive.value) return
  touchActive.value = false
  const dx = touchDeltaX.value
  const SWIPE_THRESHOLD = 50
  if (Math.abs(dx) < SWIPE_THRESHOLD) {
    touchDeltaX.value = 0
    return
  }
  if (dx < 0) nextBank()
  else prevBank()
  touchDeltaX.value = 0
}

/**
 * 轮盘上每张试卷的视觉参数（相对 index 的偏移量决定 3D 位置）
 * 返回与参考「轮盘书册」一致的 translate/rotate/scale/opacity/zIndex
 */
function wheelStyle(i) {
  const n = wheelBanks.value.length
  let d = i - wheelIndex.value
  if (d > n / 2) d -= n
  if (d < -n / 2) d += n

  const ad = Math.abs(d)
  const dir = d === 0 ? 0 : (d > 0 ? 1 : -1)
  let x, z, ry, sc, op
  if (ad === 0) { x = 0; z = 150; ry = 0; sc = 1; op = 1 }
  else if (ad === 1) { x = 210 * dir; z = -10; ry = -26 * dir; sc = 0.88; op = 0.85 }
  else { x = 396 * dir; z = -180; ry = -38 * dir; sc = 0.72; op = 0.48 }

  return {
    transform: `translate3d(${x}px,0,${z}px) rotateY(${ry}deg) scale(${sc})`,
    opacity: op,
    zIndex: String(30 - ad * 8),
    filter: ad === 0 ? 'none' : `brightness(${(1 - ad * 0.06).toFixed(2)}) saturate(${(1 - ad * 0.08).toFixed(2)})`,
    active: ad === 0
  }
}

/* ---------------- 筛选 ---------------- */

const fBank = ref('')
const fSubject = ref('')
const fType = ref('')
const fDifficulty = ref('')
const fWrongOnly = ref(false)
const keyword = ref('')

const filtered = computed(() => {
  const k = keyword.value.trim().toLowerCase()
  return study.questions.filter((q) => {
    const bank = String(q.bank || '').trim() || DEFAULT_BANK
    if (fBank.value && bank !== fBank.value) return false
    if (fSubject.value && q.subject !== fSubject.value) return false
    if (fType.value && q.type !== fType.value) return false
    if (fDifficulty.value && q.difficulty !== fDifficulty.value) return false
    if (fWrongOnly.value && study.progress.questions[q.id]?.last_result !== 'wrong') return false
    if (k) {
      const hay = [q.stem, q.analysis, q.source, ...(q.tags || []), ...q.options.map((o) => o.text)]
        .join(' ')
        .toLowerCase()
      if (!hay.includes(k)) return false
    }
    return true
  })
})

const banksInUse = computed(() => study.questionBanks)

const subjectsInUse = computed(() => {
  const set = new Set(study.questions.map((q) => q.subject).filter(Boolean))
  return study.allSubjects.filter((s) => set.has(s.value))
})

function resetFilter() {
  // 题库由轮盘驱动，重置时保留当前选中试卷
  fSubject.value = ''
  fType.value = ''
  fDifficulty.value = ''
  fWrongOnly.value = false
  keyword.value = ''
}

onMounted(() => {
  // 初始：选中第一个试卷（按名称排序）
  if (wheelBanks.value.length) {
    wheelIndex.value = 0
    fBank.value = wheelBanks.value[0].name
  }
})

/* ---------------- 练习 ---------------- */

const practicing = ref(false)
const queue = ref([])
const index = ref(0)
const picked = ref([])       // 单选/判断：字符串；多选：数组
const fillText = ref('')
const revealed = ref(false)  // 问答/填空是否已看答案
const answered = ref(null)   // null | true | false
const session = ref({ total: 0, right: 0 })

const current = computed(() => queue.value[index.value] || null)
const progressText = computed(() => `${index.value + 1} / ${queue.value.length}`)
const percent = computed(() =>
  queue.value.length ? Math.round(((index.value + 1) / queue.value.length) * 100) : 0
)

function shuffle(list) {
  const a = [...list]
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[a[i], a[j]] = [a[j], a[i]]
  }
  return a
}

/** 随机模式一次抽多少题，避免题库过大时一轮刷不完 */
const RANDOM_BATCH = 20

/**
 * 开始一轮练习
 * @param {'order'|'random'|'wrong'|'filter'} mode
 */
function startPractice(mode = 'order') {
  let list = filtered.value
  if (mode === 'wrong') {
    // 错题本跨题库：只看错题，不受当前试卷筛选限制
    list = study.questions.filter((q) => study.progress.questions[q.id]?.last_result === 'wrong')
  } else if (mode === 'random') {
    list = shuffle(list)
  }
  if (!list.length) {
    toast.warn(mode === 'wrong' ? '错题本是空的，先去练几道吧' : '当前筛选下没有题目')
    return
  }
  startSession(mode === 'random' ? list.slice(0, RANDOM_BATCH) : list)
}

/** 只练列表里点中的这一道 */
function startSingle(q) {
  startSession([q])
}

function startSession(list) {
  queue.value = list
  index.value = 0
  session.value = { total: list.length, right: 0 }
  resetAnswer()
  practicing.value = true
}

function resetAnswer() {
  picked.value = []
  fillText.value = ''
  revealed.value = false
  answered.value = null
}

/** 判断题选项：T/F */
const judgeOptions = [
  { key: 'T', text: '正确' },
  { key: 'F', text: '错误' }
]

const displayOptions = computed(() => {
  const q = current.value
  if (!q) return []
  return q.type === 'judge' ? judgeOptions : q.options
})

const isMulti = computed(() => current.value?.type === 'multiple')

function togglePick(key) {
  if (answered.value !== null) return
  if (isMulti.value) {
    const i = picked.value.indexOf(key)
    if (i >= 0) picked.value.splice(i, 1)
    else picked.value.push(key)
  } else {
    picked.value = [key]
  }
}

function isPicked(key) {
  return picked.value.includes(key)
}

/** 标准答案文本，用于对照展示 */
const answerText = computed(() => {
  const q = current.value
  if (!q) return ''
  if (q.type === 'single' || q.type === 'multiple') {
    return q.answer
      .map((k) => {
        const o = q.options.find((x) => x.key === k)
        return o ? `${k}. ${o.text}` : k
      })
      .join('　')
  }
  if (q.type === 'judge') return q.answer[0] === 'T' ? '正确' : '错误'
  return q.answer.join(' / ')
})

const canSubmit = computed(() => {
  const q = current.value
  if (!q) return false
  if (q.type === 'fill') return fillText.value.trim() !== ''
  if (q.type === 'answer') return revealed.value
  return picked.value.length > 0
})

function submit() {
  const q = current.value
  if (!q || answered.value !== null) return

  let correct = false
  if (q.type === 'fill') {
    // 填空：标准答案用 | 分隔多个可接受写法，任一匹配即算对
    const accepts = q.answer.flatMap((a) => a.split('|'))
    correct = accepts.some((a) => looseEq(a, fillText.value))
  } else if (q.type === 'answer') {
    correct = true // 问答题自评：看到答案后由用户自行判断
  } else {
    const mine = [...picked.value].sort()
    const std = [...q.answer].sort()
    correct = mine.length === std.length && mine.every((v, i) => v === std[i])
  }

  answered.value = correct
  study.recordAnswer(q.id, correct)
  if (correct) session.value.right += 1
}

/** 问答题：先自己作答，再翻开答案自评 */
function reveal() {
  revealed.value = true
}

function selfJudge(correct) {
  const q = current.value
  if (!q || answered.value !== null) return
  answered.value = correct
  study.recordAnswer(q.id, correct)
  if (correct) session.value.right += 1
}

function next() {
  if (index.value < queue.value.length - 1) {
    index.value += 1
    resetAnswer()
  } else {
    practicing.value = false
    const rate = session.value.total ? Math.round((session.value.right / session.value.total) * 100) : 0
    toast.success(`本轮完成：${session.value.right}/${session.value.total}，正确率 ${rate}%`)
  }
}

function quit() {
  if (answered.value === null && !confirm('本轮还没结束，确定退出吗？已作答的题目进度已保存。')) return
  practicing.value = false
}

/** 选项在已作答后的样式 */
function optionClass(key) {
  if (answered.value === null) return isPicked(key) ? 'opt on' : 'opt'
  const std = current.value.answer.includes(key)
  const mine = isPicked(key)
  if (std && mine) return 'opt right'
  if (std) return 'opt right-dim'
  if (mine) return 'opt wrong'
  return 'opt dim'
}

const wrongCount = computed(() => study.wrongQuestions.length)
</script>

<template>
  <div class="page page--flat quiz">
    <div class="page-body">
      <header class="q-head">
        <h1 class="q-title">✏️ 刷题</h1>
        <p class="q-sub">
          共 {{ study.questions.length }} 道题 · {{ banksInUse.length }} 个题库
          <template v-if="wrongCount"> · <b class="warn">{{ wrongCount }} 道待订正</b></template>
        </p>
      </header>

      <!-- 空态 -->
      <div v-if="!study.questions.length" class="empty">
        <span class="empty-emoji">📭</span>
        <p class="empty-title">题库还是空的</p>
        <p class="empty-desc">回到「学习」页，用导入 JSON 把题目批量导进来</p>
        <router-link to="/tools/study" class="btn btn--primary btn--sm mt-md">去导入</router-link>
      </div>

      <template v-else>
        <!-- 试卷轮盘（题库切换） -->
        <section v-if="showWheel" class="wheel-wrap">
          <div
            class="wheel-stage"
            @touchstart="onTouchStart"
            @touchmove="onTouchMove"
            @touchend="onTouchEnd"
            @touchcancel="onTouchEnd"
          >
            <div class="wheel-scene">
              <div class="wheel-glow" />
              <div class="wheel-track" />
              <div class="wheel">
                <button
                  v-for="(b, i) in wheelBanks"
                  :key="b.name"
                  type="button"
                  class="wbook"
                  :class="{ 'is-active': wheelStyle(i).active }"
                  :style="{
                    transform: wheelStyle(i).transform,
                    opacity: wheelStyle(i).opacity,
                    zIndex: wheelStyle(i).zIndex,
                    filter: wheelStyle(i).filter
                  }"
                  :tabindex="wheelStyle(i).active ? 0 : -1"
                  :aria-current="wheelStyle(i).active ? 'true' : 'false'"
                  @click="selectBank(i)"
                >
                  <template v-if="b.cover">
                    <img :src="b.cover" :alt="b.name" draggable="false" />
                  </template>
                  <template v-else>
                    <span class="wbook-fallback" :style="{ background: bankHue(i) }">📄</span>
                  </template>
                  <span class="wbook-scrim" />
                  <span class="wbook-name">{{ b.name }}</span>
                  <span class="wbook-count">{{ b.count }} 题</span>
                </button>
              </div>
            </div>

            <button class="warrow prev" type="button" aria-label="上一份试卷" @click="prevBank">
              <svg viewBox="0 0 24 24" fill="none"><path d="M15 5L8 12L15 19" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
            <button class="warrow next" type="button" aria-label="下一份试卷" @click="nextBank">
              <svg viewBox="0 0 24 24" fill="none"><path d="M9 5L16 12L9 19" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
          </div>

          <div class="wdots">
            <button
              v-for="(b, i) in wheelBanks"
              :key="b.name"
              type="button"
              class="wdot"
              :class="{ on: i === wheelIndex }"
              :aria-label="b.name"
              @click="selectBank(i)"
            />
          </div>

          <!-- 选中试卷信息卡 -->
          <div v-if="activeBank" class="winfo">
            <div class="wcard">
              <div class="wmeta">
                <span class="wchip">📚 {{ activeBank.count }} 道题</span>
                <span class="wchip wchip--ghost">试卷 {{ wheelIndex + 1 }} / {{ wheelBanks.length }}</span>
              </div>
              <h2 class="wtitle">{{ activeBank.name }}</h2>
              <p class="wdesc">{{ activeBank.desc || '这份试卷还没有说明，点击「开始刷题」直接进入。' }}</p>
              <div class="wactions">
                <button class="btn btn--primary" type="button" @click="enterBank">开始刷题</button>
              </div>
            </div>
          </div>
        </section>

        <!-- 单题库时：不显示轮盘，直接给一个进入按钮 -->
        <section v-else-if="!practicing && wheelBanks.length === 1" class="wheel-single">
          <button class="btn btn--primary" type="button" @click="startPractice('order')">
            📚 开始刷「{{ wheelBanks[0].name }}」
          </button>
        </section>

        <!-- 练习模式 -->
        <div v-if="!practicing" class="mode-row">
          <button class="mode-card" @click="startPractice('order')">
            <span class="mode-emoji">📝</span>
            <span class="mode-name">顺序练习</span>
            <span class="mode-hint">按题库顺序刷</span>
          </button>
          <button class="mode-card" @click="startPractice('random')">
            <span class="mode-emoji">🎲</span>
            <span class="mode-name">随机 20 题</span>
            <span class="mode-hint">打乱抽取</span>
          </button>
          <button class="mode-card" :class="{ 'mode-card--off': !wrongCount }" @click="startPractice('wrong')">
            <span class="mode-emoji">🎯</span>
            <span class="mode-name">错题本</span>
            <span class="mode-hint">{{ wrongCount ? `${wrongCount} 道待订正` : '暂无错题' }}</span>
          </button>
        </div>

        <!-- 筛选 -->
        <section class="filter">
          <input v-model="keyword" class="input search" type="search" placeholder="搜题干、解析、标签…" />

          <div v-if="subjectsInUse.length" class="chips">
            <button class="chip-btn" :class="{ on: !fSubject }" @click="fSubject = ''">全部学科</button>
            <button
              v-for="s in subjectsInUse"
              :key="s.value"
              class="chip-btn"
              :class="{ on: fSubject === s.value }"
              @click="fSubject = s.value"
            >{{ s.emoji }} {{ s.name }}</button>
          </div>

          <div class="chips">
            <button class="chip-btn" :class="{ on: !fType }" @click="fType = ''">全部题型</button>
            <button
              v-for="t in QUESTION_TYPES"
              :key="t.value"
              class="chip-btn"
              :class="{ on: fType === t.value }"
              @click="fType = t.value"
            >{{ t.label }}</button>
          </div>

          <div class="chips">
            <button class="chip-btn" :class="{ on: !fDifficulty }" @click="fDifficulty = ''">全部难度</button>
            <button
              v-for="d in DIFFICULTIES"
              :key="d.value"
              class="chip-btn"
              :class="{ on: fDifficulty === d.value }"
              @click="fDifficulty = d.value"
            >{{ d.label }}</button>
            <button class="chip-btn chip-btn--danger" :class="{ on: fWrongOnly }" @click="fWrongOnly = !fWrongOnly">
              只看错题
            </button>
          </div>

          <div class="filter-foot">
            <span class="fcount">筛出 {{ filtered.length }} 道</span>
            <button class="btn btn--soft btn--sm" @click="startPractice('filter')">练这批</button>
            <button v-if="fBank || fSubject || fType || fDifficulty || fWrongOnly || keyword" class="text-link" @click="resetFilter">
              重置
            </button>
          </div>
        </section>

        <!-- 题目列表预览 -->
        <section v-if="!practicing" class="section">
          <div class="section-head">
            <h2 class="section-title">题目预览</h2>
            <span class="section-more">{{ filtered.length }} 道</span>
          </div>
          <div v-if="filtered.length" class="qlist">
            <button
              v-for="q in filtered.slice(0, 40)"
              :key="q.id"
              class="qitem"
              @click="startSingle(q)"
            >
              <span class="qitem-type">{{ questionTypeMeta(q.type).emoji }}</span>
              <div class="qitem-body">
                <p class="qitem-stem ellipsis-2">{{ q.stem }}</p>
                <p class="qitem-meta">
                  <span v-if="q.subject">{{ study.subjectName(q.subject) }}</span>
                  <span>{{ difficultyMeta(q.difficulty).label }}</span>
                  <span v-if="study.progress.questions[q.id]">
                    练过 {{ study.progress.questions[q.id].attempts }} 次
                  </span>
                  <span v-if="study.progress.questions[q.id]?.last_result === 'wrong'" class="warn">错题</span>
                </p>
              </div>
              <span class="qitem-go">›</span>
            </button>
            <p v-if="filtered.length > 40" class="more-tip">仅显示前 40 道，点「练这批」刷全部</p>
          </div>
          <div v-else class="empty empty--sm">
            <span class="empty-emoji">🔍</span>
            <p class="empty-title">没有符合条件的题目</p>
            <p class="empty-desc">换个筛选条件，或点「重置」</p>
          </div>
        </section>
      </template>

      <div class="bottom-space" />
    </div>

    <!-- 答题弹层 -->
    <transition name="sheet">
      <div v-if="practicing && current" class="sheet-mask">
        <div class="sheet">
          <div class="sheet-head">
            <span class="scount num">{{ progressText }}</span>
            <div class="sbar"><i :style="{ width: percent + '%' }" /></div>
            <button class="sheet-close" aria-label="退出练习" @click="quit">✕</button>
          </div>

          <div class="sheet-body">
            <div class="qtags">
              <span class="chip chip--brand">{{ questionTypeMeta(current.type).label }}</span>
              <span class="chip" :class="`chip--${difficultyMeta(current.difficulty).tone}`">
                {{ difficultyMeta(current.difficulty).label }}
              </span>
              <span v-if="current.subject" class="chip">
                {{ study.subjectMeta(current.subject).emoji }} {{ study.subjectName(current.subject) }}
              </span>
              <span v-for="t in current.tags" :key="t" class="chip chip--info">{{ t }}</span>
            </div>

            <p class="stem">{{ current.stem }}</p>

            <!-- 选择题 / 判断题 -->
            <div v-if="current.type === 'single' || current.type === 'multiple' || current.type === 'judge'" class="opts">
              <button
                v-for="o in displayOptions"
                :key="o.key"
                class="opt-row"
                :class="optionClass(o.key)"
                @click="togglePick(o.key)"
              >
                <span class="opt-key">{{ o.key }}</span>
                <span class="opt-text">{{ o.text }}</span>
              </button>
              <p v-if="isMulti && answered === null" class="multi-tip">多选题，选完点「提交答案」</p>
            </div>

            <!-- 填空题 -->
            <div v-else-if="current.type === 'fill'" class="fill">
              <input
                v-model="fillText"
                class="input fill-input"
                :disabled="answered !== null"
                placeholder="输入答案"
                @keyup.enter="answered === null && submit()"
              />
            </div>

            <!-- 问答题 -->
            <div v-else class="qa">
              <p class="qa-hint">先在心里或纸上作答，再翻开参考答案自评。</p>
              <button v-if="!revealed" class="btn btn--soft btn--block" @click="reveal">翻开参考答案</button>
            </div>

            <!-- 答案与解析 -->
            <div v-if="answered !== null || (current.type === 'answer' && revealed)" class="reveal">
              <div class="reveal-line">
                <span v-if="current.type !== 'answer'" class="verdict" :class="answered ? 'ok' : 'no'">
                  {{ answered ? '✅ 答对了' : '❌ 答错了' }}
                </span>
                <span class="ans">正确答案：{{ answerText }}</span>
              </div>
              <p v-if="current.analysis" class="analysis">{{ current.analysis }}</p>
              <p v-if="current.source" class="source">来源：{{ current.source }}</p>

              <!-- 问答题自评 -->
              <div v-if="current.type === 'answer' && answered === null" class="self-judge">
                <p class="sj-label">你答得怎么样？</p>
                <div class="sj-btns">
                  <button class="btn btn--ghost grow" @click="selfJudge(false)">没答好</button>
                  <button class="btn btn--primary grow" @click="selfJudge(true)">答对了</button>
                </div>
              </div>
            </div>
          </div>

          <div class="sheet-foot">
            <template v-if="answered === null && !(current.type === 'answer' && revealed)">
              <button class="btn btn--primary btn--block" :disabled="!canSubmit" @click="submit">提交答案</button>
            </template>
            <template v-else-if="current.type === 'answer' && answered === null">
              <button class="btn btn--ghost btn--block" disabled>请先自评</button>
            </template>
            <template v-else>
              <button class="btn btn--primary btn--block" @click="next">
                {{ index < queue.length - 1 ? '下一题' : '完成本轮' }}
              </button>
            </template>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.q-head { padding: calc(var(--safe-top) + 16px) 0 16px; }
.q-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.q-sub { font-size: 13px; color: var(--text-3); margin-top: 5px; }
.warn { color: var(--danger); font-weight: 700; }

/* 模式卡 */
.mode-row { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; }
.mode-card {
  display: flex; flex-direction: column; align-items: center; gap: 3px;
  padding: 14px 8px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  cursor: pointer; font-family: inherit;
  transition: transform var(--dur-fast) var(--ease), box-shadow var(--dur-fast);
}
.mode-card:active { transform: scale(.96); }
.mode-card--off { opacity: .55; }
.mode-emoji { font-size: 24px; }
.mode-name { font-size: 13.5px; font-weight: 700; }
.mode-hint { font-size: 11px; color: var(--text-3); text-align: center; }

/* 筛选 */
.filter { margin-top: 14px; }
.search { margin-bottom: 10px; }
.chips { display: flex; flex-wrap: wrap; gap: 7px; margin-bottom: 9px; }
.chip-btn {
  padding: 6px 12px;
  border-radius: var(--r-full);
  border: 1px solid var(--border-strong);
  background: var(--surface);
  color: var(--text-2);
  font-size: 12.5px; font-weight: 600;
  cursor: pointer; font-family: inherit;
  transition: all var(--dur-fast);
}
.chip-btn.on { background: var(--brand-gradient); border-color: transparent; color: #fff; box-shadow: var(--sh-brand); }
.chip-btn--danger.on { background: var(--danger); box-shadow: none; }
.chip-btn--bank { border-color: var(--brand-100); background: var(--brand-50); color: var(--brand-700); }
.chip-btn--bank.on { background: var(--brand-gradient); color: #fff; }
.chip-count {
  display: inline-block;
  margin-left: 3px;
  padding: 0 5px;
  border-radius: var(--r-full);
  background: color-mix(in srgb, currentColor 12%, transparent);
  font-size: 10.5px;
  font-weight: 700;
  line-height: 1.6;
}
.chip-btn.on .chip-count { background: rgba(255, 255, 255, .22); }
.filter-foot { display: flex; align-items: center; gap: 12px; margin-top: 4px; }
.fcount { font-size: 12.5px; color: var(--text-3); flex: 1; }
.text-link { background: none; border: none; color: var(--brand-600); font-size: 12.5px; cursor: pointer; font-family: inherit; padding: 4px; }

/* 列表 */
.qlist { display: flex; flex-direction: column; gap: 8px; }
.qitem {
  display: flex; align-items: center; gap: 11px;
  padding: 12px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  box-shadow: var(--sh-xs);
  cursor: pointer; font-family: inherit; text-align: left; width: 100%;
  transition: transform var(--dur-fast) var(--ease);
}
.qitem:active { transform: scale(.98); }
.qitem-type { width: 34px; height: 34px; flex: none; border-radius: var(--r-sm); background: var(--bg-subtle); display: flex; align-items: center; justify-content: center; font-size: 17px; }
.qitem-body { flex: 1; min-width: 0; }
.qitem-stem { font-size: 14px; font-weight: 600; line-height: 1.5; }
.qitem-meta { display: flex; flex-wrap: wrap; gap: 6px; font-size: 11.5px; color: var(--text-3); margin-top: 3px; }
.qitem-go { flex: none; font-size: 20px; color: var(--text-4); }
.more-tip { font-size: 11.5px; color: var(--text-4); text-align: center; padding: 8px; }

/* 空态 */
.empty { text-align: center; padding: 44px 20px; background: var(--surface); border-radius: var(--r-lg); border: 1px dashed var(--border-strong); }
.empty--sm { padding: 30px 18px; }
.empty-emoji { font-size: 38px; }
.empty-title { font-size: 15px; font-weight: 700; margin-top: 10px; }
.empty-desc { font-size: 12.5px; color: var(--text-3); margin-top: 4px; }

.bottom-space { height: 20px; }

/* ================= 试卷轮盘 ================= */
.wheel-wrap { margin-bottom: 18px; }
.wheel-stage {
  position: relative;
  height: 300px;
  overflow: hidden;
  perspective: 1400px;
  perspective-origin: 50% 150px;
  touch-action: pan-y;
  user-select: none;
  -webkit-user-select: none;
}
.wheel-scene {
  position: absolute;
  left: 0; right: 0; top: 0; height: 300px;
  transform-style: preserve-3d;
}
.wheel-glow {
  position: absolute; left: 50%; top: -30px; width: 560px; height: 320px;
  transform: translateX(-50%); pointer-events: none;
  background: radial-gradient(closest-side,
    color-mix(in srgb, var(--brand-500) 16%, transparent) 0%,
    color-mix(in srgb, var(--brand-500) 8%, transparent) 46%,
    transparent 74%);
}
.wheel-track {
  position: absolute; left: 50%; top: 175px; width: 640px; height: 100px;
  transform: translateX(-50%); border-radius: 50%; pointer-events: none;
  border: 1px solid var(--border);
  box-shadow: inset 0 0 40px color-mix(in srgb, var(--brand-500) 6%, transparent);
}
.wheel {
  position: absolute; left: 50%; top: 0; width: 0; height: 0;
  transform-style: preserve-3d;
}
.wbook {
  position: absolute; left: 0; top: 36px; width: 150px; height: 210px; margin-left: -75px;
  padding: 0; border: 1px solid var(--border); border-radius: 12px; overflow: hidden;
  background: var(--surface-2); cursor: pointer; transform-style: preserve-3d;
  box-shadow: var(--sh-lg);
  transition: transform .8s var(--ease), opacity .6s ease, filter .6s ease, box-shadow .5s ease;
  will-change: transform, opacity;
  -webkit-appearance: none; appearance: none; font: inherit; color: inherit;
}
.wbook img { width: 100%; height: 100%; object-fit: cover; display: block; pointer-events: none; }
.wbook-fallback {
  position: absolute; inset: 0; display: flex; align-items: center; justify-content: center;
  font-size: 52px; pointer-events: none;
}
.wbook-scrim {
  position: absolute; left: 0; right: 0; bottom: 0; height: 78px;
  background: linear-gradient(180deg, rgba(0,0,0,0) 0%, rgba(0,0,0,.55) 55%, rgba(0,0,0,.86) 100%);
}
.wbook-name {
  position: absolute; left: 10px; right: 10px; bottom: 28px;
  font-size: 14px; font-weight: 700; color: #fff;
  text-shadow: 0 2px 12px rgba(0,0,0,.7);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-align: left;
}
.wbook-count {
  position: absolute; left: 10px; right: 10px; bottom: 11px;
  font-size: 11px; color: rgba(255,255,255,.72);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-align: left;
}
.wbook.is-active { box-shadow: var(--sh-xl); }
.wbook:not(.is-active):hover { filter: brightness(1.1) saturate(1.04); }
.wbook:focus-visible { outline: 2px solid var(--brand-500); outline-offset: 4px; }

.warrow {
  position: absolute; top: 50%; width: 44px; height: 44px;
  transform: translateY(-50%);
  display: grid; place-items: center; cursor: pointer; z-index: 20;
  border-radius: 50%; border: 1px solid var(--border-strong);
  background: color-mix(in srgb, var(--surface) 72%, transparent);
  backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px);
  color: var(--text-2);
  transition: border-color .25s, background .25s, transform .25s, color .25s;
  padding: 0;
}
.warrow svg { width: 20px; height: 20px; display: block; }
.warrow:hover { border-color: var(--brand-500); color: var(--brand-600); background: var(--brand-50); }
.warrow:active { transform: translateY(-50%) scale(.94); }
.warrow.prev { left: 4px; }
.warrow.next { right: 4px; }

.wdots {
  display: flex; align-items: center; justify-content: center; gap: 7px;
  padding: 8px 0 4px;
}
.wdot {
  width: 7px; height: 7px; padding: 0; border: 0; border-radius: 99px; cursor: pointer;
  background: var(--border-strong);
  transition: width .38s var(--ease), background .38s;
}
.wdot:hover { background: var(--text-3); }
.wdot.on { width: 22px; background: var(--brand-500); }

.winfo { display: flex; justify-content: center; margin-top: 10px; }
.wcard {
  width: 100%; max-width: 720px; padding: 18px 20px; border-radius: var(--r-lg);
  background: var(--surface);
  border: 1px solid var(--border);
  box-shadow: var(--sh-sm);
}
.wmeta { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
.wchip {
  display: inline-flex; align-items: center; padding: 4px 12px; border-radius: 100px;
  font-size: 11.5px; font-weight: 600; color: var(--brand-700);
  background: var(--brand-50); border: 1px solid var(--brand-100);
}
.wchip--ghost { color: var(--text-2); background: var(--bg-subtle); border-color: var(--border); }
.wtitle { margin: 0 0 8px; font-size: 20px; font-weight: 700; letter-spacing: -.01em; line-height: 1.35; }
.wdesc { margin: 0 0 14px; font-size: 13px; line-height: 1.7; color: var(--text-2); }
.wactions { display: flex; gap: 10px; }

.wheel-single { margin-bottom: 16px; }

@media (min-width: 700px) {
  .wheel-stage { height: 340px; perspective-origin: 50% 170px; }
  .wheel-scene { height: 340px; }
  .wbook { width: 176px; height: 248px; margin-left: -88px; }
  .wheel-track { top: 195px; width: 720px; }
  .warrow.prev { left: 10px; }
  .warrow.next { right: 10px; }
}

/* 答题弹层 */
.sheet-mask { position: fixed; inset: 0; z-index: 100; background: rgba(0,0,0,.4); display: flex; align-items: flex-end; justify-content: center; }
.sheet { width: 100%; max-width: 640px; background: var(--surface); border-radius: var(--r-2xl) var(--r-2xl) 0 0; max-height: 94vh; display: flex; flex-direction: column; box-shadow: var(--sh-xl); }
.sheet-head { display: flex; align-items: center; gap: 12px; padding: 16px 20px 10px; }
.scount { font-size: 13px; font-weight: 700; color: var(--text-2); flex: none; }
.sbar { flex: 1; height: 6px; border-radius: var(--r-full); background: var(--bg-subtle); overflow: hidden; }
.sbar i { display: block; height: 100%; border-radius: var(--r-full); background: var(--brand-gradient); transition: width var(--dur) var(--ease); }
.sheet-close { width: 32px; height: 32px; flex: none; border-radius: 50%; border: none; background: var(--bg-subtle); color: var(--text-3); font-size: 15px; cursor: pointer; }
.sheet-body { padding: 4px 20px 10px; overflow-y: auto; flex: 1; }
.sheet-foot { padding: 14px 20px calc(14px + var(--safe-bottom)); border-top: 1px solid var(--border); }

.qtags { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 12px; }
.stem { font-size: 16px; font-weight: 600; line-height: 1.65; white-space: pre-wrap; }

.opts { display: flex; flex-direction: column; gap: 9px; margin-top: 16px; }
.opt-row {
  display: flex; align-items: flex-start; gap: 11px;
  padding: 12px 14px;
  border-radius: var(--r-md);
  border: 2px solid var(--border);
  background: var(--surface-2);
  cursor: pointer; font-family: inherit; text-align: left; width: 100%;
  transition: all var(--dur-fast);
}
.opt-key {
  width: 26px; height: 26px; flex: none;
  border-radius: 50%;
  background: var(--bg-subtle);
  color: var(--text-2);
  display: flex; align-items: center; justify-content: center;
  font-size: 13px; font-weight: 700;
}
.opt-text { flex: 1; font-size: 14.5px; line-height: 1.6; padding-top: 2px; }
.opt-row.on { border-color: var(--brand-500); background: var(--brand-50); }
.opt-row.on .opt-key { background: var(--brand-500); color: #fff; }
.opt-row.right { border-color: var(--success); background: var(--success-bg); }
.opt-row.right .opt-key { background: var(--success); color: #fff; }
.opt-row.right-dim { border-color: var(--success); background: var(--success-bg); opacity: .75; }
.opt-row.wrong { border-color: var(--danger); background: var(--danger-bg); }
.opt-row.wrong .opt-key { background: var(--danger); color: #fff; }
.opt-row.dim { opacity: .5; }
.multi-tip { font-size: 11.5px; color: var(--text-3); text-align: center; margin-top: 2px; }

.fill { margin-top: 16px; }
.fill-input { font-size: 17px; font-weight: 600; min-height: 52px; }
.qa { margin-top: 16px; }
.qa-hint { font-size: 12.5px; color: var(--text-3); margin-bottom: 10px; line-height: 1.6; }

.reveal { margin-top: 18px; padding: 14px; background: var(--bg-subtle); border-radius: var(--r-md); }
.reveal-line { display: flex; flex-wrap: wrap; align-items: baseline; gap: 8px; }
.verdict { font-size: 13.5px; font-weight: 700; }
.verdict.ok { color: var(--success); }
.verdict.no { color: var(--danger); }
.ans { font-size: 13.5px; font-weight: 600; color: var(--text-1); }
.analysis { font-size: 13px; color: var(--text-2); line-height: 1.7; margin-top: 9px; white-space: pre-wrap; }
.source { font-size: 11.5px; color: var(--text-4); margin-top: 8px; }
.self-judge { margin-top: 12px; padding-top: 11px; border-top: 1px dashed var(--border-strong); }
.sj-label { font-size: 12.5px; color: var(--text-2); font-weight: 600; margin-bottom: 8px; }
.sj-btns { display: flex; gap: 10px; }

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }

@media (min-width: 700px) {
  .mode-row { gap: 14px; }
  .qlist { display: grid; grid-template-columns: repeat(2, 1fr); }
  .more-tip { grid-column: 1 / -1; }
}
</style>
