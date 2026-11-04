<script setup>
/**
 * 菜谱详情
 *
 * Hero 封面 + 元信息 + 食材（分组、可换算份数）+ 做法步骤 + 小贴士。
 * 底部固定操作条：收藏 / 编辑 / 加入点餐单。
 */
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useRecipeStore } from '@/stores/recipe'
import { useCartStore } from '@/stores/cart'
import { useAppStore } from '@/stores/app'
import DishCover from '@/components/DishCover.vue'
import RatingStars from '@/components/RatingStars.vue'
import Stepper from '@/components/Stepper.vue'
import YanLoader from '@/components/YanLoader.vue'
import { toast } from '@/composables/useToast'
import {
  difficultyLabel, difficultyClass, durationText, splitTags, ratingText, quantityText, num
} from '@/utils/format'

const route = useRoute()
const router = useRouter()
const store = useRecipeStore()
const cart = useCartStore()
const app = useAppStore()

const id = Number(route.params.id)
const detail = ref(null)
const loading = ref(true)
const heroImgError = ref(false)
const servings = ref(2)

async function load() {
  loading.value = true
  try {
    detail.value = await store.fetchDetail(id)
    servings.value = detail.value.servings || 2
  } catch (e) {
    toast.error(e.message || '菜谱加载失败')
    router.replace('/recipes')
  } finally {
    loading.value = false
  }
}
onMounted(load)

const tags = computed(() => {
  const d = detail.value
  if (!d) return []
  if (Array.isArray(d.tags)) return d.tags
  return splitTags(d.tag_names).map((name) => ({ name }))
})

// 按当前份数换算食材用量
const scaledGroups = computed(() => {
  const d = detail.value
  if (!d?.ingredient_groups) return []
  const base = d.servings || 1
  const ratio = servings.value / base
  return d.ingredient_groups.map((g) => ({
    group_name: g.group_name,
    items: g.items.map((it) => ({ ...it, scaled: quantityText(Number(it.quantity) * ratio) }))
  }))
})

const totalIngredients = computed(() =>
  (detail.value?.ingredient_groups || []).reduce((s, g) => s + g.items.length, 0)
)

const favorited = computed(() => store.isFavorite(id))
const inCart = computed(() => cart.has(id))

async function toggleFav() {
  if (!app.currentMember) { toast.warn('请先在「家庭成员」选择你是谁'); router.push('/members'); return }
  try {
    const res = await store.toggleFavorite(id, app.currentMember.id)
    toast.success(res.favorited ? '已收藏' : '已取消收藏')
  } catch (e) { toast.error(e.message || '操作失败') }
}

function addCart() {
  cart.add(detail.value, 1, app.currentMember?.id)
  toast.success(`已加入「${detail.value.name}」`)
  router.push('/order')
}

async function remove() {
  if (!confirm(`确定要下架「${detail.value.name}」吗？历史订单不受影响。`)) return
  try {
    await store.removeRecipe(id, false)
    toast.success('已下架')
    router.replace('/recipes')
  } catch (e) { toast.error(e.message || '删除失败') }
}
</script>

<template>
  <div class="page page--flat detail">
    <!-- Hero -->
    <div v-if="detail" class="detail-hero" :style="{ background: detail.cover_url && !heroImgError ? '#2A2118' : detail.cover_gradient }">
      <img v-if="detail.cover_url && !heroImgError" :src="detail.cover_url" alt="" class="hero-img" @error="heroImgError = true" />
      <div v-else class="hero-emoji">{{ detail.cover_emoji }}</div>
      <div class="hero-shade" />
    </div>

    <div v-if="loading" class="page-body">
      <YanLoader :size="180" tip="菜谱加载中" />
    </div>

    <template v-else-if="detail">
      <div class="page-body">
        <!-- 标题卡（叠在 hero 上） -->
        <div class="title-card card card--pad">
          <div class="title-row">
            <h1 class="title-name">{{ detail.name }}</h1>
            <button class="fav-btn" :class="{ on: favorited }" @click="toggleFav" aria-label="收藏">
              {{ favorited ? '♥' : '♡' }}
            </button>
          </div>
          <p v-if="detail.description" class="title-desc">{{ detail.description }}</p>

          <div class="title-tags" v-if="tags.length">
            <span v-for="(t, i) in tags" :key="i" class="chip chip--brand"># {{ t.name }}</span>
          </div>

          <div class="meta-row">
            <div class="meta">
              <RatingStars :value="Number(detail.avg_rating) || 0" :size="14" />
              <span class="meta-rating num">{{ ratingText(detail.avg_rating) }}</span>
            </div>
            <span class="chip" :class="difficultyClass(detail.difficulty)">{{ difficultyLabel(detail.difficulty) }}</span>
            <span class="chip">{{ detail.category }}</span>
            <span class="chip">{{ detail.cuisine }}</span>
          </div>

          <div class="stat-row">
            <div class="stat">
              <span class="stat-value num">{{ durationText(detail.total_minutes) }}</span>
              <span class="stat-label">总耗时</span>
            </div>
            <div class="stat">
              <span class="stat-value num">{{ detail.servings }} 人份</span>
              <span class="stat-label">默认份量</span>
            </div>
            <div class="stat">
              <span class="stat-value num">{{ detail.calories || '—' }}</span>
              <span class="stat-label">千卡/份</span>
            </div>
            <div class="stat">
              <span class="stat-value num">{{ num(detail.cook_count) }}</span>
              <span class="stat-label">点过</span>
            </div>
          </div>
        </div>

        <!-- 食材 -->
        <section class="section">
          <div class="section-head">
            <h2 class="section-title">食材用量</h2>
            <div class="serving-ctrl">
              <span class="serving-label">份数</span>
              <Stepper v-model="servings" :min="1" :max="20" size="sm" />
            </div>
          </div>
          <div class="card card--pad ing-card">
            <template v-if="scaledGroups.length">
              <div v-for="g in scaledGroups" :key="g.group_name" class="ing-group">
                <h4 class="ing-group-name">{{ g.group_name }}</h4>
                <ul class="ing-list">
                  <li v-for="it in g.items" :key="it.id" class="ing-item">
                    <span class="ing-emoji">{{ it.emoji }}</span>
                    <span class="ing-name ellipsis">
                      {{ it.name }}
                      <span v-if="it.is_optional" class="ing-opt">可选</span>
                    </span>
                    <span class="ing-qty num">{{ it.scaled }}{{ it.unit }}</span>
                  </li>
                </ul>
              </div>
            </template>
            <p v-else class="ing-empty">这道菜还没有登记食材，点右上角编辑补上吧</p>
          </div>
        </section>

        <!-- 步骤 -->
        <section v-if="detail.steps?.length" class="section">
          <div class="section-head">
            <h2 class="section-title">做法步骤</h2>
            <span class="text-sm text-muted">共 {{ detail.steps.length }} 步</span>
          </div>
          <ol class="steps">
            <li v-for="s in detail.steps" :key="s.step_no" class="step">
              <span class="step-no num">{{ s.step_no }}</span>
              <div class="step-body">
                <p class="step-text">{{ s.content }}</p>
                <span v-if="s.duration_sec" class="step-dur num">⏱ {{ Math.round(s.duration_sec / 60) || 1 }} 分钟</span>
              </div>
            </li>
          </ol>
        </section>

        <!-- 小贴士 -->
        <section v-if="detail.tips" class="section">
          <div class="tips-card">
            <span class="tips-icon">💡</span>
            <div>
              <p class="tips-title">小贴士</p>
              <p class="tips-text">{{ detail.tips }}</p>
            </div>
          </div>
        </section>

        <p v-if="detail.creator_name" class="creator">由 {{ detail.creator_name }} 记录 · 共 {{ totalIngredients }} 种食材</p>

        <div class="bottom-space" />
      </div>

      <!-- 底部操作条 -->
      <div class="action-bar">
        <div class="page-body action-inner">
          <button class="btn btn--ghost btn--icon act-mini" @click="router.push(`/recipes/${id}/edit`)" aria-label="编辑">✎</button>
          <button class="btn btn--danger btn--icon act-mini" @click="remove" aria-label="下架">🗑</button>
          <button class="btn btn--primary grow act-main" :class="{ 'act-added': inCart }" @click="inCart ? router.push('/order') : addCart()">
            {{ inCart ? '✓ 已在点餐单，去看看' : '＋ 加入点餐单' }}
          </button>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.detail-hero { position: relative; height: 220px; display: flex; align-items: center; justify-content: center; overflow: hidden; }
.hero-img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; animation: hero-in var(--dur-slow) var(--ease-out); }
.hero-emoji { font-size: 104px; filter: drop-shadow(0 6px 18px rgba(0,0,0,.25)); z-index: 1; animation: hero-in var(--dur-slow) var(--ease-out); }
@keyframes hero-in { from { transform: scale(.8) translateY(10px); opacity: 0; } to { transform: none; opacity: 1; } }
.hero-shade { position: absolute; inset: 0; background: radial-gradient(120% 90% at 20% 0%, rgba(255,255,255,.3), transparent 55%); }

.title-card { margin-top: -54px; position: relative; z-index: 2; box-shadow: var(--sh-md); }
.title-row { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
.title-name { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.fav-btn { width: 40px; height: 40px; flex: none; border-radius: 50%; border: none; background: var(--bg-subtle); color: var(--text-4); font-size: 22px; cursor: pointer; transition: all var(--dur-fast) var(--ease-bounce); }
.fav-btn:active { transform: scale(.85); }
.fav-btn.on { color: #D4537E; background: #FBEAF0; }

.title-desc { margin-top: 8px; font-size: 14px; color: var(--text-2); line-height: 1.6; }
.title-tags { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 12px; }
.meta-row { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin-top: 14px; }
.meta { display: flex; align-items: center; gap: 4px; }
.meta-rating { font-size: 13px; font-weight: 700; color: var(--star); }

.stat-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; margin-top: 16px; padding-top: 16px; border-top: 1px solid var(--border); }
.stat { display: flex; flex-direction: column; align-items: center; gap: 2px; }
.stat-value { font-size: 16px; font-weight: 700; color: var(--text-1); }
.stat-label { font-size: 11px; color: var(--text-3); }

.serving-ctrl { display: flex; align-items: center; gap: 8px; }
.serving-label { font-size: 13px; color: var(--text-3); }

.ing-group + .ing-group { margin-top: 16px; padding-top: 14px; border-top: 1px dashed var(--border-strong); }
.ing-group-name { font-size: 13px; font-weight: 700; color: var(--brand-600); margin-bottom: 8px; }
.ing-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 9px; }
.ing-item { display: flex; align-items: center; gap: 10px; }
.ing-emoji { font-size: 18px; width: 24px; text-align: center; flex: none; }
.ing-name { flex: 1; font-size: 14.5px; color: var(--text-1); min-width: 0; }
.ing-opt { font-size: 10px; color: var(--text-4); border: 1px solid var(--border-strong); border-radius: 4px; padding: 0 4px; margin-left: 6px; }
.ing-qty { font-size: 14px; font-weight: 600; color: var(--text-2); flex: none; }
.ing-empty { font-size: 13px; color: var(--text-3); text-align: center; padding: 12px 0; }

.steps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 12px; }
.step { display: flex; gap: 12px; background: var(--surface); border-radius: var(--r-md); padding: 14px; box-shadow: var(--sh-xs); border: 1px solid var(--border); }
.step-no { flex: none; width: 26px; height: 26px; border-radius: 50%; background: var(--brand-gradient); color: #fff; font-size: 13px; font-weight: 700; display: flex; align-items: center; justify-content: center; }
.step-body { flex: 1; min-width: 0; }
.step-text { font-size: 14.5px; line-height: 1.65; color: var(--text-1); }
.step-dur { display: inline-block; margin-top: 6px; font-size: 11.5px; color: var(--brand-600); background: var(--brand-50); padding: 2px 8px; border-radius: var(--r-full); }

.tips-card { display: flex; gap: 12px; background: linear-gradient(135deg, #FFF8E7, #FFF3D6); border: 1px solid #F3E4B8; border-radius: var(--r-lg); padding: 16px; }
.tips-icon { font-size: 22px; flex: none; }
.tips-title { font-size: 13px; font-weight: 700; color: #B8860B; margin-bottom: 4px; }
.tips-text { font-size: 13.5px; color: #7a5c12; line-height: 1.6; }

.creator { text-align: center; font-size: 12px; color: var(--text-4); margin-top: 20px; }

.action-bar { position: fixed; left: 0; right: 0; bottom: 0; z-index: 50; padding: 10px 0 calc(10px + var(--safe-bottom)); background: rgba(255,253,247,.94); backdrop-filter: blur(16px); border-top: 1px solid var(--border); }
.action-inner { display: flex; align-items: center; gap: 8px; max-width: 1180px; }
.act-mini { width: 44px; height: 44px; min-height: 44px; font-size: 18px; flex: none; }
.act-main { height: 46px; }
.act-added { background: var(--success) !important; box-shadow: 0 8px 22px rgba(46,158,107,.3); }
.bottom-space { height: 90px; }
</style>
