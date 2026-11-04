<script setup>
/**
 * 登录页
 *
 * 家庭场景的轻量登录：选身份（成员卡片）→ 输密码 → 进家门。
 * 「记住登录状态」勾选走 localStorage，不勾选只保留到关闭标签页。
 * 未登录访问任何页面都会被路由守卫带到这里，登录后回跳原目标页。
 */
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useAuthStore } from '@/stores/auth'
import { api } from '@/api'
import { toast } from '@/composables/useToast'

const app = useAppStore()
const auth = useAuthStore()
const router = useRouter()
const route = useRoute()

/** 登录页成员卡片：走公开接口 /api/auth/members（登录前 /api/members 需要令牌，取不到） */
const loginMembers = ref([])
const members = computed(() => loginMembers.value)
const selectedMember = computed(
  () => members.value.find((m) => m.id === selectedId.value) || null
)
const pwdPlaceholder = computed(() =>
  selectedMember.value?.has_password ? '请输入密码' : '未设密码，直接进家门'
)

const selectedId = ref(null)
const password = ref('')
const showPwd = ref(false)
const remember = ref(true)
const error = ref('')
const shaking = ref(false)
const loading = ref(false)

/** 背景漂浮的食材 emoji，纯氛围 */
const floatEmojis = [
  { char: '🥘', style: 'top:12%;left:8%;animation-delay:0s;font-size:34px' },
  { char: '🍚', style: 'top:22%;right:10%;animation-delay:1.2s;font-size:28px' },
  { char: '🥟', style: 'bottom:26%;left:12%;animation-delay:2.1s;font-size:30px' },
  { char: '🌶️', style: 'bottom:14%;right:14%;animation-delay:.6s;font-size:26px' },
  { char: '🧄', style: 'top:46%;left:4%;animation-delay:1.8s;font-size:22px' },
  { char: '🍜', style: 'top:8%;right:26%;animation-delay:2.6s;font-size:24px' }
]

onMounted(async () => {
  // 公开接口：登录页无需令牌；字典 /api/meta 也是公开的
  try {
    loginMembers.value = (await api.auth.members()) || []
  } catch (e) {
    console.warn('加载登录成员失败', e)
    loginMembers.value = []
  }
  if (!app.meta) await app.loadMeta()
  if (members.value.length && !selectedId.value) {
    selectedId.value = members.value[0].id
  }
})

function select(m) {
  selectedId.value = m.id
  error.value = ''
}

function fail(msg) {
  error.value = msg
  shaking.value = true
  setTimeout(() => { shaking.value = false }, 500)
}

async function submit() {
  error.value = ''
  if (!selectedId.value) { fail('先选一位家庭成员'); return }
  loading.value = true
  // 轻微延迟，让按钮的验证态可见，也避免误触连点
  await new Promise((r) => setTimeout(r, 320))
  const res = await auth.login(selectedId.value, password.value, remember.value)
  loading.value = false
  if (!res.ok) {
    fail(res.error || '密码不对，再想想～')
    password.value = ''
    return
  }
  // 登录前启动 init 的成员/四模块请求都 401 落空了，拿上令牌强制重拉
  await app.init(true)
  const m = members.value.find((x) => x.id === selectedId.value)
  toast.success(`欢迎回家，${m?.nickname || '家人'}`)
  const redirect = route.query.redirect
  const target = typeof redirect === 'string' && redirect.startsWith('/') && !redirect.startsWith('//')
    ? redirect
    : '/'
  router.replace(target)
}
</script>

<template>
  <div class="login">
    <!-- 背景氛围层 -->
    <div class="login-glow" aria-hidden="true" />
    <div class="login-emojis" aria-hidden="true">
      <span v-for="(e, i) in floatEmojis" :key="i" class="fe" :style="e.style">{{ e.char }}</span>
    </div>

    <div class="login-card">
      <!-- 品牌头 -->
      <header class="login-head">
        <div class="login-logo">
          <img src="/logo.png" alt="孙府" class="login-logo-img" />
        </div>
        <h1 class="login-title">孙府</h1>
        <p class="login-slogan font-kai">一屋五人，三餐四季</p>
      </header>

      <form class="login-form" @submit.prevent="submit">
        <!-- 选身份 -->
        <div class="login-block">
          <p class="login-label">选择你的身份</p>
          <div class="role-grid">
            <button
              v-for="m in members"
              :key="m.id"
              type="button"
              class="role-card"
              :class="{ on: selectedId === m.id }"
              :style="{ '--mc': m.theme_color }"
              @click="select(m)"
            >
              <span class="role-avatar">
                <img v-if="m.avatar_url" :src="m.avatar_url" alt="" class="role-avatar-img" />
                <template v-else>{{ m.avatar_emoji }}</template>
              </span>
              <span class="role-name ellipsis">{{ m.nickname }}</span>
              <span v-if="selectedId === m.id" class="role-check">✓</span>
            </button>
          </div>
          <p v-if="!members.length" class="login-empty">
            还没有家庭成员，先去「家庭成员」页添加一位吧
          </p>
        </div>

        <!-- 密码 -->
        <div class="login-block">
          <p class="login-label">登录密码</p>
          <div class="pwd-wrap" :class="{ shake: shaking, 'pwd-wrap--err': !!error }">
            <input
              :type="showPwd ? 'text' : 'password'"
              v-model="password"
              class="pwd-input"
              :placeholder="pwdPlaceholder"
              autocomplete="current-password"
              @input="error = ''"
            />
            <button type="button" class="pwd-eye" :aria-label="showPwd ? '隐藏密码' : '显示密码'" @click="showPwd = !showPwd">
              {{ showPwd ? '🙈' : '👁️' }}
            </button>
          </div>
          <transition name="err-fade">
            <p v-if="error" class="login-error">⚠ {{ error }}</p>
          </transition>
        </div>

        <!-- 记住登录状态 -->
        <label class="remember">
          <input v-model="remember" type="checkbox" class="remember-native" />
          <span class="remember-box"><span class="remember-tick">✓</span></span>
          <span class="remember-text">记住登录状态</span>
          <span class="remember-hint">不勾选，关闭页面后需重新登录</span>
        </label>

        <button type="submit" class="login-btn" :disabled="loading || !members.length">
          <span v-if="!loading">进家门 →</span>
          <span v-else class="login-btn-loading"><i class="dot" /><i class="dot" /><i class="dot" /></span>
        </button>
      </form>

      <footer class="login-foot">
        未设密码的家人可直接进家门，已设密码的请输入自己的密码
      </footer>
    </div>
  </div>
</template>

<style scoped>
.login {
  position: relative;
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px var(--page-pad) calc(24px + var(--safe-bottom));
  overflow: hidden;
}

/* 顶部暖光，和全局背景呼应 */
.login-glow {
  position: absolute;
  inset: 0;
  background: radial-gradient(90% 55% at 50% -10%, #FFF9EC 0%, rgba(255, 249, 236, 0) 60%);
  pointer-events: none;
}

/* 漂浮食材 */
.login-emojis { position: absolute; inset: 0; pointer-events: none; }
.fe {
  position: absolute;
  opacity: .12;
  filter: blur(.4px);
  animation: fe-drift 9s ease-in-out infinite;
}
@keyframes fe-drift {
  0%, 100% { transform: translateY(0) rotate(-6deg); }
  50% { transform: translateY(-16px) rotate(8deg); }
}

/* 登录卡片 */
.login-card {
  position: relative;
  z-index: 1;
  width: 100%;
  max-width: 460px;
  background: var(--surface);
  border-radius: var(--r-2xl);
  border: 1px solid var(--border);
  box-shadow: var(--sh-lg);
  padding: 34px 26px 24px;
  animation: card-in var(--dur-slow) var(--ease-out);
}
@keyframes card-in {
  from { opacity: 0; transform: translateY(18px) scale(.98); }
  to { opacity: 1; transform: none; }
}

/* 品牌头 */
.login-head { text-align: center; margin-bottom: 26px; }
.login-logo {
  width: 96px; height: 96px;
  margin: 0 auto 10px;
  display: flex; align-items: center; justify-content: center;
}
.login-logo-img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  display: block;
  filter: drop-shadow(0 6px 14px rgba(194, 112, 61, .18));
}
.login-title {
  font-family: var(--font-display);
  font-size: 30px;
  font-weight: 800;
  letter-spacing: .12em;
  color: var(--brand-700);
}
.login-slogan {
  margin-top: 6px;
  font-size: 13.5px;
  color: var(--text-3);
  letter-spacing: .18em;
}

/* 表单分块 */
.login-block { margin-bottom: 20px; }
.login-label {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-2);
  margin-bottom: 10px;
}

/* 身份卡片 */
.role-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}
@media (min-width: 420px) {
  .role-grid { grid-template-columns: repeat(4, 1fr); }
}
.role-card {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 5px;
  padding: 13px 6px 11px;
  border: 2px solid var(--border);
  border-radius: var(--r-md);
  background: var(--surface-2);
  cursor: pointer;
  transition: transform var(--dur-fast) var(--ease), border-color var(--dur-fast),
              box-shadow var(--dur-fast), background var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
  font-family: inherit;
}
.role-card:active { transform: scale(.95); }
.role-card.on {
  border-color: var(--mc, var(--brand-500));
  background: var(--surface);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--mc, #C2703D) 16%, transparent);
}
.role-avatar {
  width: 46px; height: 46px;
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 25px;
  background: color-mix(in srgb, var(--mc, #C2703D) 15%, white);
  overflow: hidden;
}
.role-avatar-img { width: 100%; height: 100%; object-fit: cover; display: block; }
.role-name { font-size: 13px; font-weight: 700; color: var(--text-1); max-width: 100%; }
.role-check {
  position: absolute;
  top: -7px; right: -7px;
  width: 20px; height: 20px;
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 11px; font-weight: 800;
  color: #fff;
  background: var(--mc, var(--brand-500));
  border: 2px solid var(--surface);
}
.login-empty { font-size: 12.5px; color: var(--text-3); line-height: 1.6; margin-top: 4px; }

/* 密码框 */
.pwd-wrap {
  display: flex;
  align-items: center;
  border: 1.5px solid var(--border);
  border-radius: var(--r-sm);
  background: var(--surface-2);
  transition: border-color var(--dur-fast), box-shadow var(--dur-fast), background var(--dur-fast);
}
.pwd-wrap:focus-within {
  border-color: var(--brand-400);
  background: var(--surface);
  box-shadow: 0 0 0 3.5px var(--brand-100);
}
.pwd-wrap--err { border-color: var(--danger); }
.pwd-input {
  flex: 1;
  min-width: 0;
  padding: 13px 4px 13px 14px;
  font-size: 16px;
  font-family: inherit;
  color: var(--text-1);
  background: transparent;
  border: none;
  outline: none;
  letter-spacing: .06em;
}
.pwd-input::placeholder { color: var(--text-4); letter-spacing: normal; }
.pwd-eye {
  flex: none;
  width: 46px;
  height: 46px;
  border: none;
  background: transparent;
  font-size: 17px;
  cursor: pointer;
  border-radius: var(--r-sm);
  -webkit-tap-highlight-color: transparent;
}
.pwd-eye:active { background: var(--bg-subtle); }

/* 错误提示 + 抖动 */
.login-error {
  margin-top: 8px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--danger);
}
.err-fade-enter-active, .err-fade-leave-active { transition: opacity var(--dur-fast); }
.err-fade-enter-from, .err-fade-leave-to { opacity: 0; }
.shake { animation: shake .45s var(--ease); }
@keyframes shake {
  0%, 100% { transform: translateX(0); }
  20% { transform: translateX(-7px); }
  40% { transform: translateX(6px); }
  60% { transform: translateX(-4px); }
  80% { transform: translateX(3px); }
}

/* 记住登录状态 */
.remember {
  display: flex;
  align-items: center;
  gap: 9px;
  margin: 4px 0 22px;
  cursor: pointer;
  user-select: none;
  -webkit-tap-highlight-color: transparent;
}
.remember-native { position: absolute; opacity: 0; width: 0; height: 0; }
.remember-box {
  flex: none;
  width: 21px; height: 21px;
  border-radius: 6px;
  border: 1.5px solid var(--border-strong);
  background: var(--surface-2);
  display: flex; align-items: center; justify-content: center;
  transition: all var(--dur-fast) var(--ease);
}
.remember-tick {
  font-size: 12px; font-weight: 800;
  color: #fff;
  opacity: 0; transform: scale(.4);
  transition: all var(--dur-fast) var(--ease-bounce);
}
.remember-native:checked + .remember-box {
  background: var(--brand-gradient);
  border-color: transparent;
  box-shadow: var(--sh-brand);
}
.remember-native:checked + .remember-box .remember-tick { opacity: 1; transform: none; }
.remember-native:focus-visible + .remember-box { box-shadow: 0 0 0 3.5px var(--brand-100); }
.remember-text { font-size: 14px; font-weight: 600; color: var(--text-1); }
.remember-hint { font-size: 11px; color: var(--text-4); margin-left: auto; }

/* 登录按钮 */
.login-btn {
  width: 100%;
  min-height: 52px;
  border: none;
  border-radius: var(--r-full);
  background: var(--brand-gradient);
  color: #fff;
  font-size: 16.5px;
  font-weight: 700;
  font-family: var(--font-display);
  letter-spacing: .1em;
  cursor: pointer;
  box-shadow: var(--sh-brand);
  transition: transform var(--dur-fast) var(--ease), box-shadow var(--dur-fast), opacity var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
.login-btn:active { transform: scale(.97); box-shadow: 0 4px 14px rgba(194, 112, 61, .34); }
.login-btn:disabled { opacity: .5; pointer-events: none; }
.login-btn-loading { display: inline-flex; align-items: center; gap: 6px; }
.login-btn-loading .dot {
  width: 6px; height: 6px;
  border-radius: 50%;
  background: #fff;
  animation: btn-dot 1s ease-in-out infinite;
}
.login-btn-loading .dot:nth-child(2) { animation-delay: .15s; }
.login-btn-loading .dot:nth-child(3) { animation-delay: .3s; }
@keyframes btn-dot {
  0%, 100% { opacity: .35; transform: scale(.8); }
  50% { opacity: 1; transform: scale(1.15); }
}

/* 底部提示 */
.login-foot {
  margin-top: 18px;
  text-align: center;
  font-size: 12px;
  color: var(--text-3);
}
.login-foot b { color: var(--brand-600); font-weight: 700; }

/* 小屏微调 */
@media (max-width: 360px) {
  .login-card { padding: 28px 18px 20px; }
  .remember-hint { display: none; }
}
</style>
