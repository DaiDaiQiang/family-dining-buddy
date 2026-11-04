<script setup>
/**
 * 订单列表
 *
 * 顶部状态筛选（全部/进行中/已完成），主区按日期倒序的订单卡片。
 */
import { computed, onActivated, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { api } from '@/api'
import DishCover from '@/components/DishCover.vue'
import EmptyState from '@/components/EmptyState.vue'
import YanLoader from '@/components/YanLoader.vue'
import { toast } from '@/composables/useToast'
import { relativeDay, mealLabel, mealEmoji, statusLabel, statusClass } from '@/utils/format'

defineOptions({ name: 'OrderListView' })

const router = useRouter()
const loading = ref(true)
const orders = ref([])
const filter = ref('all')

const filters = [
  { key: 'all', label: '全部' },
  { key: 'active', label: '进行中' },
  { key: 'served', label: '已完成' }
]

async function load() {
  loading.value = true
  try {
    const params = {}
    if (filter.value === 'served') params.status = 'served'
    const res = await api.orders.list(params)
    let list = res?.list || res || []
    if (filter.value === 'active') {
      list = list.filter((o) => ['draft', 'confirmed', 'cooking'].includes(o.status))
    }
    orders.value = list
  } catch (e) {
    toast.error(e.message || '订单加载失败')
  } finally {
    loading.value = false
  }
}

function pickFilter(k) { filter.value = k; load() }

onMounted(load)
onActivated(() => { if (!loading.value) load() })

// 按日期分组
const grouped = computed(() => {
  const map = new Map()
  orders.value.forEach((o) => {
    const key = o.meal_date
    if (!map.has(key)) map.set(key, [])
    map.get(key).push(o)
  })
  return [...map.entries()].map(([date, list]) => ({ date, list }))
})

function coverOf(o) {
  // 用第一道菜的 emoji 做卡片视觉
  const first = (o.cover_emojis || '🍽️')[0] || '🍽️'
  return first
}
</script>

<template>
  <div class="page orders">
    <header class="ol-head">
      <div class="page-body">
        <h1 class="ol-title">📋 点餐记录</h1>
        <div class="filter-tabs">
          <button
            v-for="f in filters"
            :key="f.key"
            class="ftab"
            :class="{ on: filter === f.key }"
            @click="pickFilter(f.key)"
          >{{ f.label }}</button>
        </div>
      </div>
    </header>

    <div class="page-body">
      <div v-if="loading">
        <YanLoader :size="180" tip="订单加载中" />
      </div>

      <template v-else-if="orders.length">
        <section v-for="g in grouped" :key="g.date" class="day-group">
          <h3 class="day-label">{{ relativeDay(g.date) }}</h3>
          <div class="order-list stagger">
            <div
              v-for="o in g.list"
              :key="o.id"
              class="order-card clickable"
              @click="router.push(`/orders/${o.id}`)"
            >
              <div class="oc-meal">
                <span class="oc-emoji">{{ mealEmoji(o.meal_type) }}</span>
                <span class="oc-meal-name">{{ mealLabel(o.meal_type) }}</span>
              </div>
              <div class="oc-main">
                <div class="oc-top">
                  <span class="oc-dishes ellipsis">{{ o.dish_names || '空订单' }}</span>
                  <span class="chip" :class="statusClass(o.status)">{{ statusLabel(o.status) }}</span>
                </div>
                <div class="oc-bottom">
                  <span class="oc-host">{{ o.member_avatar }} {{ o.member_nickname }}</span>
                  <span class="oc-count num">{{ o.dish_count }} 道 · {{ o.diners }} 人 · {{ o.total_portions }} 份</span>
                </div>
              </div>
            </div>
          </div>
        </section>
      </template>

      <EmptyState v-else emoji="📭" title="还没有点餐记录" desc="去点餐页选一桌好菜吧">
        <button class="btn btn--primary btn--sm" @click="router.push('/order')">去点餐</button>
      </EmptyState>
      <div class="bottom-space" />
    </div>
  </div>
</template>

<style scoped>
.ol-head { padding-top: calc(var(--safe-top) + 20px); padding-bottom: 4px; }
.ol-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.filter-tabs { display: flex; gap: 8px; margin-top: 14px; }
.ftab { padding: 7px 16px; border-radius: var(--r-full); border: 1.5px solid var(--border-strong); background: var(--surface); color: var(--text-2); font-size: 13px; font-weight: 600; cursor: pointer; transition: all var(--dur-fast); font-family: inherit; }
.ftab.on { background: var(--brand-gradient); border-color: transparent; color: #fff; box-shadow: var(--sh-brand); }

.day-group { margin-top: 20px; }
.day-label { font-size: 13px; font-weight: 700; color: var(--text-3); margin-bottom: 10px; padding-left: 2px; }

.order-list { display: flex; flex-direction: column; gap: 10px; }
.order-card { display: flex; gap: 12px; background: var(--surface); border-radius: var(--r-lg); padding: 13px; box-shadow: var(--sh-sm); border: 1px solid var(--border); }
.order-card:active { transform: scale(.99); }

.oc-meal { display: flex; flex-direction: column; align-items: center; gap: 3px; width: 44px; flex: none; padding-top: 2px; }
.oc-emoji { font-size: 24px; }
.oc-meal-name { font-size: 11px; color: var(--text-3); font-weight: 600; }

.oc-main { flex: 1; min-width: 0; }
.oc-top { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.oc-dishes { font-size: 15px; font-weight: 600; flex: 1; min-width: 0; }
.oc-bottom { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: 8px; }
.oc-host { font-size: 12.5px; color: var(--text-2); }
.oc-count { font-size: 11.5px; color: var(--text-3); }
.bottom-space { height: 30px; }
</style>
