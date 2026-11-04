<script setup>
/**
 * 底部标签栏
 *
 * 移动端固定在底部（避开手势安全区），平板居中限宽。
 * 购物车角标实时反映待下单数量。
 */
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useCartStore } from '@/stores/cart'
import { useHealthStore } from '@/stores/health'

const route = useRoute()
const cart = useCartStore()
const health = useHealthStore()

const tabs = [
  { key: 'home', label: '今天', icon: '🏠', to: '/' },
  { key: 'recipes', label: '菜谱', icon: '📖', to: '/recipes' },
  { key: 'order', label: '点餐', icon: '🍽️', to: '/order' },
  { key: 'orders', label: '订单', icon: '📋', to: '/orders' },
  { key: 'tools', label: '常用', icon: '🧰', to: '/tools' }
]

const active = computed(() => route.meta?.tab)
const cartCount = computed(() => cart.count)
/** 常用功能角标：经期提醒条数 */
const toolsCount = computed(() => health.alerts.length)
</script>

<template>
  <nav class="tabbar" role="navigation" aria-label="主导航">
    <div class="tabbar-inner">
      <router-link
        v-for="t in tabs"
        :key="t.key"
        :to="t.to"
        class="tab"
        :class="{ 'tab--active': active === t.key }"
        :aria-current="active === t.key ? 'page' : undefined"
      >
        <span class="tab-icon-wrap">
          <span class="tab-icon">{{ t.icon }}</span>
          <span v-if="t.key === 'order' && cartCount > 0" class="tab-badge num">{{ cartCount > 9 ? '9+' : cartCount }}</span>
          <span v-else-if="t.key === 'tools' && toolsCount > 0" class="tab-badge num">{{ toolsCount > 9 ? '9+' : toolsCount }}</span>
        </span>
        <span class="tab-label">{{ t.label }}</span>
      </router-link>
    </div>
  </nav>
</template>

<style scoped>
.tabbar {
  position: fixed;
  left: 0; right: 0; bottom: 0;
  z-index: 50;
  padding-bottom: var(--safe-bottom);
  background: rgba(255, 253, 247, .88);
  backdrop-filter: saturate(180%) blur(18px);
  -webkit-backdrop-filter: saturate(180%) blur(18px);
  border-top: 1px solid rgba(227, 207, 168, .9);
}

.tabbar-inner {
  max-width: 620px;
  margin: 0 auto;
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  height: var(--tabbar-h);
}

.tab {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 3px;
  color: var(--text-3);
  text-decoration: none;
  transition: color var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}

.tab-icon-wrap { position: relative; display: flex; }
.tab-icon {
  font-size: 21px;
  line-height: 1;
  filter: grayscale(.5) opacity(.72);
  transition: filter var(--dur), transform var(--dur) var(--ease-bounce);
}
.tab-label { font-size: 11px; font-weight: 500; letter-spacing: .02em; }

.tab--active { color: var(--brand-600); }
.tab--active .tab-icon {
  filter: none;
  transform: translateY(-1px) scale(1.12);
}
.tab--active .tab-label { font-weight: 700; }
.tab:active .tab-icon { transform: scale(.9); }

.tab-badge {
  position: absolute;
  top: -5px; right: -9px;
  min-width: 16px; height: 16px;
  padding: 0 4px;
  border-radius: var(--r-full);
  background: var(--danger);
  color: #fff;
  font-size: 10px; font-weight: 700;
  line-height: 16px; text-align: center;
  box-shadow: 0 2px 6px rgba(229,72,77,.4);
  animation: badge-pop var(--dur) var(--ease-bounce);
}
@keyframes badge-pop {
  from { transform: scale(0); }
  to { transform: scale(1); }
}

/* 窄屏手机：6 个标签时适当收紧，避免文字挤压 */
@media (max-width: 420px) {
  .tab-icon { font-size: 19px; }
  .tab-label { font-size: 10px; letter-spacing: 0; }
  .tab { gap: 2px; }
}

/* 平板：整体加高一点更好点 */
@media (min-width: 700px) {
  .tab-icon { font-size: 23px; }
  .tab-label { font-size: 12px; }
}
</style>
