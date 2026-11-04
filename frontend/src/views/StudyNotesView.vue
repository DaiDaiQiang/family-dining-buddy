<script setup>
/**
 * 学习 · 知识点
 *
 * 笔记式知识卡：按学科 / 标签 / 考点筛选，支持搜索与「已掌握」标记，
 * 可在此页新增、编辑、删除单条知识点。批量录入走「学习」页的 JSON 导入。
 */
import { computed, reactive, ref } from 'vue'
import { useStudyStore } from '@/stores/study'
import { toast } from '@/composables/useToast'

defineOptions({ name: 'StudyNotesView' })

const study = useStudyStore()
const stats = computed(() => study.stats.notes)

/* ---------------- 筛选 ---------------- */

const fSubject = ref('')
const fTag = ref('')
const fExamOnly = ref(false)
const fUndone = ref(false)
const keyword = ref('')

const filtered = computed(() => {
  const k = keyword.value.trim().toLowerCase()
  return study.notes
    .filter((n) => {
      if (fSubject.value && n.subject !== fSubject.value) return false
      if (fTag.value && !(n.tags || []).includes(fTag.value)) return false
      if (fExamOnly.value && !n.is_exam_point) return false
      if (fUndone.value && study.progress.notes[n.id]?.mastered) return false
      if (k && !`${n.title} ${n.content} ${(n.key_points || []).join(' ')} ${(n.tags || []).join(' ')}`.toLowerCase().includes(k)) {
        return false
      }
      return true
    })
    .sort((a, b) => Number(!!b.is_exam_point) - Number(!!a.is_exam_point) || b.updated_at.localeCompare(a.updated_at))
})

const subjectsInUse = computed(() => {
  const set = new Set(study.notes.map((n) => n.subject).filter(Boolean))
  return study.allSubjects.filter((s) => set.has(s.value))
})

const noteTags = computed(() => {
  const tally = new Map()
  study.notes.forEach((n) => (n.tags || []).forEach((t) => tally.set(t, (tally.get(t) || 0) + 1)))
  return [...tally.entries()].map(([name, count]) => ({ name, count })).sort((a, b) => b.count - a.count)
})

const filterOn = computed(() => !!(fSubject.value || fTag.value || fExamOnly.value || fUndone.value || keyword.value))

function resetFilter() {
  fSubject.value = ''
  fTag.value = ''
  fExamOnly.value = false
  fUndone.value = false
  keyword.value = ''
}

/* ---------------- 展开 ---------------- */

const expanded = ref(null)
function toggle(id) {
  if (expanded.value === id) {
    expanded.value = null
    return
  }
  expanded.value = id
  study.viewNote(id)
}

function toggleMastered(n) {
  const on = study.toggleNoteMastered(n.id)
  toast.success(on ? `已标记掌握「${n.title}」` : `已取消掌握「${n.title}」`)
}

/* ---------------- 新增 / 编辑 ---------------- */

const editing = ref(false)
const editId = ref(null)
const saving = ref(false)
const form = reactive({
  subject: '',
  title: '',
  content: '',
  key_points: '',
  tags: '',
  is_exam_point: false
})

function openCreate() {
  editId.value = null
  Object.assign(form, {
    subject: fSubject.value || '',
    title: '',
    content: '',
    key_points: '',
    tags: fTag.value || '',
    is_exam_point: fExamOnly.value
  })
  editing.value = true
}

function openEdit(n) {
  editId.value = n.id
  Object.assign(form, {
    subject: n.subject || '',
    title: n.title || '',
    content: n.content || '',
    key_points: (n.key_points || []).join('\n'),
    tags: (n.tags || []).join(','),
    is_exam_point: !!n.is_exam_point
  })
  editing.value = true
}

function save() {
  if (!form.title.trim() && !form.content.trim()) {
    toast.warn('标题和内容至少要填一项')
    return
  }
  saving.value = true
  try {
    const payload = {
      id: editId.value || undefined,
      subject: form.subject.trim(),
      title: form.title.trim(),
      content: form.content.trim(),
      key_points: form.key_points.split('\n').map((x) => x.trim()).filter(Boolean),
      tags: form.tags,
      is_exam_point: form.is_exam_point
    }
    const saved = study.saveNote(payload)
    toast.success(editId.value ? '已更新' : '已添加')
    editing.value = false
    expanded.value = saved.id
  } catch (e) {
    toast.error(e.message || '保存失败')
  } finally {
    saving.value = false
  }
}

function removeNote(n) {
  if (!confirm(`确定删除这条知识点吗？\n${n.title}`)) return
  try {
    study.removeNote(n.id)
    if (expanded.value === n.id) expanded.value = null
    toast.success('已删除')
  } catch (e) {
    toast.error(e.message || '删除失败')
  }
}

const mastered = (id) => !!study.progress.notes[id]?.mastered
const views = (id) => Number(study.progress.notes[id]?.views) || 0
</script>

<template>
  <div class="page page--flat notes">
    <div class="page-body">
      <header class="n-head">
        <h1 class="n-title">📘 知识点</h1>
        <p class="n-sub">
          共 {{ stats.total }} 条 · 已掌握 {{ stats.mastered }} 条
          <template v-if="stats.examPoints"> · <b class="warn">{{ stats.examPoints }} 个考点</b></template>
        </p>
      </header>

      <!-- 空态 -->
      <div v-if="!study.notes.length" class="empty">
        <span class="empty-emoji">📭</span>
        <p class="empty-title">还没有知识点</p>
        <p class="empty-desc">
          可以手动添加一条，或回到「学习」页用导入 JSON 批量录入
        </p>
        <div class="empty-actions">
          <button class="btn btn--primary btn--sm" @click="openCreate">＋ 添加知识点</button>
          <router-link to="/tools/study" class="btn btn--ghost btn--sm">去导入</router-link>
        </div>
      </div>

      <template v-else>
        <div class="action-row">
          <button class="btn btn--primary grow" @click="openCreate">＋ 添加知识点</button>
          <button class="btn btn--ghost" :class="{ 'btn--soft': fUndone }" @click="fUndone = !fUndone">
            只看未掌握
          </button>
        </div>

        <!-- 筛选 -->
        <section class="filter">
          <input v-model="keyword" class="input search" type="search" placeholder="搜标题、内容、要点…" />

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

          <div v-if="noteTags.length" class="chips">
            <button class="chip-btn" :class="{ on: !fTag }" @click="fTag = ''">全部标签</button>
            <button
              v-for="t in noteTags.slice(0, 14)"
              :key="t.name"
              class="chip-btn"
              :class="{ on: fTag === t.name }"
              @click="fTag = t.name"
            ># {{ t.name }}</button>
          </div>

          <div class="filter-foot">
            <span class="fcount">筛出 {{ filtered.length }} 条</span>
            <button class="chip-btn chip-btn--danger" :class="{ on: fExamOnly }" @click="fExamOnly = !fExamOnly">
              ⭐ 只看考点
            </button>
            <button v-if="filterOn" class="text-link" @click="resetFilter">重置</button>
          </div>
        </section>

        <!-- 知识点列表 -->
        <section class="section">
          <div class="section-head">
            <h2 class="section-title">知识卡片</h2>
            <span class="section-more">{{ filtered.length }} 条</span>
          </div>

          <div v-if="filtered.length" class="klist">
            <article
              v-for="n in filtered"
              :key="n.id"
              class="kcard"
              :class="{ 'kcard--done': mastered(n.id), 'kcard--open': expanded === n.id }"
            >
              <div class="khead" @click="toggle(n.id)">
                <div class="kmain">
                  <p class="ktitle">
                    <span v-if="n.is_exam_point" class="kstar" title="考点">⭐</span>
                    {{ n.title }}
                  </p>
                  <p class="kmeta">
                    <span v-if="n.subject">
                      {{ study.subjectMeta(n.subject).emoji }} {{ study.subjectName(n.subject) }}
                    </span>
                    <span v-for="t in n.tags.slice(0, 3)" :key="t"># {{ t }}</span>
                    <span v-if="views(n.id)">看过 {{ views(n.id) }} 次</span>
                  </p>
                </div>
                <span class="kchev" :class="{ open: expanded === n.id }">›</span>
              </div>

              <!-- 折叠时显示内容摘要 -->
              <p v-if="expanded !== n.id && n.content" class="kpreview ellipsis-2">{{ n.content }}</p>

              <div v-if="expanded === n.id" class="kbody">
                <p v-if="n.content" class="kcontent">{{ n.content }}</p>

                <div v-if="n.key_points.length" class="kpoints">
                  <p class="kpoints-title">要点</p>
                  <ul>
                    <li v-for="(p, i) in n.key_points" :key="i">{{ p }}</li>
                  </ul>
                </div>

                <div class="kactions">
                  <button
                    class="btn btn--sm"
                    :class="mastered(n.id) ? 'btn--primary' : 'btn--ghost'"
                    @click="toggleMastered(n)"
                  >{{ mastered(n.id) ? '✅ 已掌握' : '标记掌握' }}</button>
                  <button class="btn btn--ghost btn--sm" @click="openEdit(n)">编辑</button>
                  <button class="btn btn--danger btn--sm" @click="removeNote(n)">删除</button>
                </div>
              </div>
            </article>
          </div>

          <div v-else class="empty empty--sm">
            <span class="empty-emoji">🔍</span>
            <p class="empty-title">没有符合条件的知识点</p>
            <p class="empty-desc">换个筛选条件，或点「重置」</p>
          </div>
        </section>
      </template>

      <p class="n-tip">批量录入请用「学习」页的导入 JSON，格式固定、可反复导入</p>
      <div class="bottom-space" />
    </div>

    <!-- 新增 / 编辑弹层 -->
    <transition name="sheet">
      <div v-if="editing" class="sheet-mask" @click.self="editing = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>{{ editId ? '编辑知识点' : '添加知识点' }}</h2>
            <button class="sheet-close" aria-label="关闭" @click="editing = false">✕</button>
          </div>

          <div class="sheet-body">
            <div class="field">
              <label class="field-label">标题 <span class="req">*</span></label>
              <input v-model="form.title" class="input" placeholder="如：一元二次方程求根公式" maxlength="60" />
            </div>

            <div class="field">
              <label class="field-label">所属学科</label>
              <div class="seg seg--wrap">
                <button class="seg-btn" :class="{ on: !form.subject }" @click="form.subject = ''">不指定</button>
                <button
                  v-for="s in study.allSubjects"
                  :key="s.value"
                  class="seg-btn"
                  :class="{ on: form.subject === s.value }"
                  @click="form.subject = s.value"
                >{{ s.emoji }} {{ s.name }}</button>
              </div>
            </div>

            <div class="field">
              <label class="field-label">内容</label>
              <textarea v-model="form.content" class="textarea" placeholder="知识点讲解，可直接换行分段" />
            </div>

            <div class="field">
              <label class="field-label">要点</label>
              <textarea
                v-model="form.key_points"
                class="textarea textarea--sm"
                placeholder="一行一条，如：&#10;判别式 Δ = b² - 4ac&#10;Δ > 0 有两个不等实根"
              />
              <p class="field-hint">每行会成为一条要点</p>
            </div>

            <div class="field">
              <label class="field-label">标签</label>
              <input v-model="form.tags" class="input" placeholder="用逗号分隔，如：代数,必考" maxlength="80" />
            </div>

            <label class="switch-row">
              <input v-model="form.is_exam_point" type="checkbox" />
              <span>⭐ 标记为考点（列表里优先显示）</span>
            </label>
          </div>

          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="editing = false">取消</button>
            <button class="btn btn--primary grow" :disabled="saving" @click="save">
              {{ saving ? '保存中…' : '保存' }}
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.n-head { padding: calc(var(--safe-top) + 16px) 0 16px; }
.n-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.n-sub { font-size: 13px; color: var(--text-3); margin-top: 5px; }
.warn { color: var(--warning); font-weight: 700; }

.action-row { display: flex; gap: 10px; }

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
.chip-btn--danger.on { background: var(--warning); box-shadow: none; }
.filter-foot { display: flex; align-items: center; gap: 10px; }
.fcount { font-size: 12.5px; color: var(--text-3); flex: 1; }
.text-link { background: none; border: none; color: var(--brand-600); font-size: 12.5px; cursor: pointer; font-family: inherit; padding: 4px; }

/* 卡片列表 */
.klist { display: flex; flex-direction: column; gap: 10px; }
.kcard {
  background: var(--surface);
  border: 1px solid var(--border);
  border-left: 3px solid var(--border-strong);
  border-radius: var(--r-md);
  box-shadow: var(--sh-xs);
  overflow: hidden;
  transition: border-color var(--dur-fast), box-shadow var(--dur-fast);
}
.kcard--done { border-left-color: var(--success); opacity: .82; }
.kcard--open { box-shadow: var(--sh-md); border-left-color: var(--brand-500); opacity: 1; }

.khead { display: flex; align-items: flex-start; gap: 10px; padding: 13px 14px; cursor: pointer; }
.khead:active { background: var(--bg-subtle); }
.kmain { flex: 1; min-width: 0; }
.ktitle { font-size: 15px; font-weight: 700; line-height: 1.5; }
.kstar { margin-right: 3px; }
.kmeta { display: flex; flex-wrap: wrap; gap: 8px; font-size: 11.5px; color: var(--text-3); margin-top: 4px; }
.kchev { flex: none; font-size: 20px; color: var(--text-4); line-height: 1.2; transform: rotate(90deg); transition: transform var(--dur) var(--ease); }
.kchev.open { transform: rotate(-90deg); }

.kpreview { font-size: 13px; color: var(--text-3); line-height: 1.6; padding: 0 14px 13px; }

.kbody { padding: 0 14px 14px; border-top: 1px dashed var(--border); }
.kcontent { font-size: 14px; line-height: 1.75; color: var(--text-1); padding-top: 12px; white-space: pre-wrap; }
.kpoints { margin-top: 12px; padding: 11px 13px; background: var(--brand-50); border-radius: var(--r-sm); }
.kpoints-title { font-size: 11.5px; font-weight: 700; color: var(--brand-700); }
.kpoints ul { margin: 6px 0 0; padding-left: 18px; font-size: 13px; line-height: 1.75; color: var(--text-2); }
.kactions { display: flex; gap: 8px; margin-top: 13px; flex-wrap: wrap; }
.kactions .btn { flex: 1; min-width: 86px; }

/* 空态 */
.empty { text-align: center; padding: 44px 20px; background: var(--surface); border-radius: var(--r-lg); border: 1px dashed var(--border-strong); }
.empty--sm { padding: 30px 18px; }
.empty-emoji { font-size: 38px; }
.empty-title { font-size: 15px; font-weight: 700; margin-top: 10px; }
.empty-desc { font-size: 12.5px; color: var(--text-3); margin-top: 4px; line-height: 1.6; }
.empty-actions { display: flex; gap: 10px; justify-content: center; margin-top: 16px; }

.n-tip { margin-top: 18px; font-size: 11.5px; color: var(--text-4); text-align: center; line-height: 1.5; }
.bottom-space { height: 20px; }

/* 弹层 */
.sheet-mask { position: fixed; inset: 0; z-index: 100; background: rgba(0,0,0,.4); display: flex; align-items: flex-end; justify-content: center; }
.sheet { width: 100%; max-width: 620px; background: var(--surface); border-radius: var(--r-2xl) var(--r-2xl) 0 0; max-height: 92vh; display: flex; flex-direction: column; box-shadow: var(--sh-xl); }
.sheet-head { display: flex; align-items: center; justify-content: space-between; padding: 18px 20px 12px; }
.sheet-head h2 { font-size: 18px; font-weight: 700; }
.sheet-close { width: 32px; height: 32px; border-radius: 50%; border: none; background: var(--bg-subtle); color: var(--text-3); font-size: 15px; cursor: pointer; }
.sheet-body { padding: 0 20px; overflow-y: auto; flex: 1; }
.sheet-foot { display: flex; gap: 10px; padding: 14px 20px calc(14px + var(--safe-bottom)); border-top: 1px solid var(--border); }

.seg { display: flex; gap: 6px; background: var(--bg-subtle); padding: 4px; border-radius: var(--r-sm); }
.seg--wrap { flex-wrap: wrap; }
.seg-btn { padding: 8px 12px; border: none; background: transparent; border-radius: var(--r-xs); font-size: 12.5px; font-weight: 600; color: var(--text-2); cursor: pointer; font-family: inherit; white-space: nowrap; }
.seg-btn.on { background: var(--surface); color: var(--brand-600); box-shadow: var(--sh-xs); }
.textarea--sm { min-height: 76px; }

.switch-row { display: flex; align-items: center; gap: 9px; padding: 12px 0 4px; font-size: 13.5px; color: var(--text-2); cursor: pointer; }
.switch-row input { width: 18px; height: 18px; accent-color: var(--brand-500); cursor: pointer; }

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }

@media (min-width: 700px) {
  .klist { display: grid; grid-template-columns: repeat(2, 1fr); align-items: start; }
}
</style>
