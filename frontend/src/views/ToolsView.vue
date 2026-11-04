<script setup>
/**
 * 常用功能
 *
 * 结构：顶部高频功能列表卡片（带数据摘要）＋「全部功能」网格（一行 3 个）。
 * 家庭成员管理仅管理员可见（非管理员仍可在成员页切换身份）。
 */
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useLedgerStore } from '@/stores/ledger'
import { useHealthStore } from '@/stores/health'
import { useParcelStore } from '@/stores/parcel'
import { useStudyStore } from '@/stores/study'
import { useReminderStore } from '@/stores/reminder'

defineOptions({ name: 'ToolsView' })

const router = useRouter()
const app = useAppStore()
const ledger = useLedgerStore()
const health = useHealthStore()
const parcel = useParcelStore()
const study = useStudyStore()
const reminder = useReminderStore()

/** 是否管理员（掌勺人）：家庭成员管理仅管理员可用 */
const isAdmin = computed(() => app.currentMember?.role === 'admin')

const monthLabel = computed(() => {
  const d = new Date()
  return `${d.getFullYear()} 年 ${d.getMonth() + 1} 月`
})

const money = (n) => Number(n || 0).toFixed(2)

const alertCount = computed(() => health.alerts.length)

/** 学习板块概览：错题数优先提示，其次显示内容总量 */
const studyStats = computed(() => study.stats)
const studyBadge = computed(() => {
  const s = studyStats.value
  if (s.questions.wrong) return `${s.questions.wrong} 道错题`
  const total = s.questions.total + s.english.total + s.notes.total
  return total ? `${total} 条内容` : ''
})

/** 顶部高频功能（列表样式，带数据摘要） */
const featured = computed(() => [
  {
    key: 'ledger',
    emoji: '📒',
    title: '记账',
    desc: '购物、买菜、生活开销随手记',
    to: '/tools/ledger',
    badge: ledger.records.length ? `${ledger.records.length} 笔` : '',
    summary: ledger.records.length
      ? `本月支出 ¥${money(ledger.monthSummary.expense)} · 本月结余 ¥${money(ledger.monthSummary.balance)}`
      : ''
  },
  {
    key: 'health',
    emoji: '🌸',
    title: '经期提醒',
    desc: '经期、排卵期、黄体期与经前 3 天提醒',
    to: '/tools/health',
    badge: alertCount.value ? `${alertCount.value} 条提醒` : '',
    badgeLevel: 'danger',
    summary: health.trackedStatuses.length ? `${health.trackedStatuses.length} 人关注 · ${monthLabel.value}预测中` : ''
  },
  {
    key: 'parcel',
    emoji: '📦',
    title: '快递取件码',
    desc: '驿站、快递柜的取件码存一处',
    to: '/tools/parcel',
    badge: parcel.pendingCount ? `${parcel.pendingCount} 件待取` : '',
    badgeLevel: 'danger',
    summary: parcel.records.length
      ? `待取 ${parcel.pendingCount} 件${parcel.expiredCount ? ` · 已超时 ${parcel.expiredCount} 件` : ''}`
      : ''
  },
  {
    key: 'reminders',
    emoji: '🔔',
    title: '提醒中心',
    desc: '经期推算、快递超时，站内自动提醒',
    to: '/tools/reminders',
    badge: reminder.pendingCount ? `${reminder.pendingCount} 条待办` : '',
    badgeLevel: reminder.pendingCount ? 'danger' : '',
    summary: reminder.pendingCount
      ? `健康提醒 ${reminder.countByType.health || 0} 条 · 快递提醒 ${reminder.countByType.parcel || 0} 条`
      : ''
  }
])

/** 全部功能（网格，一行 3 个）：家庭成员仅管理员可见 */
const allTools = computed(() => {
  const list = [
    { key: 'ledger', emoji: '📒', title: '记账', to: '/tools/ledger' },
    { key: 'health', emoji: '🌸', title: '经期提醒', to: '/tools/health' },
    { key: 'parcel', emoji: '📦', title: '快递取件码', to: '/tools/parcel' },
    {
      key: 'study',
      emoji: '🎓',
      title: '学习',
      to: '/tools/study',
      // 有错题亮红点角标，提示优先处理
      badge: studyStats.value.questions.wrong ? String(studyStats.value.questions.wrong) : '',
      badgeLevel: 'danger'
    },
    { key: 'stats', emoji: '📊', title: '家的味道', to: '/stats' },
    { key: 'order', emoji: '🛒', title: '订单', to: '/orders' },
    { key: 'recipes', emoji: '📖', title: '菜谱库', to: '/recipes' },
    { key: 'members', emoji: '👨‍👩‍👧', title: '家庭成员', to: '/members', adminOnly: true }
  ]
  return list.filter((t) => !t.adminOnly || isAdmin.value)
})
</script>

<template>
  <div class="page tools">
    <div class="page-body">
      <header class="t-head">
        <h1 class="t-title">🧰 常用功能</h1>
        <p class="t-sub">把家里的小账本和贴心提醒放在一处</p>
      </header>

      <!-- 顶部高频功能：列表卡片 -->
      <div class="featured-list stagger">
        <button
          v-for="t in featured"
          :key="t.key"
          class="tool-card clickable"
          @click="router.push(t.to)"
        >
          <div class="tool-top">
            <span class="tool-emoji">{{ t.emoji }}</span>
            <div class="tool-main">
              <div class="tool-line1">
                <p class="tool-title">{{ t.title }}</p>
                <span v-if="t.badge" class="tool-badge" :class="{ 'tool-badge--alert': t.badgeLevel === 'danger' }">
                  {{ t.badge }}
                </span>
              </div>
              <p class="tool-desc">{{ t.desc }}</p>
              <p v-if="t.summary" class="tool-summary num">{{ t.summary }}</p>
            </div>
            <span class="tool-arrow" aria-hidden="true">→</span>
          </div>
        </button>
      </div>

      <!-- 全部功能：一行 3 个网格 -->
      <section class="all-section">
        <div class="all-head">
          <h2 class="section-title">全部功能</h2>
        </div>
        <div class="all-grid stagger">
          <button
            v-for="t in allTools"
            :key="t.key"
            class="all-item clickable"
            @click="router.push(t.to)"
          >
            <span class="all-emoji">
              {{ t.emoji }}
              <span v-if="t.badge" class="all-badge" :class="{ 'all-badge--alert': t.badgeLevel === 'danger' }">
                {{ t.badge }}
              </span>
            </span>
            <span class="all-label">{{ t.title }}</span>
          </button>
        </div>
      </section>

      <p class="t-tip">应用内功能说明仅供参考</p>
    </div>
  </div>
</template>

<style scoped>
.t-head { padding: calc(var(--safe-top) + 20px) 0 18px; }
.t-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.t-sub { font-size: 13px; color: var(--text-3); margin-top: 5px; }

/* ---------- 顶部高频功能：列表卡片 ---------- */
.featured-list { display: flex; flex-direction: column; gap: 12px; }

.tool-card {
  width: 100%;
  text-align: left;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  padding: 16px;
  cursor: pointer;
  font-family: inherit;
  transition: transform var(--dur-fast) var(--ease), box-shadow var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
.tool-card:active { transform: scale(.98); box-shadow: var(--sh-md); }

.tool-top { display: flex; align-items: center; gap: 12px; }
.tool-emoji {
  width: 52px; height: 52px; flex: none;
  border-radius: var(--r-md);
  background: var(--brand-50);
  border: 1px solid var(--brand-100);
  display: flex; align-items: center; justify-content: center;
  font-size: 26px;
}
.tool-main { flex: 1; min-width: 0; }
.tool-line1 { display: flex; align-items: center; gap: 8px; }
.tool-title { font-size: 16.5px; font-weight: 700; color: var(--brand-700); }
.tool-desc { font-size: 12.5px; color: var(--text-2); margin-top: 3px; line-height: 1.45; }
.tool-summary { font-size: 12px; color: var(--text-3); margin-top: 4px; }

.tool-badge {
  flex: none;
  padding: 2px 9px;
  border-radius: var(--r-full);
  background: var(--brand-50);
  color: var(--brand-600);
  font-size: 11px;
  font-weight: 700;
  white-space: nowrap;
}
.tool-badge--alert { background: var(--danger-bg); color: var(--danger); }

.tool-arrow {
  flex: none;
  width: 30px; height: 30px;
  border-radius: var(--r-full);
  border: 1.5px solid var(--brand-200);
  color: var(--brand-500);
  font-size: 15px;
  display: flex; align-items: center; justify-content: center;
  background: var(--surface-2);
}

/* ---------- 全部功能：一行 3 个网格 ---------- */
.all-section { margin-top: var(--gap-2xl); }
.all-head { margin-bottom: var(--gap-md); }

.all-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}
.all-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 18px 6px 14px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-xs);
  cursor: pointer;
  font-family: inherit;
  transition: transform var(--dur-fast) var(--ease), box-shadow var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
.all-item:active { transform: scale(.95); box-shadow: var(--sh-md); }

.all-emoji {
  position: relative;
  width: 48px; height: 48px;
  border-radius: var(--r-md);
  background: var(--brand-50);
  display: flex; align-items: center; justify-content: center;
  font-size: 24px;
}
.all-badge {
  position: absolute;
  top: -6px; right: -6px;
  min-width: 16px; height: 16px;
  padding: 0 4px;
  border-radius: var(--r-full);
  background: var(--brand-500);
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  line-height: 16px;
  text-align: center;
  border: 1.5px solid var(--surface);
}
.all-badge--alert { background: var(--danger); }
.all-label { font-size: 12.5px; font-weight: 600; color: var(--text-1); }

.t-tip { margin-top: 18px; font-size: 11.5px; color: var(--text-4); text-align: center; line-height: 1.5; }

/* 平板以上：网格保持 3 列，卡片加宽留白 */
@media (min-width: 700px) {
  .all-item { padding: 22px 8px 18px; }
  .all-emoji { width: 54px; height: 54px; font-size: 27px; }
}
</style>
