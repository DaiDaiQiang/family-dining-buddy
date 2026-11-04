<script setup>
/**
 * 妍 · 全局加载动画组件
 *
 * 完整还原 yan-loader.html 的动画：
 * 猫耳轮廓 + 底部大弧 + 粉色飘带逐笔描边、「妍」字蒙版从左往右写出、
 * 两枚爱心弹出、底部三点跳动提示。
 *
 * props：
 *   size     — logo 宽度（px），默认 160；整屏启动可传 260+
 *   tip      — 底部提示文案，默认「正在加载」
 *   showTip  — 是否显示底部提示
 *   full     — 整屏模式（fixed 覆盖全屏，用于首屏接管 / 全屏加载）
 *   blobs    — 是否显示背景光斑
 *   backdrop — 是否绘制粉白渐变底（false 时透明，融入所在页面）
 *
 * 多实例安全：SVG defs 的 id 按组件实例唯一化，避免同页多个加载动画串色。
 */
import { getCurrentInstance } from 'vue'

const props = defineProps({
  size: { type: Number, default: 160 },
  tip: { type: String, default: '正在加载' },
  showTip: { type: Boolean, default: true },
  full: { type: Boolean, default: false },
  blobs: { type: Boolean, default: true },
  backdrop: { type: Boolean, default: true },
})

const uid = getCurrentInstance().uid
const gid = (name) => `yan-${uid}-${name}`
const curl = (name) => `url(#${gid(name)})`
</script>

<template>
  <div
    class="yan"
    :class="{ 'yan--full': full, 'yan--flat': !backdrop }"
    role="status"
    :aria-label="tip"
  >
    <template v-if="blobs">
      <span class="yan-blob yan-blob--blue" />
      <span class="yan-blob yan-blob--pink" />
    </template>

    <div class="yan-loader">
      <svg
        class="yan-logo"
        :style="{ width: size + 'px' }"
        viewBox="0 0 240 240"
        xmlns="http://www.w3.org/2000/svg"
      >
        <defs>
          <linearGradient :id="gid('gBlue')" x1="0.05" y1="0" x2="0.95" y2="1">
            <stop offset="0%"   stop-color="#F2C89B" />
            <stop offset="55%"  stop-color="#D1915A" />
            <stop offset="100%" stop-color="#B85C38" />
          </linearGradient>
          <linearGradient :id="gid('gPink')" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%"   stop-color="#F7E8D2" />
            <stop offset="100%" stop-color="#D1915A" />
          </linearGradient>
          <linearGradient :id="gid('gMix')" x1="0.05" y1="0.1" x2="0.95" y2="0.95">
            <stop offset="0%"   stop-color="#F7E8D2" />
            <stop offset="42%"  stop-color="#EFD4B2" />
            <stop offset="62%"  stop-color="#D1915A" />
            <stop offset="100%" stop-color="#B85C38" />
          </linearGradient>
          <linearGradient :id="gid('wipeGrad')" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%"   stop-color="#fff" />
            <stop offset="55%"  stop-color="#fff" />
            <stop offset="100%" stop-color="#fff" stop-opacity="0" />
          </linearGradient>
          <mask :id="gid('wipe')" maskUnits="userSpaceOnUse" x="0" y="0" width="240" height="240">
            <rect class="yan-wipe-rect" x="-280" y="0" width="280" height="240" :fill="curl('wipeGrad')" />
          </mask>
        </defs>

        <!-- 1. 外圈底部大弧 -->
        <path
          class="yan-stroke yan-stroke--arc"
          :stroke="curl('gBlue')"
          pathLength="100"
          d="M20 140 C22 190 70 224 124 224 C164 224 200 206 216 176"
        />

        <!-- 2. 猫耳头部轮廓 -->
        <path
          class="yan-stroke yan-stroke--head"
          :stroke="curl('gBlue')"
          pathLength="100"
          d="M62 196 C36 184 22 154 24 124 C26 106 34 92 46 82 L40 26 L90 64
             C106 56 130 56 146 64 L198 24 L190 84 C202 96 212 116 212 138
             C210 168 190 190 162 198 C128 208 90 204 62 196 Z"
        />

        <!-- 3. 主体字形（蒙版扫过 = 一笔写出来） -->
        <g :mask="curl('wipe')">
          <text class="yan-brand-halo" x="120" y="164">妍</text>
          <text class="yan-brand" :fill="curl('gMix')" :stroke="curl('gMix')" x="120" y="164">妍</text>
        </g>

        <!-- 4. 粉色飘带笔触 -->
        <path
          class="yan-stroke yan-stroke--ribbon"
          :stroke="curl('gPink')"
          pathLength="100"
          d="M186 90 C170 126 144 150 112 162 C86 172 58 168 36 152"
        />

        <!-- 5. 两枚爱心 -->
        <g transform="translate(10,54)">
          <path
            class="yan-heart yan-heart--1"
            :fill="curl('gPink')"
            d="M12 21 C5 16.5 0 12.2 0 7.2 C0 3.2 3 .6 6.2 .6
               C8.6 .6 10.8 1.9 12 3.9 C13.2 1.9 15.4 .6 17.8 .6
               C21 .6 24 3.2 24 7.2 C24 12.2 19 16.5 12 21 Z"
          />
        </g>
        <g transform="translate(176,166) scale(.85)">
          <path
            class="yan-heart yan-heart--2"
            :fill="curl('gPink')"
            d="M12 21 C5 16.5 0 12.2 0 7.2 C0 3.2 3 .6 6.2 .6
               C8.6 .6 10.8 1.9 12 3.9 C13.2 1.9 15.4 .6 17.8 .6
               C21 .6 24 3.2 24 7.2 C24 12.2 19 16.5 12 21 Z"
          />
        </g>
      </svg>

      <div v-if="showTip" class="yan-tip">{{ tip }}<i /><i /><i /></div>
    </div>
  </div>
</template>

<style scoped>
.yan {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px 16px;
  overflow: hidden;
  border-radius: var(--r-lg, 16px);
  background: radial-gradient(120% 95% at 50% -10%, #FFFDF6 0%, #FBF0DC 45%, #F5E3C4 100%);
  font-family: var(--font-sans, inherit);
  -webkit-font-smoothing: antialiased;
}
.yan--flat { background: none; }

/* 整屏模式：覆盖视口，用于首屏 / 全屏加载 */
.yan--full {
  position: fixed;
  inset: 0;
  z-index: 999;
  padding: 0;
  border-radius: 0;
  min-height: 100vh;
}

/* ---------- 背景光斑 ---------- */
.yan-blob {
  position: absolute;
  border-radius: 50%;
  filter: blur(70px);
  pointer-events: none;
  z-index: 0;
}
.yan-blob--blue {
  width: 340px; height: 340px;
  background: #F2C89B; opacity: .5;
  top: -120px; left: -100px;
  animation: yan-drift1 11s ease-in-out infinite;
}
.yan-blob--pink {
  width: 320px; height: 320px;
  background: #E2B686; opacity: .45;
  bottom: -130px; right: -110px;
  animation: yan-drift2 13s ease-in-out infinite;
}
@keyframes yan-drift1 {
  0%, 100% { transform: translate(0, 0) scale(1); }
  50%      { transform: translate(40px, 30px) scale(1.1); }
}
@keyframes yan-drift2 {
  0%, 100% { transform: translate(0, 0) scale(1); }
  50%      { transform: translate(-36px, -26px) scale(1.08); }
}

/* ---------- 容器 ---------- */
.yan-loader {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}
.yan-loader::before {
  content: "";
  position: absolute;
  top: 44%; left: 50%;
  width: 78%; aspect-ratio: 1 / 1;
  transform: translate(-50%, -50%);
  border-radius: 50%;
  z-index: -1;
  filter: blur(12px);
  background: radial-gradient(circle, rgba(255, 255, 255, .92) 0%, rgba(255, 255, 255, .55) 45%, rgba(255, 255, 255, 0) 72%);
}

.yan-logo {
  height: auto;
  max-width: 80%;
  overflow: visible;
  animation: yan-life 2.8s ease-in-out infinite;
}
/* 整体：浮现 → 停留 → 上浮消散 */
@keyframes yan-life {
  0%   { opacity: 0; transform: translateY(9px)  scale(.94); }
  7%   { opacity: 1; transform: translateY(0)    scale(1); }
  88%  { opacity: 1; transform: translateY(0)    scale(1); }
  100% { opacity: 0; transform: translateY(-9px) scale(1.03); }
}

/* ---------- 描边笔触 ---------- */
.yan-stroke {
  fill: none;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-dasharray: 100;
  stroke-dashoffset: 100;
  animation-timing-function: cubic-bezier(.55, .05, .25, 1);
}
.yan-stroke--head {
  stroke-width: 6.8;
  filter: drop-shadow(0 3px 7px rgba(194, 112, 61, .30));
  animation: yan-draw-head 2.8s cubic-bezier(.55, .05, .25, 1) infinite;
}
.yan-stroke--arc {
  stroke-width: 8;
  opacity: .92;
  filter: drop-shadow(0 5px 10px rgba(194, 112, 61, .26));
  animation: yan-draw-arc 2.8s cubic-bezier(.55, .05, .25, 1) infinite;
}
.yan-stroke--ribbon {
  stroke-width: 9.5;
  opacity: .94;
  filter: drop-shadow(0 4px 9px rgba(184, 92, 56, .34));
  animation: yan-draw-ribbon 2.8s cubic-bezier(.55, .05, .25, 1) infinite;
}
@keyframes yan-draw-head  { 0% { stroke-dashoffset: 100; } 50%, 100% { stroke-dashoffset: 0; } }
@keyframes yan-draw-arc   { 0%, 7% { stroke-dashoffset: 100; } 58%, 100% { stroke-dashoffset: 0; } }
@keyframes yan-draw-ribbon{ 0%, 46% { stroke-dashoffset: 100; } 84%, 100% { stroke-dashoffset: 0; } }

/* ---------- 主体字形：从左往右「写」出来 ---------- */
.yan-brand, .yan-brand-halo {
  font-family: "STXingkai", "华文行楷", "STKaiti", "华文楷体", "KaiTi", "楷体", "Kaiti SC", "SimSun", serif;
  font-size: 118px;
  text-anchor: middle;
}
.yan-brand-halo {
  fill: none;
  stroke: #fff;
  stroke-width: 6.5;
  opacity: .72;
}
.yan-brand {
  stroke-width: 2.6;
  paint-order: stroke;
  filter: drop-shadow(0 4px 10px rgba(138, 69, 34, .22));
}
.yan-wipe-rect { animation: yan-wipe 2.8s cubic-bezier(.7, 0, .3, 1) infinite; }
@keyframes yan-wipe {
  0%, 18%   { transform: translateX(-280px); }
  66%, 100% { transform: translateX(340px); }
}

/* ---------- 爱心 ---------- */
.yan-heart {
  transform-box: fill-box;
  transform-origin: center;
  transform: scale(0);
  filter: drop-shadow(0 3px 6px rgba(184, 92, 56, .30));
}
.yan-heart--1 { animation: yan-pop1 2.8s ease-in-out infinite; }
.yan-heart--2 { animation: yan-pop2 2.8s ease-in-out infinite; }
@keyframes yan-pop1 {
  0%, 52%  { transform: scale(0);    opacity: 0; }
  58%      { transform: scale(1.38); opacity: 1; }
  63%      { transform: scale(.9); }
  68%      { transform: scale(1.14); }
  73%      { transform: scale(1); }
  78%      { transform: scale(1.2); }
  83%, 100%{ transform: scale(1);    opacity: 1; }
}
@keyframes yan-pop2 {
  0%, 62%  { transform: scale(0);    opacity: 0; }
  68%      { transform: scale(1.38); opacity: 1; }
  73%      { transform: scale(.9); }
  78%      { transform: scale(1.14); }
  83%      { transform: scale(1); }
  88%      { transform: scale(1.2); }
  93%, 100%{ transform: scale(1);    opacity: 1; }
}

/* ---------- 底部提示 ---------- */
.yan-tip {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 13.5px;
  letter-spacing: .2em;
  color: #A85A2E;
  animation: yan-tip-pulse 2.8s ease-in-out infinite;
}
.yan-tip i {
  width: 6px; height: 6px;
  border-radius: 50%;
  background: linear-gradient(135deg, #D1915A, #B85C38);
  animation: yan-dot 1.2s ease-in-out infinite;
}
.yan-tip i:nth-child(2) { animation-delay: .15s; }
.yan-tip i:nth-child(3) { animation-delay: .3s; }
@keyframes yan-dot {
  0%, 100% { transform: translateY(0);  opacity: .35; }
  50%      { transform: translateY(-5px); opacity: 1; }
}
@keyframes yan-tip-pulse {
  0%  { opacity: 0; }
  12% { opacity: 1; }
  85% { opacity: 1; }
  100%{ opacity: 0; }
}

@media (prefers-reduced-motion: reduce) {
  .yan * {
    animation-duration: .001s !important;
    animation-iteration-count: 1 !important;
  }
}
</style>
