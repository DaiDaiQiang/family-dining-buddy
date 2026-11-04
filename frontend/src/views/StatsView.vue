<script setup>
/**
 * 家的味道 · 统计
 *
 * 概览数字、分类分布、菜品排行、每位成员的口味画像。
 */
import { computed, onActivated, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { api } from '@/api'
import DishCover from '@/components/DishCover.vue'
import RatingStars from '@/components/RatingStars.vue'
import YanLoader from '@/components/YanLoader.vue'
import { toast } from '@/composables/useToast'
import { ratingText, num } from '@/utils/format'

defineOptions({ name: 'StatsView' })

const router = useRouter()
const loading = ref(true)
const overview = ref(null)
const categories = ref([])
const popular = ref([])
const members = ref([])
const activeTaste = ref(null)

async function load() {
  loading.value = true
  try {
    const [ov, cat, pop, mem] = await Promise.all([
      api.stats.overview().catch(() => null),
      api.stats.categories().catch(() => []),
      api.stats.popular({ limit: 8 }).catch(() => []),
      api.stats.members().catch(() => [])
    ])
    overview.value = ov
    categories.value = cat || []
    popular.value = pop || []
    members.value = mem || []
    if (mem?.length && !activeTaste.value) loadTaste(mem[0].member_id)
  } catch (e) {
    toast.error(e.message || '统计加载失败')
  } finally {
    loading.value = false
  }
}

async function loadTaste(mid) {
  try {
    activeTaste.value = await api.stats.memberTaste(mid)
  } catch (e) {
    activeTaste.value = null
  }
}

onMounted(load)
onActivated(() => { if (!loading.value) load() })

const maxCat = computed(() => Math.max(1, ...categories.value.map((c) => c.order_count)))
const maxPop = computed(() => Math.max(1, ...popular.value.map((p) => p.order_count)))
</script>

<template>
  <div class="page page--flat stats">
    <header class="st-head">
      <div class="page-body">
        <h1 class="st-title">📊 家的味道</h1>
        <p class="st-sub">这些数字，藏着一家人的口味偏好</p>
      </div>
    </header>

    <div class="page-body">
      <YanLoader v-if="loading && !overview" :size="180" tip="统计加载中" />

      <!-- 概览 -->
      <section v-if="overview" class="ov-grid">
        <div class="ov-card ov-card--hero">
          <span class="ov-emoji">{{ overview.favorite_dish?.cover_emoji || '🏆' }}</span>
          <div>
            <p class="ov-hero-label">全家最爱</p>
            <p class="ov-hero-name">{{ overview.favorite_dish?.name || '暂无' }}</p>
            <p v-if="overview.favorite_dish" class="ov-hero-sub num">点过 {{ overview.favorite_dish.order_count }} 次 · {{ ratingText(overview.favorite_dish.avg_rating) }} 分</p>
          </div>
        </div>
        <div class="ov-card">
          <span class="ov-v num">{{ num(overview.dishes_served) }}</span>
          <span class="ov-l">累计上菜</span>
        </div>
        <div class="ov-card">
          <span class="ov-v num">{{ ratingText(overview.avg_rating) }}</span>
          <span class="ov-l">平均评分</span>
        </div>
        <div class="ov-card">
          <span class="ov-v num">{{ overview.top_category?.category || '—' }}</span>
          <span class="ov-l">最常做的分类</span>
        </div>
      </section>

      <!-- 分类分布 -->
      <section v-if="categories.length" class="section">
        <h2 class="section-title">分类分布</h2>
        <div class="cat-bars">
          <div v-for="c in categories" :key="c.category" class="cat-bar">
            <span class="cat-name">{{ c.category }}</span>
            <div class="cat-track">
              <div class="cat-fill" :style="{ width: (c.order_count / maxCat * 100) + '%' }" />
            </div>
            <span class="cat-num num">{{ c.order_count }}<small>次</small></span>
          </div>
        </div>
      </section>

      <!-- 菜品排行 -->
      <section v-if="popular.length" class="section">
        <h2 class="section-title">人气菜品</h2>
        <div class="pop-list">
          <div v-for="(p, i) in popular" :key="p.id" class="pop-row clickable" @click="router.push(`/recipes/${p.id}`)">
            <span class="pop-rank" :class="{ top: i < 3 }">{{ i + 1 }}</span>
            <DishCover :emoji="p.cover_emoji" :gradient="p.cover_gradient" :url="p.cover_url" size="sm" />
            <div class="pop-info">
              <p class="pop-name ellipsis">{{ p.name }}</p>
              <div class="pop-bar-track">
                <div class="pop-bar-fill" :style="{ width: (p.order_count / maxPop * 100) + '%' }" />
              </div>
            </div>
            <div class="pop-right">
              <span class="pop-count num">{{ p.order_count }}</span>
              <RatingStars :value="Number(p.avg_rating) || 0" :size="10" />
            </div>
          </div>
        </div>
      </section>

      <!-- 成员口味 -->
      <section v-if="members.length" class="section">
        <h2 class="section-title">全家口味</h2>
        <div class="member-tabs">
          <button
            v-for="m in members"
            :key="m.member_id"
            class="mtab"
            :class="{ on: activeTaste?.member?.id === m.member_id }"
            :style="{ '--mc': m.theme_color }"
            @click="loadTaste(m.member_id)"
          >
            <span class="mtab-avatar">{{ m.avatar_emoji }}</span>
            <span class="mtab-name">{{ m.nickname }}</span>
          </button>
        </div>

        <div v-if="activeTaste" class="taste-card card card--pad">
          <div class="taste-top">
            <span class="taste-avatar" :style="{ background: activeTaste.member.theme_color }">{{ activeTaste.member.avatar_emoji }}</span>
            <div class="taste-head">
              <p class="taste-name">{{ activeTaste.member.nickname }}</p>
              <p v-if="activeTaste.member.taste_note" class="taste-note">{{ activeTaste.member.taste_note }}</p>
            </div>
          </div>

          <div class="taste-stats">
            <div class="ts"><span class="ts-v num">{{ activeTaste.summary.order_count }}</span><span class="ts-l">点过</span></div>
            <div class="ts"><span class="ts-v num">{{ activeTaste.summary.total_portions }}</span><span class="ts-l">总份数</span></div>
            <div class="ts"><span class="ts-v num">{{ ratingText(activeTaste.summary.avg_rating) }}</span><span class="ts-l">常打分</span></div>
          </div>

          <div v-if="activeTaste.by_category?.length" class="taste-cats">
            <span v-for="c in activeTaste.by_category.slice(0, 4)" :key="c.category" class="tcat">
              {{ c.category }} <b class="num">{{ c.order_count }}</b>
            </span>
          </div>

          <div v-if="activeTaste.top_dishes?.length" class="taste-dishes">
            <p class="td-label">常点</p>
            <div class="td-list">
              <span v-for="d in activeTaste.top_dishes" :key="d.id" class="td-chip clickable" @click="router.push(`/recipes/${d.id}`)">
                {{ d.cover_emoji }} {{ d.name }}
              </span>
            </div>
          </div>
        </div>
      </section>

      <div class="manage-link">
        <button class="btn btn--soft btn--block" @click="router.push('/members')">👨‍👩‍👧 管理家庭成员</button>
      </div>
      <div class="bottom-space" />
    </div>
  </div>
</template>

<style scoped>
.st-head { padding-top: calc(var(--safe-top) + 20px); }
.st-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.st-sub { font-size: 13px; color: var(--text-3); margin-top: 4px; }

.ov-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; margin-top: 16px; }
.ov-card { background: var(--surface); border-radius: var(--r-lg); padding: 16px; box-shadow: var(--sh-sm); border: 1px solid var(--border); display: flex; flex-direction: column; gap: 3px; }
.ov-card--hero { grid-column: span 2; flex-direction: row; align-items: center; gap: 14px; background: linear-gradient(135deg, #FBF0DC, #F7E8D2); }
.ov-emoji { font-size: 40px; }
.ov-hero-label { font-size: 12px; color: var(--brand-600); font-weight: 600; }
.ov-hero-name { font-size: 20px; font-weight: 800; margin: 2px 0; }
.ov-hero-sub { font-size: 12px; color: var(--text-3); }
.ov-v { font-size: 24px; font-weight: 800; color: var(--brand-600); }
.ov-l { font-size: 12px; color: var(--text-3); }

.cat-bars { display: flex; flex-direction: column; gap: 12px; background: var(--surface); border-radius: var(--r-lg); padding: 16px; box-shadow: var(--sh-sm); }
.cat-bar { display: flex; align-items: center; gap: 10px; }
.cat-name { width: 60px; flex: none; font-size: 13px; color: var(--text-2); }
.cat-track { flex: 1; height: 10px; background: var(--bg-subtle); border-radius: var(--r-full); overflow: hidden; }
.cat-fill { height: 100%; background: var(--brand-gradient); border-radius: var(--r-full); transition: width var(--dur-slow) var(--ease-out); }
.cat-num { width: 44px; flex: none; text-align: right; font-size: 13px; font-weight: 700; color: var(--text-1); }
.cat-num small { font-size: 10px; color: var(--text-3); font-weight: 400; }

.pop-list { display: flex; flex-direction: column; gap: 4px; background: var(--surface); border-radius: var(--r-lg); padding: 6px; box-shadow: var(--sh-sm); }
.pop-row { display: flex; align-items: center; gap: 11px; padding: 9px 8px; border-radius: var(--r-md); }
.pop-row:active { background: var(--bg-subtle); }
.pop-rank { width: 22px; text-align: center; font-size: 15px; font-weight: 800; color: var(--text-4); font-family: var(--font-num); flex: none; }
.pop-rank.top { color: var(--brand-500); }
.pop-info { flex: 1; min-width: 0; }
.pop-name { font-size: 15px; font-weight: 600; margin-bottom: 5px; }
.pop-bar-track { height: 6px; background: var(--bg-subtle); border-radius: var(--r-full); overflow: hidden; }
.pop-bar-fill { height: 100%; background: linear-gradient(90deg, #E2B686, #C2703D); border-radius: var(--r-full); }
.pop-right { display: flex; flex-direction: column; align-items: flex-end; gap: 3px; flex: none; }
.pop-count { font-size: 16px; font-weight: 800; color: var(--brand-600); }

.member-tabs { display: flex; gap: 8px; overflow-x: auto; padding-bottom: 8px; scrollbar-width: none; margin-bottom: 4px; }
.member-tabs::-webkit-scrollbar { display: none; }
.mtab { display: flex; flex-direction: column; align-items: center; gap: 3px; padding: 8px 14px; border-radius: var(--r-md); border: 1.5px solid var(--border); background: var(--surface); cursor: pointer; flex: none; transition: all var(--dur-fast); font-family: inherit; }
.mtab-avatar { font-size: 22px; }
.mtab-name { font-size: 12px; font-weight: 600; color: var(--text-2); }
.mtab.on { border-color: var(--mc); background: color-mix(in srgb, var(--mc) 12%, white); }
.mtab.on .mtab-name { color: var(--mc); font-weight: 700; }

.taste-top { display: flex; align-items: center; gap: 12px; }
.taste-avatar { width: 48px; height: 48px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 26px; flex: none; }
.taste-name { font-size: 17px; font-weight: 700; }
.taste-note { font-size: 12.5px; color: var(--text-3); margin-top: 2px; }
.taste-stats { display: flex; gap: 8px; margin-top: 16px; padding-top: 14px; border-top: 1px solid var(--border); }
.ts { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 2px; }
.ts-v { font-size: 19px; font-weight: 800; color: var(--brand-600); }
.ts-l { font-size: 11px; color: var(--text-3); }
.taste-cats { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 14px; }
.tcat { font-size: 12.5px; color: var(--text-2); background: var(--bg-subtle); padding: 4px 10px; border-radius: var(--r-full); }
.tcat b { color: var(--brand-600); }
.taste-dishes { margin-top: 16px; }
.td-label { font-size: 12px; color: var(--text-3); margin-bottom: 8px; }
.td-list { display: flex; flex-wrap: wrap; gap: 8px; }
.td-chip { font-size: 13px; padding: 5px 12px; background: var(--brand-50); color: var(--brand-600); border-radius: var(--r-full); font-weight: 500; }

.manage-link { margin-top: 24px; }
.bottom-space { height: 30px; }
</style>
