<script setup>
/**
 * 系统版本详情
 *
 * 个人中心入口落点：展示当前版本号与历次更新内容（时间线）。
 * 每次系统更新由掌勺人追加一条记录（版本号 + 更新内容），存于 system_version 表。
 */
import { computed, onMounted, reactive, ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { api } from '@/api'
import { toast } from '@/composables/useToast'
import EmptyState from '@/components/EmptyState.vue'

const app = useAppStore()

const loading = ref(false)
const list = ref([])
/** 当前版本 = 最新一条记录 */
const current = computed(() => list.value[0] || null)

const isAdmin = computed(() => app.currentMember?.role === 'admin')

const adding = ref(false)
const saving = ref(false)
const form = reactive({ version: '', title: '', content: '' })

onMounted(load)

async function load() {
  loading.value = true
  try {
    list.value = (await api.versions.list()) || []
  } catch (e) {
    toast.error(e.message || '版本记录加载失败')
  } finally {
    loading.value = false
  }
}

/** 发布日期显示：YYYY-MM-DD */
function dateText(dt) {
  if (!dt) return ''
  return String(dt).slice(0, 10)
}

/** 更新内容按行展示 */
function contentLines(text) {
  return String(text || '').split('\n').map((s) => s.trim()).filter(Boolean)
}

function openAdd() {
  Object.assign(form, { version: '', title: '', content: '' })
  adding.value = true
}

async function save() {
  const version = form.version.trim()
  if (!version) { toast.warn('请填写版本号，如 1.1'); return }
  if (!form.content.trim()) { toast.warn('请填写更新内容'); return }
  saving.value = true
  try {
    await api.versions.create({
      version,
      title: form.title.trim() || null,
      content: form.content.trim()
    })
    toast.success('版本记录已追加')
    adding.value = false
    await load()
  } catch (e) {
    toast.error(e.message || '保存失败，请重试')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="page page--flat ver">
    <div class="page-body">
      <!-- 当前版本卡 -->
      <section class="hero stagger">
        <div class="hero-badge">🏷️</div>
        <p class="hero-label">当前版本</p>
        <p class="hero-version">v{{ current?.version || '1.0' }}</p>
        <p v-if="current" class="hero-date">{{ dateText(current.released_at) }}</p>
      </section>

      <!-- 更新记录 -->
      <section class="log stagger">
        <div class="log-head">
          <h2 class="log-title">更新记录</h2>
          <button v-if="isAdmin" class="log-add" @click="openAdd">＋ 记录新版本</button>
        </div>

        <EmptyState
          v-if="!loading && !list.length"
          emoji="📜"
          title="还没有版本记录"
          desc="系统每次更新后会在这里追加一条更新内容"
        />

        <ol v-else class="tl">
          <li v-for="(v, i) in list" :key="v.id" class="tl-item">
            <div class="tl-rail">
              <span class="tl-dot" :class="{ on: i === 0 }"></span>
              <span v-if="i < list.length - 1" class="tl-line"></span>
            </div>
            <div class="tl-card">
              <div class="tl-top">
                <span class="tl-ver" :class="{ on: i === 0 }">v{{ v.version }}</span>
                <span v-if="i === 0" class="tl-now">当前</span>
                <span class="tl-date">{{ dateText(v.released_at) }}</span>
              </div>
              <p v-if="v.title" class="tl-name">{{ v.title }}</p>
              <ul class="tl-content">
                <li v-for="(line, j) in contentLines(v.content)" :key="j">{{ line }}</li>
              </ul>
            </div>
          </li>
        </ol>
      </section>

      <p class="foot-note">系统每次更新都会在这里追加一条记录，方便家人了解新功能</p>
    </div>

    <!-- 记录新版本弹层 -->
    <transition name="sheet">
      <div v-if="adding" class="sheet-mask" @click.self="adding = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>记录新版本</h2>
            <button class="sheet-close" @click="adding = false">✕</button>
          </div>
          <div class="sheet-body">
            <div class="field">
              <label class="field-label">版本号</label>
              <input v-model="form.version" type="text" class="input" placeholder="如 1.1" maxlength="32" />
            </div>
            <div class="field">
              <label class="field-label">更新主题（可选）</label>
              <input v-model="form.title" type="text" class="input" placeholder="一句话概括这次更新" maxlength="128" />
            </div>
            <div class="field">
              <label class="field-label">更新内容（一行一点）</label>
              <textarea
                v-model="form.content"
                class="input textarea"
                rows="6"
                placeholder="例：&#10;- 新增系统版本详情页&#10;- 修复了若干问题"
              ></textarea>
            </div>
          </div>
          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="adding = false">取消</button>
            <button class="btn btn--primary grow" :disabled="saving" @click="save">{{ saving ? '保存中…' : '保存' }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 36px 0 28px;
}
.hero-badge {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  background: var(--brand-50);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 36px;
  box-shadow: var(--sh-sm);
}
.hero-label {
  margin-top: 14px;
  font-size: 12.5px;
  font-weight: 700;
  color: var(--text-3);
  letter-spacing: .08em;
}
.hero-version {
  margin-top: 4px;
  font-size: 34px;
  font-weight: 800;
  letter-spacing: -.02em;
  color: var(--brand-600);
}
.hero-date {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-4);
}

.log {
  background: var(--surface);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  padding: 16px 16px 8px;
}
.log-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}
.log-title { font-size: 16px; font-weight: 800; }
.log-add {
  border: none;
  background: var(--brand-50);
  color: var(--brand-600);
  font-family: inherit;
  font-size: 12.5px;
  font-weight: 700;
  padding: 7px 12px;
  border-radius: var(--r-full);
  cursor: pointer;
  transition: all var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
.log-add:active { transform: scale(.95); background: var(--brand-100); }

/* 时间线 */
.tl { list-style: none; margin: 0; padding: 4px 0 8px; }
.tl-item { display: flex; gap: 12px; }
.tl-rail {
  flex: none;
  width: 14px;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-top: 6px;
}
.tl-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--border);
  flex: none;
}
.tl-dot.on { background: var(--brand-500); box-shadow: 0 0 0 4px var(--brand-50); }
.tl-line {
  flex: 1;
  width: 2px;
  background: var(--border);
  margin: 4px 0;
  border-radius: 1px;
}
.tl-card { flex: 1; min-width: 0; padding-bottom: 18px; }
.tl-top { display: flex; align-items: center; gap: 8px; }
.tl-ver {
  font-size: 15px;
  font-weight: 800;
  color: var(--text-1);
}
.tl-ver.on { color: var(--brand-600); }
.tl-now {
  font-size: 10.5px;
  font-weight: 700;
  color: var(--brand-600);
  background: var(--brand-50);
  padding: 2px 8px;
  border-radius: var(--r-full);
}
.tl-date { margin-left: auto; font-size: 11.5px; color: var(--text-4); }
.tl-name { margin-top: 4px; font-size: 13.5px; font-weight: 700; color: var(--text-2); }
.tl-content {
  margin: 6px 0 0;
  padding-left: 18px;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.tl-content li {
  font-size: 13px;
  color: var(--text-2);
  line-height: 1.65;
  list-style: disc;
}

.foot-note {
  margin-top: 18px;
  text-align: center;
  font-size: 11.5px;
  color: var(--text-4);
  line-height: 1.6;
}

/* 弹层 */
.sheet-mask {
  position: fixed;
  inset: 0;
  z-index: 100;
  background: rgba(0, 0, 0, .4);
  display: flex;
  align-items: flex-end;
  justify-content: center;
}
.sheet {
  width: 100%;
  max-width: 560px;
  background: var(--surface);
  border-radius: var(--r-2xl) var(--r-2xl) 0 0;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--sh-xl);
}
.sheet-head { display: flex; align-items: center; justify-content: space-between; padding: 18px 20px 12px; }
.sheet-head h2 { font-size: 18px; font-weight: 700; }
.sheet-close {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: none;
  background: var(--bg-subtle);
  color: var(--text-3);
  font-size: 15px;
  cursor: pointer;
}
.sheet-body { padding: 0 20px; overflow-y: auto; flex: 1; }
.sheet-foot {
  display: flex;
  gap: 10px;
  padding: 14px 20px calc(14px + var(--safe-bottom));
  border-top: 1px solid var(--border);
}
.textarea { resize: vertical; min-height: 120px; line-height: 1.6; }

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }

@media (min-width: 700px) {
  .log { max-width: 560px; margin: 0 auto; }
}
</style>
