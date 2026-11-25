<script setup>
/**
 * 点餐页（挑菜列表）
 *
 * 参照美团：本页直接展示全部菜品（图/名/分类/时长），
 * 支持搜索、分类筛选；每张卡底部是「－ 数量 ＋」步进，
 * 减到 0 自动移出购物车。点卡片进入详情页看完整介绍。
 * 底部固定结算条实时汇总，点击进入结算页（/order/checkout）。
 *
 * 购物车数据存 Pinia + localStorage，列表页与详情页天然同步。
 */
import { computed, onActivated, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useRecipeStore } from '@/stores/recipe'
import { useCartStore } from '@/stores/cart'
import { useAppStore } from '@/stores/app'
import DishCover from '@/components/DishCover.vue'
import EmptyState from '@/components/EmptyState.vue'
import YanLoader from '@/components/YanLoader.vue'
import { toast } from '@/composables/useToast'
import { durationText, difficultyLabel } from '@/utils/format'

defineOptions({ name: 'OrderView' })

const router = useRouter()
const store = useRecipeStore()
const cart = useCartStore()
const app = useAppStore()

const keyword = ref(store.filters.keyword)

const categoryChips = computed(() => [
  { name: '全部', count: store.total },
  ...(store.facets.categories || [])
])

let searchTimer = null
watch(keyword, (v) => {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    store.setFilter({ keyword: v })
    store.fetchList()
  }, 320)
})

function pickCategory(name) {
  store.setFilter({ category: name === '全部' ? '' : name })
  store.fetchList()
}

async function bootstrap() {
  if (!store.facets.categories?.length) await store.fetchFacets()
  await store.fetchList()
}
onMounted(bootstrap)
onActivated(() => {
  if (!store.loaded) bootstrap()
})

// ---------- 购物车操作（与详情页共享同一份 store，实时同步） ----------
function qtyOf(id) { return cart.qtyOf(id) }

function increase(r) {
  // 已在车里则 +1，否则以 1 份加入
  cart.add(r, 1, app.currentMember?.id)
}

function decrease(id) {
  cart.setQuantity(id, cart.qtyOf(id) - 1)
}

function goCheckout() {
  if (cart.isEmpty) { toast.warn('先挑几道菜吧'); return }
  router.push('/order/checkout')
}

function goDetail(r) {
  router.push(`/recipes/${r.id}`)
}
</script>

<template>
  <div class="page order">
    <!-- 搜索头 -->
    <div class="o-head">
      <div class="page-body">
        <div class="search-bar">
          <span class="search-icon">🔍</span>
          <input
            v-model="keyword"
            class="search-input"
            type="search"
            placeholder="搜菜名、食材或做法…"
            enterkeyhint="search"
          />
          <button v-if="keyword" class="search-clear" @click="keyword = ''" aria-label="清空">✕</button>
        </div>
      </div>

      <!-- 分类横滑 -->
      <div class="cat-rail">
        <button
          v-for="c in categoryChips"
          :key="c.name"
          class="cat-chip"
          :class="{ 'cat-chip--on': store.filters.category === c.name || (c.name === '全部' && !store.filters.category) }"
          @click="pickCategory(c.name)"
        >
          {{ c.name }}
          <span v-if="c.count" class="cat-count num">{{ c.count }}</span>
        </button>
      </div>
    </div>

    <div class="page-body">
      <!-- 菜品列表 -->
      <div v-if="store.loading && !store.list.length" class="order-list">
        <YanLoader v-for="i in 6" :key="i" class="order-sk" :size="80" :blobs="false" :show-tip="false" />
      </div>

      <div v-else-if="store.list.length" class="order-list">
        <article
          v-for="r in store.list"
          :key="r.id"
          class="dish-row card"
          :class="{ 'dish-row--in': qtyOf(r.id) > 0 }"
          @click="goDetail(r)"
        >
          <DishCover :emoji="r.cover_emoji" :gradient="r.cover_gradient" :url="r.cover_url" size="md" rounded="md" />
          <div class="dr-info">
            <p class="dr-name ellipsis">{{ r.name }}</p>
            <p class="dr-meta">
              <span v-if="r.category" class="dr-chip">{{ r.category }}</span>
              <span>⏱ {{ durationText(r.total_minutes) }}</span>
              <span>{{ difficultyLabel(r.difficulty) }}</span>
            </p>
            <p v-if="r.description" class="dr-desc ellipsis-2">{{ r.description }}</p>
          </div>

          <div class="dr-ctrl" @click.stop>
            <template v-if="qtyOf(r.id) > 0">
              <button class="q-btn" aria-label="减少" @click="decrease(r.id)">−</button>
              <span class="q-val num">{{ qtyOf(r.id) }}</span>
              <button class="q-btn q-btn--plus" aria-label="增加" @click="increase(r)">＋</button>
            </template>
            <button v-else class="q-add" @click="increase(r)">＋ 加入</button>
          </div>
        </article>
      </div>

      <EmptyState v-else emoji="🍽️" title="没找到合适的菜" desc="换个关键词，或去菜谱库新建">
        <button class="btn btn--primary btn--sm" @click="router.push('/recipes/new')">＋ 新建菜谱</button>
      </EmptyState>

      <div v-if="store.loading && store.list.length" class="loading-more">
        <YanLoader :size="72" :blobs="false" :backdrop="false" tip="正在加载更多" />
      </div>

      <div class="bottom-space" />
    </div>

    <!-- 底部固定结算条 -->
    <transition name="cartbar">
      <div v-if="!cart.isEmpty" class="checkout-bar">
        <div class="page-body checkout-inner">
          <div class="co-summary" @click="router.push('/order/checkout')">
            <span class="co-emoji">🛒</span>
            <span class="co-text">已选 <b class="num">{{ cart.count }}</b> 道 · <b class="num">{{ cart.totalPortions }}</b> 份</span>
          </div>
          <button class="co-btn" @click="goCheckout">去结算 ›</button>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.o-head {
  position: sticky;
  top: 0;
  z-index: 40;
  background: var(--bg);
  padding-top: calc(var(--safe-top) + 10px);
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border);
}
.search-bar {
  display: flex; align-items: center; gap: 8px;
  background: var(--surface);
  border: 1.5px solid var(--border);
  border-radius: var(--r-full);
  padding: 0 14px;
  height: 44px;
  transition: border-color var(--dur-fast), box-shadow var(--dur-fast);
}
.search-bar:focus-within { border-color: var(--brand-400); box-shadow: 0 0 0 3.5px var(--brand-100); }
.search-icon { font-size: 15px; opacity: .6; }
.search-input { flex: 1; border: none; outline: none; background: transparent; font-size: 15px; font-family: inherit; color: var(--text-1); min-width: 0; }
.search-input::-webkit-search-cancel-button { display: none; }
.search-clear { border: none; background: var(--bg-subtle); width: 22px; height: 22px; border-radius: 50%; color: var(--text-3); font-size: 12px; cursor: pointer; flex: none; }

.cat-rail {
  display: flex; gap: 8px; overflow-x: auto;
  padding: 12px var(--page-pad) 2px;
  scrollbar-width: none;
}
.cat-rail::-webkit-scrollbar { display: none; }
.cat-chip {
  display: inline-flex; align-items: center; gap: 5px;
  flex: none;
  padding: 7px 14px;
  border-radius: var(--r-full);
  border: 1.5px solid var(--border-strong);
  background: var(--surface);
  color: var(--text-2);
  font-size: 13px; font-weight: 600;
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease);
  -webkit-tap-highlight-color: transparent;
}
.cat-chip:active { transform: scale(.95); }
.cat-chip--on { background: var(--brand-gradient); border-color: transparent; color: #fff; box-shadow: var(--sh-brand); }
.cat-count { font-size: 11px; opacity: .7; }

/* ---------- 菜品行 ---------- */
.order-list { display: flex; flex-direction: column; gap: 10px; padding-top: 14px; }
.order-sk { height: 96px; }
.dish-row {
  display: flex; align-items: center; gap: 12px;
  padding: 12px;
  cursor: pointer;
  border: 1px solid var(--border);
  animation: dr-in var(--dur) var(--ease-out);
  -webkit-tap-highlight-color: transparent;
}
@keyframes dr-in { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: none; } }
.dish-row:active { transform: scale(.99); }
.dish-row--in { border-color: var(--brand-300); box-shadow: 0 0 0 2px var(--brand-100); }
.dish-row :deep(.cover) { flex: none; }

.dr-info { flex: 1; min-width: 0; }
.dr-name { font-size: 15.5px; font-weight: 700; color: var(--text-1); }
.dr-meta { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; margin-top: 4px; font-size: 12px; color: var(--text-2); }
.dr-chip { background: var(--bg-subtle); padding: 1px 7px; border-radius: var(--r-full); }
.dr-desc { margin-top: 5px; font-size: 12px; color: var(--text-3); line-height: 1.5; }

.dr-ctrl { flex: none; display: flex; align-items: center; gap: 2px; }
.q-btn {
  width: 30px; height: 30px; border: none; border-radius: 50%;
  background: var(--bg-subtle); color: var(--brand-600);
  font-size: 18px; font-weight: 600; line-height: 1; cursor: pointer;
  transition: transform var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
.q-btn:active { transform: scale(.86); }
.q-btn--plus { background: var(--brand-gradient); color: #fff; box-shadow: var(--sh-brand); }
.q-val { min-width: 28px; text-align: center; font-size: 15px; font-weight: 700; color: var(--text-1); }
.q-add {
  padding: 8px 14px; border: none; border-radius: var(--r-full);
  background: var(--brand-gradient); color: #fff;
  font-size: 13px; font-weight: 700; cursor: pointer;
  box-shadow: var(--sh-brand);
  transition: transform var(--dur-fast) var(--ease-bounce);
  -webkit-tap-highlight-color: transparent;
}
.q-add:active { transform: scale(.92); }

.loading-more { text-align: center; padding: 22px; font-size: 13px; color: var(--text-4); }
.loading-more :deep(.yan) { padding: 12px; }

.bottom-space { height: 96px; }

/* ---------- 底部结算条 ---------- */
.checkout-bar {
  position: fixed; left: 0; right: 0; bottom: calc(var(--tabbar-h) + var(--safe-bottom));
  z-index: 48;
  padding: 10px 0;
  background: rgba(255,253,247,.96);
  backdrop-filter: blur(16px);
  border-top: 1px solid var(--border);
}
.checkout-inner { display: flex; align-items: center; gap: 12px; max-width: 1180px; }
.co-summary { flex: 1; display: flex; align-items: center; gap: 8px; cursor: pointer; min-width: 0; }
.co-emoji { font-size: 20px; }
.co-text { font-size: 14px; color: var(--text-2); }
.co-text b { color: var(--brand-600); font-size: 16px; font-weight: 800; }
.co-btn {
  flex: none; padding: 12px 24px; border: none; border-radius: var(--r-full);
  background: var(--brand-gradient); color: #fff;
  font-size: 15px; font-weight: 700; cursor: pointer;
  box-shadow: var(--sh-brand);
  transition: transform var(--dur-fast) var(--ease-bounce);
  -webkit-tap-highlight-color: transparent;
}
.co-btn:active { transform: scale(.95); }

.cartbar-enter-active, .cartbar-leave-active { transition: all var(--dur) var(--ease-bounce); }
.cartbar-enter-from, .cartbar-leave-to { opacity: 0; transform: translateY(20px); }
</style>
