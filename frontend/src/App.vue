<script setup>
/**
 * 应用根组件
 *
 * 负责：初始化 store、根据路由决定是否显示底部导航、页面切换动画、
 * 后端不可达时的全屏阻断页（本前端不允许离线使用）、全局 Toast 容器。
 */
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { ToastHost } from '@/composables/useToast'
import TabBar from '@/components/TabBar.vue'
import TopBar from '@/components/TopBar.vue'

const route = useRoute()
const app = useAppStore()

const booting = ref(true)
const retrying = ref(false)
const showTabBar = computed(() => !!route.meta?.tab)
const isBackPage = computed(() => !!route.meta?.back)
const blocked = computed(() => app.backendDown && !booting)

onMounted(async () => {
  try {
    await app.init()
  } finally {
    // 留一点点时间让启动动画自然收尾
    setTimeout(() => { booting.value = false }, 260)
  }
})

/** 阻断页「重新连接」：强制重新探测后端并重拉数据 */
async function retry() {
  retrying.value = true
  try {
    await app.init(true)
  } finally {
    retrying.value = false
  }
}
</script>

<template>
  <div class="app-shell">
    <!-- 后端不可达：全屏阻断。本前端已禁用离线演示模式，不展示任何示例数据 -->
    <div v-if="blocked" class="offline-block" role="alert">
      <div class="ob-card">
        <div class="ob-emoji">📡</div>
        <h2 class="ob-title">无法连接后端服务</h2>
        <p class="ob-reason">{{ app.offlineReason || '请检查后端是否已启动' }}</p>
        <p class="ob-tip">本前端不允许离线使用，所有数据均实时来自后端数据库。</p>
        <button class="ob-retry" :disabled="retrying" @click="retry">
          {{ retrying ? '连接中…' : '重新连接' }}
        </button>
      </div>
    </div>

    <template v-else>
      <TopBar v-if="isBackPage" :title="route.meta.title" />

      <main class="app-main" :class="{ 'app-main--tabbed': showTabBar, 'app-main--backed': isBackPage }">
        <router-view v-slot="{ Component, route: r }">
          <transition name="fade-slide" mode="out-in">
            <keep-alive :include="['HomeView', 'RecipeListView', 'OrderListView', 'StatsView', 'ToolsView']">
              <component :is="Component" :key="r.fullPath" />
            </keep-alive>
          </transition>
        </router-view>
      </main>

      <TabBar v-if="showTabBar" />
    </template>

    <ToastHost />
  </div>
</template>

<style scoped>
.app-shell {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg);
}

.app-main {
  flex: 1;
  min-height: 0;
}
.app-main--tabbed { padding-bottom: 0; }
.app-main--backed { padding-top: var(--navbar-h); }

/* 后端不可达阻断页 */
.offline-block {
  position: fixed;
  inset: 0;
  z-index: 90;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: var(--bg);
}
.ob-card {
  width: 100%;
  max-width: 360px;
  text-align: center;
  background: var(--surface, #fff);
  border: 1px solid var(--border, #eee);
  border-radius: 16px;
  padding: 36px 28px 28px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, .06);
}
.ob-emoji { font-size: 44px; line-height: 1; }
.ob-title {
  margin: 14px 0 8px;
  font-size: 18px;
  color: var(--text, #333);
}
.ob-reason {
  margin: 0 0 10px;
  font-size: 13px;
  color: var(--danger, #d64545);
  word-break: break-all;
}
.ob-tip {
  margin: 0 0 20px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-2, #888);
}
.ob-retry {
  appearance: none;
  border: none;
  border-radius: 999px;
  padding: 10px 28px;
  font-size: 14px;
  color: #fff;
  background: var(--primary, #C2703D);
  cursor: pointer;
  transition: opacity .15s ease;
}
.ob-retry:disabled { opacity: .6; cursor: default; }
.ob-retry:not(:disabled):hover { opacity: .88; }

/* 平板以上：内容居中限宽，两侧留白 */
@media (min-width: 768px) {
  .app-main { max-width: 1180px; width: 100%; margin: 0 auto; }
}
</style>
