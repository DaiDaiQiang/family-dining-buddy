<script setup>
/**
 * 菜谱编辑 / 新建
 *
 * 新建与编辑共用本组件：有 :id 为编辑，无则新建。
 * 也用于点餐页「顺手建档」弹出的精简表单（通过 query.from=cart 标识）。
 * 表单分三块：基础信息、食材用量（动态增删）、做法步骤（动态增删）。
 */
import { computed, onMounted, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useRecipeStore } from '@/stores/recipe'
import { useCartStore } from '@/stores/cart'
import { useAppStore } from '@/stores/app'
import { api } from '@/api'
import DishCover from '@/components/DishCover.vue'
import YanLoader from '@/components/YanLoader.vue'
import { compressToJpeg } from '@/utils/image'
import { toast } from '@/composables/useToast'

const route = useRoute()
const router = useRouter()
const store = useRecipeStore()
const cart = useCartStore()
const app = useAppStore()

const id = route.params.id ? Number(route.params.id) : null
const isEdit = !!id
const fromCart = route.query.from === 'cart'
const loading = ref(isEdit)
const saving = ref(false)

const meta = ref(null)
const ingredientsLib = ref([])

const form = reactive({
  name: '',
  category: '家常菜',
  cuisine: '中餐',
  difficulty: 'easy',
  prep_minutes: 10,
  cook_minutes: 15,
  servings: 2,
  cover_emoji: '🍲',
  cover_gradient: 'linear-gradient(135deg,#F2C89B,#DE9A66)',
  cover_url: '',
  description: '',
  tips: '',
  calories: null,
  tags: [],
  ingredients: [],
  steps: []
})

const tagInput = ref('')
const ingPickerOpen = ref(false)

// ---------- 封面照片（拍照 / 相册） ----------
const cameraInput = ref(null)
const albumInput = ref(null)
const coverUploading = ref(false)
const coverImgError = ref(false)

// 有地址且未加载失败时展示真实照片，否则回退 emoji + 渐变
const showCoverImg = computed(() => !!form.cover_url && !coverImgError.value)

function pickCamera() { cameraInput.value?.click() }
function pickAlbum() { albumInput.value?.click() }

async function onCoverFile(ev) {
  const input = ev.target
  const file = input.files && input.files[0]
  input.value = '' // 允许连续选同一张图
  if (!file) return
  if (!/^image\//.test(file.type)) { toast.warn('请选择图片文件'); return }
  if (file.size > 12 * 1024 * 1024) { toast.warn('图片过大（>12MB），请换一张'); return }

  coverUploading.value = true
  try {
    const dataUrl = await compressToJpeg(file)
    const res = await api.recipes.uploadCover(dataUrl)
    form.cover_url = res?.url || dataUrl
    toast.success('封面照片已上传')
  } catch (e) {
    toast.error(e.message || '照片上传失败')
  } finally {
    coverUploading.value = false
  }
}

function removeCover() { form.cover_url = '' }

onMounted(async () => {
  try {
    meta.value = await api.meta()
  } catch { meta.value = null }

  // 预加载食材库，方便下拉选择（离线模式下为空，用手动输入）
  api.ingredients.list({ page_size: 500 }).then((r) => {
    ingredientsLib.value = r?.list || r || []
  }).catch(() => {})

  if (isEdit) {
    try {
      const d = await store.fetchDetail(id)
      Object.assign(form, {
        name: d.name || '',
        category: d.category || '家常菜',
        cuisine: d.cuisine || '中餐',
        difficulty: d.difficulty || 'easy',
        prep_minutes: d.prep_minutes ?? 10,
        cook_minutes: d.cook_minutes ?? 15,
        servings: d.servings ?? 2,
        cover_emoji: d.cover_emoji || '🍲',
        cover_gradient: d.cover_gradient || 'linear-gradient(135deg,#F2C89B,#DE9A66)',
        cover_url: d.cover_url || '',
        description: d.description || '',
        tips: d.tips || '',
        calories: d.calories ?? null,
        tags: (d.tags || []).map((t) => t.name),
        ingredients: (d.ingredient_groups || []).flatMap((g) =>
          g.items.map((it) => ({
            name: it.name, quantity: Number(it.quantity), unit: it.unit,
            group_name: it.group_name, is_optional: it.is_optional
          }))
        ),
        steps: (d.steps || []).map((s) => ({ content: s.content, duration_sec: s.duration_sec }))
      })
    } catch (e) {
      toast.error(e.message || '加载失败')
      router.replace('/recipes')
    } finally {
      loading.value = false
    }
  } else {
    loading.value = false
  }
})

const totalMinutes = computed(() => (Number(form.prep_minutes) || 0) + (Number(form.cook_minutes) || 0))
const presets = computed(() => meta.value?.cover_presets || [])
const categories = computed(() => meta.value?.categories || ['家常菜', '汤羹', '主食', '凉菜', '甜点', '饮品'])
const cuisines = computed(() => meta.value?.cuisines || ['中餐', '川菜', '粤菜', '西餐', '日韩', '东南亚'])

const canSave = computed(() => form.name.trim().length > 0 && !saving.value)

// ---------- 标签 ----------
function addTag() {
  const t = tagInput.value.trim().replace(/^#/, '')
  if (!t) return
  if (!form.tags.includes(t)) form.tags.push(t)
  tagInput.value = ''
}
function removeTag(t) { form.tags = form.tags.filter((x) => x !== t) }

// ---------- 食材 ----------
function addIngredient(fromLib) {
  const base = fromLib
    ? { name: fromLib.name, unit: fromLib.unit || '克', quantity: 1, group_name: '主料', is_optional: false, ingredient_id: fromLib.id }
    : { name: '', unit: '克', quantity: 1, group_name: form.ingredients.length ? '辅料' : '主料', is_optional: false }
  form.ingredients.push({ ...base })
  ingPickerOpen.value = false
}
function removeIngredient(i) { form.ingredients.splice(i, 1) }

// ---------- 步骤 ----------
function addStep() { form.steps.push({ content: '', duration_sec: null }) }
function removeStep(i) { form.steps.splice(i, 1) }

function pickCover(p) { form.cover_emoji = p.emoji; form.cover_gradient = p.gradient; form.cover_url = '' }

// ---------- 提交 ----------
function buildPayload() {
  const ings = form.ingredients
    .filter((i) => i.name && i.name.trim())
    .map((i) => ({
      name: i.name.trim(),
      quantity: Number(i.quantity) || 0,
      unit: i.unit || '克',
      group_name: i.group_name || '辅料',
      is_optional: !!i.is_optional,
      ingredient_id: i.ingredient_id || undefined
    }))
  const steps = form.steps.filter((s) => s.content && s.content.trim())
    .map((s) => ({ content: s.content.trim(), duration_sec: s.duration_sec ? Number(s.duration_sec) : null }))

  return {
    name: form.name.trim(),
    category: form.category,
    cuisine: form.cuisine,
    difficulty: form.difficulty,
    prep_minutes: Number(form.prep_minutes) || 0,
    cook_minutes: Number(form.cook_minutes) || 0,
    servings: Number(form.servings) || 2,
    cover_emoji: form.cover_emoji,
    cover_gradient: form.cover_gradient,
    cover_url: form.cover_url || undefined,
    description: form.description.trim() || undefined,
    tips: form.tips.trim() || undefined,
    calories: form.calories ? Number(form.calories) : undefined,
    created_by: app.currentMember?.id,
    tags: form.tags,
    ingredients: ings,
    steps
  }
}

async function submit() {
  if (!canSave.value) { toast.warn('请先填写菜名'); return }
  saving.value = true
  try {
    const payload = buildPayload()
    let saved
    if (isEdit) {
      saved = await store.updateRecipe(id, payload)
      toast.success('菜谱已更新')
    } else {
      saved = await store.createRecipe(payload)
      toast.success('菜谱已建档')
      // 从点餐页顺手建档：建好后直接加入点餐单
      if (fromCart && saved) {
        cart.add(saved, 1, app.currentMember?.id)
        toast.success(`已把「${saved.name}」加入点餐单`)
      }
    }
    if (fromCart) router.replace('/order')
    else if (isEdit) router.replace(`/recipes/${id}`)
    else router.replace(`/recipes/${saved?.id || ''}`)
  } catch (e) {
    toast.error(e.message || '保存失败')
  } finally {
    saving.value = false
  }
}

function cancel() {
  if (fromCart) router.replace('/order')
  else router.back()
}
</script>

<template>
  <div class="page page--flat edit">
    <div v-if="loading" class="page-body">
      <YanLoader :size="180" tip="菜谱加载中" />
    </div>

    <template v-else>
      <!-- 封面预览 -->
      <div class="cover-preview" :style="{ background: showCoverImg ? '#2A2118' : form.cover_gradient }">
        <img v-if="showCoverImg" :src="form.cover_url" alt="封面照片" class="cover-preview-img" @error="coverImgError = true" />
        <span v-else class="cover-preview-emoji">{{ form.cover_emoji }}</span>

        <!-- 拍照 / 相册上传 -->
        <div class="cover-actions">
          <button class="cover-btn" :disabled="coverUploading" @click="pickCamera">📷 拍照</button>
          <button class="cover-btn" :disabled="coverUploading" @click="pickAlbum">🖼️ 相册</button>
          <button v-if="form.cover_url" class="cover-btn cover-btn--del" :disabled="coverUploading" @click="removeCover">✕ 移除</button>
        </div>
        <div v-if="coverUploading" class="cover-uploading">
          <YanLoader :size="64" :blobs="false" :backdrop="false" tip="上传中" />
        </div>

        <div class="cover-presets">
          <button
            v-for="(p, i) in presets"
            :key="i"
            class="preset"
            :class="{ on: !form.cover_url && p.gradient === form.cover_gradient && p.emoji === form.cover_emoji }"
            :style="{ background: p.gradient }"
            @click="pickCover(p)"
          >{{ p.emoji }}</button>
        </div>
        <input ref="cameraInput" type="file" accept="image/*" capture="environment" class="file-hidden" @change="onCoverFile" />
        <input ref="albumInput" type="file" accept="image/*" class="file-hidden" @change="onCoverFile" />
      </div>

      <div class="page-body">
        <!-- 基础信息 -->
        <section class="card card--pad form-card">
          <div class="field">
            <label class="field-label">菜名 <span class="req">*</span></label>
            <input v-model="form.name" class="input" placeholder="例如：红烧肉" maxlength="64" />
          </div>

          <div class="field-row">
            <div class="field grow">
              <label class="field-label">分类</label>
              <select v-model="form.category" class="select">
                <option v-for="c in categories" :key="c" :value="c">{{ c }}</option>
              </select>
            </div>
            <div class="field grow">
              <label class="field-label">菜系</label>
              <select v-model="form.cuisine" class="select">
                <option v-for="c in cuisines" :key="c" :value="c">{{ c }}</option>
              </select>
            </div>
          </div>

          <div class="field">
            <label class="field-label">难度</label>
            <div class="seg">
              <button
                v-for="d in [{ v: 'easy', l: '简单 🟢' }, { v: 'medium', l: '中等 🟡' }, { v: 'hard', l: '挑战 🔴' }]"
                :key="d.v"
                class="seg-btn"
                :class="{ on: form.difficulty === d.v }"
                @click="form.difficulty = d.v"
              >{{ d.l }}</button>
            </div>
          </div>

          <div class="field-row">
            <div class="field grow">
              <label class="field-label">备菜(分)</label>
              <input v-model.number="form.prep_minutes" class="input num" type="number" min="0" max="600" />
            </div>
            <div class="field grow">
              <label class="field-label">烹饪(分)</label>
              <input v-model.number="form.cook_minutes" class="input num" type="number" min="0" max="600" />
            </div>
            <div class="field grow">
              <label class="field-label">份数</label>
              <input v-model.number="form.servings" class="input num" type="number" min="1" max="20" />
            </div>
          </div>
          <p class="field-hint">总耗时约 {{ totalMinutes }} 分钟 · 份数是「食材用量」的换算基准</p>

          <div class="field-row">
            <div class="field grow">
              <label class="field-label">热量(千卡/份)</label>
              <input v-model.number="form.calories" class="input num" type="number" min="0" placeholder="选填" />
            </div>
          </div>

          <div class="field">
            <label class="field-label">一句话简介</label>
            <input v-model="form.description" class="input" placeholder="这道菜的特点，选填" maxlength="200" />
          </div>
        </section>

        <!-- 标签 -->
        <section class="section">
          <h2 class="form-title">标签</h2>
          <div class="tag-editor">
            <span v-for="t in form.tags" :key="t" class="tag-pill">
              # {{ t }}
              <button class="tag-x" @click="removeTag(t)" aria-label="移除">✕</button>
            </span>
            <input
              v-model="tagInput"
              class="tag-input"
              placeholder="加标签后回车，如 下饭"
              @keyup.enter.prevent="addTag"
            />
          </div>
          <div class="tag-quick">
            <button v-for="t in ['下饭', '快手菜', '低卡', '宝宝爱吃', '无辣', '高蛋白']" :key="t"
              class="tag-suggest" :class="{ on: form.tags.includes(t) }" @click="form.tags.includes(t) ? removeTag(t) : form.tags.push(t)">
              {{ t }}
            </button>
          </div>
        </section>

        <!-- 食材 -->
        <section class="section">
          <div class="form-title-row">
            <h2 class="form-title">食材用量</h2>
            <div class="title-btns">
              <button v-if="ingredientsLib.length" class="mini-btn" @click="ingPickerOpen = !ingPickerOpen">从库中选</button>
              <button class="mini-btn mini-btn--primary" @click="addIngredient(null)">＋ 手动添加</button>
            </div>
          </div>

          <!-- 食材库快选 -->
          <div v-if="ingPickerOpen" class="ing-lib">
            <button v-for="ig in ingredientsLib.slice(0, 40)" :key="ig.id" class="ing-lib-item" @click="addIngredient(ig)">
              {{ ig.emoji }} {{ ig.name }}
            </button>
          </div>

          <div class="ing-rows">
            <div v-for="(ing, i) in form.ingredients" :key="i" class="ing-row">
              <input v-model="ing.name" class="input ing-name" placeholder="食材名" />
              <input v-model.number="ing.quantity" class="input ing-qty num" type="number" min="0" step="0.5" placeholder="量" />
              <input v-model="ing.unit" class="input ing-unit" placeholder="单位" />
              <select v-model="ing.group_name" class="select ing-group">
                <option>主料</option><option>辅料</option><option>调味料</option>
              </select>
              <button class="ing-del" @click="removeIngredient(i)" aria-label="删除">✕</button>
            </div>
            <p v-if="!form.ingredients.length" class="form-empty">还没添加食材</p>
          </div>
        </section>

        <!-- 步骤 -->
        <section class="section">
          <div class="form-title-row">
            <h2 class="form-title">做法步骤</h2>
            <button class="mini-btn mini-btn--primary" @click="addStep">＋ 加一步</button>
          </div>
          <div class="step-rows">
            <div v-for="(s, i) in form.steps" :key="i" class="step-row">
              <span class="step-row-no num">{{ i + 1 }}</span>
              <textarea v-model="s.content" class="textarea step-textarea" rows="2" placeholder="描述这一步…" />
              <button class="ing-del" @click="removeStep(i)" aria-label="删除">✕</button>
            </div>
            <p v-if="!form.steps.length" class="form-empty">还没添加步骤，点「加一步」开始</p>
          </div>
        </section>

        <!-- 小贴士 -->
        <section class="section">
          <h2 class="form-title">小贴士</h2>
          <textarea v-model="form.tips" class="textarea" rows="3" placeholder="成功要点、注意事项…选填" maxlength="500" />
        </section>

        <div class="bottom-space" />
      </div>

      <!-- 底部保存条 -->
      <div class="save-bar">
        <div class="page-body save-inner">
          <button class="btn btn--ghost save-cancel" @click="cancel">取消</button>
          <button class="btn btn--primary grow" :disabled="!canSave" @click="submit">
            {{ saving ? '保存中…' : (isEdit ? '保存修改' : (fromCart ? '建档并加入点餐' : '建档')) }}
          </button>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.cover-preview {
  height: 186px;
  display: flex; align-items: center; justify-content: center;
  position: relative; overflow: hidden;
}
.cover-preview-emoji { font-size: 64px; filter: drop-shadow(0 4px 12px rgba(0,0,0,.2)); }
.cover-preview-img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
/* 预设条上移到 bottom:56px，避开表单卡片 -24px 的叠压区，修复遮挡 */
.cover-presets { position: absolute; bottom: 56px; left: 0; right: 0; display: flex; gap: 8px; overflow-x: auto; padding: 0 var(--page-pad); scrollbar-width: none; z-index: 2; }
.cover-presets::-webkit-scrollbar { display: none; }
.preset { width: 38px; height: 38px; flex: none; border-radius: 12px; border: 2px solid rgba(255,255,255,.5); font-size: 18px; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: transform var(--dur-fast) var(--ease-bounce); }
.preset:active { transform: scale(.9); }
.preset.on { border-color: #fff; box-shadow: 0 0 0 2px var(--brand-500); }

.cover-actions { position: absolute; top: 10px; right: 10px; display: flex; gap: 6px; z-index: 3; }
.cover-btn { padding: 6px 12px; border-radius: var(--r-full); border: none; background: rgba(0,0,0,.42); color: #fff; font-size: 12.5px; font-weight: 600; cursor: pointer; backdrop-filter: blur(6px); font-family: inherit; }
.cover-btn:disabled { opacity: .6; cursor: wait; }
.cover-btn--del { background: rgba(198,40,40,.72); }
.file-hidden { display: none; }
.cover-uploading { position: absolute; inset: 0; z-index: 4; display: flex; align-items: center; justify-content: center; background: rgba(0,0,0,.35); }
.cover-uploading :deep(.yan) { padding: 0; }

.form-card { margin-top: -24px; position: relative; z-index: 2; }
.form-title { font-size: 16px; font-weight: 700; margin-bottom: 12px; display: flex; align-items: center; gap: 6px; }
.form-title::before { content: ''; width: 4px; height: 15px; border-radius: 2px; background: var(--brand-gradient); }
.form-title-row { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
.form-title-row .form-title { margin-bottom: 0; }
.title-btns { display: flex; gap: 8px; }

.field-row { display: flex; gap: 10px; }
.field-row .field { margin-bottom: var(--gap-lg); }

.seg { display: flex; gap: 6px; background: var(--bg-subtle); padding: 4px; border-radius: var(--r-sm); }
.seg-btn { flex: 1; padding: 8px; border: none; background: transparent; border-radius: var(--r-xs); font-size: 13px; font-weight: 600; color: var(--text-2); cursor: pointer; transition: all var(--dur-fast); font-family: inherit; }
.seg-btn.on { background: var(--surface); color: var(--brand-600); box-shadow: var(--sh-xs); }

.mini-btn { padding: 6px 12px; border-radius: var(--r-full); border: 1.5px solid var(--border-strong); background: var(--surface); color: var(--text-2); font-size: 12.5px; font-weight: 600; cursor: pointer; }
.mini-btn--primary { background: var(--brand-50); border-color: transparent; color: var(--brand-600); }

.tag-editor { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; padding: 8px; background: var(--surface-2); border: 1.5px solid var(--border); border-radius: var(--r-sm); }
.tag-pill { display: inline-flex; align-items: center; gap: 4px; padding: 4px 10px; background: var(--brand-50); color: var(--brand-600); border-radius: var(--r-full); font-size: 13px; font-weight: 600; }
.tag-x { border: none; background: transparent; color: var(--brand-400); cursor: pointer; font-size: 11px; padding: 0; }
.tag-input { flex: 1; min-width: 120px; border: none; outline: none; background: transparent; font-size: 14px; font-family: inherit; padding: 4px; }
.tag-quick { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 10px; }
.tag-suggest { padding: 5px 12px; border-radius: var(--r-full); border: 1.5px dashed var(--border-strong); background: transparent; color: var(--text-3); font-size: 12.5px; cursor: pointer; font-family: inherit; }
.tag-suggest.on { border-style: solid; border-color: var(--brand-300); background: var(--brand-50); color: var(--brand-600); }

.ing-lib { display: flex; flex-wrap: wrap; gap: 8px; padding: 12px; background: var(--surface); border-radius: var(--r-md); margin-bottom: 12px; box-shadow: var(--sh-xs); max-height: 160px; overflow-y: auto; }
.ing-lib-item { padding: 5px 11px; border-radius: var(--r-full); border: 1px solid var(--border); background: var(--surface-2); font-size: 13px; cursor: pointer; }
.ing-lib-item:active { background: var(--brand-50); }

.ing-rows { display: flex; flex-direction: column; gap: 8px; }
.ing-row { display: grid; grid-template-columns: 1fr 62px 54px 78px 34px; gap: 6px; align-items: center; }
.ing-row .input, .ing-row .select { min-height: 42px; padding: 8px 10px; font-size: 14px; }
.ing-del { width: 34px; height: 42px; border: none; background: var(--danger-bg); color: var(--danger); border-radius: var(--r-xs); cursor: pointer; font-size: 14px; }
.ing-group { font-size: 13px !important; }

.step-rows { display: flex; flex-direction: column; gap: 10px; }
.step-row { display: flex; gap: 10px; align-items: flex-start; }
.step-row-no { flex: none; width: 26px; height: 26px; margin-top: 8px; border-radius: 50%; background: var(--brand-gradient); color: #fff; font-size: 13px; font-weight: 700; display: flex; align-items: center; justify-content: center; }
.step-textarea { flex: 1; }

.form-empty { text-align: center; color: var(--text-4); font-size: 13px; padding: 16px; }

.save-bar { position: fixed; left: 0; right: 0; bottom: 0; z-index: 50; padding: 10px 0 calc(10px + var(--safe-bottom)); background: rgba(255,253,247,.95); backdrop-filter: blur(16px); border-top: 1px solid var(--border); }
.save-inner { display: flex; gap: 10px; max-width: 1180px; align-items: center; }
.save-cancel { flex: none; width: 84px; }
.bottom-space { height: 90px; }

@media (min-width: 700px) {
  .ing-row { grid-template-columns: 2fr 80px 70px 100px 40px; }
}
</style>
