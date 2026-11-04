<script setup>
/**
 * 详情页顶栏：返回 + 标题 + 右侧插槽
 */
import { useRouter } from 'vue-router'

defineProps({
  title: { type: String, default: '' }
})

const router = useRouter()

function back() {
  if (window.history.state?.back) router.back()
  else router.replace('/')
}
</script>

<template>
  <header class="topbar">
    <div class="topbar-inner">
      <button class="topbar-back" aria-label="返回" @click="back">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M15 19l-7-7 7-7" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      <h1 class="topbar-title ellipsis">{{ title }}</h1>
      <div class="topbar-actions">
        <slot name="actions" />
      </div>
    </div>
  </header>
</template>

<style scoped>
.topbar {
  position: fixed;
  top: 0; left: 0; right: 0;
  z-index: 55;
  padding-top: var(--safe-top);
  background: rgba(255, 253, 247, .92);
  backdrop-filter: saturate(180%) blur(16px);
  -webkit-backdrop-filter: saturate(180%) blur(16px);
  border-bottom: 1px solid rgba(227, 207, 168, .85);
}
.topbar-inner {
  max-width: 1180px;
  margin: 0 auto;
  height: var(--navbar-h);
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 var(--page-pad);
}
.topbar-back {
  width: 38px; height: 38px;
  flex: none;
  display: flex; align-items: center; justify-content: center;
  border: none; background: transparent;
  color: var(--text-1);
  border-radius: var(--r-full);
  cursor: pointer;
  transition: background var(--dur-fast), transform var(--dur-fast);
  margin-left: -8px;
}
.topbar-back:active { background: var(--bg-subtle); transform: scale(.92); }
.topbar-title {
  flex: 1;
  font-size: 17px;
  font-weight: 700;
  font-family: var(--font-display);
  text-align: center;
  color: var(--text-1);
}
.topbar-actions {
  flex: none;
  min-width: 38px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 6px;
}
</style>
