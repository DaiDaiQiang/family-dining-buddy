<script setup>
/**
 * 家庭账本 · 记账
 *
 * 记录日常收支：买菜、购物、外食、生活开销等。
 * 支持按月份查看明细、分类占比与收支结余。
 */
import { computed, reactive, ref } from 'vue'
import { useLedgerStore, LEDGER_CATEGORIES, ledgerTypeMeta, ledgerCategoryMeta } from '@/stores/ledger'
import { useAppStore } from '@/stores/app'
import { toast } from '@/composables/useToast'
import { relativeDay } from '@/utils/format'
import { toDateStr } from '@/utils/cycle'

defineOptions({ name: 'LedgerView' })

const ledger = useLedgerStore()
const app = useAppStore()

/** 当前查看的月份 'YYYY-MM' */
const month = ref(toDateStr(new Date()).slice(0, 7))
const editing = ref(false)
const editId = ref(null)
const saving = ref(false)

const todayStr = toDateStr(new Date())

const form = reactive({
  type: 'expense',
  amount: '',
  category: 'grocery',
  note: '',
  date: todayStr,
  memberId: null
})

/** 各分类归属：工资/奖金归收入，其余归支出 */
const incomeCategories = ['salary', 'bonus', 'other']
const categoriesByType = computed(() =>
  form.type === 'income'
    ? LEDGER_CATEGORIES.filter((c) => incomeCategories.includes(c.value))
    : LEDGER_CATEGORIES.filter((c) => !incomeCategories.includes(c.value))
)

const summary = computed(() => ledger.summaryOf(month.value))
const groups = computed(() => ledger.groupedByDate(month.value))
const breakdown = computed(() => (month.value === toDateStr(new Date()).slice(0, 7) ? ledger.categoryBreakdown : []))

const monthTitle = computed(() => {
  const [y, m] = month.value.split('-')
  return `${Number(y)} 年 ${Number(m)} 月`
})

const memberName = (id) => app.members.find((m) => m.id === Number(id))?.nickname || ''
const memberEmoji = (id) => app.members.find((m) => m.id === Number(id))?.avatar_emoji || ''
const money = (n) => Number(n || 0).toFixed(2)

function shiftMonth(step) {
  const [y, m] = month.value.split('-').map(Number)
  const d = new Date(y, m - 1 + step, 1)
  month.value = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`
}

function openCreate(type = 'expense') {
  editId.value = null
  Object.assign(form, {
    type,
    amount: '',
    category: type === 'income' ? 'salary' : 'grocery',
    note: '',
    date: todayStr,
    memberId: app.currentMemberId
  })
  editing.value = true
}

function openEdit(r) {
  editId.value = r.id
  Object.assign(form, {
    type: r.type,
    amount: String(r.amount),
    category: r.category,
    note: r.note || '',
    date: r.date,
    memberId: r.member_id
  })
  editing.value = true
}

/** 切换收支类型时，把分类纠正到该类型下 */
function switchType(t) {
  form.type = t
  if (!categoriesByType.value.some((c) => c.value === form.category)) {
    form.category = t === 'income' ? 'salary' : 'grocery'
  }
}

async function save() {
  const amount = Number(form.amount)
  if (!Number.isFinite(amount) || amount <= 0) {
    toast.warn('请填写大于 0 的金额')
    return
  }
  saving.value = true
  try {
    const payload = {
      type: form.type,
      amount,
      category: form.category,
      note: form.note,
      date: form.date,
      memberId: form.memberId
    }
    if (editId.value) {
      await ledger.update(editId.value, payload)
      toast.success('已更新')
    } else {
      await ledger.add(payload)
      toast.success('已记一笔')
    }
    month.value = (form.date || todayStr).slice(0, 7)
    editing.value = false
  } catch (e) {
    toast.error(e.message || '保存失败')
  } finally {
    saving.value = false
  }
}

async function removeRecord(r) {
  const label = `${ledgerCategoryMeta(r.category).label} ¥${money(r.amount)}`
  if (!confirm(`确定删除这笔记录吗？\n${label}`)) return
  try {
    await ledger.remove(r.id)
    toast.success('已删除')
  } catch (e) {
    toast.error(e.message || '删除失败')
  }
}

/** 快捷金额，减少输入 */
const QUICK_AMOUNTS = [5, 10, 20, 50, 100]
function addQuick(v) {
  const cur = Number(form.amount) || 0
  form.amount = String(Math.round((cur + v) * 100) / 100)
}
</script>

<template>
  <div class="page page--flat ledger">
    <div class="page-body">
      <header class="l-head">
        <h1 class="l-title">📒 家庭账本</h1>
        <p class="l-sub">买菜、购物、生活开销，随手记一笔</p>
      </header>

      <!-- 月份切换 + 汇总 -->
      <section class="month-card">
        <div class="month-nav">
          <button class="mnav" aria-label="上个月" @click="shiftMonth(-1)">‹</button>
          <span class="month-name num">{{ monthTitle }}</span>
          <button class="mnav" :disabled="month >= todayStr.slice(0, 7)" aria-label="下个月" @click="shiftMonth(1)">›</button>
        </div>

        <div class="sum-row">
          <div class="sum">
            <span class="sum-label">支出</span>
            <span class="sum-value sum-value--out num">¥{{ money(summary.expense) }}</span>
          </div>
          <div class="sum-divider" />
          <div class="sum">
            <span class="sum-label">收入</span>
            <span class="sum-value sum-value--in num">¥{{ money(summary.income) }}</span>
          </div>
          <div class="sum-divider" />
          <div class="sum">
            <span class="sum-label">结余</span>
            <span class="sum-value num" :class="summary.balance < 0 ? 'sum-value--out' : ''">¥{{ money(summary.balance) }}</span>
          </div>
        </div>

        <!-- 分类占比（仅当月） -->
        <div v-if="breakdown.length" class="breakdown">
          <p class="bd-title">本月支出构成</p>
          <div v-for="b in breakdown.slice(0, 5)" :key="b.value" class="bd-row">
            <span class="bd-emoji">{{ b.emoji }}</span>
            <span class="bd-label ellipsis">{{ b.label }}</span>
            <span class="bd-bar"><i :style="{ width: b.percent + '%' }" /></span>
            <span class="bd-amount num">¥{{ money(b.amount) }}</span>
          </div>
        </div>
      </section>

      <!-- 记一笔 -->
      <div class="action-row">
        <button class="btn btn--primary grow" @click="openCreate('expense')">＋ 记一笔支出</button>
        <button class="btn btn--ghost" @click="openCreate('income')">记收入</button>
      </div>

      <!-- 明细列表 -->
      <section class="section">
        <div class="section-head">
          <h2 class="section-title">明细</h2>
          <span class="section-more">{{ summary.count }} 笔</span>
        </div>

        <div v-if="groups.length" class="day-groups stagger">
          <div v-for="g in groups" :key="g.date" class="day-group">
            <div class="day-head">
              <span class="day-date">{{ relativeDay(g.date) }}</span>
              <span class="day-sum num">
                <template v-if="g.income">收 ¥{{ money(g.income) }}</template>
                <template v-if="g.income && g.expense"> · </template>
                <template v-if="g.expense">支 ¥{{ money(g.expense) }}</template>
              </span>
            </div>
            <div class="rec-list">
              <div
                v-for="r in g.list"
                :key="r.id"
                class="rec clickable"
                @click="openEdit(r)"
              >
                <span class="rec-emoji">{{ ledgerCategoryMeta(r.category).emoji }}</span>
                <div class="rec-info">
                  <p class="rec-name ellipsis">{{ r.note || ledgerCategoryMeta(r.category).label }}</p>
                  <p class="rec-meta">
                    {{ ledgerCategoryMeta(r.category).label }}
                    <template v-if="r.member_id"> · {{ memberEmoji(r.member_id) }} {{ memberName(r.member_id) }}</template>
                  </p>
                </div>
                <span class="rec-amount num" :class="r.type === 'income' ? 'in' : 'out'">
                  {{ r.type === 'income' ? '+' : '-' }}{{ money(r.amount) }}
                </span>
                <button class="rec-del" aria-label="删除" @click.stop="removeRecord(r)">✕</button>
              </div>
            </div>
          </div>
        </div>

        <div v-else class="empty">
          <span class="empty-emoji">🧾</span>
          <p class="empty-title">这个月还没有记录</p>
          <p class="empty-desc">点上面的按钮，记下第一笔开销吧</p>
        </div>
      </section>

      <div class="bottom-space" />
    </div>

    <!-- 记一笔弹层 -->
    <transition name="sheet">
      <div v-if="editing" class="sheet-mask" @click.self="editing = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>{{ editId ? '编辑记录' : '记一笔' }}</h2>
            <button class="sheet-close" @click="editing = false">✕</button>
          </div>

          <div class="sheet-body">
            <div class="field">
              <label class="field-label">类型</label>
              <div class="seg">
                <button
                  v-for="t in [{ value: 'expense', label: '💸 支出' }, { value: 'income', label: '💰 收入' }]"
                  :key="t.value"
                  class="seg-btn"
                  :class="{ on: form.type === t.value }"
                  @click="switchType(t.value)"
                >{{ t.label }}</button>
              </div>
            </div>

            <div class="field">
              <label class="field-label">金额 <span class="req">*</span></label>
              <div class="amount-input">
                <span class="amount-yuan">¥</span>
                <input
                  v-model="form.amount"
                  class="input amount-field num"
                  type="number"
                  inputmode="decimal"
                  step="0.01"
                  min="0"
                  placeholder="0.00"
                />
              </div>
              <div class="quick-amounts">
                <button v-for="q in QUICK_AMOUNTS" :key="q" class="qa" @click="addQuick(q)">+{{ q }}</button>
              </div>
            </div>

            <div class="field">
              <label class="field-label">分类</label>
              <div class="cat-grid">
                <button
                  v-for="c in categoriesByType"
                  :key="c.value"
                  class="cat"
                  :class="{ on: form.category === c.value }"
                  @click="form.category = c.value"
                >
                  <span class="cat-emoji">{{ c.emoji }}</span>
                  <span class="cat-label">{{ c.label }}</span>
                </button>
              </div>
            </div>

            <div class="field">
              <label class="field-label">日期</label>
              <input v-model="form.date" class="input" type="date" :max="todayStr" />
            </div>

            <div v-if="app.activeMembers.length" class="field">
              <label class="field-label">记在谁名下</label>
              <div class="seg seg--wrap">
                <button class="seg-btn" :class="{ on: !form.memberId }" @click="form.memberId = null">不指定</button>
                <button
                  v-for="m in app.activeMembers"
                  :key="m.id"
                  class="seg-btn"
                  :class="{ on: form.memberId === m.id }"
                  @click="form.memberId = m.id"
                >{{ m.avatar_emoji }} {{ m.nickname }}</button>
              </div>
            </div>

            <div class="field">
              <label class="field-label">备注</label>
              <input v-model="form.note" class="input" placeholder="如：菜市场买排骨、超市纸巾" maxlength="60" />
            </div>
          </div>

          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="editing = false">取消</button>
            <button class="btn btn--primary grow" :disabled="saving" @click="save">{{ saving ? '保存中…' : '保存' }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.l-head { padding: calc(var(--safe-top) + 56px) 0 16px; }
.l-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.l-sub { font-size: 13px; color: var(--text-3); margin-top: 5px; }

/* 月份卡 */
.month-card {
  background: var(--surface);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  border: 1px solid var(--border);
  padding: 16px;
}
.month-nav { display: flex; align-items: center; justify-content: center; gap: 18px; }
.mnav {
  width: 30px; height: 30px; flex: none;
  border-radius: 50%; border: 1px solid var(--border-strong);
  background: var(--surface); color: var(--text-2);
  font-size: 18px; line-height: 1; cursor: pointer;
  transition: all var(--dur-fast);
}
.mnav:disabled { opacity: .35; cursor: not-allowed; }
.mnav:not(:disabled):active { background: var(--brand-50); color: var(--brand-600); }
.month-name { font-size: 15px; font-weight: 700; }

.sum-row { display: flex; align-items: center; margin-top: 16px; padding-top: 14px; border-top: 1px dashed var(--border); }
.sum { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 3px; }
.sum-label { font-size: 11.5px; color: var(--text-3); }
.sum-value { font-size: 17px; font-weight: 800; }
.sum-value--out { color: var(--danger); }
.sum-value--in { color: var(--success); }
.sum-divider { width: 1px; height: 26px; background: var(--border); }

/* 分类占比 */
.breakdown { margin-top: 16px; padding-top: 14px; border-top: 1px dashed var(--border); }
.bd-title { font-size: 12px; font-weight: 700; color: var(--text-3); margin-bottom: 9px; }
.bd-row { display: flex; align-items: center; gap: 8px; margin-bottom: 7px; }
.bd-emoji { font-size: 14px; flex: none; }
.bd-label { width: 46px; flex: none; font-size: 12px; color: var(--text-2); }
.bd-bar { flex: 1; height: 7px; border-radius: var(--r-full); background: var(--bg-subtle); overflow: hidden; }
.bd-bar i { display: block; height: 100%; border-radius: var(--r-full); background: var(--brand-gradient); }
.bd-amount { width: 66px; flex: none; text-align: right; font-size: 12px; font-weight: 600; }

/* 操作 */
.action-row { display: flex; gap: 10px; margin-top: 14px; }

/* 明细 */
.day-groups { display: flex; flex-direction: column; gap: 14px; }
.day-head { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 7px; }
.day-date { font-size: 13px; font-weight: 700; color: var(--text-2); }
.day-sum { font-size: 11.5px; color: var(--text-3); }

.rec-list { background: var(--surface); border-radius: var(--r-lg); box-shadow: var(--sh-sm); padding: 5px; border: 1px solid var(--border); }
.rec { display: flex; align-items: center; gap: 11px; padding: 10px 8px; border-radius: var(--r-md); transition: background var(--dur-fast); }
.rec:active { background: var(--bg-subtle); }
.rec-emoji { width: 34px; height: 34px; flex: none; border-radius: var(--r-sm); background: var(--bg-subtle); display: flex; align-items: center; justify-content: center; font-size: 18px; }
.rec-info { flex: 1; min-width: 0; }
.rec-name { font-size: 14.5px; font-weight: 600; }
.rec-meta { font-size: 11.5px; color: var(--text-3); margin-top: 1px; }
.rec-amount { flex: none; font-size: 15px; font-weight: 700; }
.rec-amount.out { color: var(--danger); }
.rec-amount.in { color: var(--success); }
.rec-del { flex: none; width: 26px; height: 26px; border-radius: 50%; border: none; background: transparent; color: var(--text-4); font-size: 12px; cursor: pointer; transition: all var(--dur-fast); }
.rec-del:active { background: var(--danger-bg); color: var(--danger); }

/* 空态 */
.empty { text-align: center; padding: 44px 20px; background: var(--surface); border-radius: var(--r-lg); border: 1px dashed var(--border-strong); }
.empty-emoji { font-size: 38px; }
.empty-title { font-size: 15px; font-weight: 700; margin-top: 10px; }
.empty-desc { font-size: 12.5px; color: var(--text-3); margin-top: 4px; }

.bottom-space { height: 20px; }

/* 弹层（与成员页一致） */
.sheet-mask { position: fixed; inset: 0; z-index: 100; background: rgba(0,0,0,.4); display: flex; align-items: flex-end; justify-content: center; }
.sheet { width: 100%; max-width: 560px; background: var(--surface); border-radius: var(--r-2xl) var(--r-2xl) 0 0; max-height: 90vh; display: flex; flex-direction: column; box-shadow: var(--sh-xl); }
.sheet-head { display: flex; align-items: center; justify-content: space-between; padding: 18px 20px 12px; }
.sheet-head h2 { font-size: 18px; font-weight: 700; }
.sheet-close { width: 32px; height: 32px; border-radius: 50%; border: none; background: var(--bg-subtle); color: var(--text-3); font-size: 15px; cursor: pointer; }
.sheet-body { padding: 0 20px; overflow-y: auto; flex: 1; }
.sheet-foot { display: flex; gap: 10px; padding: 14px 20px calc(14px + var(--safe-bottom)); border-top: 1px solid var(--border); }

.seg { display: flex; gap: 6px; background: var(--bg-subtle); padding: 4px; border-radius: var(--r-sm); }
.seg--wrap { flex-wrap: wrap; }
.seg-btn { flex: 1; padding: 9px; border: none; background: transparent; border-radius: var(--r-xs); font-size: 13px; font-weight: 600; color: var(--text-2); cursor: pointer; font-family: inherit; white-space: nowrap; }
.seg-btn.on { background: var(--surface); color: var(--brand-600); box-shadow: var(--sh-xs); }

.amount-input { position: relative; display: flex; align-items: center; }
.amount-yuan { position: absolute; left: 14px; font-size: 16px; font-weight: 700; color: var(--text-3); }
.amount-field { padding-left: 32px; font-size: 22px; font-weight: 800; min-height: 54px; }
.quick-amounts { display: flex; gap: 7px; margin-top: 9px; }
.qa { flex: 1; padding: 8px 0; border-radius: var(--r-sm); border: 1px solid var(--border-strong); background: var(--surface); color: var(--text-2); font-size: 13px; font-weight: 600; cursor: pointer; font-family: var(--font-num); transition: all var(--dur-fast); }
.qa:active { background: var(--brand-50); border-color: var(--brand-300); color: var(--brand-600); }

.cat-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
.cat { display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 10px 4px; border-radius: var(--r-sm); border: 2px solid transparent; background: var(--bg-subtle); cursor: pointer; font-family: inherit; transition: all var(--dur-fast); }
.cat.on { border-color: var(--brand-500); background: var(--brand-50); }
.cat-emoji { font-size: 21px; }
.cat-label { font-size: 11.5px; font-weight: 600; color: var(--text-2); }
.cat.on .cat-label { color: var(--brand-600); }

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }
</style>
