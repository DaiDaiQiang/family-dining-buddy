<script setup>
/**
 * 站内提醒中心（阶段 2）
 *
 * 两类自动生成提醒：经期推算（health）、快递超时未取（parcel）。
 * 操作：完成 / 忽略 / 推迟 1 天。页签切换 待办 / 已完成 / 已忽略。
 */
import { computed, onMounted, ref } from 'vue'
import { useReminderStore } from '@/stores/reminder'
import { toast } from '@/composables/useToast'

const store = useReminderStore()
const tab = ref('pending')
const acting = ref(false)

const TYPE_META = {
  health: { label: '健康', emoji: '🌸', color: '#E91E63' },
  parcel: { label: '快递', emoji: '📦', color: '#FF8A65' },
  consumable: { label: '耗材', emoji: '🧴', color: '#C2703D' },
  bill: { label: '账单', emoji: '💰', color: '#C98A1E' },
  device: { label: '设备', emoji: '🔌', color: '#4F8A5B' },
  other: { label: '其他', emoji: '📌', color: '#6E7F96' }
}

const tabs = computed(() => [
  { key: 'pending', label: '待办', count: store.pending.length },
  { key: 'done', label: '已完成', count: store.done.length },
  { key: 'dismissed', label: '已忽略', count: store.dismissed.length }
])

const list = computed(() =>
  tab.value === 'pending' ? store.pending
    : tab.value === 'done' ? store.done
    : store.dismissed
)

function meta(type) {
  return TYPE_META[type] || TYPE_META.other
}

function dueText(dueAt) {
  if (!dueAt) return ''
  const d = new Date(dueAt)
  const now = new Date()
  const days = Math.floor((d.setHours(0, 0, 0, 0) - now.setHours(0, 0, 0, 0)) / 86400000)
  if (days < 0) return `逾期 ${-days} 天`
  if (days === 0) return '今天'
  if (days === 1) return '明天'
  return `${days} 天后`
}

async function act(fn, okMsg) {
  if (acting.value) return
  acting.value = true
  const res = await fn()
  acting.value = false
  if (res.ok) toast.success(okMsg)
  else toast.error(res.error || '操作失败')
}

const markDone = (r) => act(() => store.setStatus(r.id, 'done'), '已完成')
const dismiss = (r) => act(() => store.setStatus(r.id, 'dismissed'), '已忽略')
const reopen = (r) => act(() => store.setStatus(r.id, 'pending'), '已重新打开')
const postpone = (r) => act(() => store.postpone(r.id, 1), '已推迟 1 天')

onMounted(() => store.load())
</script>

<template>
  <div class="rc">
    <header class="rc-head">
      <h1 class="rc-title">🔔 提醒中心</h1>
      <p class="rc-sub font-kai">经期推算 · 快递超时 · 耗材补货，站内自动提醒</p>
      <button class="rc-sync" :disabled="store.loading" @click="store.load()">
        {{ store.loading ? '同步中…' : '同步生成' }}
      </button>
    </header>

    <!-- 页签 -->
    <nav class="rc-tabs">
      <button
        v-for="t in tabs"
        :key="t.key"
        class="rc-tab"
        :class="{ on: tab === t.key }"
        @click="tab = t.key"
      >
        {{ t.label }}
        <span v-if="t.count" class="rc-tab-n">{{ t.count }}</span>
      </button>
    </nav>

    <!-- 列表 -->
    <ul v-if="list.length" class="rc-list">
      <li
        v-for="r in list"
        :key="r.id"
        class="rc-item"
        :style="{ '--mc': meta(r.type).color }"
      >
        <div class="rc-emoji">{{ meta(r.type).emoji }}</div>
        <div class="rc-body">
          <p class="rc-item-title">{{ r.title }}</p>
          <p class="rc-item-meta">
            <span class="rc-tag">{{ meta(r.type).label }}</span>
            <span :class="{ overdue: dueText(r.due_at).startsWith('逾期') }">
              {{ dueText(r.due_at) }}
            </span>
          </p>
        </div>
        <div class="rc-actions">
          <template v-if="r.status === 'pending'">
            <button class="rc-btn rc-btn--ok" :disabled="acting" @click="markDone(r)">✓ 完成</button>
            <button class="rc-btn" :disabled="acting" @click="postpone(r)">推迟</button>
            <button class="rc-btn rc-btn--mute" :disabled="acting" @click="dismiss(r)">忽略</button>
          </template>
          <button v-else class="rc-btn" :disabled="acting" @click="reopen(r)">重新打开</button>
        </div>
      </li>
    </ul>

    <div v-else class="rc-empty">
      <div class="rc-empty-emoji">{{ tab === 'pending' ? '🍵' : '🗂️' }}</div>
      <p>{{ tab === 'pending' ? '没有待办提醒，岁月静好' : '这里还没有记录' }}</p>
      <p v-if="tab === 'pending'" class="rc-empty-tip">
        提醒由系统自动生成：经期预计开始、快递到件超 2 天未取、耗材库存低于预警线
      </p>
    </div>
  </div>
</template>

<style scoped>
.rc { padding: 20px var(--page-pad) calc(28px + var(--safe-bottom)); }

/* 头部 */
.rc-head { position: relative; margin-bottom: 18px; }
.rc-title { font-family: var(--font-display); font-size: 22px; font-weight: 800; color: var(--text-1); }
.rc-sub { margin-top: 4px; font-size: 12.5px; color: var(--text-3); letter-spacing: .08em; }
.rc-sync {
  position: absolute; top: 2px; right: 0;
  padding: 7px 14px;
  border: 1.5px solid var(--border);
  border-radius: var(--r-full);
  background: var(--surface);
  color: var(--text-2);
  font-size: 12.5px; font-weight: 600;
  cursor: pointer;
}
.rc-sync:disabled { opacity: .55; }

/* 页签 */
.rc-tabs { display: flex; gap: 8px; margin-bottom: 14px; }
.rc-tab {
  padding: 8px 16px;
  border: 1.5px solid var(--border);
  border-radius: var(--r-full);
  background: var(--surface);
  color: var(--text-2);
  font-size: 13.5px; font-weight: 600;
  cursor: pointer;
}
.rc-tab.on {
  background: var(--brand-gradient);
  border-color: transparent;
  color: #fff;
  box-shadow: var(--sh-brand);
}
.rc-tab-n {
  display: inline-block;
  min-width: 18px;
  margin-left: 4px;
  padding: 0 5px;
  border-radius: var(--r-full);
  background: color-mix(in srgb, currentColor 14%, transparent);
  font-size: 11px;
}

/* 列表 */
.rc-list { display: flex; flex-direction: column; gap: 10px; }
.rc-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 13px 14px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-left: 4px solid var(--mc, var(--brand-500));
  border-radius: var(--r-md);
}
.rc-emoji {
  flex: none;
  width: 42px; height: 42px;
  display: flex; align-items: center; justify-content: center;
  border-radius: var(--r-sm);
  font-size: 22px;
  background: color-mix(in srgb, var(--mc, #C2703D) 12%, white);
}
.rc-body { flex: 1; min-width: 0; }
.rc-item-title { font-size: 14px; font-weight: 700; color: var(--text-1); line-height: 1.45; }
.rc-item-meta {
  display: flex; align-items: center; gap: 8px;
  margin-top: 4px;
  font-size: 11.5px; color: var(--text-3);
}
.rc-tag {
  padding: 1px 8px;
  border-radius: var(--r-full);
  background: color-mix(in srgb, var(--mc, #C2703D) 12%, white);
  color: var(--mc, var(--brand-600));
  font-weight: 600;
}
.overdue { color: var(--danger); font-weight: 700; }

.rc-actions { display: flex; flex-direction: column; gap: 6px; }
.rc-btn {
  padding: 5px 12px;
  border: 1.5px solid var(--border);
  border-radius: var(--r-full);
  background: var(--surface-2);
  color: var(--text-2);
  font-size: 11.5px; font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
}
.rc-btn:disabled { opacity: .5; }
.rc-btn--ok {
  background: color-mix(in srgb, var(--brand-500) 12%, white);
  border-color: color-mix(in srgb, var(--brand-500) 35%, white);
  color: var(--brand-700);
}
.rc-btn--mute { color: var(--text-4); }

/* 空态 */
.rc-empty { padding: 56px 20px; text-align: center; }
.rc-empty-emoji { font-size: 42px; }
.rc-empty p { margin-top: 12px; font-size: 14px; color: var(--text-3); }
.rc-empty-tip { margin-top: 6px; font-size: 11.5px; color: var(--text-4); line-height: 1.7; }
</style>
