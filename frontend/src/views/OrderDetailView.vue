<script setup>
/**
 * 订单详情 · 一餐的完整视图
 *
 * 三大块：
 * 1. 菜单卡片 —— 这一餐点了什么（可一键查看全部做法）
 * 2. 采购清单 —— 后端按份数聚合去重的食材，可勾选已买，常备品折叠
 * 3. 出菜进度 + 餐后评分
 *
 * 草稿态可确认（触发采购清单生成）；进行中可标记出菜；已开饭可评分。
 */
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { api } from '@/api'
import DishCover from '@/components/DishCover.vue'
import RatingStars from '@/components/RatingStars.vue'
import EmptyState from '@/components/EmptyState.vue'
import YanLoader from '@/components/YanLoader.vue'
import { toast } from '@/composables/useToast'
import {
  relativeDay, mealLabel, mealEmoji, statusLabel, statusClass,
  difficultyLabel, durationText, quantityText
} from '@/utils/format'

const route = useRoute()
const router = useRouter()

const id = Number(route.params.id)
const order = ref(null)
const loading = ref(true)
const busy = ref(false)
const showStaples = ref(false)

async function load() {
  loading.value = true
  try {
    order.value = await api.orders.detail(id)
  } catch (e) {
    toast.error(e.message || '订单加载失败')
    router.replace('/orders')
  } finally {
    loading.value = false
  }
}
onMounted(load)

const items = computed(() => order.value?.items || [])
const activeItems = computed(() => items.value.filter((i) => i.item_status !== 'skipped'))
const shopping = computed(() => order.value?.shopping_list || { groups: [], staples: [], total_kinds: 0, purchased_kinds: 0 })

const isDraft = computed(() => order.value?.status === 'draft')
const isCancelled = computed(() => order.value?.status === 'cancelled')
const canCook = computed(() => ['confirmed', 'cooking'].includes(order.value?.status))
const canRate = computed(() => order.value?.status === 'served')

async function doAction(fn, okMsg) {
  if (busy.value) return
  busy.value = true
  try {
    await fn()
    if (okMsg) toast.success(okMsg)
    await load()
  } catch (e) {
    toast.error(e.message || '操作失败')
  } finally {
    busy.value = false
  }
}

const confirmOrder = () => doAction(() => api.orders.confirm(id), '订单已确认，采购清单已生成')
const cancelOrder = () => { if (confirm('确定取消这个订单吗？')) doAction(() => api.orders.cancel(id), '订单已取消') }
const reorder = () => doAction(async () => { const r = await api.orders.reorder(id); router.replace(`/orders/${r?.order?.id || r?.id || ''}`) }, '已按这单新建草稿')

async function setItemStatus(item, status) {
  await doAction(() => api.orders.updateItem(item.id, { item_status: status }))
}
async function rate(item, rating) {
  await doAction(() => api.orders.rateItem(item.id, rating), `给「${item.recipe_snapshot}」打了 ${rating} 星`)
}
async function toggleShop(si) {
  // 乐观更新
  si.is_purchased = !si.is_purchased
  try {
    await api.orders.toggleShopping(si.id, si.is_purchased)
  } catch (e) {
    si.is_purchased = !si.is_purchased
    toast.error(e.message || '更新失败')
  }
}

function viewRecipes() { router.push(`/orders/${id}/recipes`) }
function viewRecipe(rid) { if (rid) router.push(`/recipes/${rid}`) }

const allDone = computed(() => activeItems.value.length && activeItems.value.every((i) => i.item_status === 'done'))
const progress = computed(() => {
  const total = activeItems.value.length || 1
  const done = activeItems.value.filter((i) => i.item_status === 'done').length
  return Math.round((done / total) * 100)
})
</script>

<template>
  <div class="page page--flat od">
    <div v-if="loading" class="page-body">
      <YanLoader :size="180" tip="订单加载中" />
    </div>

    <template v-else-if="order">
      <!-- 头部 -->
      <header class="od-hero" :class="`od-hero--${order.status}`">
        <div class="page-body">
          <div class="od-hero-top">
            <span class="od-meal">{{ mealEmoji(order.meal_type) }} {{ mealLabel(order.meal_type) }}</span>
            <span class="chip od-status" :class="statusClass(order.status)">{{ statusLabel(order.status) }}</span>
          </div>
          <h1 class="od-date">{{ relativeDay(order.meal_date) }}</h1>
          <p class="od-no num">{{ order.order_no }}</p>
          <div class="od-host">
            <span class="od-avatar">{{ order.member_avatar }}</span>
            <span>{{ order.member_nickname }} 点的 · {{ order.diners }} 人就餐</span>
          </div>
          <p v-if="order.note" class="od-note">📝 {{ order.note }}</p>
        </div>
      </header>

      <div class="page-body">
        <!-- 汇总 -->
        <div class="od-summary card">
          <div class="sum"><span class="sum-v num">{{ activeItems.length }}</span><span class="sum-l">道菜</span></div>
          <div class="sum-div" />
          <div class="sum"><span class="sum-v num">{{ order.total_portions }}</span><span class="sum-l">总份数</span></div>
          <div class="sum-div" />
          <div class="sum"><span class="sum-v num">{{ order.total_calories || '—' }}</span><span class="sum-l">千卡</span></div>
          <div class="sum-div" />
          <div class="sum"><span class="sum-v num">{{ durationText(order.total_cook_minutes) }}</span><span class="sum-l">烹饪</span></div>
        </div>

        <!-- 菜单 -->
        <section class="section">
          <div class="section-head">
            <h2 class="section-title">这一餐的菜单</h2>
            <button class="section-more" @click="viewRecipes">看全部做法 ›</button>
          </div>

          <!-- 出菜进度 -->
          <div v-if="canCook || order.status === 'served'" class="progress-wrap">
            <div class="progress-bar"><div class="progress-fill" :style="{ width: progress + '%' }" /></div>
            <span class="progress-text num">{{ progress }}% 出菜</span>
          </div>

          <div class="menu-list">
            <div v-for="it in items" :key="it.id" class="menu-item" :class="{ skipped: it.item_status === 'skipped', done: it.item_status === 'done' }">
              <DishCover
                :emoji="it.cover_emoji || '🍽️'"
                :gradient="it.cover_gradient || 'linear-gradient(135deg,#F2C89B,#DE9A66)'"
                size="md"
                class="mi-cover"
                @click="viewRecipe(it.recipe_id)"
              />
              <div class="mi-body" @click="viewRecipe(it.recipe_id)">
                <div class="mi-head">
                  <p class="mi-name ellipsis">{{ it.recipe_snapshot }}</p>
                  <span class="mi-portions num">×{{ it.portions }}</span>
                </div>
                <div class="mi-meta">
                  <span v-if="it.category" class="mi-tag">{{ it.category }}</span>
                  <span v-if="it.total_minutes" class="mi-tag">{{ durationText(it.total_minutes) }}</span>
                  <span v-if="it.ordered_by" class="mi-by">{{ it.ordered_by_avatar }} {{ it.ordered_by }}</span>
                </div>
                <p v-if="it.note" class="mi-note">💬 {{ it.note }}</p>
                <!-- 评分 -->
                <div v-if="canRate || it.rating" class="mi-rate">
                  <RatingStars
                    :value="it.rating || 0"
                    :size="18"
                    :readonly="!canRate"
                    @rate="(v) => rate(it, v)"
                  />
                  <span v-if="!canRate && !it.rating" class="mi-rate-hint">开饭后可评分</span>
                </div>
              </div>
              <!-- 出菜操作 -->
              <div v-if="canCook && it.item_status !== 'skipped'" class="mi-actions">
                <button v-if="it.item_status !== 'done'" class="mi-act mi-act--done" @click="setItemStatus(it, 'done')">✓ 好了</button>
                <button v-else class="mi-act mi-act--undo" @click="setItemStatus(it, 'pending')">撤销</button>
                <button class="mi-act mi-act--skip" @click="setItemStatus(it, 'skipped')">跳过</button>
              </div>
            </div>
          </div>
        </section>

        <!-- 采购清单 -->
        <section class="section">
          <div class="section-head">
            <h2 class="section-title">采购清单</h2>
            <span class="text-sm text-muted num">
              {{ shopping.purchased_kinds }}/{{ shopping.total_kinds }} 已备
            </span>
          </div>

          <div v-if="shopping.groups.length || shopping.staples.length" class="shop-card card card--pad">
            <p v-if="isDraft" class="shop-hint">💡 确认订单后，采购清单会锁定并保存勾选状态</p>

            <div v-for="g in shopping.groups" :key="g.category" class="shop-group">
              <h4 class="shop-group-name">{{ g.category }} <span class="num">{{ g.items.length }}</span></h4>
              <ul class="shop-list">
                <li v-for="si in g.items" :key="si.id" class="shop-item" :class="{ bought: si.is_purchased }">
                  <button class="shop-check" :class="{ on: si.is_purchased }" @click="toggleShop(si)" aria-label="勾选">
                    <span v-if="si.is_purchased">✓</span>
                  </button>
                  <span class="shop-emoji">{{ si.emoji }}</span>
                  <span class="shop-name ellipsis">{{ si.name_snapshot }}</span>
                  <span class="shop-qty num">{{ quantityText(si.total_quantity) }}{{ si.unit }}</span>
                </li>
              </ul>
            </div>

            <!-- 常备折叠 -->
            <div v-if="shopping.staples.length" class="shop-staples">
              <button class="staple-toggle" @click="showStaples = !showStaples">
                <span>🧂 家中常备 {{ shopping.staples.length }} 项</span>
                <span class="staple-caret" :class="{ up: showStaples }">▾</span>
              </button>
              <ul v-show="showStaples" class="shop-list staple-list">
                <li v-for="si in shopping.staples" :key="si.id" class="shop-item" :class="{ bought: si.is_purchased }">
                  <button class="shop-check" :class="{ on: si.is_purchased }" @click="toggleShop(si)" aria-label="勾选">
                    <span v-if="si.is_purchased">✓</span>
                  </button>
                  <span class="shop-emoji">{{ si.emoji }}</span>
                  <span class="shop-name ellipsis">{{ si.name_snapshot }}</span>
                  <span class="shop-qty num">{{ quantityText(si.total_quantity) }}{{ si.unit }}</span>
                </li>
              </ul>
            </div>
          </div>
          <EmptyState v-else emoji="🧺" title="暂无需采购的食材" desc="这单可能都是家中常备，或还没确认订单" />
        </section>

        <div class="bottom-space" />
      </div>

      <!-- 底部操作 -->
      <div class="od-bar">
        <div class="page-body od-bar-inner">
          <template v-if="isDraft">
            <button class="btn btn--danger btn--icon od-mini" @click="cancelOrder" aria-label="取消">✕</button>
            <button class="btn btn--ghost grow" @click="router.push('/order')">继续加菜</button>
            <button class="btn btn--primary grow" :disabled="busy" @click="confirmOrder">确认并生成采购单</button>
          </template>
          <template v-else-if="isCancelled">
            <button class="btn btn--soft grow" :disabled="busy" @click="reorder">照这单再来一次</button>
          </template>
          <template v-else-if="order.status === 'confirmed' || order.status === 'cooking'">
            <button class="btn btn--ghost grow" @click="viewRecipes">👨‍🍳 看做法</button>
            <button class="btn btn--primary grow" :disabled="busy || !allDone" @click="doAction(() => api.orders.update(id, { status: 'served' }), '开饭啦！')">
              {{ allDone ? '🍚 开饭' : '菜还没做完' }}
            </button>
          </template>
          <template v-else>
            <button class="btn btn--ghost grow" @click="viewRecipes">👨‍🍳 回看做法</button>
            <button class="btn btn--soft grow" :disabled="busy" @click="reorder">再来一次</button>
          </template>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.od-hero { padding: calc(var(--safe-top) + 56px) 0 22px; color: #fff; background: linear-gradient(135deg, #C2703D, #8A4522); }
.od-hero--served { background: linear-gradient(135deg, #5B8A5F, #3F6E4B); }
.od-hero--cancelled { background: linear-gradient(135deg, #A89A88, #8A7B6A); }
.od-hero--cooking { background: linear-gradient(135deg, #FF9800, #F57C00); }
.od-hero-top { display: flex; align-items: center; justify-content: space-between; }
.od-meal { font-size: 15px; font-weight: 600; }
.od-status { background: rgba(255,255,255,.25) !important; color: #fff !important; }
.od-date { font-size: 30px; font-weight: 800; margin-top: 6px; letter-spacing: -.02em; }
.od-no { font-size: 12px; opacity: .8; margin-top: 2px; }
.od-host { display: flex; align-items: center; gap: 7px; margin-top: 12px; font-size: 14px; opacity: .95; }
.od-avatar { font-size: 20px; }
.od-note { margin-top: 10px; font-size: 13px; background: rgba(255,255,255,.18); padding: 8px 12px; border-radius: var(--r-sm); line-height: 1.5; }

.od-summary { display: flex; align-items: center; margin-top: -30px; position: relative; z-index: 2; padding: 16px 8px; box-shadow: var(--sh-md); }
.sum { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 2px; }
.sum-v { font-size: 19px; font-weight: 800; color: var(--brand-600); }
.sum-l { font-size: 11px; color: var(--text-3); }
.sum-div { width: 1px; height: 28px; background: var(--border); }

.progress-wrap { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; }
.progress-bar { flex: 1; height: 7px; background: var(--bg-subtle); border-radius: var(--r-full); overflow: hidden; }
.progress-fill { height: 100%; background: var(--brand-gradient); border-radius: var(--r-full); transition: width var(--dur-slow) var(--ease); }
.progress-text { font-size: 12px; font-weight: 700; color: var(--brand-600); flex: none; }

.menu-list { display: flex; flex-direction: column; gap: 10px; }
.menu-item { display: flex; gap: 12px; align-items: flex-start; background: var(--surface); border-radius: var(--r-lg); padding: 12px; box-shadow: var(--sh-sm); border: 1px solid var(--border); transition: opacity var(--dur); }
.menu-item.skipped { opacity: .45; }
.menu-item.done { border-color: var(--success); background: linear-gradient(135deg, #fff, #F1FBF5); }
.mi-cover { cursor: pointer; }
.mi-body { flex: 1; min-width: 0; cursor: pointer; }
.mi-head { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; }
.mi-name { font-size: 16px; font-weight: 700; flex: 1; min-width: 0; }
.mi-portions { font-size: 14px; font-weight: 700; color: var(--brand-600); flex: none; }
.mi-meta { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 5px; align-items: center; }
.mi-tag { font-size: 11px; color: var(--text-3); background: var(--bg-subtle); padding: 2px 8px; border-radius: var(--r-full); }
.mi-by { font-size: 11.5px; color: var(--text-3); }
.mi-note { font-size: 12.5px; color: var(--text-2); margin-top: 6px; }
.mi-rate { display: flex; align-items: center; gap: 8px; margin-top: 8px; }
.mi-rate-hint { font-size: 11px; color: var(--text-4); }
.mi-actions { display: flex; flex-direction: column; gap: 6px; flex: none; }
.mi-act { padding: 6px 10px; border-radius: var(--r-sm); border: none; font-size: 12px; font-weight: 600; cursor: pointer; white-space: nowrap; font-family: inherit; transition: transform var(--dur-fast); }
.mi-act:active { transform: scale(.94); }
.mi-act--done { background: var(--success); color: #fff; }
.mi-act--undo { background: var(--bg-subtle); color: var(--text-2); }
.mi-act--skip { background: transparent; color: var(--text-4); font-size: 11px; }

.shop-card { }
.shop-hint { font-size: 12.5px; color: #B8860B; background: var(--warning-bg); padding: 8px 12px; border-radius: var(--r-sm); margin-bottom: 12px; }
.shop-group + .shop-group { margin-top: 16px; }
.shop-group-name { font-size: 13px; font-weight: 700; color: var(--brand-600); margin-bottom: 8px; display: flex; align-items: center; gap: 6px; }
.shop-group-name .num { background: var(--brand-50); border-radius: var(--r-full); padding: 0 7px; font-size: 11px; }
.shop-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
.shop-item { display: flex; align-items: center; gap: 10px; padding: 7px 4px; border-radius: var(--r-xs); transition: opacity var(--dur); }
.shop-item.bought { opacity: .5; }
.shop-item.bought .shop-name { text-decoration: line-through; }
.shop-check { width: 24px; height: 24px; flex: none; border-radius: 7px; border: 2px solid var(--border-strong); background: var(--surface); cursor: pointer; display: flex; align-items: center; justify-content: center; color: #fff; font-size: 14px; font-weight: 700; transition: all var(--dur-fast) var(--ease-bounce); }
.shop-check.on { background: var(--success); border-color: var(--success); }
.shop-check:active { transform: scale(.85); }
.shop-emoji { font-size: 18px; flex: none; }
.shop-name { flex: 1; font-size: 14.5px; min-width: 0; }
.shop-qty { font-size: 14px; font-weight: 600; color: var(--text-2); flex: none; }

.shop-staples { margin-top: 16px; padding-top: 12px; border-top: 1px dashed var(--border-strong); }
.staple-toggle { width: 100%; display: flex; align-items: center; justify-content: space-between; background: none; border: none; font-size: 13px; font-weight: 600; color: var(--text-3); cursor: pointer; font-family: inherit; padding: 4px 0; }
.staple-caret { transition: transform var(--dur-fast); }
.staple-caret.up { transform: rotate(180deg); }
.staple-list { margin-top: 8px; }

.od-bar { position: fixed; left: 0; right: 0; bottom: 0; z-index: 50; padding: 10px 0 calc(10px + var(--safe-bottom)); background: rgba(255,253,247,.95); backdrop-filter: blur(16px); border-top: 1px solid var(--border); }
.od-bar-inner { display: flex; gap: 8px; align-items: center; max-width: 1180px; }
.od-mini { width: 44px; height: 44px; min-height: 44px; flex: none; }
.bottom-space { height: 90px; }
</style>
