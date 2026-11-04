<script setup>
import { useRouter } from 'vue-router'
const router = useRouter()
</script>

<template>
  <div class="page page--flat nf">
    <div class="nf-body">
      <div class="nf-emoji">🍜</div>
      <h1 class="nf-title">这道菜还没上桌</h1>
      <p class="nf-desc">你访问的页面走丢了，回首页看看今天吃什么吧</p>
      <button class="btn btn--primary" @click="router.replace('/')">回到首页</button>
    </div>
  </div>
</template>

<style scoped>
.nf { display: flex; align-items: center; justify-content: center; min-height: 100vh; }
.nf-body { text-align: center; padding: 24px; }
.nf-emoji { font-size: 72px; margin-bottom: 16px; animation: nf-bob 2.4s ease-in-out infinite; }
@keyframes nf-bob { 0%, 100% { transform: translateY(0) rotate(-5deg); } 50% { transform: translateY(-10px) rotate(5deg); } }
.nf-title { font-size: 22px; font-weight: 800; }
.nf-desc { font-size: 14px; color: var(--text-3); margin: 10px 0 24px; }
</style>
