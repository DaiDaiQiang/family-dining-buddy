<script setup>
/**
 * 下厨房菜谱导入弹窗
 *
 * 两步流程：
 *   1. 粘贴下厨房分享链接 → 点「解析」→ 后端抓取并返回预览（不落库）
 *   2. 预览里可编辑菜名/分类/菜系/难度，确认后落库建档 → 跳到详情页
 */
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import api from '@/api'
import { toast } from '@/composables/useToast'

defineOptions({ name: 'RecipeImportDialog' })

const props = defineProps({
  visible: { type: Boolean, default: false }
})
const emit = defineEmits(['update:visible', 'imported'])

const router = useRouter()
const app = useAppStore()

const url = ref('')
const parsing = ref(false)
const saving = ref(false)

// 预览结果（解析成功后填充）
const preview = ref(null)

// 编辑态字段
const form = ref({
  name: '',
  category: '',
  cuisine: '',
  difficulty: ''
})

const categories = computed(() => app.categories)
const cuisines = computed(() => app.cuisines)
const difficulties = computed(() => app.meta?.difficulties || [])

function close() {
  emit('update:visible', false)
  url.value = ''
  preview.value = null
  form.value = { name: '', category: '', cuisine: '', difficulty: '' }
}

function normalizeUrl(raw) {
  let u = (raw || '').trim()
  if (!u) return ''
  // 去掉分享文案里可能夹带的空格/换行，只取第一段 URL
  u = u.split(/\s+/)[0]
  return u
}

async function doParse() {
  const u = normalizeUrl(url.value)
  if (!u) {
    toast.warn('请先粘贴下厨房菜谱链接')
    return
  }
  parsing.value = true
  preview.value = null
  try {
    const data = await api.recipes.importPreview(u)
    preview.value = data
    form.value = {
      name: data.name || '',
      category: '',
      cuisine: '',
      difficulty: ''
    }
  } catch (e) {
    toast.error(e?.message || '解析失败，请检查链接后重试')
  } finally {
    parsing.value = false
  }
}

async function doSave() {
  if (!preview.value) return
  if (!form.value.name.trim()) {
    toast.warn('菜名不能为空')
    return
  }
  saving.value = true
  try {
    // 把预览的用料/步骤 + 用户编辑的分类等字段，整体提交建档
    const payload = {
      name: form.value.name.trim(),
      category: form.value.category || undefined,
      cuisine: form.value.cuisine || undefined,
      difficulty: form.value.difficulty || undefined,
      description: preview.value.description || undefined,
      ingredients: (preview.value.ingredients || []).map((ing) => ({
        name: ing.name,
        quantity: ing.quantity ?? undefined,
        unit: ing.unit || undefined
      })),
      steps: (preview.value.steps || []).map((s) => ({ content: s }))
    }
    const detail = await api.recipes.importCreate(payload)
    toast.success('导入成功，菜谱已建档')
    emit('imported', detail)
    close()
    if (detail?.id) router.push(`/recipes/${detail.id}`)
  } catch (e) {
    toast.error(e?.message || '建档失败，请重试')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <transition name="fade">
    <div v-if="visible" class="import-mask" @click.self="close">
      <div class="import-panel">
        <div class="import-head">
          <h3 class="import-title">🔗 导入下厨房菜谱</h3>
          <button class="import-close" @click="close" aria-label="关闭">✕</button>
        </div>

        <!-- 第一步：贴链接 -->
        <div class="import-body">
          <p class="import-tip">
            粘贴「下厨房」菜谱分享链接，自动抓取用料与做法步骤。
          </p>
          <div class="import-input-row">
            <input
              v-model="url"
              class="import-input"
              type="url"
              inputmode="url"
              placeholder="https://www.xiachufang.com/recipe/104218740/"
              @keyup.enter="doParse"
            />
          </div>
          <button
            class="btn btn--primary import-parse"
            :disabled="parsing"
            @click="doParse"
          >
            {{ parsing ? '正在解析…' : '解析链接' }}
          </button>

          <!-- 第二步：预览 + 编辑确认 -->
          <div v-if="preview" class="import-preview">
            <div class="preview-field">
              <label class="preview-label">菜名</label>
              <input v-model="form.name" class="preview-input" placeholder="菜名" />
            </div>

            <div class="preview-grid">
              <div class="preview-field">
                <label class="preview-label">分类</label>
                <select v-model="form.category" class="preview-input">
                  <option value="">家常菜（默认）</option>
                  <option v-for="c in categories" :key="c" :value="c">{{ c }}</option>
                </select>
              </div>
              <div class="preview-field">
                <label class="preview-label">菜系</label>
                <select v-model="form.cuisine" class="preview-input">
                  <option value="">中餐（默认）</option>
                  <option v-for="c in cuisines" :key="c" :value="c">{{ c }}</option>
                </select>
              </div>
              <div class="preview-field">
                <label class="preview-label">难度</label>
                <select v-model="form.difficulty" class="preview-input">
                  <option value="">简单（默认）</option>
                  <option v-for="d in difficulties" :key="d.value" :value="d.value">
                    {{ d.label }}
                  </option>
                </select>
              </div>
            </div>

            <div v-if="preview.ingredients?.length" class="preview-section">
              <div class="preview-section-title">用料（{{ preview.ingredients.length }}）</div>
              <div class="preview-chips">
                <span v-for="(ing, i) in preview.ingredients" :key="i" class="preview-chip">
                  {{ ing.name }}<template v-if="ing.amount"> · {{ ing.amount }}</template>
                </span>
              </div>
            </div>

            <div v-if="preview.steps?.length" class="preview-section">
              <div class="preview-section-title">步骤（{{ preview.steps.length }}）</div>
              <ol class="preview-steps">
                <li v-for="(s, i) in preview.steps" :key="i">{{ s }}</li>
              </ol>
            </div>

            <button
              class="btn btn--primary import-save"
              :disabled="saving"
              @click="doSave"
            >
              {{ saving ? '正在建档…' : '确认建档' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.import-mask {
  position: fixed; inset: 0; z-index: 200;
  background: rgba(0, 0, 0, .45);
  display: flex; align-items: flex-end; justify-content: center;
}
@media (min-width: 560px) { .import-mask { align-items: center; } }

.import-panel {
  width: 100%; max-width: 480px;
  max-height: 86vh; overflow-y: auto;
  background: var(--surface);
  border-radius: var(--r-lg) var(--r-lg) 0 0;
  padding: 18px 16px calc(18px + var(--safe-bottom));
  box-shadow: var(--sh-lg);
}
@media (min-width: 560px) {
  .import-panel { border-radius: var(--r-lg); padding-bottom: 18px; }
}

.import-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
.import-title { font-size: 17px; font-weight: 700; color: var(--text-1); }
.import-close {
  border: none; background: var(--bg-subtle); width: 28px; height: 28px;
  border-radius: 50%; color: var(--text-3); cursor: pointer; font-size: 14px;
}

.import-tip { font-size: 13px; color: var(--text-3); margin: 0 0 10px; line-height: 1.5; }

.import-input-row { margin-bottom: 10px; }
.import-input {
  width: 100%; height: 44px; padding: 0 14px;
  border: 1.5px solid var(--border); border-radius: var(--r-md);
  font-size: 14px; color: var(--text-1); background: var(--bg-subtle);
  font-family: inherit; outline: none;
}
.import-input:focus { border-color: var(--brand-400); box-shadow: 0 0 0 3.5px var(--brand-100); }

.import-parse { width: 100%; margin-bottom: 6px; }

.import-preview { margin-top: 16px; border-top: 1px dashed var(--border); padding-top: 16px; }

.preview-field { margin-bottom: 10px; }
.preview-label { display: block; font-size: 12px; color: var(--text-3); margin-bottom: 5px; }
.preview-input {
  width: 100%; height: 40px; padding: 0 12px;
  border: 1.5px solid var(--border); border-radius: var(--r-sm);
  font-size: 14px; color: var(--text-1); background: var(--surface);
  font-family: inherit; outline: none;
}
.preview-input:focus { border-color: var(--brand-400); }
.preview-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }

.preview-section { margin-bottom: 12px; }
.preview-section-title { font-size: 13px; font-weight: 700; color: var(--text-2); margin-bottom: 8px; }
.preview-chips { display: flex; flex-wrap: wrap; gap: 6px; }
.preview-chip {
  font-size: 12.5px; color: var(--text-2); background: var(--bg-subtle);
  padding: 5px 10px; border-radius: var(--r-full);
}
.preview-steps { margin: 0; padding-left: 20px; }
.preview-steps li { font-size: 13px; color: var(--text-2); line-height: 1.6; margin-bottom: 6px; }

.import-save { width: 100%; margin-top: 8px; }

.fade-enter-active, .fade-leave-active { transition: opacity var(--dur); }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
