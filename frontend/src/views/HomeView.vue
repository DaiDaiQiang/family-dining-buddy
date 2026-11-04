<script setup>
/**
 * 首页 · 今天吃什么
 *
 * 一屏聚合：今日三餐概览、智能推荐、常点榜单、快速入口。
 */
import { computed, nextTick, onActivated, onDeactivated, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useCartStore } from '@/stores/cart'
import { useRecipeStore } from '@/stores/recipe'
import { useHealthStore } from '@/stores/health'
import { useParcelStore, courierMeta, stationMeta, formatCode } from '@/stores/parcel'
import { useReminderStore } from '@/stores/reminder'
import { api } from '@/api'
import DishCard from '@/components/DishCard.vue'
import DishCover from '@/components/DishCover.vue'
import RatingStars from '@/components/RatingStars.vue'
import EmptyState from '@/components/EmptyState.vue'
import YanLoader from '@/components/YanLoader.vue'
import { toast } from '@/composables/useToast'
import { relativeDay, mealLabel, mealEmoji, statusLabel, statusClass, durationText, ratingText } from '@/utils/format'

defineOptions({ name: 'HomeView' })

const router = useRouter()
const app = useAppStore()
const cart = useCartStore()
const recipeStore = useRecipeStore()
const health = useHealthStore()
const parcel = useParcelStore()

const loading = ref(true)
const overview = ref(null)
const suggestions = ref([])
const popular = ref([])
const todayOrders = ref([])

async function load() {
  loading.value = true
  try {
    const [ov, sg, pp, td] = await Promise.all([
      api.stats.overview().catch(() => null),
      api.stats.suggestion({ count: 6 }).catch(() => ({ recipes: [] })),
      api.stats.popular({ limit: 5 }).catch(() => []),
      api.orders.today().catch(() => ({ orders: [] }))
    ])
    overview.value = ov
    suggestions.value = sg?.recipes || []
    popular.value = pp || []
    todayOrders.value = td?.orders || []
  } catch (e) {
    toast.error(e.message || '首页加载失败')
  } finally {
    loading.value = false
  }
}

onActivated(() => {
  load().then(async () => {
    await nextTick()
    measureDots()
    carRef.value?.scrollTo({ left: 0, behavior: 'auto' })
    startAuto()
  })
})
onDeactivated(stopAuto)
onUnmounted(stopAuto)

const greeting = computed(() => {
  const h = new Date().getHours()
  if (h < 6) return '夜深了'
  if (h < 11) return '早上好'
  if (h < 14) return '中午好'
  if (h < 18) return '下午好'
  return '晚上好'
})

const member = computed(() => app.currentMember)

// ---------- 顶部滚动显示区 ----------
const quotePool = [
  '好好吃饭，是每天都在实践的生活哲学。',
  '人间烟火气，最抚凡人心。',
  '是谁来自山川湖海，却囿于昼夜、厨房与爱。',
  '吃喜欢吃的菜，爱值得爱的人。',
  '生活可以不富裕，但胃不能受委屈。',
  '把日子过成一顿饭：食材新鲜，火候刚好。',
  '今天也要按时吃饭呀。',
  '厨房有烟火，饭桌有美食，家里有牵挂。',
  '一屋两人三餐四季，就是最好的浪漫。',
  '吃饱喝足，才有力气想念和奔赴。',
  '味道是独家的记忆，也是全家的暗号。',
  '慢慢烹，细细品，好日子都是熬出来的。'
]
const dailyQuote = computed(() => {
  const now = new Date()
  const day = Math.floor(Date.UTC(now.getFullYear(), now.getMonth(), now.getDate()) / 86400000)
  return quotePool[day % quotePool.length]
})

const heroPick = computed(() => suggestions.value[0] || null)
const heroStar = computed(() => popular.value[0] || null)

/** 经期等健康提醒：滚动栏首屏展示，无提醒时不占位 */
const healthAlerts = computed(() => health.alerts)

/** 站内提醒摘要（阶段 2）：待办前 4 条 */
const reminder = useReminderStore()
const reminderBrief = computed(() => reminder.brief)
const reminderPending = computed(() => reminder.pendingCount)

/** ---------- 快递代取（全家待取件）：有包裹才占一屏 ---------- */
const parcelGroups = computed(() => parcel.pendingByMember)
const hasParcel = computed(() => parcelGroups.value.length > 0)

/** 成员 id → 成员对象，用于取昵称与头像 */
const memberMap = computed(() => {
  const map = new Map()
  for (const m of app.members) map.set(Number(m.id), m)
  return map
})
function memberName(id) {
  if (id == null) return '家人'
  return memberMap.value.get(Number(id))?.nickname || '家人'
}
function memberEmoji(id) {
  if (id == null) return '📦'
  return memberMap.value.get(Number(id))?.avatar_emoji || '📦'
}

/**
 * 首页滚动卡展示用：每位收件人压成一行，最多带 2 个取件码，其余折叠成「等 N 件」。
 * 全部收件人都要露出（.parcel-list 卡内滚动兜底），所以折叠的是每人的件数而非人数，
 * 避免把整条横向滚动栏撑高、影响其他卡片对齐。
 */
const parcelCards = computed(() =>
  parcelGroups.value.map((g) => {
    const peek = g.items.slice(0, 2)
    const codes = peek
      .map((r) => {
        const c = courierMeta(r.courier_code)
        const where = r.station || stationMeta(r.station_type).label
        return `${c.emoji} ${where} · ${formatCode(r.pickup_code)}`
      })
      .join('　')
    return {
      key: g.memberId ?? 'none',
      name: memberName(g.memberId),
      emoji: memberEmoji(g.memberId),
      codes,
      total: g.items.length,
      rest: g.items.length - peek.length,
      expired: g.items.filter((r) => r.status === 'expired').length
    }
  })
)

/** 卡片标题上的待取总数（含已超时） */
const parcelTotal = computed(() => parcel.pending.length)

const carRef = ref(null)
const heroPage = ref(0)
const heroDots = ref(1)
let carTimer = null

function measureDots() {
  if (carRef.value) heroDots.value = carRef.value.children.length
}
function onCarScroll() {
  const el = carRef.value
  if (!el || !el.clientWidth) return
  heroPage.value = Math.round(el.scrollLeft / el.clientWidth)
}
function stepSlide() {
  const el = carRef.value
  if (!el || heroDots.value <= 1) return
  const next = (Math.round(el.scrollLeft / el.clientWidth) + 1) % heroDots.value
  el.scrollTo({ left: next * el.clientWidth, behavior: 'smooth' })
}
function startAuto() {
  stopAuto()
  carTimer = setInterval(stepSlide, 4000)
}
function stopAuto() {
  if (carTimer) { clearInterval(carTimer); carTimer = null }
}

watch(heroDots, (n) => { if (heroPage.value >= n) heroPage.value = 0 })
// 提醒条数变化（如刚在提醒页设置了周期）时，重测滚动圆点
watch(() => healthAlerts.value.length, async () => {
  await nextTick()
  measureDots()
})
// 待取快递增减（如刚标记已取）时，同样重测滚动圆点
watch(hasParcel, async () => {
  await nextTick()
  measureDots()
})

function addToCart(recipe) {
  cart.add(recipe, 1, member.value?.id)
  toast.success(`已加入「${recipe.name}」`)
}

function refreshSuggestion() {
  api.stats.suggestion({ count: 6 }).then((r) => {
    suggestions.value = r?.recipes || []
    toast.info('换一批推荐')
  }).catch(() => toast.error('推荐刷新失败'))
}

function openRecipe(r) { router.push(`/recipes/${r.id}`) }
function goOrder() { router.push('/order') }
</script>

<template>
  <div class="page home">
    <!-- 顶部问候卡 -->
    <header class="hero">
      <div class="hero-bg" aria-hidden="true" />
      <div class="page-body hero-body">
        <div class="hero-top">
          <div class="hero-hello">
            <p class="hero-greeting">{{ greeting }}，{{ member?.nickname || '家人' }} {{ member?.avatar_emoji || '👋' }}</p>
            <h1 class="hero-title">今天想吃点什么？</h1>
          </div>
          <button class="hero-avatar" @click="router.push('/profile')" aria-label="个人中心">
            <img v-if="member?.avatar_url" :src="member.avatar_url" alt="" class="hero-avatar-img" />
            <span v-else>{{ member?.avatar_emoji || '👨‍👩‍👧' }}</span>
          </button>
        </div>

        <!-- 滚动显示区：语录 / 推荐菜 / 点餐统计 / 人气 Top1 -->
        <div
          class="hero-car"
          ref="carRef"
          @scroll.passive="onCarScroll"
          @touchstart.passive="stopAuto"
          @touchend.passive="startAuto"
        >
          <!-- 经期等健康提醒：置于滚动栏首位，点击跳转到提醒页 -->
          <section
            v-for="a in healthAlerts"
            :key="a.id"
            class="car-slide slide-alert"
            :class="`alert-${a.level}`"
            @click="router.push('/tools/health')"
          >
            <span class="alert-emoji">{{ a.emoji }}</span>
            <div class="alert-main">
              <p class="slide-tag alert-tag">{{ a.title }}</p>
              <p class="alert-text ellipsis-2">{{ a.text }}</p>
            </div>
            <span class="alert-go">›</span>
          </section>

          <!-- 站内提醒摘要：待办提醒，点击跳转提醒中心 -->
          <section
            v-if="reminderBrief.length"
            class="car-slide slide-alert alert-info"
            @click="router.push('/tools/reminders')"
          >
            <span class="alert-emoji">🔔</span>
            <div class="alert-main">
              <p class="slide-tag alert-tag">提醒中心 · 待办 {{ reminderPending }} 条</p>
              <p class="alert-text ellipsis-2">
                {{ reminderBrief.map((r) => r.title).join('；') }}
              </p>
            </div>
            <span class="alert-go">›</span>
          </section>

          <!-- 快递代取：全家待取件，点击跳转取件码页 -->
          <section
            v-if="hasParcel"
            class="car-slide slide-parcel"
            @click="router.push('/tools/parcel')"
          >
            <p class="slide-tag">
              📦 快递代取 · 待取 {{ parcelTotal }} 件
              <span v-if="parcel.expiredCount" class="parcel-expired">含 {{ parcel.expiredCount }} 件已超时</span>
            </p>
            <div class="parcel-list">
              <div v-for="c in parcelCards" :key="c.key" class="parcel-row">
                <span class="parcel-who">
                  <i class="parcel-avatar">{{ c.emoji }}</i>{{ c.name }}
                </span>
                <span class="parcel-codes ellipsis">{{ c.codes }}</span>
                <span v-if="c.rest > 0" class="parcel-rest">等{{ c.total }}件</span>
              </div>
            </div>
          </section>

          <section class="car-slide slide-quote">
            <p class="slide-tag">🍀 今日厨房语录</p>
            <p class="quote-text">“{{ dailyQuote }}”</p>
          </section>

          <section v-if="heroPick" class="car-slide slide-dish" @click="openRecipe(heroPick)">
            <div class="slide-dish-main">
              <p class="slide-tag">✨ 今天推荐吃</p>
              <p class="slide-dish-name ellipsis">{{ heroPick.name }}</p>
              <p class="slide-dish-sub">
                {{ durationText(heroPick.total_minutes) }}<template v-if="heroPick.difficulty"> · {{ heroPick.difficulty }}</template><template v-if="heroPick.calories"> · {{ heroPick.calories }} 千卡</template>
              </p>
            </div>
            <button class="slide-add" :class="{ on: cart.has(heroPick.id) }" @click.stop="addToCart(heroPick)" aria-label="加入点餐车">
              {{ cart.has(heroPick.id) ? '✓' : '＋' }}
            </button>
          </section>

          <section v-if="overview" class="car-slide slide-stats">
            <p class="slide-tag">📊 家的账本</p>
            <div class="hero-stats">
              <div class="hstat">
                <span class="hstat-num num">{{ overview.recipe_active }}</span>
                <span class="hstat-label">道菜</span>
              </div>
              <div class="hstat-divider" />
              <div class="hstat">
                <span class="hstat-num num">{{ overview.order_total }}</span>
                <span class="hstat-label">次点餐</span>
              </div>
              <div class="hstat-divider" />
              <div class="hstat">
                <span class="hstat-num num">{{ ratingText(overview.avg_rating) }}</span>
                <span class="hstat-label">平均评分</span>
              </div>
            </div>
          </section>

          <section v-if="heroStar" class="car-slide slide-dish" @click="openRecipe(heroStar)">
            <div class="slide-dish-main">
              <p class="slide-tag">🏆 常点榜第一</p>
              <p class="slide-dish-name ellipsis">{{ heroStar.name }}</p>
              <p class="slide-dish-sub">点过 {{ heroStar.order_count }} 次 · 评分 {{ ratingText(heroStar.avg_rating) }}</p>
            </div>
            <DishCover :emoji="heroStar.cover_emoji" :gradient="heroStar.cover_gradient" :url="heroStar.cover_url" size="sm" />
          </section>
        </div>
        <div v-if="heroDots > 1" class="car-dots" aria-hidden="true">
          <i v-for="n in heroDots" :key="n" class="car-dot" :class="{ on: heroPage === n - 1 }" />
        </div>
      </div>
    </header>

    <div class="page-body">
      <!-- 今日餐次 -->
      <section v-if="todayOrders.length" class="section">
        <div class="section-head">
          <h2 class="section-title">今日安排</h2>
          <button class="section-more" @click="router.push('/orders')">全部订单 ›</button>
        </div>
        <div class="today-list">
          <div
            v-for="o in todayOrders"
            :key="o.id"
            class="today-card clickable"
            @click="router.push(`/orders/${o.id}`)"
          >
            <div class="today-meal">
              <span class="today-meal-emoji">{{ mealEmoji(o.meal_type) }}</span>
              <span class="today-meal-name">{{ mealLabel(o.meal_type) }}</span>
            </div>
            <div class="today-info">
              <div class="today-emojis">{{ o.cover_emojis || '🍽️' }}</div>
              <p class="today-dishes ellipsis">{{ o.dish_names || '还没点菜' }}</p>
            </div>
            <div class="today-right">
              <span class="chip" :class="statusClass(o.status)">{{ statusLabel(o.status) }}</span>
              <span class="today-count num">{{ o.dish_count }} 道 · {{ o.diners }} 人</span>
            </div>
          </div>
        </div>
      </section>

      <!-- 智能推荐 -->
      <section class="section">
        <div class="section-head">
          <h2 class="section-title">猜你想吃</h2>
          <button class="section-more" @click="refreshSuggestion">🎲 换一批</button>
        </div>
        <div v-if="loading && !suggestions.length" class="rail">
          <YanLoader v-for="i in 3" :key="i" class="rail-sk" :size="120" :blobs="false" :show-tip="false" />
        </div>
        <div v-else-if="suggestions.length" class="rail">
          <DishCard
            v-for="r in suggestions"
            :key="r.id"
            :recipe="r"
            layout="grid"
            class="rail-item"
            :in-cart="cart.has(r.id)"
            @click="openRecipe"
            @add="addToCart"
          />
        </div>
        <EmptyState v-else emoji="🍳" title="还没有菜谱" desc="先去菜谱库建几道菜，就能智能推荐啦">
          <button class="btn btn--primary btn--sm" @click="router.push('/recipes/new')">＋ 新建菜谱</button>
        </EmptyState>
      </section>

      <!-- 快速入口 -->
      <section class="section">
        <div class="quick-grid">
          <button class="quick" @click="goOrder">
            <span class="quick-icon">🛒</span>
            <span class="quick-label">开始点餐</span>
            <span v-if="cart.count" class="quick-badge num">{{ cart.count }}</span>
          </button>
          <button class="quick" @click="router.push('/recipes')">
            <span class="quick-icon">📖</span>
            <span class="quick-label">菜谱库</span>
          </button>
          <button class="quick" @click="router.push('/recipes/new')">
            <span class="quick-icon">✍️</span>
            <span class="quick-label">记一道菜</span>
          </button>
          <button class="quick" @click="router.push('/stats')">
            <span class="quick-icon">📊</span>
            <span class="quick-label">家的味道</span>
          </button>
        </div>
      </section>

      <!-- 常点榜 -->
      <section v-if="popular.length" class="section">
        <div class="section-head">
          <h2 class="section-title">家常榜单</h2>
          <button class="section-more" @click="router.push('/stats')">查看统计 ›</button>
        </div>
        <div class="rank-list">
          <div
            v-for="(r, i) in popular"
            :key="r.id"
            class="rank-row clickable"
            @click="openRecipe(r)"
          >
            <span class="rank-no" :class="{ 'rank-no--top': i < 3 }">{{ i + 1 }}</span>
            <DishCover :emoji="r.cover_emoji" :gradient="r.cover_gradient" :url="r.cover_url" size="sm" />
            <div class="rank-info">
              <p class="rank-name ellipsis">{{ r.name }}</p>
              <div class="rank-sub">
                <RatingStars :value="Number(r.avg_rating) || 0" :size="11" />
                <span class="rank-times num">点过 {{ r.order_count }} 次</span>
              </div>
            </div>
            <button class="rank-add" :class="{ on: cart.has(r.id) }" @click.stop="addToCart(r)" aria-label="加入">
              {{ cart.has(r.id) ? '✓' : '＋' }}
            </button>
          </div>
        </div>
      </section>

      <div class="home-bottom-space" />
    </div>
  </div>
</template>

<style scoped>
/* ---------- Hero ---------- */
.hero {
  position: relative;
  overflow: hidden;
  padding-top: calc(var(--safe-top) + 20px);
  padding-bottom: 22px;
  color: #fff;
}
.hero-bg {
  position: absolute; inset: 0;
  background: linear-gradient(135deg, #C2703D 0%, #A85A2E 45%, #8A4522 100%);
  z-index: -2;
}
.hero-bg::after {
  content: '';
  position: absolute; inset: 0;
  background:
    radial-gradient(70% 50% at 85% 10%, rgba(255,255,255,.16), transparent 60%),
    radial-gradient(60% 60% at 10% 100%, rgba(120,20,20,.22), transparent 55%);
}
.hero-body { position: relative; z-index: 1; }

.hero-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
.hero-greeting { font-size: 14px; opacity: .92; }
.hero-title { font-size: 26px; font-weight: 800; margin-top: 4px; letter-spacing: -.02em; text-shadow: 0 2px 10px rgba(0,0,0,.12); }

.hero-avatar {
  width: 48px; height: 48px;
  border-radius: 50%;
  border: 2px solid rgba(255,255,255,.6);
  background: rgba(255,255,255,.2);
  backdrop-filter: blur(6px);
  font-size: 24px;
  display: flex; align-items: center; justify-content: center;
  overflow: hidden;
  cursor: pointer;
  flex: none;
  transition: transform var(--dur-fast) var(--ease-bounce);
}
.hero-avatar:active { transform: scale(.9); }
.hero-avatar-img { width: 100%; height: 100%; border-radius: 50%; object-fit: cover; display: block; }

.hero-stats {
  display: flex;
  align-items: center;
  gap: 4px;
  width: 100%;
  padding: 14px 8px;
  border-radius: var(--r-lg);
}
.hstat { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 2px; }
.hstat-num { font-size: 22px; font-weight: 800; line-height: 1.1; color: var(--brand-600); }
.hstat-label { font-size: 11.5px; color: var(--text-3); }
.hstat-divider { width: 1px; height: 26px; background: var(--border); }

/* ---------- 顶部滚动显示区（白底卡片，与深色顶部形成对比） ---------- */
.hero-car {
  display: flex;
  margin-top: 18px;
  overflow-x: auto;
  scroll-snap-type: x mandatory;
  scroll-behavior: smooth;
  -webkit-overflow-scrolling: touch;
  scrollbar-width: none;
  border-radius: var(--r-lg);
}
.hero-car::-webkit-scrollbar { display: none; }
.car-slide {
  flex: none;
  width: 100%;
  scroll-snap-align: start;
  box-sizing: border-box;
  min-height: 96px;
  padding: 14px 16px;
  background: var(--surface, #fff);
  color: var(--text-1, #33291f);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-md, 0 6px 18px rgba(0,0,0,.14));
  cursor: pointer;
}
.slide-quote { display: flex; flex-direction: column; justify-content: center; gap: 8px; cursor: default; }
.quote-text { font-size: 17px; font-weight: 700; line-height: 1.6; color: var(--text-1); font-family: var(--font-kai); }

/* 经期提醒卡片 */
.slide-alert {
  display: flex;
  align-items: center;
  gap: 12px;
  border-left: 4px solid var(--tone, var(--brand-500));
  background: var(--tone-bg, var(--surface));
}
.slide-alert .alert-tag { color: var(--tone); margin-bottom: 3px; }
.alert-emoji { font-size: 28px; flex: none; }
.alert-main { flex: 1; min-width: 0; }
.alert-text { font-size: 13px; color: var(--text-2); line-height: 1.45; }
.alert-go { flex: none; font-size: 24px; color: var(--tone); opacity: .6; line-height: 1; }
.alert-danger { --tone: var(--danger); --tone-bg: var(--danger-bg); }
.alert-warning { --tone: #A9761F; --tone-bg: var(--warning-bg); }
.alert-success { --tone: var(--success); --tone-bg: var(--success-bg); }
.alert-info { --tone: var(--info); --tone-bg: var(--info-bg); }

/* 快递代取卡片 */
.slide-parcel {
  border-left: 4px solid var(--brand-500);
  background: var(--surface);
}
.slide-parcel .slide-tag {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
}
.parcel-expired {
  font-size: 11px;
  font-weight: 700;
  color: var(--danger);
  background: var(--danger-bg);
  border-radius: var(--r-full);
  padding: 1px 7px;
}
.parcel-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
  /* 收件人多时卡内滚动，避免撑高横向滚动栏、破坏其他卡片对齐 */
  max-height: 92px;
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-width: none;
}
.parcel-list::-webkit-scrollbar { display: none; }
.parcel-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  line-height: 1.4;
}
.parcel-who {
  flex: none;
  display: flex;
  align-items: center;
  gap: 3px;
  font-weight: 700;
  color: var(--text-1);
}
.parcel-avatar { font-style: normal; font-size: 14px; }
.parcel-codes { flex: 1; min-width: 0; color: var(--text-2); }
.parcel-rest {
  flex: none;
  font-size: 11px;
  color: var(--text-3);
  background: var(--bg-subtle);
  border-radius: var(--r-full);
  padding: 1px 7px;
}
.slide-tag { font-size: 11.5px; font-weight: 700; color: var(--brand-600); margin-bottom: 6px; letter-spacing: .04em; }
.slide-quote .slide-tag { margin-bottom: 0; }
.slide-dish { display: flex; align-items: center; gap: 12px; }
.slide-dish-main { flex: 1; min-width: 0; }
.slide-dish-name { font-size: 19px; font-weight: 800; letter-spacing: -.01em; color: var(--text-1); }
.slide-dish-sub { font-size: 12px; color: var(--text-3); margin-top: 4px; }
.slide-stats { display: flex; flex-direction: column; align-items: stretch; cursor: default; }
.slide-stats .hero-stats { background: var(--bg-subtle); }
.slide-add {
  flex: none; width: 40px; height: 40px; border-radius: 50%;
  border: none;
  background: var(--brand-gradient); color: #fff;
  box-shadow: var(--sh-brand, 0 4px 12px rgba(194, 112, 61, .35));
  font-size: 22px; line-height: 1; cursor: pointer;
  transition: transform var(--dur-fast) var(--ease-bounce);
  -webkit-tap-highlight-color: transparent;
}
.slide-add:active { transform: scale(.86); }
.slide-add.on { background: var(--success); box-shadow: none; font-size: 18px; }
.car-dots { display: flex; justify-content: center; gap: 6px; margin-top: 10px; }
.car-dot {
  width: 6px; height: 6px; border-radius: 50%;
  background: rgba(255,255,255,.45);
  transition: all var(--dur-fast) var(--ease);
}
.car-dot.on { width: 16px; border-radius: var(--r-full); background: #fff; }

/* ---------- 今日安排 ---------- */
.today-list { display: flex; flex-direction: column; gap: 10px; }
.today-card {
  display: flex; align-items: center; gap: 12px;
  background: var(--surface);
  border-radius: var(--r-lg);
  padding: 12px 14px;
  box-shadow: var(--sh-sm);
  border: 1px solid var(--border);
}
.today-meal { display: flex; flex-direction: column; align-items: center; gap: 2px; width: 44px; flex: none; }
.today-meal-emoji { font-size: 22px; }
.today-meal-name { font-size: 11px; color: var(--text-3); font-weight: 600; }
.today-info { flex: 1; min-width: 0; }
.today-emojis { font-size: 15px; letter-spacing: 1px; margin-bottom: 2px; }
.today-dishes { font-size: 13.5px; color: var(--text-2); }
.today-right { display: flex; flex-direction: column; align-items: flex-end; gap: 5px; flex: none; }
.today-count { font-size: 11px; color: var(--text-3); }

/* ---------- 推荐横滑 ---------- */
.rail {
  display: flex;
  gap: 12px;
  overflow-x: auto;
  padding-bottom: 6px;
  scroll-snap-type: x mandatory;
  -webkit-overflow-scrolling: touch;
  margin: 0 calc(var(--page-pad) * -1);
  padding-left: var(--page-pad);
  padding-right: var(--page-pad);
  scrollbar-width: none;
}
.rail::-webkit-scrollbar { display: none; }
.rail-item { width: 210px; flex: none; scroll-snap-align: start; }
.rail-sk { width: 210px; height: 240px; flex: none; }

@media (min-width: 700px) {
  .rail-item { width: 240px; }
}

/* ---------- 快速入口 ---------- */
.quick-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
.quick {
  position: relative;
  display: flex; flex-direction: column; align-items: center; gap: 7px;
  padding: 16px 6px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-xs);
  cursor: pointer;
  transition: transform var(--dur-fast) var(--ease), box-shadow var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
.quick:active { transform: scale(.95); box-shadow: var(--sh-md); }
.quick-icon { font-size: 26px; }
.quick-label { font-size: 12.5px; font-weight: 600; color: var(--text-2); }
.quick-badge {
  position: absolute; top: 8px; right: 10px;
  min-width: 17px; height: 17px; padding: 0 4px;
  border-radius: var(--r-full);
  background: var(--danger); color: #fff;
  font-size: 10px; font-weight: 700; line-height: 17px;
}

/* ---------- 榜单 ---------- */
.rank-list { display: flex; flex-direction: column; gap: 4px; background: var(--surface); border-radius: var(--r-lg); padding: 6px; box-shadow: var(--sh-sm); }
.rank-row { display: flex; align-items: center; gap: 11px; padding: 9px 8px; border-radius: var(--r-md); transition: background var(--dur-fast); }
.rank-row:active { background: var(--bg-subtle); }
.rank-no { width: 22px; text-align: center; font-size: 15px; font-weight: 800; color: var(--text-4); font-family: var(--font-num); flex: none; }
.rank-no--top { color: var(--brand-500); }
.rank-info { flex: 1; min-width: 0; }
.rank-name { font-size: 15px; font-weight: 600; }
.rank-sub { display: flex; align-items: center; gap: 8px; margin-top: 2px; }
.rank-times { font-size: 11.5px; color: var(--text-3); }
.rank-add {
  flex: none; width: 32px; height: 32px; border-radius: 50%;
  border: 1.5px solid var(--brand-200); background: var(--surface);
  color: var(--brand-600); font-size: 18px; line-height: 1;
  cursor: pointer; transition: all var(--dur-fast) var(--ease-bounce);
}
.rank-add:active { transform: scale(.86); }
.rank-add.on { background: var(--success); border-color: var(--success); color: #fff; font-size: 15px; }

.home-bottom-space { height: 20px; }
</style>
