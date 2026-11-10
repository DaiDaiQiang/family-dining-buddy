<script setup>
/**
 * 耗材维护与消耗统计（阶段 3）
 *
 * 家庭日用耗材（洗洁精、纸巾、调味料…）的库存维护、消耗登记与统计。
 * 库存低于预警阈值时，在首页滚动栏与提醒中心提示补货。
 */
import { computed, onActivated, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useConsumableStore } from '@/stores/consumable'
import { useAppStore } from '@/stores/app'
import { toast } from '@/composables/useToast'

defineOptions({ name: 'ConsumableView' })

const router = useRouter()
const consumable = useConsumableStore()
const app = useAppStore()

const CATEGORIES = ['清洁', '纸品', '调味料', '个护', '其他']
const EMOJIS = ['🧴', '🧻', '🧂', '🧼', '🧻', '🥄', '🫧', '🧽', '🛒', '🧴']

const loading = ref(false)
const tab = ref('list') // list / stats

// ---------- 新增 / 编辑耗材 ----------
const editing = ref(false)
const editId = ref(null)
const saving = ref(false)
const form = reactive({
  name: '',
  category: '清洁',
  unit: '瓶',
  emoji: '🧴',
  stock: 0,
  lowThreshold: 0,
  note: ''
})

// ---------- 消耗登记 ----------
const consuming = ref(false)
const consumeId = ref(null)
const consumingSaving = ref(false)
const consumeForm = reactive({
  quantity: 1,
  memberId: null,
  note: ''
})
const consumeTarget = computed(() =>
  consumable.items.find((c) => c.id === consumeId.value)
)

function openCreate() {
  editId.value = null
  Object.assign(form, { name: '', category: '清洁', unit: '瓶', emoji: '🧴', stock: 0, lowThreshold: 0, note: '' })
  editing.value = true
}

function openEdit(c) {
  editId.value = c.id
  Object.assign(form, {
    name: c.name,
    category: c.category,
    unit: c.unit,
    emoji: c.emoji,
    stock: Number(c.stock),
    lowThreshold: Number(c.low_threshold),
    note: c.note || ''
  })
  editing.value = true
}

async function save() {
  if (!form.name.trim()) {
    toast.warn('请填写耗材名称')
    return
  }
  saving.value = true
  try {
    const payload = {
      name: form.name.trim(),
      category: form.category,
      unit: form.unit.trim() || '个',
      emoji: form.emoji,
      stock: Number(form.stock) || 0,
      low_threshold: Number(form.lowThreshold) || 0,
      note: form.note.trim() || null
    }
    if (editId.value) {
      await consumable.update(editId.value, payload)
      toast.success('已更新')
    } else {
      await consumable.create(payload)
      toast.success('耗材已添加')
    }
    editing.value = false
  } catch (e) {
    toast.error(e.message || '保存失败')
  } finally {
    saving.value = false
  }
}

async function removeItem(c) {
  if (!confirm(`停用「${c.name}」？\n停用后不再出现在列表，历史消耗记录保留。`)) return
  try {
    await consumable.remove(c.id)
    toast.success('已停用')
  } catch (e) {
    toast.error(e.message || '停用失败')
  }
}

// ---------- 消耗登记 ----------
function openConsume(c) {
  consumeId.value = c.id
  Object.assign(consumeForm, { quantity: c.unit === '个' || c.unit === '包' || c.unit === '卷' || c.unit === '盒' ? 1 : 1, memberId: app.currentMemberId, note: '' })
  consuming.value = true
}

async function saveConsume() {
  const qty = Number(consumeForm.quantity)
  if (!qty || qty <= 0) {
    toast.warn('请填写消耗数量')
    return
  }
  consumingSaving.value = true
  try {
    const res = await consumable.consume(consumeId.value, {
      quantity: qty,
      member_id: consumeForm.memberId || null,
      note: consumeForm.note.trim() || null
    })
    toast.success(`已登记消耗 ${qty} ${consumeTarget.value?.unit || ''}`)
    consuming.value = false
    // 若扣到预警线以下，友情提示
    if (res?.stock !== undefined && consumeTarget.value && res.stock <= consumeTarget.value.low_threshold && consumeTarget.value.low_threshold > 0) {
      toast.warn('已到补货预警线，记得及时补充', 3000)
    }
  } catch (e) {
    toast.error(e.message || '登记失败')
  } finally {
    consumingSaving.value = false
  }
}

/** 库存状态：正常 / 预警 / 用尽 */
function stockState(c) {
  if (c.low_threshold > 0 && Number(c.stock) <= 0) return 'empty'
  if (c.low_threshold > 0 && Number(c.stock) <= Number(c.low_threshold)) return 'low'
  return 'ok'
}

const lowCount = computed(() => consumable.lowCount)

// ---------- 统计 ----------
const stats = ref(null)
async function loadStats() {
  stats.value = await consumable.loadStats()
}

// 近 30 天折线图：归一化到 SVG 视口
const chartPoints = computed(() => {
  const daily = stats.value?.daily || []
  if (!daily.length) return []
  const max = Math.max(...daily.map((d) => Number(d.quantity)), 0.001)
  const W = 320
  const H = 120
  const pad = 6
  return daily.map((d, i) => {
    const x = pad + (i / (daily.length - 1)) * (W - pad * 2)
    const y = H - pad - (Number(d.quantity) / max) * (H - pad * 2)
    return { x, y, qty: Number(d.quantity), day: d.day.slice(5) }
  })
})
const chartPolyline = computed(() => chartPoints.value.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(' '))

const money = (n) => Number(n || 0)

onActivated(async () => {
  loading.value = true
  try {
    await consumable.load()
  } catch (e) {
    toast.error(e.message || '加载失败')
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="page page--flat consumable">
    <div class="page-body">
      <header class="p-head">
        <h1 class="p-title">🧴 耗材管理</h1>
        <p class="p-sub">洗洁精、纸巾、调味料……用完了随手记，缺货早知道</p>
      </header>

      <!-- 概览 -->
      <section class="ov-card">
        <div class="ov-main">
          <span class="ov-num num">{{ consumable.items.length }}</span>
          <span class="ov-label">种在库耗材</span>
        </div>
        <div class="ov-side">
          <div class="ov-item" :class="{ 'ov-item--alert': lowCount }">
            <span class="ov-item-num num">{{ lowCount }}</span>
            <span class="ov-item-label">待补货</span>
          </div>
          <div class="ov-divider" />
          <div class="ov-item">
            <span class="ov-item-num num">{{ money(stats?.month_count ?? 0) }}</span>
            <span class="ov-item-label">本月消耗笔数</span>
          </div>
        </div>
      </section>

      <!-- 操作 -->
      <div class="action-row">
        <button class="btn btn--primary grow" @click="openCreate">＋ 添加耗材</button>
        <button class="btn btn--ghost" :class="{ on: tab === 'stats' }" @click="tab = tab === 'stats' ? 'list' : 'stats'; if (tab === 'stats') loadStats()">
          {{ tab === 'stats' ? '返回清单' : '消耗统计' }}
        </button>
      </div>

      <!-- 统计面板 -->
      <section v-if="tab === 'stats'" class="section">
        <div class="stats-grid">
          <div class="stat-card">
            <span class="stat-num num">{{ money(stats?.total_quantity) }}</span>
            <span class="stat-label">累计消耗总量</span>
          </div>
          <div class="stat-card">
            <span class="stat-num num">{{ money(stats?.month_quantity) }}</span>
            <span class="stat-label">本月消耗</span>
          </div>
          <div class="stat-card">
            <span class="stat-num num">{{ money(stats?.last30_quantity) }}</span>
            <span class="stat-label">近 30 天消耗</span>
          </div>
          <div class="stat-card">
            <span class="stat-num num">{{ money(stats?.total_count) }}</span>
            <span class="stat-label">累计登记笔数</span>
          </div>
        </div>

        <div class="chart-card">
          <p class="chart-title">近 30 天消耗趋势</p>
          <svg v-if="chartPoints.length" class="chart" viewBox="0 0 320 120" preserveAspectRatio="none">
            <polyline :points="chartPolyline" fill="none" stroke="var(--brand-500)" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
            <circle v-for="(p, i) in chartPoints" :key="i" :cx="p.x" :cy="p.y" r="2.5" fill="var(--brand-500)" />
          </svg>
          <p v-else class="chart-empty">还没有消耗记录，先记几笔吧</p>
        </div>
      </section>

      <!-- 耗材清单 -->
      <section v-else class="section">
        <div class="section-head">
          <h2 class="section-title">在库清单</h2>
          <span v-if="lowCount" class="section-more chip chip--danger">{{ lowCount }} 种待补货</span>
        </div>

        <div v-if="consumable.items.length" class="rec-list stagger">
          <div
            v-for="c in consumable.items"
            :key="c.id"
            class="cd"
            :class="`cd--${stockState(c)}`"
          >
            <div class="cd-top">
              <span class="cd-emoji">{{ c.emoji }}</span>
              <div class="cd-head">
                <p class="cd-name ellipsis">
                  {{ c.name }}
                  <span v-if="c.note" class="cd-note">· {{ c.note }}</span>
                </p>
                <p class="cd-meta">{{ c.category }} · 单位 {{ c.unit }}</p>
              </div>
              <span class="chip cd-status" :class="stockState(c) === 'ok' ? 'chip--success' : 'chip--danger'">
                {{ stockState(c) === 'empty' ? '已用尽' : stockState(c) === 'low' ? '待补货' : '充足' }}
              </span>
            </div>

            <div class="cd-stock">
              <div class="cd-stock-main">
                <span class="cd-stock-num num">{{ money(c.stock) }}</span>
                <span class="cd-stock-unit">{{ c.unit }}</span>
                <span v-if="c.low_threshold > 0" class="cd-stock-th">预警线 {{ money(c.low_threshold) }}</span>
              </div>
              <button class="btn btn--soft btn--sm" @click="openConsume(c)">登记消耗</button>
            </div>

            <div class="cd-foot">
              <button class="pa" @click="openEdit(c)">编辑</button>
              <button class="pa" @click="openConsume(c)">＋ 消耗</button>
              <button class="pa pa--del" @click="removeItem(c)">停用</button>
            </div>
          </div>
        </div>

        <div v-else class="empty">
          <span class="empty-emoji">🧺</span>
          <p class="empty-title">还没有耗材</p>
          <p class="empty-desc">把家里常用的洗洁精、纸巾、调味料加进来，用完了随手记一笔</p>
          <button class="btn btn--primary btn--sm" @click="openCreate">＋ 添加第一个耗材</button>
        </div>
      </section>

      <p class="p-tip">库存低于预警线时，会在首页滚动栏和提醒中心提示补货</p>

      <div class="bottom-space" />
    </div>

    <!-- 新增 / 编辑耗材弹层 -->
    <transition name="sheet">
      <div v-if="editing" class="sheet-mask" @click.self="editing = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>{{ editId ? '编辑耗材' : '添加耗材' }}</h2>
            <button class="sheet-close" @click="editing = false">✕</button>
          </div>
          <div class="sheet-body">
            <div class="field">
              <label class="field-label">名称 <span class="req">*</span></label>
              <input v-model="form.name" class="input" placeholder="如 洗洁精、卷纸、生抽" maxlength="64" />
            </div>

            <div class="field">
              <label class="field-label">分类</label>
              <div class="seg seg--wrap">
                <button
                  v-for="c in CATEGORIES"
                  :key="c"
                  class="seg-btn"
                  :class="{ on: form.category === c }"
                  @click="form.category = c"
                >{{ c }}</button>
              </div>
            </div>

            <div class="field">
              <label class="field-label">图标</label>
              <div class="emoji-row">
                <button
                  v-for="e in EMOJIS"
                  :key="e"
                  class="emoji-opt"
                  :class="{ on: form.emoji === e }"
                  @click="form.emoji = e"
                >{{ e }}</button>
              </div>
            </div>

            <div class="field-row">
              <div class="field grow">
                <label class="field-label">单位</label>
                <input v-model="form.unit" class="input" placeholder="瓶 / 包 / 卷 / 个" maxlength="16" />
              </div>
              <div class="field grow">
                <label class="field-label">当前库存</label>
                <input v-model.number="form.stock" class="input num" type="number" min="0" step="0.5" />
              </div>
            </div>

            <div class="field">
              <label class="field-label">补货预警线</label>
              <input v-model.number="form.lowThreshold" class="input num" type="number" min="0" step="0.5" placeholder="低于此数量时提醒补货" />
              <p class="field-hint">如洗洁精剩 1 瓶就提醒，预警线填 1；填 0 表示不提醒</p>
            </div>

            <div class="field">
              <label class="field-label">备注</label>
              <input v-model="form.note" class="input" placeholder="品牌、规格等（选填）" maxlength="255" />
            </div>
          </div>
          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="editing = false">取消</button>
            <button class="btn btn--primary grow" :disabled="saving" @click="save">{{ saving ? '保存中…' : '保存' }}</button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 消耗登记弹层 -->
    <transition name="sheet">
      <div v-if="consuming" class="sheet-mask" @click.self="consuming = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>{{ consumeTarget?.emoji }} 登记消耗</h2>
            <button class="sheet-close" @click="consuming = false">✕</button>
          </div>
          <div class="sheet-body">
            <div class="consume-target">
              <span class="ct-name">{{ consumeTarget?.name }}</span>
              <span class="ct-stock num">当前库存 {{ money(consumeTarget?.stock) }} {{ consumeTarget?.unit }}</span>
            </div>

            <div class="field">
              <label class="field-label">消耗数量 <span class="req">*</span></label>
              <input v-model.number="consumeForm.quantity" class="input num" type="number" min="0" step="0.5" />
            </div>

            <div v-if="app.activeMembers.length" class="field">
              <label class="field-label">谁登记的</label>
              <div class="seg seg--wrap">
                <button class="seg-btn" :class="{ on: !consumeForm.memberId }" @click="consumeForm.memberId = null">不指定</button>
                <button
                  v-for="m in app.activeMembers"
                  :key="m.id"
                  class="seg-btn"
                  :class="{ on: consumeForm.memberId === m.id }"
                  @click="consumeForm.memberId = m.id"
                >{{ m.avatar_emoji }} {{ m.nickname }}</button>
              </div>
            </div>

            <div class="field">
              <label class="field-label">用途备注</label>
              <input v-model="consumeForm.note" class="input" placeholder="如 洗碗、擦桌子（选填）" maxlength="255" />
            </div>
          </div>
          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="consuming = false">取消</button>
            <button class="btn btn--primary grow" :disabled="consumingSaving" @click="saveConsume">{{ consumingSaving ? '登记中…' : '登记' }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.p-head { padding: calc(var(--safe-top) + 56px) 0 16px; }
.p-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.p-sub { font-size: 13px; color: var(--text-3); margin-top: 5px; }

.ov-card {
  display: flex; align-items: center; justify-content: space-between;
  background: var(--surface); border-radius: var(--r-lg);
  padding: 18px 20px; box-shadow: var(--sh-sm); border: 1px solid var(--border);
}
.ov-main { display: flex; align-items: baseline; gap: 8px; }
.ov-num { font-size: 34px; font-weight: 800; color: var(--brand-600); }
.ov-label { font-size: 13px; color: var(--text-3); }
.ov-side { display: flex; align-items: center; gap: 18px; }
.ov-item { display: flex; flex-direction: column; align-items: center; gap: 2px; }
.ov-item-num { font-size: 20px; font-weight: 800; }
.ov-item--alert .ov-item-num { color: var(--danger); }
.ov-item-label { font-size: 11px; color: var(--text-3); }
.ov-divider { width: 1px; height: 28px; background: var(--border); }

.action-row { display: flex; gap: 10px; margin-top: 14px; }
.action-row .btn--ghost.on { border-color: var(--brand-500); color: var(--brand-600); background: var(--brand-50); }

/* 统计 */
.stats-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; }
.stat-card {
  background: var(--surface); border: 1px solid var(--border); border-radius: var(--r-lg);
  padding: 16px; box-shadow: var(--sh-xs);
  display: flex; flex-direction: column; gap: 4px;
}
.stat-num { font-size: 24px; font-weight: 800; color: var(--brand-600); }
.stat-label { font-size: 12px; color: var(--text-3); }
.chart-card {
  margin-top: 12px; background: var(--surface); border: 1px solid var(--border);
  border-radius: var(--r-lg); padding: 16px; box-shadow: var(--sh-sm);
}
.chart-title { font-size: 14px; font-weight: 700; margin-bottom: 10px; }
.chart { width: 100%; height: 120px; display: block; }
.chart-empty { font-size: 13px; color: var(--text-3); text-align: center; padding: 30px 0; }

/* 清单 */
.rec-list { display: flex; flex-direction: column; gap: 11px; }
.cd {
  background: var(--surface); border: 1px solid var(--border); border-radius: var(--r-lg);
  padding: 14px 16px; box-shadow: var(--sh-sm);
}
.cd--low { border-left: 4px solid var(--warning); }
.cd--empty { border-left: 4px solid var(--danger); }
.cd--ok { border-left: 4px solid var(--brand-200); }
.cd-top { display: flex; align-items: flex-start; gap: 11px; }
.cd-emoji {
  width: 40px; height: 40px; flex: none; border-radius: var(--r-md);
  background: var(--brand-50); border: 1px solid var(--brand-100);
  display: flex; align-items: center; justify-content: center; font-size: 22px;
}
.cd-head { flex: 1; min-width: 0; }
.cd-name { font-size: 15.5px; font-weight: 700; }
.cd-note { font-size: 12px; font-weight: 500; color: var(--text-3); }
.cd-meta { font-size: 11.5px; color: var(--text-3); margin-top: 2px; }
.cd-status { flex: none; font-size: 11px; }

.cd-stock { display: flex; align-items: center; justify-content: space-between; margin-top: 12px; padding-top: 12px; border-top: 1px dashed var(--border); }
.cd-stock-main { display: flex; align-items: baseline; gap: 5px; }
.cd-stock-num { font-size: 22px; font-weight: 800; color: var(--text-1); }
.cd--low .cd-stock-num, .cd--empty .cd-stock-num { color: var(--danger); }
.cd-stock-unit { font-size: 12px; color: var(--text-3); }
.cd-stock-th { font-size: 11px; color: var(--text-4); margin-left: 4px; }

.cd-foot { display: flex; gap: 6px; margin-top: 10px; justify-content: flex-end; }
.pa {
  padding: 6px 12px; border: none; background: var(--bg-subtle); border-radius: var(--r-sm);
  font-size: 12.5px; font-weight: 600; color: var(--text-2); cursor: pointer; font-family: inherit;
}
.pa--del { color: var(--danger); background: var(--danger-bg); }

.empty { text-align: center; padding: 44px 20px; background: var(--surface); border-radius: var(--r-lg); border: 1px dashed var(--border-strong); }
.empty-emoji { font-size: 38px; }
.empty-title { font-size: 15px; font-weight: 700; margin-top: 10px; }
.empty-desc { font-size: 12.5px; color: var(--text-3); margin-top: 4px; line-height: 1.5; margin-bottom: 14px; }

.p-tip { margin-top: 18px; font-size: 11.5px; color: var(--text-4); text-align: center; line-height: 1.5; }
.bottom-space { height: 20px; }

/* 弹层 */
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

.field-row { display: flex; gap: 10px; align-items: flex-start; }
.field-row .field { flex: none; }
.field-row .field.grow { flex: 1; }

.emoji-row { display: flex; flex-wrap: wrap; gap: 8px; }
.emoji-opt {
  width: 40px; height: 40px; border-radius: var(--r-md); border: 2px solid transparent;
  background: var(--bg-subtle); font-size: 20px; cursor: pointer; display: flex; align-items: center; justify-content: center;
  transition: all var(--dur-fast);
}
.emoji-opt.on { border-color: var(--brand-500); background: var(--brand-50); }

.consume-target {
  display: flex; align-items: baseline; justify-content: space-between;
  background: var(--brand-50); border-radius: var(--r-md); padding: 12px 14px; margin-bottom: 16px;
}
.ct-name { font-size: 15px; font-weight: 700; color: var(--brand-700); }
.ct-stock { font-size: 12px; color: var(--text-2); }

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }
</style>
