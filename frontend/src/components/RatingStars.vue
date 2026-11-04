<script setup>
/**
 * 评分星
 *
 * 只读展示或可交互打分。半星用叠加裁剪实现。
 */
import { computed, ref } from 'vue'

const props = defineProps({
  value: { type: [Number, String], default: 0 },
  size: { type: Number, default: 18 },
  readonly: { type: Boolean, default: true },
  count: { type: Number, default: 5 }
})
const emit = defineEmits(['update:value', 'rate'])

const hover = ref(0)
const display = computed(() => {
  const v = Number(hover.value || props.value || 0)
  return Math.max(0, Math.min(props.count, v))
})

function pick(i) {
  if (props.readonly) return
  emit('update:value', i)
  emit('rate', i)
}
</script>

<template>
  <div
    class="stars"
    :class="{ 'stars--interactive': !readonly }"
    :style="{ '--star-size': size + 'px' }"
    role="img"
    :aria-label="`评分 ${value} 星`"
  >
    <span
      v-for="i in count"
      :key="i"
      class="star"
      :class="{ 'star--on': display >= i, 'star--half': display >= i - 0.5 && display < i }"
      @mouseenter="!readonly && (hover = i)"
      @mouseleave="!readonly && (hover = 0)"
      @click="pick(i)"
    >
      <span class="star-glyph" aria-hidden="true">★</span>
    </span>
  </div>
</template>

<style scoped>
.stars { display: inline-flex; align-items: center; gap: 2px; line-height: 1; }
.stars--interactive .star { cursor: pointer; transition: transform var(--dur-fast) var(--ease-bounce); }
.stars--interactive .star:active { transform: scale(1.25); }

.star {
  position: relative;
  width: var(--star-size);
  height: var(--star-size);
  font-size: var(--star-size);
  color: #E3CFA8;
}
.star-glyph { display: block; }
.star--on { color: var(--star); }
/* 半星：底部灰色星，上面盖一个裁剪到 50% 宽度的金色星 */
.star--half::after {
  content: '★';
  position: absolute;
  left: 0; top: 0;
  width: 50%;
  overflow: hidden;
  color: var(--star);
}
</style>
