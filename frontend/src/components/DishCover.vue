<script setup>
/**
 * 菜品封面
 *
 * 免图床方案：默认用 emoji + 渐变底色渲染出好看的封面；
 * 若菜谱配置了 cover_url，则显示真实图片并优雅降级。
 */
import { computed, ref } from 'vue'

const props = defineProps({
  emoji: { type: String, default: '🍲' },
  gradient: { type: String, default: 'linear-gradient(135deg,#F2C89B,#DE9A66)' },
  url: { type: String, default: '' },
  size: { type: String, default: 'md' }, // sm | md | lg | hero
  rounded: { type: String, default: 'lg' }
})

const imgError = ref(false)
const showImg = computed(() => !!props.url && !imgError.value)

const sizeMap = {
  sm: { emoji: 22, radius: 'var(--r-sm)' },
  md: { emoji: 34, radius: 'var(--r-md)' },
  lg: { emoji: 48, radius: 'var(--r-lg)' },
  hero: { emoji: 88, radius: 'var(--r-xl)' }
}
const dim = computed(() => sizeMap[props.size] || sizeMap.md)
const radiusVar = computed(() => ({
  sm: 'var(--r-sm)', md: 'var(--r-md)', lg: 'var(--r-lg)', xl: 'var(--r-xl)', full: 'var(--r-full)'
}[props.rounded] || 'var(--r-lg)'))
</script>

<template>
  <div
    class="cover"
    :class="`cover--${size}`"
    :style="{ background: showImg ? undefined : gradient, borderRadius: radiusVar }"
  >
    <img v-if="showImg" :src="url" alt="" class="cover-img" @error="imgError = true" />
    <span v-else class="cover-emoji" :style="{ fontSize: dim.emoji + 'px' }">{{ emoji }}</span>
    <span class="cover-gloss" aria-hidden="true" />
  </div>
</template>

<style scoped>
.cover {
  position: relative;
  flex: none;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  isolation: isolate;
}
.cover--sm { width: 46px; height: 46px; }
.cover--md { width: 72px; height: 72px; }
.cover--lg { width: 96px; height: 96px; }
.cover--hero { width: 100%; aspect-ratio: 16 / 10; }

.cover-img {
  width: 100%; height: 100%;
  object-fit: cover;
}
.cover-emoji {
  line-height: 1;
  filter: drop-shadow(0 3px 6px rgba(0,0,0,.18));
  z-index: 1;
}
/* 高光，让纯色封面更有质感 */
.cover-gloss {
  position: absolute; inset: 0;
  background:
    radial-gradient(120% 80% at 15% 0%, rgba(255,255,255,.35), transparent 55%),
    linear-gradient(180deg, rgba(255,255,255,.06), rgba(0,0,0,.1));
  pointer-events: none;
}
.cover--hero .cover-emoji { font-size: 92px; }
</style>
