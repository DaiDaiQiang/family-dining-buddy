<script setup>
/**
 * 菜谱卡片
 *
 * 两种形态：
 * - grid：竖版，用于菜谱库瀑布/网格
 * - row ：横版，用于今日推荐、订单里的菜品行
 */
import { computed } from 'vue'
import DishCover from './DishCover.vue'
import RatingStars from './RatingStars.vue'
import { difficultyLabel, durationText, splitTags, ratingText } from '@/utils/format'

const props = defineProps({
  recipe: { type: Object, required: true },
  layout: { type: String, default: 'grid' }, // grid | row
  showTags: { type: Boolean, default: true },
  inCart: { type: Boolean, default: false }
})
const emit = defineEmits(['click', 'add'])

const tags = computed(() => splitTags(props.recipe.tag_names || props.recipe.tags?.map?.((t) => t.name)).slice(0, 2))
const rating = computed(() => Number(props.recipe.avg_rating) || 0)
</script>

<template>
  <article
    class="dish"
    :class="[`dish--${layout}`, { 'dish--in-cart': inCart }]"
    @click="emit('click', recipe)"
  >
    <div class="dish-cover-box">
      <DishCover
        :emoji="recipe.cover_emoji"
        :gradient="recipe.cover_gradient"
        :url="recipe.cover_url"
        :size="layout === 'grid' ? 'hero' : 'md'"
        :rounded="layout === 'grid' ? 'none' : 'md'"
      />
      <span v-if="inCart" class="dish-cart-flag">✓ 已选</span>
      <span v-if="recipe.calories" class="dish-cal num">{{ recipe.calories }}kcal</span>
    </div>

    <div class="dish-body">
      <div class="dish-head">
        <h3 class="dish-name ellipsis">{{ recipe.name }}</h3>
        <div class="dish-rating">
          <RatingStars :value="rating" :size="12" />
          <span class="dish-rating-num num">{{ ratingText(rating) }}</span>
        </div>
      </div>

      <p v-if="recipe.description && layout === 'grid'" class="dish-desc ellipsis-2">{{ recipe.description }}</p>

      <div class="dish-meta">
        <span class="dish-meta-item">⏱ {{ durationText(recipe.total_minutes) }}</span>
        <span class="dish-meta-item">{{ difficultyLabel(recipe.difficulty) }}</span>
        <span v-if="recipe.category" class="dish-meta-item chip">{{ recipe.category }}</span>
      </div>

      <div v-if="showTags && tags.length" class="dish-tags">
        <span v-for="t in tags" :key="t" class="dish-tag"># {{ t }}</span>
      </div>
    </div>

    <button
      v-if="layout === 'row'"
      class="dish-add"
      :class="{ 'dish-add--on': inCart }"
      aria-label="加入购物车"
      @click.stop="emit('add', recipe)"
    >
      <span v-if="inCart">✓</span><span v-else>＋</span>
    </button>
  </article>
</template>

<style scoped>
.dish {
  position: relative;
  background: var(--surface);
  border-radius: var(--r-lg);
  overflow: hidden;
  box-shadow: var(--sh-sm);
  border: 1px solid var(--border);
  cursor: pointer;
  transition: transform var(--dur) var(--ease), box-shadow var(--dur);
  -webkit-tap-highlight-color: transparent;
}
.dish:active { transform: scale(.985); }
.dish--in-cart { border-color: var(--brand-300); box-shadow: 0 0 0 2px var(--brand-100), var(--sh-sm); }

/* ---------- 竖版 ---------- */
.dish--grid { display: flex; flex-direction: column; }
.dish-cover-box { position: relative; }
.dish--grid .dish-cover-box :deep(.cover) { border-radius: 0; }

.dish-cart-flag {
  position: absolute; top: 10px; left: 10px;
  padding: 3px 9px;
  border-radius: var(--r-full);
  background: rgba(46,158,107,.95);
  color: #fff; font-size: 11px; font-weight: 700;
  box-shadow: var(--sh-sm);
}
.dish-cal {
  position: absolute; top: 10px; right: 10px;
  padding: 3px 8px;
  border-radius: var(--r-full);
  background: rgba(0,0,0,.32);
  backdrop-filter: blur(4px);
  color: #fff; font-size: 11px; font-weight: 600;
}

.dish-body { padding: 12px 14px 14px; }
.dish-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.dish-name { font-size: 16px; font-weight: 700; flex: 1; min-width: 0; font-family: var(--font-display); }
.dish-rating { display: flex; align-items: center; gap: 3px; flex: none; }
.dish-rating-num { font-size: 12px; font-weight: 700; color: var(--star); }

.dish-desc { margin-top: 5px; font-size: 12.5px; color: var(--text-3); line-height: 1.5; min-height: 36px; }

.dish-meta { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 9px; }
.dish-meta-item { font-size: 11.5px; color: var(--text-2); }
.dish-meta-item.chip { background: var(--bg-subtle); padding: 2px 8px; border-radius: var(--r-full); }

.dish-tags { display: flex; gap: 6px; margin-top: 8px; flex-wrap: wrap; }
.dish-tag { font-size: 11px; color: var(--brand-600); }

/* ---------- 横版 ---------- */
.dish--row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px;
}
.dish--row .dish-cover-box { width: 72px; height: 72px; flex: none; position: relative; }
.dish--row .dish-body { flex: 1; min-width: 0; padding: 2px 0; }
.dish--row .dish-name { font-size: 15px; }
.dish--row .dish-desc { display: none; }
.dish--row .dish-cal { top: auto; bottom: -4px; right: -4px; font-size: 10px; padding: 1px 5px; }

.dish-add {
  flex: none;
  width: 38px; height: 38px;
  border-radius: 50%;
  border: none;
  background: var(--brand-gradient);
  color: #fff;
  font-size: 22px; font-weight: 500;
  line-height: 1;
  cursor: pointer;
  box-shadow: var(--sh-brand);
  transition: transform var(--dur-fast) var(--ease-bounce);
  -webkit-tap-highlight-color: transparent;
}
.dish-add:active { transform: scale(.86); }
.dish-add--on { background: var(--success); font-size: 18px; box-shadow: 0 6px 18px rgba(79,138,91,.3); }
</style>
