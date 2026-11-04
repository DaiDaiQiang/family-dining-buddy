<script setup>
/**
 * 学习 · 英语学习
 *
 * 单词卡浏览 + 记忆模式（看英文选中文 / 看中文拼写），
 * 按等级、单元、状态、标签筛选，学习进度写入本地。
 */
import { computed, ref } from 'vue'
import { useStudyStore, WORD_STATUSES, wordStatusMeta } from '@/stores/study'
import { toast } from '@/composables/useToast'

defineOptions({ name: 'StudyEnglishView' })

const study = useStudyStore()
const stats = computed(() => study.stats.english)

/* ---------------- 筛选 ---------------- */

const fLevel = ref('')
const fUnit = ref('')
const fStatus = ref('')
const fTag = ref('')
const keyword = ref('')

const filtered = computed(() => {
  const k = keyword.value.trim().toLowerCase()
  return study.english.filter((w) => {
    if (fLevel.value && w.level !== fLevel.value) return false
    if (fUnit.value && w.unit !== fUnit.value) return false
    if (fTag.value && !(w.tags || []).includes(fTag.value)) return false
    if (fStatus.value && study.wordStatus(w.id) !== fStatus.value) return false
    if (k && !`${w.word} ${w.meaning} ${w.example}`.toLowerCase().includes(k)) return false
    return true
  })
})

const levels = computed(() => study.wordLevels.levels)
const units = computed(() => study.wordLevels.units)
const tags = computed(() => {
  const tally = new Map()
  study.english.forEach((w) => (w.tags || []).forEach((t) => tally.set(t, (tally.get(t) || 0) + 1)))
  return [...tally.entries()].map(([name, count]) => ({ name, count })).sort((a, b) => b.count - a.count)
})

function resetFilter() {
  fLevel.value = ''
  fUnit.value = ''
  fStatus.value = ''
  fTag.value = ''
  keyword.value = ''
}

const filterOn = computed(() => !!(fLevel.value || fUnit.value || fStatus.value || fTag.value || keyword.value))

/* ---------------- 单词卡 ---------------- */

const expanded = ref(null)

function setStatus(w, status) {
  study.setWordStatus(w.id, status)
  toast.success(`「${w.word}」标记为${wordStatusMeta(status).label}`)
}

/** 发音：优先浏览器语音合成，不可用时提示 */
function speak(word) {
  if (!('speechSynthesis' in window)) {
    toast.warn('当前浏览器不支持朗读')
    return
  }
  try {
    window.speechSynthesis.cancel()
    const u = new SpeechSynthesisUtterance(word)
    u.lang = 'en-US'
    u.rate = 0.92
    window.speechSynthesis.speak(u)
  } catch {
    toast.warn('朗读失败')
  }
}

/* ---------------- 记忆模式 ---------------- */

const MODES = [
  { value: 'en2cn', label: '看英文选中文', emoji: '🔤' },
  { value: 'cn2en', label: '看中文选英文', emoji: '🧠' },
  { value: 'spell', label: '拼写默写', emoji: '✍️' }
]
const practicing = ref(false)
const mode = ref('en2cn')
const queue = ref([])
const index = ref(0)
const picked = ref('')
const spellText = ref('')
const answered = ref(null)
const session = ref({ total: 0, right: 0 })
const choices = ref([])

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

function startPractice(m) {
  const pool = filtered.value.length ? filtered.value : study.english
  if (!pool.length) {
    toast.warn('还没有单词，先导入 JSON 吧')
    return
  }
  mode.value = m
  queue.value = shuffle(pool).slice(0, Math.min(20, pool.length))
  index.value = 0
  session.value = { total: queue.value.length, right: 0 }
  resetAnswer()
  buildChoices()
  practicing.value = true
}

function resetAnswer() {
  picked.value = ''
  spellText.value = ''
  answered.value = null
}

/** 生成 4 个选项（含正确答案） */
function buildChoices() {
  const q = current.value
  if (!q) {
    choices.value = []
    return
  }
  const field = mode.value === 'en2cn' ? 'meaning' : 'word'
  const others = study.english.filter((w) => w.id !== q.id && w[field])
  const pickedOthers = shuffle(others).slice(0, 3)
  choices.value = shuffle([q, ...pickedOthers]).map((w) => ({ id: w.id, text: w[field] || w.word }))
}

function choose(id) {
  if (answered.value !== null) return
  picked.value = id
  const correct = id === current.value.id
  finish(correct)
}

function submitSpell() {
  const q = current.value
  if (!q || answered.value !== null) return
  const mine = spellText.value.trim().toLowerCase().replace(/\s+/g, '')
  const std = q.word.trim().toLowerCase().replace(/\s+/g, '')
  finish(mine === std)
}

function finish(correct) {
  answered.value = correct
  study.recordWordAnswer(current.value.id, correct)
  if (correct) {
    session.value.right += 1
    study.reviewWord(current.value.id)
  }
}

function next() {
  if (index.value < queue.value.length - 1) {
    index.value += 1
    resetAnswer()
    buildChoices()
  } else {
    practicing.value = false
    const rate = session.value.total ? Math.round((session.value.right / session.value.total) * 100) : 0
    toast.success(`本轮完成：${session.value.right}/${session.value.total}，正确率 ${rate}%`)
  }
}

function quit() {
  practicing.value = false
}

function choiceClass(id) {
  if (answered.value === null) return picked.value === id ? 'cho on' : 'cho'
  if (id === current.value.id) return 'cho right'
  if (id === picked.value) return 'cho wrong'
  return 'cho dim'
}

const statusTally = computed(() =>
  WORD_STATUSES.map((s) => ({
    ...s,
    count: study.english.filter((w) => study.wordStatus(w.id) === s.value).length
  }))
)
</script>

<template>
  <div class="page page--flat english">
    <div class="page-body">
      <header class="e-head">
        <h1 class="e-title">🔤 英语学习</h1>
        <p class="e-sub">共 {{ stats.total }} 个单词 · 已掌握 {{ stats.mastered }} 个</p>
      </header>

      <div v-if="!study.english.length" class="empty">
        <span class="empty-emoji">📭</span>
        <p class="empty-title">还没有单词</p>
        <p class="empty-desc">回到「学习」页，用导入 JSON 把单词表批量导进来</p>
        <router-link to="/tools/study" class="btn btn--primary btn--sm mt-md">去导入</router-link>
      </div>

      <template v-else>
        <!-- 进度概览 -->
        <div class="status-card">
          <div v-for="s in statusTally" :key="s.value" class="scell">
            <span class="scell-emoji">{{ s.emoji }}</span>
            <span class="scell-value num">{{ s.count }}</span>
            <span class="scell-label">{{ s.label }}</span>
          </div>
        </div>

        <!-- 记忆模式 -->
        <div class="mode-row">
          <button v-for="m in MODES" :key="m.value" class="mode-card" @click="startPractice(m.value)">
            <span class="mode-emoji">{{ m.emoji }}</span>
            <span class="mode-name">{{ m.label }}</span>
          </button>
        </div>

        <!-- 筛选 -->
        <section class="filter">
          <input v-model="keyword" class="input search" type="search" placeholder="搜单词、释义、例句…" />

          <div v-if="levels.length" class="chips">
            <button class="chip-btn" :class="{ on: !fLevel }" @click="fLevel = ''">全部等级</button>
            <button
              v-for="l in levels"
              :key="l.name"
              class="chip-btn"
              :class="{ on: fLevel === l.name }"
              @click="fLevel = l.name"
            >{{ l.name }} ({{ l.count }})</button>
          </div>

          <div v-if="units.length" class="chips">
            <button class="chip-btn" :class="{ on: !fUnit }" @click="fUnit = ''">全部单元</button>
            <button
              v-for="u in units"
              :key="u.name"
              class="chip-btn"
              :class="{ on: fUnit === u.name }"
              @click="fUnit = u.name"
            >{{ u.name }}</button>
          </div>

          <div class="chips">
            <button class="chip-btn" :class="{ on: !fStatus }" @click="fStatus = ''">全部状态</button>
            <button
              v-for="s in WORD_STATUSES"
              :key="s.value"
              class="chip-btn"
              :class="{ on: fStatus === s.value }"
              @click="fStatus = s.value"
            >{{ s.emoji }} {{ s.label }}</button>
          </div>

          <div v-if="tags.length" class="chips">
            <button class="chip-btn" :class="{ on: !fTag }" @click="fTag = ''">全部标签</button>
            <button
              v-for="t in tags.slice(0, 12)"
              :key="t.name"
              class="chip-btn"
              :class="{ on: fTag === t.name }"
              @click="fTag = t.name"
            ># {{ t.name }}</button>
          </div>

          <div class="filter-foot">
            <span class="fcount">筛出 {{ filtered.length }} 个</span>
            <button v-if="filterOn" class="text-link" @click="resetFilter">重置</button>
          </div>
        </section>

        <!-- 单词列表 -->
        <section class="section">
          <div class="section-head">
            <h2 class="section-title">单词本</h2>
            <span class="section-more">{{ filtered.length }} 个</span>
          </div>

          <div v-if="filtered.length" class="wlist stagger">
            <div v-for="w in filtered" :key="w.id" class="wcard">
              <div class="wrow" @click="expanded = expanded === w.id ? null : w.id">
                <div class="wmain">
                  <p class="wword">
                    {{ w.word }}
                    <span v-if="w.phonetic" class="wphonetic">{{ w.phonetic }}</span>
                  </p>
                  <p class="wmeaning">{{ w.meaning || '—' }}</p>
                  <p v-if="w.level || w.unit" class="wmeta">
                    <span v-if="w.level">{{ w.level }}</span>
                    <span v-if="w.unit">{{ w.unit }}</span>
                    <span v-for="t in w.tags" :key="t"># {{ t }}</span>
                  </p>
                </div>
                <div class="wside">
                  <span class="chip" :class="wordStatusMeta(study.wordStatus(w.id)).tone ? `chip--${wordStatusMeta(study.wordStatus(w.id)).tone}` : ''">
                    {{ wordStatusMeta(study.wordStatus(w.id)).emoji }} {{ wordStatusMeta(study.wordStatus(w.id)).label }}
                  </span>
                  <button class="wbtn" aria-label="朗读" @click.stop="speak(w.word)">🔊</button>
                </div>
              </div>

              <div v-if="expanded === w.id" class="wdetail">
                <p v-if="w.example" class="wexample">
                  <span class="we-en">{{ w.example }}</span>
                  <span v-if="w.example_cn" class="we-cn">{{ w.example_cn }}</span>
                </p>
                <p v-else class="wnoexample">暂无例句</p>
                <div class="wactions">
                  <button
                    v-for="s in WORD_STATUSES"
                    :key="s.value"
                    class="btn btn--sm"
                    :class="study.wordStatus(w.id) === s.value ? 'btn--primary' : 'btn--ghost'"
                    @click="setStatus(w, s.value)"
                  >{{ s.emoji }} {{ s.label }}</button>
                </div>
              </div>
            </div>
          </div>

          <div v-else class="empty empty--sm">
            <span class="empty-emoji">🔍</span>
            <p class="empty-title">没有符合条件的单词</p>
            <p class="empty-desc">换个筛选条件，或点「重置」</p>
          </div>
        </section>
      </template>

      <div class="bottom-space" />
    </div>

    <!-- 记忆练习弹层 -->
    <transition name="sheet">
      <div v-if="practicing && current" class="sheet-mask">
        <div class="sheet">
          <div class="sheet-head">
            <span class="scount num">{{ progressText }}</span>
            <div class="sbar"><i :style="{ width: percent + '%' }" /></div>
            <button class="sheet-close" aria-label="退出练习" @click="quit">✕</button>
          </div>

          <div class="sheet-body">
            <p class="pmode">{{ MODES.find((m) => m.value === mode)?.label }}</p>

            <!-- 看英文选中文 / 看中文选英文 -->
            <template v-if="mode !== 'spell'">
              <p class="prompt" :class="{ 'prompt--cn': mode === 'cn2en' }">
                {{ mode === 'en2cn' ? current.word : current.meaning }}
                <span v-if="mode === 'en2cn' && current.phonetic" class="prompt-ph">{{ current.phonetic }}</span>
              </p>
              <button v-if="mode === 'en2cn'" class="listen" @click="speak(current.word)">🔊 朗读</button>

              <div class="choices">
                <button
                  v-for="c in choices"
                  :key="c.id"
                  class="cho-row"
                  :class="choiceClass(c.id)"
                  @click="choose(c.id)"
                >
                  <span class="cho-text">{{ c.text }}</span>
                  <span v-if="answered !== null && c.id === current.id" class="cho-mark">✓</span>
                  <span v-else-if="answered !== null && c.id === picked" class="cho-mark">✕</span>
                </button>
              </div>
            </template>

            <!-- 拼写默写 -->
            <template v-else>
              <p class="prompt prompt--cn">{{ current.meaning }}</p>
              <p v-if="current.phonetic" class="spell-hint">音标：{{ current.phonetic }}</p>
              <p class="spell-hint">首字母：{{ current.word.slice(0, 1) }} · 共 {{ current.word.replace(/\s/g, '').length }} 个字母</p>
              <input
                v-model="spellText"
                class="input spell-input"
                :disabled="answered !== null"
                placeholder="输入英文单词"
                autocapitalize="off"
                autocomplete="off"
                spellcheck="false"
                @keyup.enter="answered === null && submitSpell()"
              />
              <button v-if="answered === null" class="btn btn--primary btn--block mt-md" :disabled="!spellText.trim()" @click="submitSpell">
                提交
              </button>
            </template>

            <div v-if="answered !== null" class="reveal">
              <p class="verdict" :class="answered ? 'ok' : 'no'">
                {{ answered ? '✅ 答对了' : '❌ 答错了' }}
              </p>
              <p class="ans">
                <b>{{ current.word }}</b>
                <span v-if="current.phonetic"> {{ current.phonetic }}</span>
                {{ current.meaning }}
              </p>
              <p v-if="current.example" class="wexample-sm">{{ current.example }}</p>
              <p v-if="current.example_cn" class="wexample-cn">{{ current.example_cn }}</p>
            </div>
          </div>

          <div class="sheet-foot">
            <button v-if="answered !== null" class="btn btn--primary btn--block" @click="next">
              {{ index < queue.length - 1 ? '下一个' : '完成本轮' }}
            </button>
            <button v-else-if="mode !== 'spell'" class="btn btn--ghost btn--block" disabled>请选择答案</button>
            <button v-else class="btn btn--ghost btn--block" disabled>输入后回车提交</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.e-head { padding: calc(var(--safe-top) + 16px) 0 16px; }
.e-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.e-sub { font-size: 13px; color: var(--text-3); margin-top: 5px; }

/* 状态概览 */
.status-card {
  display: flex;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  padding: 14px 8px;
}
.scell { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 2px; }
.scell-emoji { font-size: 18px; }
.scell-value { font-size: 18px; font-weight: 800; }
.scell-label { font-size: 11.5px; color: var(--text-3); }

/* 模式 */
.mode-row { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-top: 12px; }
.mode-card {
  display: flex; flex-direction: column; align-items: center; gap: 5px;
  padding: 14px 6px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  cursor: pointer; font-family: inherit;
  transition: transform var(--dur-fast) var(--ease);
}
.mode-card:active { transform: scale(.96); }
.mode-emoji { font-size: 22px; }
.mode-name { font-size: 12.5px; font-weight: 700; text-align: center; }

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
.filter-foot { display: flex; align-items: center; gap: 12px; }
.fcount { font-size: 12.5px; color: var(--text-3); flex: 1; }
.text-link { background: none; border: none; color: var(--brand-600); font-size: 12.5px; cursor: pointer; font-family: inherit; padding: 4px; }

/* 单词列表 */
.wlist { display: flex; flex-direction: column; gap: 9px; }
.wcard {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  box-shadow: var(--sh-xs);
  overflow: hidden;
}
.wrow { display: flex; align-items: flex-start; gap: 10px; padding: 13px 14px; cursor: pointer; }
.wrow:active { background: var(--bg-subtle); }
.wmain { flex: 1; min-width: 0; }
.wword { font-size: 16.5px; font-weight: 700; display: flex; align-items: baseline; gap: 8px; flex-wrap: wrap; }
.wphonetic { font-size: 12px; font-weight: 500; color: var(--text-3); }
.wmeaning { font-size: 13.5px; color: var(--text-2); margin-top: 2px; }
.wmeta { display: flex; flex-wrap: wrap; gap: 8px; font-size: 11px; color: var(--text-4); margin-top: 5px; }
.wside { flex: none; display: flex; flex-direction: column; align-items: flex-end; gap: 7px; }
.wbtn {
  width: 32px; height: 32px; border-radius: 50%;
  border: 1px solid var(--border-strong); background: var(--surface);
  font-size: 15px; cursor: pointer; line-height: 1;
  transition: all var(--dur-fast);
}
.wbtn:active { background: var(--brand-50); border-color: var(--brand-300); }

.wdetail { padding: 0 14px 13px; border-top: 1px dashed var(--border); }
.wexample { padding-top: 11px; }
.we-en { display: block; font-size: 13.5px; line-height: 1.65; color: var(--text-1); font-style: italic; }
.we-cn { display: block; font-size: 12.5px; color: var(--text-3); margin-top: 3px; }
.wnoexample { font-size: 12.5px; color: var(--text-4); padding-top: 11px; }
.wactions { display: flex; gap: 8px; margin-top: 12px; flex-wrap: wrap; }
.wactions .btn { flex: 1; min-width: 92px; }

/* 空态 */
.empty { text-align: center; padding: 44px 20px; background: var(--surface); border-radius: var(--r-lg); border: 1px dashed var(--border-strong); }
.empty--sm { padding: 30px 18px; }
.empty-emoji { font-size: 38px; }
.empty-title { font-size: 15px; font-weight: 700; margin-top: 10px; }
.empty-desc { font-size: 12.5px; color: var(--text-3); margin-top: 4px; }

.bottom-space { height: 20px; }

/* 练习弹层 */
.sheet-mask { position: fixed; inset: 0; z-index: 100; background: rgba(0,0,0,.4); display: flex; align-items: flex-end; justify-content: center; }
.sheet { width: 100%; max-width: 640px; background: var(--surface); border-radius: var(--r-2xl) var(--r-2xl) 0 0; max-height: 94vh; display: flex; flex-direction: column; box-shadow: var(--sh-xl); }
.sheet-head { display: flex; align-items: center; gap: 12px; padding: 16px 20px 10px; }
.scount { font-size: 13px; font-weight: 700; color: var(--text-2); flex: none; }
.sbar { flex: 1; height: 6px; border-radius: var(--r-full); background: var(--bg-subtle); overflow: hidden; }
.sbar i { display: block; height: 100%; border-radius: var(--r-full); background: var(--brand-gradient); transition: width var(--dur) var(--ease); }
.sheet-close { width: 32px; height: 32px; flex: none; border-radius: 50%; border: none; background: var(--bg-subtle); color: var(--text-3); font-size: 15px; cursor: pointer; }
.sheet-body { padding: 4px 20px 12px; overflow-y: auto; flex: 1; }
.sheet-foot { padding: 14px 20px calc(14px + var(--safe-bottom)); border-top: 1px solid var(--border); }

.pmode { font-size: 12px; color: var(--text-3); text-align: center; }
.prompt {
  font-size: 30px; font-weight: 800; text-align: center;
  margin-top: 14px; line-height: 1.4; word-break: break-word;
}
.prompt--cn { font-size: 22px; }
.prompt-ph { display: block; font-size: 13px; font-weight: 500; color: var(--text-3); margin-top: 4px; }
.listen {
  display: block; margin: 10px auto 0;
  padding: 6px 16px; border-radius: var(--r-full);
  border: 1px solid var(--border-strong); background: var(--surface);
  font-size: 12.5px; color: var(--text-2); cursor: pointer; font-family: inherit;
}
.listen:active { background: var(--brand-50); }

.choices { display: flex; flex-direction: column; gap: 9px; margin-top: 20px; }
.cho-row {
  display: flex; align-items: center; gap: 10px;
  padding: 14px;
  border-radius: var(--r-md);
  border: 2px solid var(--border);
  background: var(--surface-2);
  cursor: pointer; font-family: inherit; text-align: left; width: 100%;
  transition: all var(--dur-fast);
}
.cho-text { flex: 1; font-size: 15px; font-weight: 600; line-height: 1.5; }
.cho-mark { flex: none; font-size: 16px; font-weight: 800; }
.cho-row.on { border-color: var(--brand-500); background: var(--brand-50); }
.cho-row.right { border-color: var(--success); background: var(--success-bg); }
.cho-row.right .cho-mark { color: var(--success); }
.cho-row.wrong { border-color: var(--danger); background: var(--danger-bg); }
.cho-row.wrong .cho-mark { color: var(--danger); }
.cho-row.dim { opacity: .5; }

.spell-hint { font-size: 12.5px; color: var(--text-3); text-align: center; margin-top: 6px; }
.spell-input { margin-top: 16px; font-size: 19px; font-weight: 700; text-align: center; min-height: 54px; }

.reveal { margin-top: 18px; padding: 14px; background: var(--bg-subtle); border-radius: var(--r-md); }
.verdict { font-size: 13.5px; font-weight: 700; }
.verdict.ok { color: var(--success); }
.verdict.no { color: var(--danger); }
.ans { font-size: 14.5px; margin-top: 6px; line-height: 1.6; }
.wexample-sm { font-size: 13px; color: var(--text-2); margin-top: 8px; font-style: italic; line-height: 1.6; }
.wexample-cn { font-size: 12px; color: var(--text-3); margin-top: 2px; }

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }

@media (min-width: 700px) {
  .wlist { display: grid; grid-template-columns: repeat(2, 1fr); }
  .choices { display: grid; grid-template-columns: repeat(2, 1fr); }
}
</style>
