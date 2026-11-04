<script setup>
/**
 * 学习板块 · 首页
 *
 * 三个子功能入口（刷题 / 英语学习 / 知识点）+ 学习概览，
 * 并提供固定格式 JSON 的导入、导出与模板下载。
 */
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useStudyStore, STUDY_SCHEMA, STUDY_VERSION } from '@/stores/study'
import { toast } from '@/composables/useToast'
import { api } from '@/api'
import { compressToJpeg } from '@/utils/image'

defineOptions({ name: 'StudyView' })

const router = useRouter()
const study = useStudyStore()
const stats = computed(() => study.stats)

const entries = computed(() => [
  {
    key: 'quiz',
    emoji: '✏️',
    title: '刷题',
    desc: '单选、多选、判断、填空、问答，自动记错题本',
    to: '/tools/study/quiz',
    badge: stats.value.questions.wrong ? `${stats.value.questions.wrong} 道错题` : '',
    badgeLevel: stats.value.questions.wrong ? 'danger' : '',
    stats: stats.value.questions.total
      ? [
          { label: '题库', value: `${stats.value.questions.total} 道` },
          { label: '已练', value: `${stats.value.questions.done} 道` },
          { label: '正确率', value: `${stats.value.questions.accuracy}%` }
        ]
      : []
  },
  {
    key: 'english',
    emoji: '🔤',
    title: '英语学习',
    desc: '单词卡、音标、例句，按等级与单元分组',
    to: '/tools/study/english',
    badge: stats.value.english.learning ? `${stats.value.english.learning} 个在学` : '',
    stats: stats.value.english.total
      ? [
          { label: '单词', value: `${stats.value.english.total} 个` },
          { label: '已掌握', value: `${stats.value.english.mastered} 个` },
          { label: '未学', value: `${stats.value.english.fresh} 个` }
        ]
      : []
  },
  {
    key: 'notes',
    emoji: '📘',
    title: '知识点',
    desc: '笔记、要点、考点标记，按学科和标签检索',
    to: '/tools/study/notes',
    badge: stats.value.notes.examPoints ? `${stats.value.notes.examPoints} 个考点` : '',
    stats: stats.value.notes.total
      ? [
          { label: '知识点', value: `${stats.value.notes.total} 条` },
          { label: '已掌握', value: `${stats.value.notes.mastered} 条` },
          { label: '学科', value: `${study.usedSubjects.length} 个` }
        ]
      : []
  }
])

/* ---------------- JSON 导入 ---------------- */

const importing = ref(false)
const importBank = ref('')
const importText = ref('')
const importResult = ref(null)
const fileInput = ref(null)

/** 题库元数据：展示图片 + 说明 */
const importCover = ref('')      // 图片地址（/api/uploads/xxx 或 dataURL 预览）
const importDesc = ref('')
const coverUploading = ref(false)
const coverInput = ref(null)

function openImport() {
  importBank.value = ''
  importText.value = ''
  importResult.value = null
  importCover.value = ''
  importDesc.value = ''
  importing.value = true
}

function pickFile() {
  fileInput.value?.click()
}

/** 选择展示图片 → 压缩 → 上传后端 → 存 /api/uploads/xxx */
async function onCoverPicked(e) {
  const file = e.target.files?.[0]
  e.target.value = ''
  if (!file) return
  coverUploading.value = true
  try {
    const dataUrl = await compressToJpeg(file, { maxSize: 800, quality: 0.85 })
    const res = await api.uploads.create(dataUrl)
    importCover.value = res?.url || dataUrl
    toast.success('展示图片已上传')
  } catch (err) {
    toast.error(err?.message || '图片上传失败')
  } finally {
    coverUploading.value = false
  }
}

function onFilePicked(e) {
  const file = e.target.files?.[0]
  e.target.value = ''
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    importText.value = String(reader.result || '')
    toast.success(`已读入 ${file.name}`)
  }
  reader.onerror = () => toast.error('文件读取失败')
  reader.readAsText(file, 'utf-8')
}

function doImport() {
  const text = importText.value.trim()
  if (!text) {
    toast.warn('请粘贴 JSON 或选择文件')
    return
  }
  const bank = importBank.value.trim()
  if (!bank) {
    toast.warn('请填写题库名称')
    return
  }

  // 名称唯一：已存在同名题库 → 直接拒绝，要求改名
  if (study.bankExists(bank)) {
    toast.error(`题库名称「${bank}」已存在，请换一个唯一名称`)
    return
  }

  const res = study.importJSON(text, {
    bank,
    bankAction: 'merge',
    bankMeta: { cover: importCover.value, desc: importDesc.value.trim() }
  })
  importResult.value = res
  if (res.ok) {
    const s = res.summary
    toast.success(`已导入「${s.bank}」：新增 ${s.added}${s.updated ? `，覆盖 ${s.updated} 题` : ''}`)
  } else {
    toast.error(res.error || '导入失败')
  }
}

function loadTemplate() {
  importText.value = study.templateJSON()
  importResult.value = null
  toast.success('已填入模板，可直接改内容再导入')
}

/* ---------------- JSON 导出 ---------------- */

function download(filename, text, mime = 'application/json') {
  const blob = new Blob([text], { type: `${mime};charset=utf-8` })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

const stamp = () => new Date().toISOString().slice(0, 19).replace(/[:T]/g, '').slice(0, 14)

function exportData() {
  if (study.isEmpty) {
    toast.warn('还没有数据可导出')
    return
  }
  download(`study-${stamp()}.json`, study.exportJSON({ withProgress: true }))
  toast.success('已导出（含学习进度）')
}

function downloadTemplate() {
  download('study-template.json', study.templateJSON())
  toast.success('模板已下载')
}

function clearAll() {
  if (study.isEmpty) {
    toast.warn('当前没有学习数据')
    return
  }
  if (!confirm('确定清空全部学习数据吗？\n题库、单词、知识点与练习进度都会删除，且无法恢复。')) return
  study.clearAll()
  toast.success('已清空')
}
</script>

<template>
  <div class="page page--flat study">
    <div class="page-body">
      <header class="s-head">
        <h1 class="s-title">🎓 学习</h1>
        <p class="s-sub">刷题、英语、知识点，一处管起来</p>
      </header>

      <!-- 空库提示 -->
      <div v-if="study.isEmpty" class="hint-card">
        <span class="hint-emoji">📥</span>
        <div class="hint-body">
          <p class="hint-title">还没有学习内容</p>
          <p class="hint-desc">
            用下方「导入 JSON」把题库、单词和知识点一次性导进来，
            格式固定为 {{ STUDY_SCHEMA }} v{{ STUDY_VERSION }}。
          </p>
        </div>
        <button class="btn btn--primary btn--sm" @click="openImport">去导入</button>
      </div>

      <!-- 三个入口 -->
      <div class="entry-list stagger">
        <button
          v-for="e in entries"
          :key="e.key"
          class="entry-card clickable"
          @click="router.push(e.to)"
        >
          <div class="entry-top">
            <span class="entry-emoji">{{ e.emoji }}</span>
            <div class="entry-head">
              <p class="entry-title">{{ e.title }}</p>
              <p class="entry-desc">{{ e.desc }}</p>
            </div>
            <span
              v-if="e.badge"
              class="entry-badge"
              :class="{ 'entry-badge--alert': e.badgeLevel === 'danger' }"
            >{{ e.badge }}</span>
            <span v-else class="entry-arrow">›</span>
          </div>

          <div v-if="e.stats.length" class="entry-stats">
            <div v-for="(s, i) in e.stats" :key="i" class="estat">
              <span class="estat-value num">{{ s.value }}</span>
              <span class="estat-label">{{ s.label }}</span>
            </div>
          </div>
        </button>
      </div>

      <!-- 数据导入导出 -->
      <section class="section">
        <div class="section-head">
          <h2 class="section-title">数据导入 / 导出</h2>
          <span class="section-more">v{{ STUDY_VERSION }}</span>
        </div>

        <div class="io-card">
          <p class="io-desc">
            固定格式 JSON，一个文件同时承载题库、单词与知识点；
            导出的文件可直接再导回来。字段说明见 <code>docs/study-json-format.md</code>。
          </p>

          <div class="io-row">
            <button class="btn btn--primary grow" @click="openImport">📥 导入 JSON</button>
            <button class="btn btn--ghost grow" @click="exportData">📤 导出数据</button>
          </div>
          <div class="io-row">
            <button class="btn btn--soft grow" @click="downloadTemplate">下载空白模板</button>
            <button class="btn btn--danger grow" @click="clearAll">清空学习数据</button>
          </div>

          <div v-if="study.lastImport" class="io-last">
            <template v-if="study.lastImport.ok">
              上次导入「{{ study.lastImport.summary.bank }}」：{{ study.lastImport.summary.bankAction === 'replace' ? '替换' : '合并' }}，
              新增 {{ study.lastImport.summary.added }} 条<template v-if="study.lastImport.summary.updated"> / 覆盖 {{ study.lastImport.summary.updated }} 条</template><template v-if="study.lastImport.summary.removed"> / 移除 {{ study.lastImport.summary.removed }} 条</template>
            </template>
            <template v-else>上次导入失败：{{ study.lastImport.error }}</template>
          </div>
        </div>
      </section>

      <p class="s-tip">数据保存在本机，换设备或清除浏览器数据后不会同步</p>
      <div class="bottom-space" />
    </div>

    <!-- 导入弹层 -->
    <transition name="sheet">
      <div v-if="importing" class="sheet-mask" @click.self="importing = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>导入学习数据</h2>
            <button class="sheet-close" aria-label="关闭" @click="importing = false">✕</button>
          </div>

          <div class="sheet-body">
            <div class="field">
              <label class="field-label">题库名称 <span class="req">*</span></label>
              <input
                v-model="importBank"
                class="input"
                type="text"
                maxlength="30"
                placeholder="如：三年级数学上册、期末冲刺卷"
              />
              <p class="field-hint">
                本次导入的题目都会归到这个题库。题库名称唯一，已存在的名称会被拒绝，请换一个。
              </p>
            </div>

            <!-- 展示图片 -->
            <div class="field">
              <label class="field-label">展示图片</label>
              <div class="cover-row">
                <div class="cover-preview" @click="coverInput?.click()">
                  <img v-if="importCover" :src="importCover" alt="题库封面" />
                  <span v-else class="cover-placeholder">
                    {{ coverUploading ? '上传中…' : '＋ 上传封面' }}
                  </span>
                </div>
                <div class="cover-side">
                  <p class="field-hint">可选。上传一张代表这套试卷的图片，刷题页轮盘上会展示。</p>
                  <button type="button" class="btn btn--ghost btn--sm" :disabled="coverUploading" @click="coverInput?.click()">
                    选择图片
                  </button>
                  <button
                    v-if="importCover"
                    type="button"
                    class="text-link"
                    @click="importCover = ''"
                  >移除</button>
                </div>
              </div>
              <input ref="coverInput" type="file" accept="image/*" class="sr-only" @change="onCoverPicked" />
            </div>

            <div class="field">
              <label class="field-label">说明</label>
              <textarea
                v-model="importDesc"
                class="textarea"
                rows="2"
                maxlength="200"
                placeholder="这套试卷的简介、适用范围等（可选）"
              />
            </div>

            <div class="field">
              <label class="field-label">JSON 内容</label>
              <textarea
                v-model="importText"
                class="textarea json-area"
                spellcheck="false"
                placeholder='粘贴完整 JSON，或点下面的「选择文件」。最小结构：{"data":{"questions":[],"english":[],"notes":[]}}'
              />
              <div class="io-row mt-sm">
                <button class="btn btn--ghost btn--sm grow" @click="pickFile">选择文件…</button>
                <button class="btn btn--soft btn--sm grow" @click="loadTemplate">填入模板</button>
              </div>
              <input ref="fileInput" type="file" accept=".json,application/json" class="sr-only" @change="onFilePicked" />
            </div>

            <!-- 导入结果 -->
            <div v-if="importResult" class="result" :class="importResult.ok ? 'result--ok' : 'result--err'">
              <p class="result-title">
                {{ importResult.ok ? '✅ 导入成功' : '❌ 导入失败' }}
              </p>
              <template v-if="importResult.ok">
                <div class="result-grid">
                  <span>题目 {{ importResult.summary.questions }}</span>
                  <span>单词 {{ importResult.summary.english }}</span>
                  <span>知识点 {{ importResult.summary.notes }}</span>
                  <span>学科 {{ importResult.summary.subjects }}</span>
                </div>
                <div v-if="importResult.summary.skipped.length" class="result-skip">
                  <p class="skip-title">已跳过 {{ importResult.summary.skipped.length }} 条：</p>
                  <ul>
                    <li v-for="(s, i) in importResult.summary.skipped.slice(0, 8)" :key="i">
                      {{ s.kind }} 第 {{ s.at }} 条{{ s.id ? `（${s.id}）` : '' }}：{{ s.reason }}
                    </li>
                    <li v-if="importResult.summary.skipped.length > 8">
                      …另有 {{ importResult.summary.skipped.length - 8 }} 条
                    </li>
                  </ul>
                </div>
              </template>
              <p v-else class="result-msg">{{ importResult.error }}</p>
            </div>
          </div>

          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="importing = false">关闭</button>
            <button class="btn btn--primary grow" @click="doImport">开始导入</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.s-head { padding: calc(var(--safe-top) + 16px) 0 18px; }
.s-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.s-sub { font-size: 13px; color: var(--text-3); margin-top: 5px; }

/* 空库提示 */
.hint-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  margin-bottom: 14px;
  background: var(--brand-50);
  border: 1px dashed var(--brand-300);
  border-radius: var(--r-lg);
}
.hint-emoji { font-size: 26px; flex: none; }
.hint-body { flex: 1; min-width: 0; }
.hint-title { font-size: 14px; font-weight: 700; color: var(--brand-700); }
.hint-desc { font-size: 12px; color: var(--text-2); margin-top: 3px; line-height: 1.5; }
.hint-desc code { font-size: 11.5px; }

/* 入口卡片 */
.entry-list { display: flex; flex-direction: column; gap: 12px; }
.entry-card {
  width: 100%;
  text-align: left;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  padding: 16px;
  cursor: pointer;
  font-family: inherit;
  transition: transform var(--dur-fast) var(--ease), box-shadow var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
.entry-card:active { transform: scale(.98); box-shadow: var(--sh-md); }

.entry-top { display: flex; align-items: flex-start; gap: 12px; }
.entry-emoji {
  width: 46px; height: 46px; flex: none;
  border-radius: var(--r-md);
  background: var(--brand-50);
  display: flex; align-items: center; justify-content: center;
  font-size: 25px;
}
.entry-head { flex: 1; min-width: 0; }
.entry-title { font-size: 16.5px; font-weight: 700; }
.entry-desc { font-size: 12.5px; color: var(--text-3); margin-top: 3px; line-height: 1.45; }

.entry-badge {
  flex: none;
  padding: 3px 10px;
  border-radius: var(--r-full);
  background: var(--bg-subtle);
  color: var(--text-2);
  font-size: 11.5px;
  font-weight: 600;
  white-space: nowrap;
}
.entry-badge--alert { background: var(--danger-bg); color: var(--danger); }
.entry-arrow { flex: none; font-size: 22px; color: var(--text-4); line-height: 1; margin-top: 2px; }

.entry-stats {
  display: flex;
  gap: 10px;
  margin-top: 14px;
  padding-top: 13px;
  border-top: 1px dashed var(--border);
}
.estat { flex: 1; display: flex; flex-direction: column; gap: 1px; }
.estat-value { font-size: 15.5px; font-weight: 700; color: var(--text-1); }
.estat-label { font-size: 11.5px; color: var(--text-3); }

/* 导入导出卡 */
.io-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  padding: 16px;
}
.io-desc { font-size: 12.5px; color: var(--text-2); line-height: 1.6; }
.io-desc code {
  font-size: 11.5px;
  background: var(--bg-subtle);
  padding: 1px 5px;
  border-radius: var(--r-xs);
  color: var(--brand-700);
}
.io-row { display: flex; gap: 10px; margin-top: 12px; }
.io-last {
  margin-top: 12px;
  padding-top: 11px;
  border-top: 1px dashed var(--border);
  font-size: 11.5px;
  color: var(--text-3);
  line-height: 1.5;
}

/* 弹层 */
.sheet-mask { position: fixed; inset: 0; z-index: 100; background: rgba(0,0,0,.4); display: flex; align-items: flex-end; justify-content: center; }
.sheet { width: 100%; max-width: 620px; background: var(--surface); border-radius: var(--r-2xl) var(--r-2xl) 0 0; max-height: 92vh; display: flex; flex-direction: column; box-shadow: var(--sh-xl); }
.sheet-head { display: flex; align-items: center; justify-content: space-between; padding: 18px 20px 12px; }
.sheet-head h2 { font-size: 18px; font-weight: 700; }
.sheet-close { width: 32px; height: 32px; border-radius: 50%; border: none; background: var(--bg-subtle); color: var(--text-3); font-size: 15px; cursor: pointer; }
.sheet-body { padding: 0 20px; overflow-y: auto; flex: 1; }
.sheet-foot { display: flex; gap: 10px; padding: 14px 20px calc(14px + var(--safe-bottom)); border-top: 1px solid var(--border); }

.seg { display: flex; gap: 6px; background: var(--bg-subtle); padding: 4px; border-radius: var(--r-sm); }
.seg-btn { flex: 1; padding: 9px; border: none; background: transparent; border-radius: var(--r-xs); font-size: 13px; font-weight: 600; color: var(--text-2); cursor: pointer; font-family: inherit; white-space: nowrap; }
.seg-btn.on { background: var(--surface); color: var(--brand-600); box-shadow: var(--sh-xs); }

.req { color: var(--danger); font-weight: 700; }

/* 封面预览 */
.cover-row { display: flex; gap: 14px; align-items: flex-start; }
.cover-preview {
  width: 96px; height: 128px; flex: none;
  border-radius: var(--r-md);
  border: 1.5px dashed var(--border-strong);
  background: var(--bg-subtle);
  overflow: hidden;
  cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  transition: border-color var(--dur-fast);
}
.cover-preview:hover { border-color: var(--brand-500); }
.cover-preview img { width: 100%; height: 100%; object-fit: cover; display: block; }
.cover-placeholder {
  font-size: 12px; color: var(--text-3); text-align: center; line-height: 1.5; padding: 0 8px;
}
.cover-side { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 8px; align-items: flex-start; }
.cover-side .field-hint { margin-top: 0; }

.json-area {
  min-height: 190px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre;
  overflow-x: auto;
}

.result { margin-bottom: 6px; padding: 12px 14px; border-radius: var(--r-sm); font-size: 12.5px; }
.result--ok { background: var(--success-bg); color: #4F7A52; }
.result--err { background: var(--danger-bg); color: #A9443C; }
.result-title { font-weight: 700; }
.result-msg { margin-top: 5px; line-height: 1.55; word-break: break-all; }
.result-grid { display: flex; flex-wrap: wrap; gap: 6px 14px; margin-top: 7px; font-weight: 600; }
.result-skip { margin-top: 9px; padding-top: 8px; border-top: 1px dashed rgba(0,0,0,.12); }
.skip-title { font-weight: 700; }
.result-skip ul { margin: 5px 0 0; padding-left: 18px; line-height: 1.7; }

.bottom-space { height: 20px; }
.s-tip { margin-top: 18px; font-size: 11.5px; color: var(--text-4); text-align: center; line-height: 1.5; }

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }

@media (min-width: 700px) {
  .entry-list { display: grid; grid-template-columns: repeat(3, 1fr); }
}
</style>
