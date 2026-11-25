<script setup>
/**
 * 登录页 · 正式版（莱茵生命风格）
 *
 * 家庭场景的轻量登录：选身份（成员卡片）→ 输凭证 → 进家门。
 * 登录逻辑、接口、跳转全部复用 auth store，视觉层为「莱茵生命」式精密医疗机构风，
 * 配色保持暖琥珀主色，与孙府暖奶油气质统一。
 *
 * 开场动画：从 LOGO 元素演化成完整界面（logo 弹出缩小 → 猫头线条描边 → 拼合成卡片）。
 */
import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
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
  selectedMember.value?.has_password ? '输入授权凭证' : '未设凭证，直接接入'
)

const selectedId = ref(null)
const password = ref('')
const showPwd = ref(false)
const remember = ref(true)
const error = ref('')
const shaking = ref(false)
const loading = ref(false)

/** 机构代号：按日期生成一个稳定「批次号」，制造档案感 */
const batchNo = computed(() => {
  const d = new Date()
  const pad = (n) => String(n).padStart(2, '0')
  return `RL-${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}-A`
})

/** 背景网格交点坐标（纯装饰） */
const gridDots = Array.from({ length: 24 }, (_, i) => ({
  x: (i % 6) * 20 + 4,
  y: Math.floor(i / 6) * 20 + 4
}))

/**
 * 开场加载动画：从 LOGO 元素演化成完整界面
 *
 * 阶段语义：
 *   boot(0~1900ms)  —— logo 全屏大 → 缩小（此时猫头描边隐藏）
 *   bloom(1900~3500ms) —— logo 隐藏，猫头线条描边构建（此时 logo 不显示）
 *   settled —— 拼合成登录卡片，平滑过渡到正式内容
 *
 * 数据拉取与动画并行；状态推进用 ref + 固定时长 setTimeout，句柄存实例作用域，抗 HMR。
 */
const introPhase = ref('boot') // 'boot' -> 'bloom' -> 'settled'

const PHASE_BOOT_MS = 1900
const PHASE_BLOOM_MS = 1600
const PHASE_SETTLED_MS = 700

let phaseTimers = []

function clearPhaseTimers() {
  phaseTimers.forEach((t) => clearTimeout(t))
  phaseTimers = []
}

onBeforeUnmount(clearPhaseTimers)

onMounted(async () => {
  // 阶段推进：boot → bloom → settled
  phaseTimers.push(setTimeout(() => { introPhase.value = 'bloom' }, PHASE_BOOT_MS))
  phaseTimers.push(setTimeout(() => { introPhase.value = 'settled' }, PHASE_BOOT_MS + PHASE_BLOOM_MS))

  // 成员数据拉取，与动画并行
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
  if (!selectedId.value) { fail('请先选择一位家庭成员'); return }
  loading.value = true
  // 轻微延迟，让按钮的验证态可见，也避免误触连点
  await new Promise((r) => setTimeout(r, 320))
  const res = await auth.login(selectedId.value, password.value, remember.value)
  loading.value = false
  if (!res.ok) {
    fail(res.error || '凭证校验失败，请重试')
    password.value = ''
    return
  }
  // 登录前启动 init 的成员/四模块请求都 401 落空了，拿上令牌强制重拉
  await app.init(true)
  const m = members.value.find((x) => x.id === selectedId.value)
  toast.success(`接入成功，${m?.nickname || '家人'}`)
  const redirect = route.query.redirect
  const target = typeof redirect === 'string' && redirect.startsWith('/') && !redirect.startsWith('//')
    ? redirect
    : '/'
  router.replace(target)
}
</script>

<template>
  <div class="rl">
    <!-- ============ 开场加载动画：从 LOGO 演化成完整界面 ============ -->
    <transition name="intro-fade">
      <div v-if="introPhase !== 'settled'" class="rl-intro" :class="`rl-intro--${introPhase}`" role="status" aria-label="正在载入">
        <!-- 中央 LOGO 种子（与正式页 logo 同源，演化后即成为卡片上的 logo） -->
        <div class="rl-seed">
          <!-- 猫头 LOGO 轮廓描边（boot 阶段逐笔画出，bloom 阶段汇聚收拢） -->
          <svg class="rl-seed-svg" viewBox="0 0 240 240" fill="none" aria-hidden="true">
            <!-- 猫头主体外轮廓：左耳 → 头顶凹弧 → 右耳 → 右脸颊 → 下巴 → 左脸颊，对称圆脸 -->
            <path class="rl-seed-path rl-seed-path--head"
              d="M120 58
                 L76 58 L52 24
                 L74 70
                 C40 84 26 118 34 150
                 C42 182 76 200 120 200
                 C164 200 198 182 206 150
                 C214 118 200 84 166 70
                 L188 24 L164 58
                 Z" />
            <!-- 左耳内沿 -->
            <path class="rl-seed-path rl-seed-path--earL" d="M72 56 L58 36 L82 66" />
            <!-- 右耳内沿 -->
            <path class="rl-seed-path rl-seed-path--earR" d="M168 56 L182 36 L158 66" />
            <!-- 尾巴（左侧甩出的大弧） -->
            <path class="rl-seed-path rl-seed-path--tail"
              d="M44 140 C20 146 14 168 26 184 C38 198 62 200 78 190" />
            <!-- 右胡须（三根扇形） -->
            <path class="rl-seed-path rl-seed-path--whisker1" d="M196 132 L230 122" />
            <path class="rl-seed-path rl-seed-path--whisker2" d="M200 146 L234 146" />
            <path class="rl-seed-path rl-seed-path--whisker3" d="M196 160 L230 170" />
            <!-- 左上一枚爱心 -->
            <path class="rl-seed-path rl-seed-path--heart1"
              d="M40 52 C34 44 26 42 22 46 C18 50 20 58 28 63 C34 67 38 66 40 60" />
            <!-- 右下二枚爱心 -->
            <path class="rl-seed-path rl-seed-path--heart2"
              d="M206 120 C200 112 192 110 188 114 C184 118 186 126 194 131 C200 135 204 134 206 128" />
          </svg>
          <!-- 真实 logo -->
          <img src="/logo.png" alt="孙府" class="rl-seed-img" />
        </div>

        <!-- 飞入的平面色块碎片 -->
        <div class="rl-shards" aria-hidden="true">
          <span class="rl-shard" v-for="n in 12" :key="n" :style="{ '--n': n }" />
        </div>

        <!-- 品牌文字（bloom 阶段浮现） -->
        <p class="rl-seed-brand">孙府 · HOME LAB</p>
        <p class="rl-seed-slogan font-kai">一屋五人，三餐四季</p>
      </div>
    </transition>

    <!-- ============ 正式页面（settled 阶段完全呈现） ============ -->
    <div class="rl-main" :class="{ 'rl-main--in': introPhase === 'settled' }">
    <!-- 精密网格背景 -->
    <div class="rl-grid" aria-hidden="true" />
    <!-- 扫描线（缓慢下移的高光线） -->
    <div class="rl-scan" aria-hidden="true" />
    <!-- 网格交点装饰 -->
    <div class="rl-dots" aria-hidden="true">
      <span v-for="(d, i) in gridDots" :key="i" class="rl-dot" :style="{ left: d.x + '%', top: d.y + '%' }" />
    </div>

    <!-- 四角档案标注 -->
    <div class="rl-corners" aria-hidden="true">
      <span class="rl-corner rl-corner--tl" />
      <span class="rl-corner rl-corner--tr" />
      <span class="rl-corner rl-corner--bl" />
      <span class="rl-corner rl-corner--br" />
    </div>

    <div class="rl-card">
      <!-- 机构品牌头 -->
      <header class="rl-head">
        <div class="rl-badge">
          <img src="/logo.png" alt="孙府" class="rl-badge-img" />
        </div>
        <div class="rl-brand">
          <h1 class="rl-title">孙府</h1>
          <p class="rl-sub">SUNFAM · HOME LAB</p>
        </div>
        <p class="rl-slogan font-kai">一屋五人，三餐四季</p>
      </header>

      <!-- 档案编号栏 -->
      <div class="rl-meta">
        <span class="rl-meta-item">批次 {{ batchNo }}</span>
        <span class="rl-meta-item">授权节点 · 家庭内网</span>
      </div>

      <form class="rl-form" @submit.prevent="submit">
        <!-- 成员选择 -->
        <div class="rl-block">
          <p class="rl-label">选择你的身份 <em>IDENTITY</em></p>
          <div class="rl-roles">
            <button
              v-for="m in members"
              :key="m.id"
              type="button"
              class="rl-role"
              :class="{ on: selectedId === m.id }"
              :style="{ '--mc': m.theme_color }"
              @click="select(m)"
            >
              <span class="rl-role-frame">
                <span class="rl-role-avatar">
                  <img v-if="m.avatar_url" :src="m.avatar_url" alt="" class="rl-role-avatar-img" />
                  <template v-else>{{ m.avatar_emoji }}</template>
                </span>
              </span>
              <span class="rl-role-name ellipsis">{{ m.nickname }}</span>
              <span class="rl-role-no">#{{ String(m.id).padStart(2, '0') }}</span>
              <span v-if="selectedId === m.id" class="rl-role-check">✓</span>
            </button>
          </div>
          <p v-if="!members.length" class="rl-empty">
            暂无登记成员，请先至「家庭成员」页录入
          </p>
        </div>

        <!-- 密码 -->
        <div class="rl-block">
          <p class="rl-label">授权凭证 <em>CREDENTIAL</em></p>
          <div class="rl-pwd" :class="{ shake: shaking, 'rl-pwd--err': !!error }">
            <span class="rl-pwd-key" aria-hidden="true">◈</span>
            <input
              :type="showPwd ? 'text' : 'password'"
              v-model="password"
              class="rl-pwd-input"
              :placeholder="pwdPlaceholder"
              autocomplete="current-password"
              @input="error = ''"
            />
            <button type="button" class="rl-pwd-eye" :aria-label="showPwd ? '隐藏凭证' : '显示凭证'" @click="showPwd = !showPwd">
              {{ showPwd ? '▣' : '▢' }}
            </button>
          </div>
          <transition name="err-fade">
            <p v-if="error" class="rl-error">▲ {{ error }}</p>
          </transition>
        </div>

        <!-- 记住状态 -->
        <label class="rl-remember">
          <input v-model="remember" type="checkbox" class="rl-remember-native" />
          <span class="rl-remember-box"><span class="rl-remember-tick">✓</span></span>
          <span class="rl-remember-text">保持接入状态</span>
          <span class="rl-remember-hint">不勾选，会话结束后断开</span>
        </label>

        <button type="submit" class="rl-btn" :disabled="loading || !members.length">
          <span v-if="!loading">接入系统 · 进入</span>
          <span v-else class="rl-btn-loading"><i class="dot" /><i class="dot" /><i class="dot" /></span>
        </button>
      </form>

      <footer class="rl-foot">
        <span class="rl-foot-line" />
        <p>已设凭证的成员请输入自己的凭证 · 未设凭证可直接接入</p>
      </footer>
    </div>

    <!-- 右下角版本水印 -->
    <div class="rl-ver" aria-hidden="true">SUNFAM SYSTEM · v1.0</div>
    </div>
  </div>
</template>

<style scoped>
/* ==========================================================================
   登录页 · 莱茵生命风格 —— 暖琥珀机构风
   冷冽精密的版式骨架 + 暖色贯穿，既像「医药公司」又保留「家」的温度
   ========================================================================== */

/* ==========================================================================
   开场加载动画 —— 从 LOGO 演化成完整界面
   三阶段：boot(logo 弹出缩小) → bloom(猫头线条描边) → settled(拼合过渡)
   ========================================================================== */

/* 全屏演化层（暖色系：米色 + 浅橙 + 暖黄渐变） */
.rl-intro {
  position: fixed;
  inset: 0;
  z-index: 30;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 18px;
  background:
    radial-gradient(120% 80% at 50% 0%, #FFF6E5 0%, #FBEBD3 45%, #F5DFC0 100%);
  overflow: hidden;
}

/* 最终过渡：演化层整体淡出（露出已拼合好的正式页面） */
.intro-fade-leave-active { transition: opacity .6s var(--ease-out); }
.intro-fade-leave-to { opacity: 0; }

/* ---------- LOGO 种子 ---------- */
.rl-seed {
  position: relative;
  width: 180px; height: 180px;
  display: flex; align-items: center; justify-content: center;
}
/* 猫头轮廓描边 SVG：boot 阶段隐藏，bloom 阶段显示并逐笔描边 */
.rl-seed-svg {
  position: absolute;
  inset: 0;
  width: 100%; height: 100%;
  overflow: visible;
  opacity: 0;
}
.rl-intro--bloom .rl-seed-svg {
  opacity: 1;
}
/* 每段描边路径：stroke-dashoffset 从满到 0，逐笔画出（bloom 阶段触发） */
.rl-seed-path {
  stroke-linecap: round;
  stroke-linejoin: round;
  fill: none;
  stroke-dasharray: 100;
  stroke-dashoffset: 100;
  filter: drop-shadow(0 1px 2px rgba(138, 69, 34, .18));
}
.rl-intro--bloom .rl-seed-path {
  animation: seed-draw 1s var(--ease-out) forwards;
}
@keyframes seed-draw {
  to { stroke-dashoffset: 0; }
}
/* 猫头主体：深焦糖主色，最粗，最先画 */
.rl-seed-path--head {
  stroke: var(--brand-700);
  stroke-width: 4;
}
/* 耳朵内沿：焦糖色 */
.rl-seed-path--earL,
.rl-seed-path--earR {
  stroke: var(--brand-600);
  stroke-width: 3;
}
.rl-intro--bloom .rl-seed-path--earL,
.rl-intro--bloom .rl-seed-path--earR {
  animation-delay: .25s;
}
/* 尾巴：粉玫红点缀（呼应 logo 的粉色尾巴） */
.rl-seed-path--tail {
  stroke: #D6336C;
  stroke-width: 3.6;
}
.rl-intro--bloom .rl-seed-path--tail {
  animation-delay: .45s;
}
/* 胡须：暖棕细线 */
.rl-seed-path--whisker1,
.rl-seed-path--whisker2,
.rl-seed-path--whisker3 {
  stroke: var(--brand-500);
  stroke-width: 2.6;
}
.rl-intro--bloom .rl-seed-path--whisker1 { animation-delay: .65s; }
.rl-intro--bloom .rl-seed-path--whisker2 { animation-delay: .72s; }
.rl-intro--bloom .rl-seed-path--whisker3 { animation-delay: .79s; }
/* 两枚爱心：粉玫红 */
.rl-seed-path--heart1,
.rl-seed-path--heart2 {
  stroke: #D6336C;
  stroke-width: 2.8;
}
.rl-intro--bloom .rl-seed-path--heart1 { animation-delay: .95s; }
.rl-intro--bloom .rl-seed-path--heart2 { animation-delay: 1.05s; }

/* 真实 logo：boot 阶段由大弹出缩小，bloom 阶段淡出隐藏（避免与猫头描边冲突） */
.rl-seed-img {
  width: 104px; height: 104px;
  object-fit: contain;
  display: block;
  position: relative;
  z-index: 1;
  filter: drop-shadow(0 0 16px rgba(194, 112, 61, .4)) drop-shadow(0 0 8px rgba(232, 163, 61, .3));
  /* 弹出放大 → 弹性缩回（boot 阶段内完成） */
  animation: logo-pop 1.9s var(--ease-bounce) both;
  transition: opacity .35s var(--ease-out);
}
/* bloom 阶段：logo 淡出隐藏 */
.rl-intro--bloom .rl-seed-img {
  opacity: 0;
  pointer-events: none;
}
/* 弹出：从无到大尺寸（1.38），弹性缩回到 1 */
@keyframes logo-pop {
  0%   { opacity: 0; transform: scale(.3); }
  55%  { opacity: 1; transform: scale(1.38); }
  80%  { opacity: 1; transform: scale(1.04); }
  100% { opacity: 1; transform: scale(1); }
}

/* ---------- 平面色块碎片（bloom 阶段从四周飞入，构建骨架；boot 阶段隐藏） ---------- */
.rl-shards {
  position: absolute;
  inset: 0;
  pointer-events: none;
}
.rl-shard {
  position: absolute;
  width: 14px; height: 14px;
  border-radius: 3px;
  background: linear-gradient(135deg, #D98E4F, #E8A33D);
  opacity: 0;
  /* 每个碎片从不同方位飞向中心附近，用 --n 决定角度 */
  --ang: calc(var(--n) * 30deg);
  left: calc(50% + cos(var(--ang)) * 46%);
  top: calc(50% + sin(var(--ang)) * 46%);
}
/* bloom 阶段：碎片飞入 */
.rl-intro--bloom .rl-shard {
  animation: shard-in 1.1s var(--ease-out) forwards;
  animation-delay: calc(.15s + var(--n) * .06s);
}
@keyframes shard-in {
  0%   { opacity: 0; transform: translate(calc(-1 * cos(var(--ang)) * 120px), calc(-1 * sin(var(--ang)) * 120px)) scale(.4); }
  60%  { opacity: 1; }
  100% { opacity: 1; transform: translate(0, 0) scale(1); }
}

/* bloom 阶段：logo 微微上移，为文字腾位 */
.rl-intro--bloom .rl-seed {
  transform: translateY(-6px);
  transition: transform .9s var(--ease-out);
}

/* 品牌文字：boot 隐藏，bloom 浮现 */
.rl-seed-brand {
  margin: 0;
  font-family: var(--font-display);
  font-size: 15px;
  font-weight: 700;
  letter-spacing: .3em;
  color: var(--brand-700);
  opacity: 0;
}
.rl-seed-slogan {
  margin: -6px 0 0;
  font-size: 13px;
  letter-spacing: .18em;
  color: var(--text-3);
  opacity: 0;
}
.rl-intro--bloom .rl-seed-brand,
.rl-intro--bloom .rl-seed-slogan {
  animation: brand-in .7s var(--ease-out) 1.3s both;
}
@keyframes brand-in {
  from { opacity: 0; transform: translateY(8px); }
  to   { opacity: 1; transform: none; }
}

/* ---------- 正式页面容器：settled 阶段拼合进入 ---------- */
.rl-main {
  position: relative;
  z-index: 1;
  opacity: 0;
}
.rl-main--in {
  animation: main-in .6s var(--ease-out) .15s forwards;
}
@keyframes main-in {
  from { opacity: 0; transform: translateY(14px) scale(.985); }
  to   { opacity: 1; transform: none; }
}

/* ---------- 卡片内区块依次入场（stagger，settled 后触发） ---------- */
.rl-main--in .rl-head,
.rl-main--in .rl-meta,
.rl-main--in .rl-block,
.rl-main--in .rl-remember,
.rl-main--in .rl-btn,
.rl-main--in .rl-foot {
  animation: block-in .55s var(--ease-out) both;
}
.rl-main--in .rl-head      { animation-delay: .30s; }
.rl-main--in .rl-meta      { animation-delay: .38s; }
.rl-main--in .rl-block:nth-of-type(1) { animation-delay: .46s; }
.rl-main--in .rl-block:nth-of-type(2) { animation-delay: .54s; }
.rl-main--in .rl-remember  { animation-delay: .62s; }
.rl-main--in .rl-btn       { animation-delay: .70s; }
.rl-main--in .rl-foot      { animation-delay: .78s; }
@keyframes block-in {
  from { opacity: 0; transform: translateY(16px); }
  to   { opacity: 1; transform: none; }
}

/* 减少动态效果偏好：关掉开场/入场动画，直接呈现正式页面 */
@media (prefers-reduced-motion: reduce) {
  .rl-seed-svg, .rl-seed-path, .rl-seed-img, .rl-shard,
  .rl-seed-brand, .rl-seed-slogan, .rl-intro { animation: none !important; transition: none !important; }
  .rl-main { opacity: 1; }
  .rl-main--in .rl-head, .rl-main--in .rl-meta, .rl-main--in .rl-block,
  .rl-main--in .rl-remember, .rl-main--in .rl-btn, .rl-main--in .rl-foot { animation: none !important; }
}

.rl {
  position: relative;
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px var(--page-pad) calc(40px + var(--safe-bottom));
  overflow: hidden;
  background: var(--bg);
  /* 暖色系背景：米色 + 浅橙 + 暖黄渐变，与孙府暖奶油风一致 */
  background-image:
    radial-gradient(120% 60% at 50% -8%, #FFF9EC 0%, rgba(255, 249, 236, 0) 60%),
    linear-gradient(180deg, #FBF1DE 0%, #F7E6C8 55%, #F3DCB8 100%);
}

/* 精密网格背景（暖底上用更深的暖棕，保证可见但不抢眼） */
.rl-grid {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(168, 90, 46, .06) 1px, transparent 1px),
    linear-gradient(90deg, rgba(168, 90, 46, .06) 1px, transparent 1px);
  background-size: 32px 32px;
  pointer-events: none;
}

/* 扫描线：缓慢下移的高光线（暖琥珀） */
.rl-scan {
  position: absolute;
  left: 0; right: 0;
  height: 120px;
  top: -140px;
  background: linear-gradient(180deg, transparent, rgba(217, 142, 79, .16), transparent);
  animation: rl-scan 7s linear infinite;
  pointer-events: none;
}
@keyframes rl-scan {
  0% { top: -140px; }
  100% { top: 110%; }
}

/* 网格交点 */
.rl-dots { position: absolute; inset: 0; pointer-events: none; }
.rl-dot {
  position: absolute;
  width: 3px; height: 3px;
  border-radius: 50%;
  background: rgba(194, 112, 61, .32);
}

/* 四角档案括号 */
.rl-corners { position: absolute; inset: 18px; pointer-events: none; }
.rl-corner {
  position: absolute;
  width: 22px; height: 22px;
  border: 1.5px solid rgba(194, 112, 61, .5);
}
.rl-corner--tl { top: 0; left: 0; border-right: none; border-bottom: none; }
.rl-corner--tr { top: 0; right: 0; border-left: none; border-bottom: none; }
.rl-corner--bl { bottom: 0; left: 0; border-right: none; border-top: none; }
.rl-corner--br { bottom: 0; right: 0; border-left: none; border-top: none; }

/* 主卡片 */
.rl-card {
  position: relative;
  z-index: 2;
  width: 100%;
  max-width: 470px;
  background: rgba(255, 254, 250, .98);
  border: 1px solid rgba(226, 182, 134, .42);
  border-radius: 28px;
  /* 分层阴影：近景暖棕柔影 + 远景弥散光，营造悬浮层次 */
  box-shadow:
    0 2px 4px rgba(168, 90, 46, .07),
    0 14px 34px rgba(168, 90, 46, .16),
    0 34px 80px rgba(168, 90, 46, .22),
    inset 0 1px 0 rgba(255, 255, 255, .9);
  padding: 38px 30px 24px;
  animation: rl-card-in var(--dur-slow) var(--ease-out);
}
@keyframes rl-card-in {
  from { opacity: 0; transform: translateY(20px) scale(.98); }
  to { opacity: 1; transform: none; }
}

/* 品牌头 */
.rl-head { text-align: center; margin-bottom: 24px; }
.rl-badge {
  width: 86px; height: 86px;
  margin: 0 auto 14px;
  display: flex; align-items: center; justify-content: center;
  border-radius: 50%;
  background: radial-gradient(circle at 35% 30%, #fff 0%, #fdf2e6 60%, #f8e6d0 100%);
  box-shadow:
    0 2px 6px rgba(168, 90, 46, .10),
    0 10px 26px rgba(194, 112, 61, .16),
    inset 0 1px 0 rgba(255, 255, 255, .9);
  transition: transform var(--dur) var(--ease-bounce);
}
/* logo 徽章 hover：轻微上浮回弹（桌面端） */
@media (hover: hover) {
  .rl-badge:hover { transform: translateY(-3px) scale(1.04); }
}
/* 品牌 logo：真实猫头图，浅色圆底衬托，不改文件本身 */
.rl-badge-img {
  width: 64px; height: 64px;
  object-fit: contain;
  display: block;
  filter: drop-shadow(0 3px 6px rgba(194, 112, 61, .24));
}
.rl-title {
  font-family: var(--font-display);
  font-size: 32px;
  font-weight: 800;
  letter-spacing: .16em;
  line-height: 1.2;
  color: var(--brand-700);
  text-shadow: 0 1px 0 rgba(255, 255, 255, .7);
}
.rl-sub {
  margin-top: 6px;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: .34em;
  color: var(--brand-400);
  text-transform: uppercase;
}
.rl-slogan {
  margin-top: 10px;
  font-size: 13.5px;
  color: var(--text-3);
  letter-spacing: .2em;
}

/* 档案编号栏 */
.rl-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  margin-bottom: 24px;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  background: linear-gradient(180deg, #FDF6EA 0%, #FAEDDA 100%);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: .08em;
  color: var(--text-2);
  font-family: var(--font-num);
}
.rl-meta-item { display: inline-flex; align-items: center; gap: 6px; }
.rl-meta-item::before {
  content: '';
  width: 4px; height: 4px;
  border-radius: 50%;
  background: var(--brand-400);
  flex: none;
}

/* 表单分块 */
.rl-block { margin-bottom: 24px; }
.rl-label {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-1);
  margin-bottom: 10px;
  display: flex;
  align-items: center;
  gap: 8px;
  letter-spacing: .04em;
}
.rl-label em {
  font-style: normal;
  font-size: 9.5px;
  font-weight: 700;
  letter-spacing: .2em;
  color: var(--text-4);
  padding: 1px 6px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--surface-2);
}

/* 成员卡片（证件照式） */
.rl-roles {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}
@media (min-width: 420px) {
  .rl-roles { grid-template-columns: repeat(4, 1fr); }
}
.rl-role {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 7px;
  padding: 16px 6px 12px;
  border: 1.5px solid var(--border);
  border-radius: 18px;
  background: linear-gradient(180deg, #FFFDF8 0%, #FBF3E6 100%);
  cursor: pointer;
  transition: transform var(--dur-fast) var(--ease), border-color var(--dur-fast),
              box-shadow var(--dur-fast), background var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
  font-family: inherit;
}
/* 悬停上浮 + 阴影增强（桌面端） */
@media (hover: hover) {
  .rl-role:hover {
    transform: translateY(-3px);
    border-color: color-mix(in srgb, var(--mc, #C2703D) 55%, transparent);
    box-shadow: 0 10px 22px rgba(168, 90, 46, .16);
  }
  .rl-role:hover .rl-role-frame {
    transform: rotate(45deg) scale(1.08);
  }
}
.rl-role:active { transform: scale(.95); }
.rl-role.on {
  border-color: var(--mc, var(--brand-500));
  background: #fff;
  box-shadow:
    0 0 0 3px color-mix(in srgb, var(--mc, #C2703D) 14%, transparent),
    0 8px 20px color-mix(in srgb, var(--mc, #C2703D) 18%, transparent);
}
/* 菱形边框头像框（莱茵生命式切角证件照） */
.rl-role-frame {
  width: 52px; height: 52px;
  display: flex; align-items: center; justify-content: center;
  transform: rotate(45deg);
  border: 1.5px solid color-mix(in srgb, var(--mc, #C2703D) 40%, transparent);
  border-radius: 14px;
  background: color-mix(in srgb, var(--mc, #C2703D) 12%, white);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, .8);
  transition: transform var(--dur) var(--ease-bounce);
}
.rl-role-avatar {
  transform: rotate(-45deg);
  width: 34px; height: 34px;
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 20px;
  overflow: hidden;
}
.rl-role-avatar-img { width: 100%; height: 100%; object-fit: cover; display: block; }
.rl-role-name { font-size: 13px; font-weight: 700; color: var(--text-1); max-width: 100%; }
.rl-role-no {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: .1em;
  color: var(--text-4);
  font-family: var(--font-num);
}
.rl-role-check {
  position: absolute;
  top: -7px; right: -7px;
  width: 20px; height: 20px;
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 11px; font-weight: 800;
  color: #fff;
  background: var(--mc, var(--brand-500));
  border: 2px solid #fff;
  box-shadow: 0 2px 6px rgba(0, 0, 0, .18);
}
.rl-empty { font-size: 12.5px; color: var(--text-3); line-height: 1.6; margin-top: 4px; }

/* 密码框 */
.rl-pwd {
  display: flex;
  align-items: center;
  border: 1.5px solid var(--border);
  border-radius: 14px;
  background: linear-gradient(180deg, #FFFDF8 0%, #FBF3E6 100%);
  box-shadow: inset 0 1px 2px rgba(168, 90, 46, .05);
  transition: border-color var(--dur-fast), box-shadow var(--dur-fast), background var(--dur-fast);
}
.rl-pwd:focus-within {
  border-color: var(--brand-400);
  background: #fff;
  box-shadow: 0 0 0 3.5px var(--brand-100);
}
.rl-pwd--err { border-color: var(--danger); box-shadow: 0 0 0 3.5px var(--danger-bg); }
.rl-pwd-key {
  flex: none;
  width: 44px;
  display: flex; align-items: center; justify-content: center;
  font-size: 14px;
  color: var(--brand-400);
}
.rl-pwd-input {
  flex: 1;
  min-width: 0;
  padding: 14px 4px 14px 0;
  font-size: 16px;
  font-family: inherit;
  color: var(--text-1);
  background: transparent;
  border: none;
  outline: none;
  letter-spacing: .06em;
}
.rl-pwd-input::placeholder { color: var(--text-4); letter-spacing: normal; }
.rl-pwd-eye {
  flex: none;
  width: 46px; height: 46px;
  border: none;
  background: transparent;
  font-size: 16px;
  color: var(--text-3);
  cursor: pointer;
  border-radius: 12px;
  -webkit-tap-highlight-color: transparent;
}
.rl-pwd-eye:active { background: var(--bg-subtle); }

/* 错误提示 */
.rl-error {
  margin-top: 8px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--danger);
}
.err-fade-enter-active, .err-fade-leave-active { transition: opacity var(--dur-fast); }
.err-fade-enter-from, .err-fade-leave-to { opacity: 0; }
.shake { animation: rl-shake .45s var(--ease); }
@keyframes rl-shake {
  0%, 100% { transform: translateX(0); }
  20% { transform: translateX(-7px); }
  40% { transform: translateX(6px); }
  60% { transform: translateX(-4px); }
  80% { transform: translateX(3px); }
}

/* 记住状态 */
.rl-remember {
  display: flex;
  align-items: center;
  gap: 9px;
  margin: 4px 0 24px;
  cursor: pointer;
  user-select: none;
  -webkit-tap-highlight-color: transparent;
}
.rl-remember-native { position: absolute; opacity: 0; width: 0; height: 0; }
.rl-remember-box {
  flex: none;
  width: 22px; height: 22px;
  border-radius: 6px;
  border: 1.5px solid var(--border-strong);
  background: var(--surface-2);
  display: flex; align-items: center; justify-content: center;
  transition: all var(--dur-fast) var(--ease);
}
.rl-remember-tick {
  font-size: 13px; font-weight: 800;
  color: #fff;
  opacity: 0; transform: scale(.4);
  transition: all var(--dur-fast) var(--ease-bounce);
}
.rl-remember-native:checked + .rl-remember-box {
  background: var(--brand-gradient);
  border-color: transparent;
  box-shadow: 0 3px 10px rgba(194, 112, 61, .32);
}
.rl-remember-native:checked + .rl-remember-box .rl-remember-tick { opacity: 1; transform: none; }
.rl-remember-native:focus-visible + .rl-remember-box { box-shadow: 0 0 0 3.5px var(--brand-100); }
.rl-remember-text { font-size: 14px; font-weight: 600; color: var(--text-1); }
.rl-remember-hint { font-size: 11px; color: var(--text-4); margin-left: auto; }

/* 登录按钮（现代感：分层渐变 + 上高光 + 柔影） */
.rl-btn {
  width: 100%;
  min-height: 54px;
  border: none;
  border-radius: 16px;
  background: linear-gradient(135deg, #E29A5B 0%, #C2703D 55%, #B05A30 100%);
  color: #fff;
  font-size: 16px;
  font-weight: 700;
  font-family: var(--font-display);
  letter-spacing: .14em;
  cursor: pointer;
  box-shadow:
    0 4px 12px rgba(194, 112, 61, .30),
    0 12px 28px rgba(168, 90, 46, .28),
    inset 0 1px 0 rgba(255, 255, 255, .32);
  transition: transform var(--dur-fast) var(--ease), box-shadow var(--dur-fast), filter var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
/* 按钮 hover：上浮 + 发光增强（桌面端） */
@media (hover: hover) {
  .rl-btn:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow:
      0 8px 18px rgba(194, 112, 61, .40),
      0 18px 40px rgba(168, 90, 46, .34),
      inset 0 1px 0 rgba(255, 255, 255, .36);
    filter: brightness(1.05);
  }
}
.rl-btn:active { transform: scale(.97); box-shadow: 0 4px 14px rgba(194, 112, 61, .34); }
.rl-btn:disabled { opacity: .5; pointer-events: none; }
.rl-btn-loading { display: inline-flex; align-items: center; gap: 6px; }
.rl-btn-loading .dot {
  width: 6px; height: 6px;
  border-radius: 50%;
  background: #fff;
  animation: rl-btn-dot 1s ease-in-out infinite;
}
.rl-btn-loading .dot:nth-child(2) { animation-delay: .15s; }
.rl-btn-loading .dot:nth-child(3) { animation-delay: .3s; }
@keyframes rl-btn-dot {
  0%, 100% { opacity: .35; transform: scale(.8); }
  50% { opacity: 1; transform: scale(1.15); }
}

/* 底部提示 */
.rl-foot { margin-top: 20px; text-align: center; }
.rl-foot-line {
  display: block;
  width: 56px; height: 2px;
  margin: 0 auto 14px;
  border-radius: 1px;
  background: linear-gradient(90deg, transparent, var(--border-strong), transparent);
}
.rl-foot p { font-size: 12.5px; color: var(--text-1); line-height: 1.6; opacity: .82; }
/* 版本水印 */
.rl-ver {
  position: absolute;
  right: 22px; bottom: 16px;
  z-index: 2;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: .2em;
  color: rgba(120, 60, 28, .6);
  font-family: var(--font-num);
}

/* 小屏微调 */
@media (max-width: 360px) {
  .rl-card { padding: 28px 18px 20px; }
  .rl-remember-hint { display: none; }
}
</style>
