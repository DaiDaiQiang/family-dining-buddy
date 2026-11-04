<script setup>
/**
 * 份数步进器：－ [数字] ＋
 */
const props = defineProps({
  modelValue: { type: Number, default: 1 },
  min: { type: Number, default: 1 },
  max: { type: Number, default: 20 },
  size: { type: String, default: 'md' } // sm | md
})
const emit = defineEmits(['update:modelValue'])

function step(delta) {
  const next = Math.max(props.min, Math.min(props.max, (props.modelValue || 0) + delta))
  if (next !== props.modelValue) emit('update:modelValue', next)
}
</script>

<template>
  <div class="stepper" :class="`stepper--${size}`">
    <button
      class="stepper-btn"
      :disabled="modelValue <= min"
      aria-label="减少"
      @click.stop="step(-1)"
    >−</button>
    <span class="stepper-value num">{{ modelValue }}</span>
    <button
      class="stepper-btn stepper-btn--plus"
      :disabled="modelValue >= max"
      aria-label="增加"
      @click.stop="step(1)"
    >＋</button>
  </div>
</template>

<style scoped>
.stepper {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  background: var(--bg-subtle);
  border-radius: var(--r-full);
  padding: 3px;
}
.stepper-btn {
  width: 30px; height: 30px;
  border: none;
  border-radius: 50%;
  background: var(--surface);
  color: var(--brand-600);
  font-size: 18px; font-weight: 600;
  line-height: 1;
  cursor: pointer;
  box-shadow: var(--sh-xs);
  transition: transform var(--dur-fast) var(--ease), opacity var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
.stepper-btn:active { transform: scale(.88); }
.stepper-btn:disabled { opacity: .3; pointer-events: none; }
.stepper-value {
  min-width: 30px;
  text-align: center;
  font-size: 15px; font-weight: 700;
  color: var(--text-1);
}
.stepper--sm .stepper-btn { width: 26px; height: 26px; font-size: 16px; }
.stepper--sm .stepper-value { min-width: 24px; font-size: 13px; }
</style>
