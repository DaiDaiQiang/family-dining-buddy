<script setup>
/**
 * 这一餐的全部做法
 *
 * 把订单里所有菜的步骤串成一份「烹饪流程单」，方便照着做。
 * 对应后端 GET /api/orders/{id}/recipes。
 */
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { api } from '@/api'
import DishCover from '@/components/DishCover.vue'
import YanLoader from '@/components/YanLoader.vue'
import { toast } from '@/composables/useToast'
import { durationText, difficultyLabel, quantityText } from '@/utils/format'

const route = useRoute()
const router = useRouter()
const id = Number(route.params.id)

const loading = ref(true)
const data = ref(null)
const openIdx = ref(0)

async function load() {
  loading.value = true
  try {
    data.value = await api.orders.recipes(id)
  } catch (e) {
    toast.error(e.message || '加载失败')
    router.back()
  } finally {
    loading.value = false
  }
}
onMounted(load)

const recipes = computed(() => data.value?.recipes || [])

function toggle(i) { openIdx.value = openIdx.value === i ? -1 : i }
</script>

<template>
  <div class="page page--flat or">
    <div v-if="loading" class="page-body">
      <YanLoader :size="180" tip="菜单加载中" />
    </div>

    <template v-else-if="data">
      <header class="or-head">
        <div class="page-body">
          <h1 class="or-title">👨‍🍳 这一餐怎么做</h1>
          <p class="or-sub">
            {{ data.recipe_count }} 道菜 · 累计烹饪约 {{ durationText(data.total_cook_minutes) }}
          </p>
        </div>
      </header>

      <div class="page-body">
        <div class="recipe-blocks">
          <article
            v-for="(r, i) in recipes"
            :key="r.id"
            class="rblock card"
            :class="{ open: openIdx === i }"
          >
            <button class="rblock-head" @click="toggle(i)">
              <DishCover :emoji="r.cover_emoji" :gradient="r.cover_gradient" :url="r.cover_url" size="sm" />
              <div class="rblock-title">
                <p class="rblock-name">{{ r.name }}</p>
                <p class="rblock-meta">{{ difficultyLabel(r.difficulty) }} · {{ durationText(r.total_minutes) }} · {{ r.servings }} 人份</p>
              </div>
              <span class="rblock-caret" :class="{ up: openIdx === i }">▾</span>
            </button>

            <div v-show="openIdx === i" class="rblock-body">
              <p v-if="r.description" class="rblock-desc">{{ r.description }}</p>

              <!-- 食材 -->
              <div v-if="r.ingredient_groups?.length" class="rb-section">
                <h4 class="rb-h">食材</h4>
                <div class="rb-ings">
                  <span v-for="g in r.ingredient_groups" :key="g.group_name" class="rb-ing-group">
                    <b>{{ g.group_name }}：</b>
                    <span v-for="(it, j) in g.items" :key="j">{{ it.name }} {{ quantityText(it.quantity) }}{{ it.unit }}<template v-if="j < g.items.length - 1">、</template></span>
                  </span>
                </div>
              </div>

              <!-- 步骤 -->
              <div v-if="r.steps?.length" class="rb-section">
                <h4 class="rb-h">步骤</h4>
                <ol class="rb-steps">
                  <li v-for="s in r.steps" :key="s.step_no" class="rb-step">
                    <span class="rb-step-no num">{{ s.step_no }}</span>
                    <span class="rb-step-text">{{ s.content }}</span>
                  </li>
                </ol>
              </div>

              <div v-if="r.tips" class="rb-tips">💡 {{ r.tips }}</div>

              <button class="btn btn--soft btn--sm rb-link" @click="router.push(`/recipes/${r.id}`)">查看完整菜谱 ›</button>
            </div>
          </article>
        </div>
        <div class="bottom-space" />
      </div>
    </template>
  </div>
</template>

<style scoped>
.or-head { padding: calc(var(--safe-top) + 56px) 0 16px; }
.or-title { font-size: 22px; font-weight: 800; }
.or-sub { font-size: 13.5px; color: var(--text-2); margin-top: 4px; }

.recipe-blocks { display: flex; flex-direction: column; gap: 12px; }
.rblock { overflow: hidden; transition: box-shadow var(--dur); }
.rblock.open { box-shadow: var(--sh-md); }
.rblock-head { display: flex; align-items: center; gap: 12px; width: 100%; padding: 12px; border: none; background: transparent; cursor: pointer; text-align: left; font-family: inherit; }
.rblock-title { flex: 1; min-width: 0; }
.rblock-name { font-size: 16px; font-weight: 700; }
.rblock-meta { font-size: 12px; color: var(--text-3); margin-top: 2px; }
.rblock-caret { color: var(--text-4); transition: transform var(--dur-fast); flex: none; }
.rblock-caret.up { transform: rotate(180deg); }

.rblock-body { padding: 4px 14px 16px; border-top: 1px solid var(--border); animation: body-in var(--dur) var(--ease-out); }
@keyframes body-in { from { opacity: 0; } to { opacity: 1; } }
.rblock-desc { font-size: 13.5px; color: var(--text-2); line-height: 1.6; margin: 12px 0; }

.rb-section { margin-top: 14px; }
.rb-h { font-size: 13px; font-weight: 700; color: var(--brand-600); margin-bottom: 8px; }
.rb-ings { display: flex; flex-direction: column; gap: 6px; font-size: 13.5px; color: var(--text-2); line-height: 1.5; }
.rb-ing-group b { color: var(--text-1); font-weight: 600; }

.rb-steps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 10px; }
.rb-step { display: flex; gap: 10px; }
.rb-step-no { flex: none; width: 22px; height: 22px; border-radius: 50%; background: var(--brand-gradient); color: #fff; font-size: 12px; font-weight: 700; display: flex; align-items: center; justify-content: center; }
.rb-step-text { flex: 1; font-size: 14px; line-height: 1.6; color: var(--text-1); }

.rb-tips { margin-top: 14px; font-size: 13px; color: #7a5c12; background: linear-gradient(135deg, #FFF8E7, #FFF3D6); padding: 10px 12px; border-radius: var(--r-sm); line-height: 1.5; }
.rb-link { margin-top: 14px; }
.bottom-space { height: 30px; }
</style>
