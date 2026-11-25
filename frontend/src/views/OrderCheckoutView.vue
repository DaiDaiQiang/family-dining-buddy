<script setup>
/**
 * 点餐结算页
 *
 * 从点餐列表页（/order）的底部结算条进入。
 * 展示已选菜品（按点餐人分组 + 忌口警示）、逐道调份数/备注/点餐人，
 * 设置用餐日期餐次人数，提交后生成订单。
 */
import { computed, onActivated, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useCartStore } from '@/stores/cart'
import { useAppStore } from '@/stores/app'
import DishCover from '@/components/DishCover.vue'
import Stepper from '@/components/Stepper.vue'
import EmptyState from '@/components/EmptyState.vue'
import { toast } from '@/composables/useToast'
import { durationText, splitTags } from '@/utils/format'

defineOptions({ name: 'OrderCheckoutView' })

const router = useRouter()
const cart = useCartStore()
const app = useAppStore()

const submitting = ref(false)
const members = computed(() => app.activeMembers)
const mealTypes = computed(() => app.meta?.meal_types || [
  { value: 'breakfast', label: '早餐', emoji: '🌅' },
  { value: 'lunch', label: '午餐', emoji: '☀️' },
  { value: 'dinner', label: '晚餐', emoji: '🌙' },
  { value: 'snack', label: '加餐', emoji: '🍪' }
])

const entries = computed(() => cart.entries)
const totalMinutes = computed(() =>
  entries.value.reduce((s, e) => s + (e.recipe?.total_minutes || 0), 0)
)

/**
 * 按点餐人分组（美团结算页那种「谁想吃什么」）。
 * 未指定点餐人的归入「未指定」组。
 */
const groupedEntries = computed(() => {
  const byMember = new Map()
  for (const e of entries.value) {
    const mid = e.memberId || null
    if (!byMember.has(mid)) byMember.set(mid, { memberId: mid, items: [] })
    byMember.get(mid).items.push(e)
  }
  const order = members.value.map((m) => m.id)
  const groups = [...byMember.values()]
  groups.sort((a, b) => {
    if (a.memberId == null) return 1
    if (b.memberId == null) return -1
    return order.indexOf(a.memberId) - order.indexOf(b.memberId)
  })
  return groups
})

function memberOf(mid) {
  return members.value.find((m) => m.id === mid) || null
}

/** 忌口浅层匹配：成员 dislikes 关键词 vs 菜名/分类 */
function dislikeHits(entry) {
  const mid = entry.memberId
  if (!mid) return []
  const m = memberOf(mid)
  if (!m?.dislikes) return []
  const keys = splitTags(m.dislikes)
  if (!keys.length) return []
  const haystack = [entry.recipe?.name || entry.newRecipe?.name || '', entry.recipe?.category || ''].join(' ')
  return keys.filter((k) => haystack.includes(k))
}

onActivated(() => {
  if (!members.value.length) app.loadMembers()
  // 结算页若被清空（如提交成功），回到列表页
  if (cart.isEmpty) router.replace('/order')
})

function setMealType(v) { cart.setOrderInfo({ mealType: v }) }
function setDate(v) { cart.setOrderInfo({ mealDate: v }) }
function setDiners(v) { cart.setOrderInfo({ diners: Number(v) }) }

function onPortions(key, val) { cart.setPortions(key, val) }
function onNote(key, val) { cart.setNote(key, val) }
function onMember(key, val) { cart.setMember(key, val) }

function removeItem(key, name) {
  cart.remove(key)
  toast.info(`已移除「${name}」`)
  if (cart.isEmpty) router.replace('/order')
}

function goBack() { router.push('/order') }
function newDish() { router.push('/recipes/new?from=cart') }

async function submit(autoConfirm) {
  if (cart.isEmpty) { toast.warn('先选几道菜吧'); return }
  if (!app.currentMember) { toast.warn('请先选择下单人'); router.push('/members'); return }

  submitting.value = true
  try {
    const order = await cart.submit({ memberId: app.currentMember.id, autoConfirm })
    toast.success(autoConfirm ? '下单成功，采购清单已生成' : '草稿已保存')
    const orderId = order?.id || order?.base?.id
    if (orderId) router.replace(`/orders/${orderId}`)
    else router.replace('/orders')
  } catch (e) {
    toast.error(e.message || '下单失败')
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="page order">
    <header class="o-head">
      <div class="page-body">
        <h1 class="o-title">🛒 结算</h1>
        <p class="o-sub">
          <template v-if="!cart.isEmpty">
            已选 <b class="num">{{ cart.count }}</b> 道菜 · 共 <b class="num">{{ cart.totalPortions }}</b> 份
            <span v-if="totalMinutes"> · 约 {{ durationText(totalMinutes) }}</span>
          </template>
          <template v-else>购物车是空的</template>
        </p>
      </div>
    </header>

    <div class="page-body">
      <!-- 用餐信息 -->
      <section class="card card--pad meal-card">
        <div class="meal-date">
          <label class="md-label">用餐日期</label>
          <input :value="cart.mealDate" type="date" class="md-input" @change="setDate($event.target.value)" />
        </div>
        <div class="meal-types">
          <button
            v-for="m in mealTypes"
            :key="m.value"
            class="mt-btn"
            :class="{ on: cart.mealType === m.value }"
            @click="setMealType(m.value)"
          >
            <span class="mt-emoji">{{ m.emoji }}</span>{{ m.label }}
          </button>
        </div>
        <div class="meal-diners">
          <label class="md-label">就餐人数</label>
          <Stepper :model-value="cart.diners" :min="1" :max="30" size="sm" @update:model-value="setDiners" />
        </div>
      </section>

      <!-- 已选菜品（按点餐人分组） -->
      <section v-if="entries.length" class="section">
        <div class="section-head">
          <h2 class="section-title">已选菜品</h2>
          <button class="section-more" @click="cart.clear()">清空</button>
        </div>

        <div v-for="g in groupedEntries" :key="g.memberId ?? 'unassigned'" class="cart-group">
          <div class="cg-head">
            <span class="cg-avatar">{{ memberOf(g.memberId)?.avatar_emoji || '👥' }}</span>
            <span class="cg-name">{{ memberOf(g.memberId)?.nickname || '未指定' }}</span>
            <span class="cg-count num">{{ g.items.reduce((s, e) => s + e.portions, 0) }} 份</span>
          </div>

          <div class="cart-list">
            <div v-for="e in g.items" :key="e.id" class="cart-item" :class="{ 'cart-item--warn': dislikeHits(e).length }">
              <div class="ci-top">
                <DishCover
                  v-if="e.recipe"
                  :emoji="e.recipe.cover_emoji"
                  :gradient="e.recipe.cover_gradient"
                  :url="e.recipe.cover_url"
                  size="sm"
                />
                <DishCover v-else emoji="🆕" gradient="linear-gradient(135deg,#66BB6A,#43A047)" size="sm" />
                <div class="ci-info">
                  <p class="ci-name ellipsis">
                    {{ e.recipe?.name || e.newRecipe?.name || '新菜' }}
                    <span v-if="!e.recipe" class="chip chip--success ci-new">待建档</span>
                  </p>
                  <p v-if="e.recipe" class="ci-meta">{{ e.recipe.category }} · {{ durationText(e.recipe.total_minutes) }}</p>
                  <p v-if="dislikeHits(e).length" class="ci-warn">⚠️ 忌口：{{ dislikeHits(e).join('、') }}</p>
                </div>
                <Stepper :model-value="e.portions" :min="1" :max="20" size="sm" @update:model-value="(v) => onPortions(String(e.id), v)" />
                <button class="ci-del" @click="removeItem(String(e.id), e.recipe?.name || e.newRecipe?.name)" aria-label="移除">✕</button>
              </div>
              <div class="ci-extra">
                <input
                  :value="e.note || ''"
                  class="ci-note"
                  placeholder="备注，如：不要辣 / 多放葱"
                  @change="onNote(String(e.id), $event.target.value)"
                />
                <select class="ci-member" :value="e.memberId || ''" @change="onMember(String(e.id), $event.target.value)">
                  <option value="">谁点的</option>
                  <option v-for="m in members" :key="m.id" :value="m.id">{{ m.avatar_emoji }} {{ m.nickname }}</option>
                </select>
              </div>
            </div>
          </div>
        </div>
      </section>

      <EmptyState v-else emoji="🛒" title="点餐单还空着" desc="回列表页挑几道菜吧">
        <div class="empty-actions">
          <button class="btn btn--primary btn--sm" @click="goBack">去挑菜</button>
        </div>
      </EmptyState>

      <!-- 整单备注 -->
      <section v-if="!cart.isEmpty" class="section">
        <div class="card card--pad">
          <label class="field-label">整单备注</label>
          <textarea
            :value="cart.note"
            class="textarea"
            rows="2"
            placeholder="给掌勺人的话，如：今晚有客人，多做两个硬菜"
            @change="cart.setOrderInfo({ note: $event.target.value })"
          />
        </div>
      </section>

      <div class="bottom-space" />
    </div>

    <!-- 底部结算条 -->
    <div v-if="!cart.isEmpty" class="submit-bar">
      <div class="page-body submit-inner">
        <button class="btn btn--ghost sb-ghost" @click="goBack">‹ 继续挑菜</button>
        <button class="btn btn--soft sb-draft" :disabled="submitting" @click="submit(false)">存草稿</button>
        <button class="btn btn--primary grow" :disabled="submitting" @click="submit(true)">
          {{ submitting ? '提交中…' : '确认下单 · 生成采购单' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.o-head { padding-top: calc(var(--safe-top) + 20px); padding-bottom: 6px; }
.o-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.o-sub { margin-top: 5px; font-size: 13.5px; color: var(--text-2); }
.o-sub b { color: var(--brand-600); font-size: 16px; }

.meal-card { display: flex; flex-direction: column; gap: 14px; margin-top: 14px; }
.meal-date, .meal-diners { display: flex; align-items: center; justify-content: space-between; }
.md-label { font-size: 14px; font-weight: 600; color: var(--text-2); }
.md-input { border: 1.5px solid var(--border); border-radius: var(--r-sm); padding: 8px 12px; font-size: 14px; font-family: inherit; background: var(--surface-2); color: var(--text-1); }

.meal-types { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
.mt-btn { display: flex; flex-direction: column; align-items: center; gap: 3px; padding: 10px 4px; border-radius: var(--r-md); border: 1.5px solid var(--border); background: var(--surface-2); font-size: 12.5px; font-weight: 600; color: var(--text-2); cursor: pointer; transition: all var(--dur-fast) var(--ease); font-family: inherit; }
.mt-emoji { font-size: 20px; }
.mt-btn:active { transform: scale(.95); }
.mt-btn.on { background: var(--brand-gradient); border-color: transparent; color: #fff; box-shadow: var(--sh-brand); }

/* ---------- 按人分组 ---------- */
.cart-group { display: flex; flex-direction: column; gap: 10px; }
.cart-group + .cart-group { margin-top: 18px; padding-top: 16px; border-top: 1px dashed var(--border-strong); }
.cg-head { display: flex; align-items: center; gap: 8px; }
.cg-avatar { font-size: 18px; }
.cg-name { font-size: 14px; font-weight: 700; color: var(--text-1); }
.cg-count { font-size: 12px; color: var(--text-3); background: var(--bg-subtle); padding: 2px 8px; border-radius: var(--r-full); }

.cart-list { display: flex; flex-direction: column; gap: 10px; }
.cart-item { background: var(--surface); border-radius: var(--r-lg); padding: 12px; box-shadow: var(--sh-sm); border: 1px solid var(--border); animation: ci-in var(--dur) var(--ease-out); }
.cart-item--warn { border-color: #EF9F27; background: linear-gradient(180deg, #FFFDF6, var(--surface)); }
@keyframes ci-in { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: none; } }
.ci-top { display: flex; align-items: center; gap: 10px; }
.ci-info { flex: 1; min-width: 0; }
.ci-name { font-size: 15px; font-weight: 600; display: flex; align-items: center; gap: 6px; }
.ci-new { font-size: 10px; padding: 1px 6px; }
.ci-meta { font-size: 12px; color: var(--text-3); margin-top: 2px; }
.ci-warn { font-size: 12px; color: #854F0B; margin-top: 3px; font-weight: 600; }
.ci-del { flex: none; width: 28px; height: 28px; border: none; background: transparent; color: var(--text-4); font-size: 14px; cursor: pointer; border-radius: 50%; }
.ci-del:active { background: var(--danger-bg); color: var(--danger); }

.ci-extra { display: flex; gap: 8px; margin-top: 10px; }
.ci-note { flex: 1; min-width: 0; border: 1px solid var(--border); border-radius: var(--r-xs); padding: 7px 10px; font-size: 13px; font-family: inherit; background: var(--surface-2); }
.ci-member { flex: none; width: 110px; border: 1px solid var(--border); border-radius: var(--r-xs); padding: 7px 8px; font-size: 13px; font-family: inherit; background: var(--surface-2); color: var(--text-2); }

.empty-actions { display: flex; gap: 10px; }

.submit-bar { position: fixed; left: 0; right: 0; bottom: calc(var(--tabbar-h) + var(--safe-bottom)); z-index: 48; padding: 10px 0; background: rgba(255,253,247,.96); backdrop-filter: blur(16px); border-top: 1px solid var(--border); }
.submit-inner { display: flex; gap: 8px; align-items: center; max-width: 1180px; }
.sb-ghost { flex: none; padding: 10px 12px; font-size: 13px; }
.sb-draft { flex: none; padding: 10px 14px; font-size: 13px; }
.bottom-space { height: 140px; }

@media (min-width: 700px) {
  .sb-ghost, .sb-draft { padding: 11px 18px; font-size: 14px; }
}
</style>
