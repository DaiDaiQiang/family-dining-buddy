<script setup>
/**
 * 快递取件码
 *
 * 把驿站 / 快递柜发来的取件码存一处，取件时直接照着念或复制。
 * 待取的排在最前，已取的沉底，支持一键复制待取清单转给家人代取。
 */
import { computed, reactive, ref } from 'vue'
import {
  useParcelStore,
  COURIERS,
  STATION_TYPES,
  PARCEL_STATUS,
  courierMeta,
  stationMeta,
  parcelStatusMeta,
  formatCode
} from '@/stores/parcel'
import { useAppStore } from '@/stores/app'
import { toast } from '@/composables/useToast'
import { toDateStr } from '@/utils/cycle'

defineOptions({ name: 'ParcelView' })

const parcel = useParcelStore()
const app = useAppStore()

const todayStr = toDateStr(new Date())

const editing = ref(false)
const editId = ref(null)
const saving = ref(false)
/** 当前筛选的状态，'all' 为全部 */
const filter = ref('pending')

const form = reactive({
  pickupCode: '',
  courierCode: 'shunfeng',
  station: '',
  stationType: 'cainiao',
  shelfNo: '',
  trackingNo: '',
  note: '',
  memberId: null,
  receivedAt: todayStr
})

const FILTERS = [
  { value: 'pending', label: '待取', count: () => parcel.pendingCount },
  { value: 'picked', label: '已取', count: () => parcel.pickedCount },
  { value: 'all', label: '全部', count: () => parcel.records.length }
]

const list = computed(() => {
  const base = filter.value === 'pending' ? parcel.sorted.filter((r) => r.status !== 'picked') : parcel.filtered(filter.value)
  return base
})

const memberName = (id) => app.members.find((m) => m.id === Number(id))?.nickname || ''
const memberEmoji = (id) => app.members.find((m) => m.id === Number(id))?.avatar_emoji || ''

/** 状态对应的 chip 配色 */
function toneClass(tone) {
  return { warn: 'chip--warning', danger: 'chip--danger', muted: '' }[tone] ?? ''
}

function openCreate() {
  editId.value = null
  Object.assign(form, {
    pickupCode: '',
    courierCode: 'shunfeng',
    station: '',
    stationType: 'cainiao',
    shelfNo: '',
    trackingNo: '',
    note: '',
    memberId: app.currentMemberId,
    receivedAt: todayStr
  })
  editing.value = true
}

function openEdit(r) {
  editId.value = r.id
  Object.assign(form, {
    pickupCode: r.pickup_code,
    courierCode: r.courier_code,
    station: r.station || '',
    stationType: r.station_type || 'cainiao',
    shelfNo: r.shelf_no || '',
    trackingNo: r.tracking_no || '',
    note: r.note || '',
    memberId: r.member_id,
    receivedAt: r.received_at || todayStr
  })
  editing.value = true
}

async function save() {
  if (!form.pickupCode.trim()) {
    toast.warn('请填写取件码')
    return
  }
  saving.value = true
  try {
    const payload = { ...form }
    if (editId.value) {
      await parcel.update(editId.value, payload)
      toast.success('已更新')
    } else {
      await parcel.add(payload)
      toast.success('取件码已记下')
    }
    editing.value = false
  } catch (e) {
    toast.error(e.message || '保存失败')
  } finally {
    saving.value = false
  }
}

async function removeRecord(r) {
  const label = `${courierMeta(r.courier_code).label} · 取件码 ${r.pickup_code}`
  if (!confirm(`确定删除这条取件码吗？\n${label}`)) return
  try {
    await parcel.remove(r.id)
    toast.success('已删除')
  } catch (e) {
    toast.error(e.message || '删除失败')
  }
}

async function togglePicked(r) {
  try {
    if (r.status === 'picked') {
      await parcel.reopen(r.id)
      toast.info('已退回待取')
    } else {
      await parcel.markPicked(r.id)
      toast.success('已标记取件')
    }
  } catch (e) {
    toast.error(e.message || '操作失败')
  }
}

async function clearPicked() {
  if (!parcel.pickedCount) return
  if (!confirm(`清除 ${parcel.pickedCount} 条已取记录？`)) return
  const n = await parcel.clearPicked()
  toast.success(`已清除 ${n} 条`)
}

/** 复制文本，兼容非 HTTPS 环境下 navigator.clipboard 不可用的情况 */
async function copyText(text, okMsg) {
  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text)
    } else {
      const ta = document.createElement('textarea')
      ta.value = text
      ta.setAttribute('readonly', '')
      ta.style.position = 'fixed'
      ta.style.opacity = '0'
      document.body.appendChild(ta)
      ta.select()
      const done = document.execCommand('copy')
      document.body.removeChild(ta)
      if (!done) throw new Error('copy rejected')
    }
    toast.success(okMsg)
  } catch {
    toast.error('复制失败，请手动选中取件码')
  }
}

function copyCode(r) {
  copyText(r.pickup_code, `已复制取件码 ${r.pickup_code}`)
}

function copyAllPending() {
  if (!parcel.pendingText) {
    toast.info('没有待取的快递')
    return
  }
  copyText(parcel.pendingText, '待取清单已复制')
}
</script>

<template>
  <div class="page page--flat parcel">
    <div class="page-body">
      <header class="p-head">
        <h1 class="p-title">📦 快递取件码</h1>
        <p class="p-sub">驿站、快递柜的取件码存一处，取件不用再翻短信</p>
      </header>

      <!-- 概览 -->
      <section class="ov-card">
        <div class="ov-main">
          <span class="ov-num num">{{ parcel.pendingCount }}</span>
          <span class="ov-label">件待取</span>
        </div>
        <div class="ov-side">
          <div class="ov-item">
            <span class="ov-item-num num">{{ parcel.expiredCount }}</span>
            <span class="ov-item-label">已超时</span>
          </div>
          <div class="ov-divider" />
          <div class="ov-item">
            <span class="ov-item-num num">{{ parcel.pickedCount }}</span>
            <span class="ov-item-label">已取</span>
          </div>
        </div>
      </section>

      <!-- 操作 -->
      <div class="action-row">
        <button class="btn btn--primary grow" @click="openCreate">＋ 记一个取件码</button>
        <button class="btn btn--ghost" :disabled="!parcel.pendingCount" @click="copyAllPending">复制待取清单</button>
      </div>

      <!-- 筛选 -->
      <div class="seg filter-seg">
        <button
          v-for="f in FILTERS"
          :key="f.value"
          class="seg-btn"
          :class="{ on: filter === f.value }"
          @click="filter = f.value"
        >
          {{ f.label }}
          <span class="seg-count num">{{ f.count() }}</span>
        </button>
      </div>

      <!-- 列表 -->
      <section class="section">
        <div class="section-head">
          <h2 class="section-title">{{ filter === 'picked' ? '已取记录' : '取件清单' }}</h2>
          <button v-if="parcel.pickedCount" class="section-more" @click="clearPicked">清除已取</button>
        </div>

        <div v-if="list.length" class="rec-list stagger">
          <div v-for="r in list" :key="r.id" class="pc" :class="{ 'pc--done': r.status === 'picked' }">
            <div class="pc-top">
              <span class="pc-emoji">{{ courierMeta(r.courier_code).emoji }}</span>
              <div class="pc-head">
                <p class="pc-name ellipsis">
                  {{ courierMeta(r.courier_code).label }}
                  <span v-if="r.station" class="pc-station ellipsis">· {{ r.station }}</span>
                </p>
                <p class="pc-meta">
                  {{ stationMeta(r.station_type).label }}
                  <template v-if="r.shelf_no"> · 货架 {{ r.shelf_no }}</template>
                  <template v-if="r.member_id"> · {{ memberEmoji(r.member_id) }} {{ memberName(r.member_id) }}</template>
                </p>
              </div>
              <span class="chip pc-status" :class="toneClass(parcelStatusMeta(r.status).tone)">
                {{ parcelStatusMeta(r.status).emoji }} {{ parcelStatusMeta(r.status).label }}
              </span>
            </div>

            <!-- 取件码单独一行大字，取件时一眼能看清 -->
            <div class="code-row">
              <span class="code-text num">{{ formatCode(r.pickup_code) }}</span>
              <button class="btn btn--soft btn--sm" @click.stop="copyCode(r)">复制</button>
            </div>

            <p v-if="r.note" class="pc-note ellipsis-2">📝 {{ r.note }}</p>

            <div class="pc-foot">
              <span class="pc-date">
                {{ r.status === 'picked' && r.picked_at ? `已取 ${r.picked_at.slice(5, 16)}` : `${r.received_at} 到件` }}
              </span>
              <div class="pc-acts">
                <button class="pa" @click="openEdit(r)">编辑</button>
                <button class="pa" :class="{ 'pa--ok': r.status !== 'picked' }" @click="togglePicked(r)">
                  {{ r.status === 'picked' ? '退回待取' : '标记已取' }}
                </button>
                <button class="pa pa--del" @click="removeRecord(r)">删除</button>
              </div>
            </div>
          </div>
        </div>

        <div v-else class="empty">
          <span class="empty-emoji">📭</span>
          <p class="empty-title">{{ filter === 'picked' ? '还没有已取记录' : '暂时没有待取的快递' }}</p>
          <p class="empty-desc">收到取件短信后点上面按钮记下来，就不会忘在驿站了</p>
        </div>
      </section>

      <p class="p-tip">数据保存在本机，换设备或清除浏览器数据后不会同步</p>

      <div class="bottom-space" />
    </div>

    <!-- 新增 / 编辑弹层 -->
    <transition name="sheet">
      <div v-if="editing" class="sheet-mask" @click.self="editing = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>{{ editId ? '编辑取件码' : '记一个取件码' }}</h2>
            <button class="sheet-close" @click="editing = false">✕</button>
          </div>

          <div class="sheet-body">
            <div class="field">
              <label class="field-label">取件码 <span class="req">*</span></label>
              <input
                v-model="form.pickupCode"
                class="input code-input num"
                placeholder="如 3-2-1024"
                maxlength="32"
                autocomplete="off"
              />
              <p class="field-hint">短信里的取件码原样填入即可，长短、横杠都不用管</p>
            </div>

            <div class="field">
              <label class="field-label">快递公司</label>
              <div class="opt-grid">
                <button
                  v-for="c in COURIERS"
                  :key="c.code"
                  class="opt"
                  :class="{ on: form.courierCode === c.code }"
                  @click="form.courierCode = c.code"
                >
                  <span class="opt-emoji">{{ c.emoji }}</span>
                  <span class="opt-label">{{ c.label }}</span>
                </button>
              </div>
            </div>

            <div class="field">
              <label class="field-label">自提点类型</label>
              <div class="seg seg--wrap">
                <button
                  v-for="s in STATION_TYPES"
                  :key="s.value"
                  class="seg-btn"
                  :class="{ on: form.stationType === s.value }"
                  @click="form.stationType = s.value"
                >{{ s.emoji }} {{ s.label }}</button>
              </div>
            </div>

            <div class="field-row">
              <div class="field grow">
                <label class="field-label">驿站名称</label>
                <input v-model="form.station" class="input" placeholder="如 3 号楼菜鸟驿站" maxlength="40" />
              </div>
              <div class="field">
                <label class="field-label">货架号</label>
                <input v-model="form.shelfNo" class="input shelf-input" placeholder="选填" maxlength="20" />
              </div>
            </div>

            <div class="field">
              <label class="field-label">快递单号</label>
              <input
                v-model="form.trackingNo"
                class="input num"
                placeholder="选填，填了将来可查物流轨迹"
                maxlength="40"
                autocomplete="off"
              />
              <p class="field-hint">留空不影响使用，只是后续对接物流查询时需要它</p>
            </div>

            <div class="field">
              <label class="field-label">到件日期</label>
              <input v-model="form.receivedAt" class="input" type="date" :max="todayStr" />
            </div>

            <div v-if="app.activeMembers.length" class="field">
              <label class="field-label">谁的快递</label>
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
              <input v-model="form.note" class="input" placeholder="如 冰箱里的冻虾、给娃的绘本" maxlength="60" />
            </div>
          </div>

          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="editing = false">取消</button>
            <button class="btn btn--primary grow" :disabled="saving" @click="save">
              {{ saving ? '保存中…' : '保存' }}
            </button>
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

/* 概览 */
.ov-card {
  display: flex; align-items: center; gap: 16px;
  background: var(--surface);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  border: 1px solid var(--border);
  padding: 16px;
}
.ov-main { flex: none; display: flex; align-items: baseline; gap: 5px; }
.ov-num { font-size: 34px; font-weight: 800; color: var(--brand-600); line-height: 1; }
.ov-label { font-size: 13px; color: var(--text-2); font-weight: 600; }
.ov-side { flex: 1; display: flex; align-items: center; justify-content: flex-end; gap: 14px; }
.ov-item { display: flex; flex-direction: column; align-items: center; gap: 2px; }
.ov-item-num { font-size: 16px; font-weight: 700; }
.ov-item-label { font-size: 11.5px; color: var(--text-3); }
.ov-divider { width: 1px; height: 24px; background: var(--border); }

/* 操作 + 筛选 */
.action-row { display: flex; gap: 10px; margin-top: 14px; }
.filter-seg { margin-top: 14px; }
.seg-count {
  display: inline-block; margin-left: 4px;
  font-size: 11px; opacity: .65;
}

/* 卡片列表 */
.rec-list { display: flex; flex-direction: column; gap: 11px; }
.pc {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  padding: 14px;
}
.pc--done { opacity: .58; }

.pc-top { display: flex; align-items: flex-start; gap: 11px; }
.pc-emoji {
  width: 40px; height: 40px; flex: none;
  border-radius: var(--r-md);
  background: var(--bg-subtle);
  display: flex; align-items: center; justify-content: center;
  font-size: 21px;
}
.pc-head { flex: 1; min-width: 0; }
.pc-name { font-size: 15px; font-weight: 700; }
.pc-station { font-size: 12.5px; font-weight: 500; color: var(--text-3); }
.pc-meta { font-size: 11.5px; color: var(--text-3); margin-top: 2px; }
.pc-status { flex: none; font-size: 11px; }

/* 取件码大字行 */
.code-row {
  display: flex; align-items: center; justify-content: space-between; gap: 10px;
  margin-top: 11px; padding: 10px 12px;
  background: var(--bg-subtle);
  border-radius: var(--r-md);
}
.code-text {
  font-size: 23px; font-weight: 800; letter-spacing: .04em;
  color: var(--text-1);
  word-break: break-all;
}
.pc--done .code-text { text-decoration: line-through; }

.pc-note { font-size: 12px; color: var(--text-3); margin-top: 9px; }

.pc-foot {
  display: flex; align-items: center; justify-content: space-between; gap: 10px;
  margin-top: 11px; padding-top: 10px;
  border-top: 1px dashed var(--border);
}
.pc-date { font-size: 11.5px; color: var(--text-4); }
.pc-acts { display: flex; gap: 6px; flex: none; }
.pa {
  padding: 5px 11px;
  border-radius: var(--r-full);
  border: 1px solid var(--border-strong);
  background: var(--surface);
  color: var(--text-2);
  font-size: 12px; font-weight: 600;
  cursor: pointer; font-family: inherit;
  transition: all var(--dur-fast);
}
.pa:active { background: var(--bg-subtle); }
.pa--ok { border-color: var(--brand-300); color: var(--brand-600); background: var(--brand-50); }
.pa--del:active { background: var(--danger-bg); color: var(--danger); border-color: transparent; }

/* 空态 */
.empty { text-align: center; padding: 44px 20px; background: var(--surface); border-radius: var(--r-lg); border: 1px dashed var(--border-strong); }
.empty-emoji { font-size: 38px; }
.empty-title { font-size: 15px; font-weight: 700; margin-top: 10px; }
.empty-desc { font-size: 12.5px; color: var(--text-3); margin-top: 4px; line-height: 1.5; }

.p-tip { margin-top: 18px; font-size: 11.5px; color: var(--text-4); text-align: center; line-height: 1.5; }
.bottom-space { height: 20px; }

/* 弹层（与账本页一致） */
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

.code-input { font-size: 22px; font-weight: 800; letter-spacing: .04em; min-height: 54px; }

.field-row { display: flex; gap: 10px; align-items: flex-start; }
.field-row .field { flex: none; }
.field-row .field.grow { flex: 1; }
.shelf-input { width: 104px; }

.opt-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
.opt { display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 10px 4px; border-radius: var(--r-sm); border: 2px solid transparent; background: var(--bg-subtle); cursor: pointer; font-family: inherit; transition: all var(--dur-fast); }
.opt.on { border-color: var(--brand-500); background: var(--brand-50); }
.opt-emoji { font-size: 20px; }
.opt-label { font-size: 11px; font-weight: 600; color: var(--text-2); text-align: center; line-height: 1.3; }
.opt.on .opt-label { color: var(--brand-600); }

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }

@media (min-width: 700px) {
  .opt-grid { grid-template-columns: repeat(5, 1fr); }
}
</style>
