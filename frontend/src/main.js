import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { useAuthStore } from './stores/auth'

import './styles/variables.css'
import './styles/global.css'
import './components/ToastHost.css'

const app = createApp(App)

// 全局错误兜底，避免白屏无提示
app.config.errorHandler = (err) => {
  console.error('[家味] 未捕获错误：', err)
}

const pinia = createPinia()
app.use(pinia)
// 路由守卫前先恢复登录态（记住我 / 本次会话）
useAuthStore().hydrate()
app.use(router)
app.mount('#app')
