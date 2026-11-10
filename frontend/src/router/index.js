import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

/**
 * 路由表
 *
 * 底部 4 个主标签用 keep-alive 保留滚动位置；详情页独立。
 * 移动端优先，平板复用同一套路由（靠 CSS 自适应）。
 */
const routes = [
  {
    path: '/',
    name: 'home',
    component: () => import('@/views/HomeView.vue'),
    meta: { title: '今天吃什么', tab: 'home', keepAlive: true }
  },
  {
    path: '/recipes',
    name: 'recipes',
    component: () => import('@/views/RecipeListView.vue'),
    meta: { title: '菜谱库', tab: 'recipes', keepAlive: true }
  },
  {
    path: '/recipes/:id',
    name: 'recipe-detail',
    component: () => import('@/views/RecipeDetailView.vue'),
    meta: { title: '菜谱详情', back: true }
  },
  {
    path: '/recipes/new',
    name: 'recipe-new',
    component: () => import('@/views/RecipeEditView.vue'),
    meta: { title: '新建菜谱', back: true }
  },
  {
    path: '/recipes/:id/edit',
    name: 'recipe-edit',
    component: () => import('@/views/RecipeEditView.vue'),
    meta: { title: '编辑菜谱', back: true }
  },
  {
    path: '/order',
    name: 'order',
    component: () => import('@/views/OrderView.vue'),
    meta: { title: '点餐', tab: 'order', keepAlive: true }
  },
  {
    path: '/orders',
    name: 'orders',
    component: () => import('@/views/OrderListView.vue'),
    meta: { title: '订单', tab: 'orders', keepAlive: true }
  },
  {
    path: '/orders/:id',
    name: 'order-detail',
    component: () => import('@/views/OrderDetailView.vue'),
    meta: { title: '订单详情', back: true }
  },
  {
    path: '/orders/:id/recipes',
    name: 'order-recipes',
    component: () => import('@/views/OrderRecipesView.vue'),
    meta: { title: '这一餐怎么做', back: true }
  },
  {
    path: '/stats',
    name: 'stats',
    component: () => import('@/views/StatsView.vue'),
    meta: { title: '家的味道', back: true, keepAlive: true }
  },
  {
    path: '/tools',
    name: 'tools',
    component: () => import('@/views/ToolsView.vue'),
    meta: { title: '常用功能', tab: 'tools', keepAlive: true }
  },
  {
    path: '/tools/ledger',
    name: 'ledger',
    component: () => import('@/views/LedgerView.vue'),
    meta: { title: '家庭账本', back: true }
  },
  {
    path: '/tools/health',
    name: 'health',
    component: () => import('@/views/HealthView.vue'),
    meta: { title: '经期提醒', back: true }
  },
  {
    path: '/tools/parcel',
    name: 'parcel',
    component: () => import('@/views/ParcelView.vue'),
    meta: { title: '快递取件码', back: true }
  },
  {
    path: '/tools/reminders',
    name: 'reminders',
    component: () => import('@/views/ReminderView.vue'),
    meta: { title: '提醒中心', back: true }
  },
  {
    path: '/tools/consumables',
    name: 'consumables',
    component: () => import('@/views/ConsumableView.vue'),
    meta: { title: '耗材管理', back: true }
  },
  {
    path: '/tools/study',
    name: 'study',
    component: () => import('@/views/StudyView.vue'),
    meta: { title: '学习', back: true }
  },
  {
    path: '/tools/study/quiz',
    name: 'study-quiz',
    component: () => import('@/views/StudyQuizView.vue'),
    meta: { title: '刷题', back: true }
  },
  {
    path: '/tools/study/english',
    name: 'study-english',
    component: () => import('@/views/StudyEnglishView.vue'),
    meta: { title: '英语学习', back: true }
  },
  {
    path: '/tools/study/notes',
    name: 'study-notes',
    component: () => import('@/views/StudyNotesView.vue'),
    meta: { title: '知识点', back: true }
  },
  {
    path: '/profile',
    name: 'profile',
    component: () => import('@/views/ProfileView.vue'),
    meta: { title: '个人中心', back: true }
  },
  {
    path: '/profile/version',
    name: 'version',
    component: () => import('@/views/VersionView.vue'),
    meta: { title: '系统版本', back: true }
  },
  {
    path: '/members',
    name: 'members',
    component: () => import('@/views/MemberView.vue'),
    meta: { title: '家庭成员', back: true }
  },
  {
    path: '/login',
    name: 'login',
    component: () => import('@/views/LoginView.vue'),
    meta: { title: '登录', public: true }
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'not-found',
    component: () => import('@/views/NotFoundView.vue'),
    meta: { title: '页面走丢了' }
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes,
  scrollBehavior(to, from, saved) {
    if (saved) return saved
    return { top: 0 }
  }
})

/**
 * 登录守卫：除 /login 外，未登录一律带去登录页；
 * 已登录再访问 /login 则直接回首页。登录成功后按 redirect 回跳原目标页。
 */
router.beforeEach((to) => {
  const auth = useAuthStore()
  if (to.name === 'login') {
    return auth.loggedIn ? { name: 'home' } : true
  }
  if (!auth.loggedIn) {
    return { name: 'login', query: to.fullPath === '/' ? {} : { redirect: to.fullPath } }
  }
  return true
})

router.afterEach((to) => {
  if (to.meta?.title) {
    document.title = `${to.meta.title} · 孙府`
  }
})

export default router
