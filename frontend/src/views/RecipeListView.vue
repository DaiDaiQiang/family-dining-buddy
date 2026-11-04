<script setup>
/**
 * 菜谱库
 *
 * 顶部搜索 + 分类横滑筛选 + 排序，主区网格瀑布。
 * 移动端 2 列，平板 3~4 列。滚动到底自动加载更多。
 */
import { computed, onActivated, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useRecipeStore } from '@/stores/recipe'
import { useCartStore } from '@/stores/cart'
import { useAppStore } from '@/stores/app'
import DishCard from '@/components/DishCard.vue'
import EmptyState from '@/components/EmptyState.vue'
import YanLoader from '@/components/YanLoader.vue'
import { toast } from '@/composables/useToast'

defineOptions({ name: 'RecipeListView' })

const router = useRouter()
const store = useRecipeStore()
const cart = useCartStore()
const app = useAppStore()

const keyword = ref(store.filters.keyword)
const sortOpen = ref(false)
const listRef = ref(null)

const sorts = [
  { value: 'rating', label: '评分最高', icon: '⭐' },
  { value: 'popular', label: '最常点', icon: '🔥' },
  { value: 'fastest', label: '最快做', icon: '⚡' },
  { value: 'newest', label: '最新加', icon: '🆕' },
  { value: 'name', label: '按名称', icon: '🔤' }
]
const currentSort = computed(() => sorts.find((s) => s.value === store.filters.sort) || sorts[0])

// 分类胶囊：全部 + facets
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
function pickSort(value) {
  store.setFilter({ sort: value })
  sortOpen.value = false
  store.fetchList()
}
function resetAll() {
  keyword.value = ''
  store.resetFilters()
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

function addToCart(recipe) {
  if (cart.has(recipe.id)) {
    toast.info(`「${recipe.name}」已在点餐单里`)
    return
  }
  cart.add(recipe, 1, app.currentMember?.id)
  toast.success(`已加入「${recipe.name}」`)
}

// 无限滚动
function onScroll() {
  const el = listRef.value
  if (!el) return
  if (el.scrollHeight - el.scrollTop - el.clientHeight < 300) {
    store.loadMore()
  }
}
onMounted(() => {
  window.addEventListener('scroll', onScroll, { passive: true })
})
</script>

<template>
  <div class="page recipes">
    <!-- 搜索头 -->
    <div class="r-head">
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
      <!-- 工具行 -->
      <div class="toolbar">
        <span class="toolbar-count num">
          共 <b>{{ store.total }}</b> 道菜
          <template v-if="store.hasActiveFilter">· 已筛选</template>
        </span>
        <div class="toolbar-actions">
          <button v-if="store.hasActiveFilter" class="tool-btn" @click="resetAll">重置</button>
          <button class="tool-btn" @click="sortOpen = !sortOpen">
            {{ currentSort.icon }} {{ currentSort.label }}
            <span class="tool-caret" :class="{ up: sortOpen }">▾</span>
          </button>
        </div>
      </div>

      <!-- 排序下拉 -->
      <transition name="drop">
        <div v-if="sortOpen" class="sort-menu">
          <button
            v-for="s in sorts"
            :key="s.value"
            class="sort-opt"
            :class="{ on: store.filters.sort === s.value }"
            @click="pickSort(s.value)"
          >
            <span>{{ s.icon }} {{ s.label }}</span>
            <span v-if="store.filters.sort === s.value" class="sort-check">✓</span>
          </button>
        </div>
      </transition>

      <!-- 网格 -->
      <div v-if="store.loading && !store.list.length" class="grid">
        <YanLoader v-for="i in 6" :key="i" class="grid-sk" :size="120" :blobs="false" :show-tip="false" />
      </div>

      <div v-else-if="store.list.length" class="grid stagger">
        <DishCard
          v-for="r in store.list"
          :key="r.id"
          :recipe="r"
          layout="grid"
          :in-cart="cart.has(r.id)"
          @click="(x) => router.push(`/recipes/${x.id}`)"
          @add="addToCart"
        />
      </div>

      <EmptyState v-else emoji="🍽️" title="没找到合适的菜" desc="换个关键词，或新建一道菜谱">
        <button class="btn btn--primary btn--sm" @click="router.push('/recipes/new')">＋ 新建菜谱</button>
      </EmptyState>

      <div v-if="store.loading && store.list.length" class="loading-more">
        <YanLoader :size="72" :blobs="false" :backdrop="false" tip="正在加载更多" />
      </div>
      <p v-else-if="store.list.length && store.list.length >= store.total" class="no-more">— 已经到底啦 —</p>
    </div>

    <!-- 悬浮新建按钮 -->
    <button class="fab" @click="router.push('/recipes/new')" aria-label="新建菜谱">
      <span>＋</span>
    </button>
  </div>
</template>

<style scoped>
.r-head {
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
.cat-chip--on {
  background: var(--brand-gradient);
  border-color: transparent;
  color: #fff;
  box-shadow: var(--sh-brand);
}
.cat-count { font-size: 11px; opacity: .7; }

.toolbar { display: flex; align-items: center; justify-content: space-between; padding: 14px 0 10px; }
.toolbar-count { font-size: 13px; color: var(--text-3); }
.toolbar-count b { color: var(--brand-600); font-size: 15px; }
.toolbar-actions { display: flex; gap: 8px; }
.tool-btn {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 5px 12px; border-radius: var(--r-full);
  border: 1.5px solid var(--border-strong); background: var(--surface);
  color: var(--text-2); font-size: 12.5px; font-weight: 600; cursor: pointer;
}
.tool-caret { font-size: 10px; transition: transform var(--dur-fast); }
.tool-caret.up { transform: rotate(180deg); }

.sort-menu {
  background: var(--surface); border-radius: var(--r-md);
  box-shadow: var(--sh-lg); border: 1px solid var(--border);
  padding: 6px; margin-bottom: 10px; overflow: hidden;
}
.sort-opt {
  display: flex; align-items: center; justify-content: space-between;
  width: 100%; padding: 11px 12px; border: none; background: transparent;
  border-radius: var(--r-sm); font-size: 14px; color: var(--text-1); cursor: pointer;
  font-family: inherit;
}
.sort-opt:active { background: var(--bg-subtle); }
.sort-opt.on { color: var(--brand-600); font-weight: 700; }
.sort-check { color: var(--brand-600); }

.drop-enter-active, .drop-leave-active { transition: all var(--dur) var(--ease-out); }
.drop-enter-from, .drop-leave-to { opacity: 0; transform: translateY(-8px); }

.grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; }
.grid-sk { height: 230px; }

@media (min-width: 560px) { .grid { grid-template-columns: repeat(3, 1fr); } }
@media (min-width: 900px) { .grid { grid-template-columns: repeat(4, 1fr); gap: 14px; } }

.loading-more, .no-more { text-align: center; padding: 22px; font-size: 13px; color: var(--text-4); }
.loading-more :deep(.yan) { padding: 12px; }

.fab {
  position: fixed;
  right: 20px;
  bottom: calc(var(--tabbar-h) + var(--safe-bottom) + 20px);
  width: 56px; height: 56px;
  border-radius: 50%;
  border: none;
  background: var(--brand-gradient);
  color: #fff; font-size: 28px; line-height: 1;
  box-shadow: var(--sh-brand);
  cursor: pointer;
  z-index: 45;
  display: flex; align-items: center; justify-content: center;
  transition: transform var(--dur-fast) var(--ease-bounce);
}
.fab:active { transform: scale(.9) rotate(90deg); }
</style>
