<script setup>
/**
 * 经期与生理周期提醒
 *
 * 为女性家庭成员记录周期设置，推算月经期、排卵期、黄体期，
 * 并在「预计月经来潮前 3 天」起给出提醒。
 *
 * 说明：本页面提供的是基于周期长度的规律推算，不能替代医学诊断。
 */
import { computed, reactive, ref } from 'vue'
import { useHealthStore, ALERT_LEAD_DAYS } from '@/stores/health'
import { useAppStore } from '@/stores/app'
import { toast } from '@/composables/useToast'
import {
  PHASES,
  phaseMeta,
  computeCycleStatus,
  parseDate,
  toDateStr,
  addDays,
  daysBetween,
  reminderText
} from '@/utils/cycle'

defineOptions({ name: 'HealthView' })

const health = useHealthStore()
const app = useAppStore()

const editing = ref(false)
const editMember = ref(null)
const saving = ref(false)

const todayStr = toDateStr(new Date())

const form = reactive({
  gender: 'female',
  enabled: true,
  lastPeriodStart: '',
  cycleLength: 28,
  periodLength: 5
})

const GENDERS = [
  { value: 'female', label: '女', emoji: '👩' },
  { value: 'male', label: '男', emoji: '👨' },
  { value: 'unknown', label: '不设置', emoji: '🙂' }
]

/** 全部成员（含未设置性别的），便于逐一配置 */
const members = computed(() => app.activeMembers)

/** 已开启提醒并有完整设置的成员 */
const tracked = computed(() => health.trackedStatuses)

const alerts = computed(() => health.alerts)
const upcoming = computed(() => health.upcomingPeriods.slice(0, 12))

/** 提示用户：有女性成员还没填周期设置 */
const needSetup = computed(() =>
  health.femaleMembers.filter((m) => !health.cycles[m.id]?.lastPeriodStart)
)

function openEdit(m) {
  editMember.value = m
  const profile = health.cycles[m.id] || {}
  Object.assign(form, {
    gender: health.genderOf(m.id),
    enabled: profile.enabled !== false,
    lastPeriodStart: profile.lastPeriodStart || '',
    cycleLength: Number(profile.cycleLength) || 28,
    periodLength: Number(profile.periodLength) || 5
  })
  editing.value = true
}

async function save() {
  if (!editMember.value) return
  try {
    await health.setGender(editMember.value.id, form.gender)

    if (form.gender === 'female') {
      if (form.enabled && !parseDate(form.lastPeriodStart)) {
        toast.warn('请填写上次月经开始日')
        return
      }
      await health.saveCycle(editMember.value.id, {
        enabled: form.enabled,
        lastPeriodStart: form.enabled ? form.lastPeriodStart : '',
        cycleLength: form.cycleLength,
        periodLength: form.periodLength
      })
    }

    toast.success('已保存')
    editing.value = false
  } catch (e) {
    toast.error(e.message || '保存失败')
  }
}

/** 一键记录「今天来月经了」 */
async function markToday(m) {
  try {
    await health.setGender(m.id, 'female')
    await health.markPeriodStart(m.id, todayStr)
    toast.success(`已记录 ${m.nickname} 今天月经开始`)
  } catch (e) {
    toast.error(e.message || '记录失败')
  }
}

/** 阶段时间轴：按周期天数展开，标出各阶段区间 */
function phaseTimeline(status) {
  const total = status.cycleLength
  const segs = []
  const push = (phase, from, to) => {
    if (to >= from) segs.push({ phase, ...phaseMeta(phase), from, to, days: to - from + 1 })
  }
  // 月经期
  push('period', 1, status.periodLength)
  // 卵泡期：经期结束次日 ~ 易孕期前一天
  const fertileStartDay = status.ovulationDay - 5
  push('follicular', status.periodLength + 1, Math.min(fertileStartDay - 1, total))
  // 排卵期
  push('fertile', fertileStartDay, Math.min(status.ovulationDay + 4, total))
  // 黄体期
  const lutealStartDay = Math.min(status.ovulationDay + 5, total)
  push('luteal', lutealStartDay, total)
  return segs.sort((a, b) => a.from - b.from)
}

/** 时间轴上每段的百分比宽度 */
function segStyle(seg, status) {
  return { width: `${(seg.days / status.cycleLength) * 100}%` }
}

const PHASE_LEGEND = Object.entries(PHASES).map(([key, v]) => ({ key, ...v }))
</script>

<template>
  <div class="page page--flat health">
    <div class="page-body">
      <header class="h-head">
        <h1 class="h-title">🌸 经期提醒</h1>
        <p class="h-sub">按周期推算经期、排卵期与黄体期，月经来潮前 {{ ALERT_LEAD_DAYS }} 天开始提醒</p>
      </header>

      <!-- 今日提醒 -->
      <section v-if="alerts.length" class="section">
        <div class="section-head">
          <h2 class="section-title">今日提醒</h2>
        </div>
        <div class="alert-list stagger">
          <div v-for="a in alerts" :key="a.id" class="alert" :class="`alert--${a.level}`">
            <span class="alert-emoji">{{ a.emoji }}</span>
            <div class="alert-body">
              <p class="alert-title">{{ a.title }}</p>
              <p class="alert-text">{{ a.text }}</p>
            </div>
          </div>
        </div>
      </section>

      <!-- 未设置引导 -->
      <section v-if="needSetup.length" class="section">
        <div class="setup-tip">
          <p class="setup-text">
            <strong>{{ needSetup.map((m) => m.nickname).join('、') }}</strong>
            还没有填写周期设置，填好后才能预测与提醒。
          </p>
          <button class="btn btn--soft btn--sm" @click="openEdit(needSetup[0])">去设置</button>
        </div>
      </section>

      <!-- 成员周期卡片 -->
      <section class="section">
        <div class="section-head">
          <h2 class="section-title">周期档案</h2>
          <span class="section-more">{{ tracked.length }} / {{ members.length }} 人已开启</span>
        </div>

        <div v-if="members.length" class="mcards stagger">
          <div
            v-for="m in members"
            :key="m.id"
            class="mcard"
            :class="{ 'mcard--off': health.genderOf(m.id) !== 'female' || !health.cycles[m.id]?.enabled }"
            :style="{ '--mc': m.theme_color }"
          >
            <div class="mcard-top">
              <span class="mcard-avatar">{{ m.avatar_emoji }}</span>
              <div class="mcard-head">
                <p class="mcard-name ellipsis">{{ m.nickname }}</p>
                <p class="mcard-gender">
                  {{ GENDERS.find((g) => g.value === health.genderOf(m.id))?.label || '未设置' }}
                  <template v-if="health.genderOf(m.id) === 'female' && health.cycles[m.id]?.enabled">
                    · 已开启提醒
                  </template>
                </p>
              </div>
              <button class="mcard-edit" @click="openEdit(m)">设置</button>
            </div>

            <template v-if="health.statusOf(m.id) && health.cycles[m.id]?.enabled && health.genderOf(m.id) === 'female'">
              <div class="mcard-phase" :class="`tone-${health.statusOf(m.id).tone}`">
                <span class="phase-emoji">{{ health.statusOf(m.id).phaseEmoji }}</span>
                <div class="phase-info">
                  <p class="phase-label">{{ health.statusOf(m.id).phaseLabel }}</p>
                  <p class="phase-sub">
                    <template v-if="health.statusOf(m.id).periodDay">
                      第 {{ health.statusOf(m.id).periodDay }} / {{ health.statusOf(m.id).periodLength }} 天
                    </template>
                    <template v-else>
                      距下次月经 {{ health.statusOf(m.id).daysToNext }} 天
                    </template>
                    · 周期第 {{ health.statusOf(m.id).dayInCycle }} 天
                  </p>
                </div>
              </div>

              <!-- 时间轴 -->
              <div class="timeline">
                <div class="tl-bar">
                  <i
                    v-for="(seg, i) in phaseTimeline(health.statusOf(m.id))"
                    :key="i"
                    class="tl-seg"
                    :class="`tone-${seg.tone}`"
                    :style="segStyle(seg, health.statusOf(m.id))"
                    :title="`${seg.label} ${seg.from}-${seg.to} 天`"
                  />
                </div>
                <div class="tl-cursor" :style="{ left: `${((health.statusOf(m.id).dayInCycle - 0.5) / health.statusOf(m.id).cycleLength) * 100}%` }">
                  <span class="tl-dot" />
                </div>
                <div class="tl-labels">
                  <span>第 1 天</span>
                  <span>第 {{ health.statusOf(m.id).cycleLength }} 天</span>
                </div>
              </div>

              <div class="mcard-dates">
                <div class="dcell">
                  <span class="dcell-label">下次月经</span>
                  <span class="dcell-value num">{{ health.statusOf(m.id).nextStart.slice(5) }}</span>
                </div>
                <div class="dcell">
                  <span class="dcell-label">排卵日</span>
                  <span class="dcell-value num">{{ health.statusOf(m.id).ovulationDate.slice(5) }}</span>
                </div>
                <div class="dcell">
                  <span class="dcell-label">易孕期</span>
                  <span class="dcell-value num">{{ health.statusOf(m.id).fertileStart.slice(5) }}~{{ health.statusOf(m.id).fertileEnd.slice(5) }}</span>
                </div>
              </div>

              <p v-if="health.statusOf(m.id).predictNote" class="predict-note">⚠️ {{ health.statusOf(m.id).predictNote }}</p>

              <button class="btn btn--soft btn--sm btn--block mt-sm" @click="markToday(m)">
                🩸 记录今天月经开始
              </button>
            </template>

            <p v-else-if="health.genderOf(m.id) === 'female'" class="mcard-empty">尚未填写周期设置，点「设置」开始</p>
            <p v-else class="mcard-empty">不适用经期提醒</p>
          </div>
        </div>
      </section>

      <!-- 阶段图例 -->
      <section class="section">
        <div class="section-head">
          <h2 class="section-title">阶段说明</h2>
        </div>
        <div class="legend">
          <div v-for="p in PHASE_LEGEND" :key="p.key" class="legend-item">
            <span class="legend-dot" :class="`tone-${p.tone}`" />
            <div class="legend-body">
              <p class="legend-label">{{ p.emoji }} {{ p.label }}</p>
              <p class="legend-desc">
                <template v-if="p.key === 'period'">月经来潮，注意保暖与补铁</template>
                <template v-else-if="p.key === 'premenstrual'">预计来潮前 {{ ALERT_LEAD_DAYS }} 天，提前备好用品</template>
                <template v-else-if="p.key === 'fertile'">排卵日前后，受孕概率较高</template>
                <template v-else-if="p.key === 'luteal'">排卵后至下次月经前，情绪食欲易波动</template>
                <template v-else>经期结束至排卵前，状态相对平稳</template>
              </p>
            </div>
          </div>
        </div>
      </section>

      <!-- 未来经期预测 -->
      <section v-if="upcoming.length" class="section">
        <div class="section-head">
          <h2 class="section-title">未来经期</h2>
        </div>
        <div class="forecast">
          <div v-for="f in upcoming" :key="f.date" class="fc-row">
            <span class="fc-date num">{{ f.date.slice(5) }}</span>
            <span class="fc-week">{{ ['周日','周一','周二','周三','周四','周五','周六'][new Date(`${f.date}T00:00:00`).getDay()] }}</span>
            <span class="fc-names">
              <span v-for="x in f.members" :key="x.memberId" class="fc-chip">{{ x.emoji }} {{ x.name }}</span>
            </span>
            <span class="fc-in num">{{ daysBetween(new Date(), parseDate(f.date)) }} 天后</span>
          </div>
        </div>
      </section>

      <p class="disclaimer">
        以上为按周期长度做的规律推算，仅供参考，不能作为避孕或医疗依据；周期异常请及时就医。
      </p>

      <div class="bottom-space" />
    </div>

    <!-- 设置弹层 -->
    <transition name="sheet">
      <div v-if="editing && editMember" class="sheet-mask" @click.self="editing = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>{{ editMember.avatar_emoji }} {{ editMember.nickname }}</h2>
            <button class="sheet-close" @click="editing = false">✕</button>
          </div>

          <div class="sheet-body">
            <div class="field">
              <label class="field-label">性别</label>
              <div class="seg">
                <button
                  v-for="g in GENDERS"
                  :key="g.value"
                  class="seg-btn"
                  :class="{ on: form.gender === g.value }"
                  @click="form.gender = g.value"
                >{{ g.emoji }} {{ g.label }}</button>
              </div>
              <p class="field-hint">仅女性成员开启经期提醒</p>
            </div>

            <template v-if="form.gender === 'female'">
              <div class="field">
                <label class="field-label">开启经期提醒</label>
                <div class="switch-row">
                  <span class="switch-text">在首页滚动栏与本页显示提醒</span>
                  <button
                    class="switch"
                    :class="{ on: form.enabled }"
                    role="switch"
                    :aria-checked="form.enabled"
                    @click="form.enabled = !form.enabled"
                  >
                    <span class="switch-knob" />
                  </button>
                </div>
              </div>

              <template v-if="form.enabled">
                <div class="field">
                  <label class="field-label">上次月经开始日 <span class="req">*</span></label>
                  <input v-model="form.lastPeriodStart" class="input" type="date" :max="todayStr" />
                  <p class="field-hint">填最近一次月经来的第一天，之后每次来潮再更新一次会更准</p>
                </div>

                <div class="field">
                  <label class="field-label">周期长度：<strong class="num">{{ form.cycleLength }}</strong> 天</label>
                  <input v-model.number="form.cycleLength" class="input" type="number" min="15" max="60" step="1" />
                  <p class="field-hint">两次月经开始日之间的天数，常见 21~35 天</p>
                </div>

                <div class="field">
                  <label class="field-label">经期长度：<strong class="num">{{ form.periodLength }}</strong> 天</label>
                  <input v-model.number="form.periodLength" class="input" type="number" min="1" max="15" step="1" />
                </div>

                <!-- 实时预览 -->
                <div v-if="parseDate(form.lastPeriodStart)" class="preview-box">
                  <p class="preview-title">推算预览</p>
                  <div class="preview-grid">
                    <div class="pv">
                      <span class="pv-label">今天处于</span>
                      <span class="pv-value">{{ computeCycleStatus(form).phaseEmoji }} {{ computeCycleStatus(form).phaseLabel }}</span>
                    </div>
                    <div class="pv">
                      <span class="pv-label">下次月经</span>
                      <span class="pv-value num">{{ computeCycleStatus(form).nextStart }}</span>
                    </div>
                    <div class="pv">
                      <span class="pv-label">排卵日</span>
                      <span class="pv-value num">{{ computeCycleStatus(form).ovulationDate }}</span>
                    </div>
                    <div class="pv">
                      <span class="pv-label">易孕期</span>
                      <span class="pv-value num">{{ computeCycleStatus(form).fertileStart.slice(5) }} ~ {{ computeCycleStatus(form).fertileEnd.slice(5) }}</span>
                    </div>
                  </div>
                  <p class="preview-tip">{{ reminderText(computeCycleStatus(form), editMember.nickname) }}</p>
                </div>
              </template>
            </template>
          </div>

          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="editing = false">取消</button>
            <button class="btn btn--primary grow" :disabled="saving" @click="save">保存</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.h-head { padding: calc(var(--safe-top) + 56px) 0 16px; }
.h-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.h-sub { font-size: 13px; color: var(--text-3); margin-top: 5px; line-height: 1.5; }

/* 阶段色调 */
.tone-danger { --tone: var(--danger); --tone-bg: var(--danger-bg); }
.tone-warning { --tone: #B8770F; --tone-bg: var(--warning-bg); }
.tone-success { --tone: var(--success); --tone-bg: var(--success-bg); }
.tone-info { --tone: var(--info); --tone-bg: var(--info-bg); }
.tone-brand { --tone: var(--brand-600); --tone-bg: var(--brand-50); }

/* 提醒 */
.alert-list { display: flex; flex-direction: column; gap: 10px; }
.alert {
  display: flex; align-items: center; gap: 12px;
  padding: 13px 15px;
  border-radius: var(--r-lg);
  background: var(--tone-bg, var(--bg-subtle));
  border-left: 4px solid var(--tone, var(--brand-500));
}
.alert--danger { --tone: var(--danger); --tone-bg: var(--danger-bg); }
.alert--warning { --tone: #B8770F; --tone-bg: var(--warning-bg); }
.alert--success { --tone: var(--success); --tone-bg: var(--success-bg); }
.alert--info { --tone: var(--info); --tone-bg: var(--info-bg); }
.alert-emoji { font-size: 24px; flex: none; }
.alert-body { flex: 1; min-width: 0; }
.alert-title { font-size: 14.5px; font-weight: 700; color: var(--tone); }
.alert-text { font-size: 12.5px; color: var(--text-2); margin-top: 2px; line-height: 1.45; }

/* 未设置引导 */
.setup-tip {
  display: flex; align-items: center; gap: 12px;
  padding: 13px 15px;
  background: var(--warning-bg);
  border-radius: var(--r-lg);
}
.setup-text { flex: 1; font-size: 12.5px; color: #8a5a12; line-height: 1.5; }

/* 成员卡 */
.mcards { display: flex; flex-direction: column; gap: 12px; }
.mcard {
  background: var(--surface);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  border: 1px solid var(--border);
  padding: 15px;
}
.mcard--off { opacity: .62; }
.mcard-top { display: flex; align-items: center; gap: 11px; }
.mcard-avatar { width: 40px; height: 40px; flex: none; border-radius: 50%; background: color-mix(in srgb, var(--mc) 16%, white); display: flex; align-items: center; justify-content: center; font-size: 21px; }
.mcard-head { flex: 1; min-width: 0; }
.mcard-name { font-size: 15.5px; font-weight: 700; }
.mcard-gender { font-size: 11.5px; color: var(--text-3); margin-top: 1px; }
.mcard-edit { flex: none; padding: 6px 14px; border-radius: var(--r-full); border: 1px solid var(--border-strong); background: var(--surface); color: var(--text-2); font-size: 12px; font-weight: 600; cursor: pointer; font-family: inherit; transition: all var(--dur-fast); }
.mcard-edit:active { background: var(--brand-50); border-color: var(--brand-300); color: var(--brand-600); }

.mcard-phase { display: flex; align-items: center; gap: 11px; margin-top: 13px; padding: 11px 13px; border-radius: var(--r-md); background: var(--tone-bg); }
.phase-emoji { font-size: 22px; flex: none; }
.phase-info { flex: 1; min-width: 0; }
.phase-label { font-size: 15px; font-weight: 800; color: var(--tone); }
.phase-sub { font-size: 11.5px; color: var(--text-3); margin-top: 1px; }

/* 时间轴 */
.timeline { position: relative; margin-top: 15px; padding-bottom: 18px; }
.tl-bar { display: flex; height: 9px; border-radius: var(--r-full); overflow: hidden; background: var(--bg-subtle); }
.tl-seg { height: 100%; }
.tl-seg.tone-danger { background: var(--danger); }
.tl-seg.tone-warning { background: var(--warning); }
.tl-seg.tone-success { background: var(--success); }
.tl-seg.tone-info { background: var(--info); }
.tl-seg.tone-brand { background: var(--brand-400); }
.tl-cursor { position: absolute; top: -4px; width: 2px; height: 17px; background: var(--text-1); transform: translateX(-1px); border-radius: 1px; }
.tl-dot { position: absolute; top: -3px; left: -3px; width: 8px; height: 8px; border-radius: 50%; background: var(--text-1); }
.tl-labels { display: flex; justify-content: space-between; margin-top: 7px; font-size: 10.5px; color: var(--text-4); }

.mcard-dates { display: flex; gap: 8px; margin-top: 4px; }
.dcell { flex: 1; display: flex; flex-direction: column; gap: 2px; padding: 9px 8px; background: var(--bg-subtle); border-radius: var(--r-sm); }
.dcell-label { font-size: 10.5px; color: var(--text-3); }
.dcell-value { font-size: 13px; font-weight: 700; }

.predict-note { margin-top: 9px; font-size: 11.5px; color: #A9761F; line-height: 1.45; }
.mcard-empty { margin-top: 12px; font-size: 12.5px; color: var(--text-4); }

/* 图例 */
.legend { display: flex; flex-direction: column; gap: 10px; background: var(--surface); border-radius: var(--r-lg); padding: 15px; box-shadow: var(--sh-sm); }
.legend-item { display: flex; align-items: flex-start; gap: 10px; }
.legend-dot { width: 10px; height: 10px; flex: none; margin-top: 5px; border-radius: 50%; background: var(--tone, var(--text-4)); }
.legend-body { flex: 1; min-width: 0; }
.legend-label { font-size: 13.5px; font-weight: 700; }
.legend-desc { font-size: 11.5px; color: var(--text-3); margin-top: 1px; line-height: 1.45; }

/* 预测 */
.forecast { background: var(--surface); border-radius: var(--r-lg); padding: 6px; box-shadow: var(--sh-sm); }
.fc-row { display: flex; align-items: center; gap: 10px; padding: 9px 10px; border-radius: var(--r-md); }
.fc-row:nth-child(odd) { background: var(--bg-subtle); }
.fc-date { font-size: 13.5px; font-weight: 700; width: 46px; flex: none; }
.fc-week { font-size: 11px; color: var(--text-3); width: 32px; flex: none; }
.fc-names { flex: 1; min-width: 0; display: flex; flex-wrap: wrap; gap: 5px; }
.fc-chip { font-size: 11.5px; color: var(--text-2); background: var(--surface); border: 1px solid var(--border); padding: 2px 8px; border-radius: var(--r-full); white-space: nowrap; }
.fc-in { flex: none; font-size: 11.5px; color: var(--danger); font-weight: 600; }

.disclaimer { margin-top: 18px; font-size: 11.5px; color: var(--text-4); line-height: 1.6; padding: 0 4px; }
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
.seg-btn { flex: 1; padding: 9px; border: none; background: transparent; border-radius: var(--r-xs); font-size: 13px; font-weight: 600; color: var(--text-2); cursor: pointer; font-family: inherit; }
.seg-btn.on { background: var(--surface); color: var(--brand-600); box-shadow: var(--sh-xs); }

.switch-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 11px 14px; background: var(--bg-subtle); border-radius: var(--r-sm); }
.switch-text { font-size: 13px; color: var(--text-2); }
.switch { position: relative; width: 48px; height: 28px; flex: none; border-radius: var(--r-full); border: none; background: var(--border-strong); cursor: pointer; transition: background var(--dur); padding: 0; }
.switch.on { background: var(--success); }
.switch-knob { position: absolute; top: 3px; left: 3px; width: 22px; height: 22px; border-radius: 50%; background: #fff; box-shadow: var(--sh-xs); transition: transform var(--dur) var(--ease-bounce); }
.switch.on .switch-knob { transform: translateX(20px); }

.preview-box { padding: 14px; background: var(--brand-50); border-radius: var(--r-md); margin-bottom: var(--gap-lg); }
.preview-title { font-size: 12px; font-weight: 700; color: var(--brand-600); margin-bottom: 10px; }
.preview-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; }
.pv { display: flex; flex-direction: column; gap: 2px; }
.pv-label { font-size: 11px; color: var(--text-3); }
.pv-value { font-size: 14px; font-weight: 700; }
.preview-tip { margin-top: 11px; padding-top: 10px; border-top: 1px dashed var(--brand-200); font-size: 12px; color: var(--text-2); line-height: 1.5; }

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }
</style>
