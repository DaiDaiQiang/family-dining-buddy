<script setup>
/**
 * 空状态占位
 */
defineProps({
  emoji: { type: String, default: '🍽️' },
  title: { type: String, default: '这里还空空的' },
  desc: { type: String, default: '' }
})
</script>

<template>
  <div class="empty">
    <div class="empty-emoji">{{ emoji }}</div>
    <p class="empty-title">{{ title }}</p>
    <p v-if="desc" class="empty-desc">{{ desc }}</p>
    <div v-if="$slots.default" class="empty-action">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 56px 24px;
  gap: 6px;
}
.empty-emoji {
  font-size: 56px;
  opacity: .5;
  margin-bottom: 6px;
  animation: empty-float 3s ease-in-out infinite;
}
@keyframes empty-float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-6px); }
}
.empty-title { font-size: 15px; font-weight: 600; color: var(--text-2); }
.empty-desc { font-size: 13px; color: var(--text-3); max-width: 260px; line-height: 1.6; }
.empty-action { margin-top: 14px; }
</style>
