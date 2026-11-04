import { defineStore } from 'pinia'
import { api } from '@/api'
import { toast } from '@/composables/useToast'

/**
 * 购物车（待下单的菜）
 *
 * 跨页面保留：从菜谱页加入，在点餐页统一结算。
 * 结构：{ [recipeId]: { recipe, portions, memberId, note } }
 */
const STORAGE_KEY = 'fd_cart'

function readCart() {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY)) || {}
  } catch {
    return {}
  }
}

export const useCartStore = defineStore('cart', {
  state: () => ({
    items: readCart(),
    /** 本次下单信息 */
    mealDate: new Date().toISOString().slice(0, 10),
    mealType: guessMealType(),
    diners: Number(localStorage.getItem('fd_diners')) || 3,
    note: ''
  }),

  getters: {
    count: (s) => Object.keys(s.items).length,
    totalPortions: (s) =>
      Object.values(s.items).reduce((sum, i) => sum + (i.portions || 1), 0),
    entries: (s) =>
      Object.entries(s.items).map(([id, v]) => ({ id: Number(id), ...v })),
    isEmpty: (s) => Object.keys(s.items).length === 0
  },

  actions: {
    persist() {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(this.items))
        localStorage.setItem('fd_diners', String(this.diners))
      } catch (e) {
        console.warn('购物车本地保存失败', e)
      }
    },

    /** 加入一道菜；已在车里则份数 +1 */
    add(recipe, portions = 1, memberId = null, note = '') {
      const id = Number(recipe.id)
      if (this.items[id]) {
        this.items[id].portions = Math.min(20, this.items[id].portions + portions)
      } else {
        this.items[id] = {
          recipe: {
            id,
            name: recipe.name,
            cover_emoji: recipe.cover_emoji,
            cover_gradient: recipe.cover_gradient,
            category: recipe.category,
            total_minutes: recipe.total_minutes,
            difficulty: recipe.difficulty,
            calories: recipe.calories
          },
          portions,
          memberId,
          note
        }
      }
      this.persist()
    },

    /** 加入一道「尚未建档」的新菜（点餐时顺手建档） */
    addNewRecipe(draft) {
      const key = `new_${Date.now()}`
      this.items[key] = { recipe: null, newRecipe: draft, portions: 1, memberId: null, note: '' }
      this.persist()
      return key
    },

    has(id) {
      return !!this.items[Number(id)]
    },

    setPortions(key, portions) {
      if (!this.items[key]) return
      this.items[key].portions = Math.max(1, Math.min(20, Number(portions) || 1))
      this.persist()
    },

    setNote(key, note) {
      if (!this.items[key]) return
      this.items[key].note = note
      this.persist()
    },

    setMember(key, memberId) {
      if (!this.items[key]) return
      this.items[key].memberId = memberId ? Number(memberId) : null
      this.persist()
    },

    remove(key) {
      const removed = this.items[key]
      delete this.items[key]
      this.persist()
      return removed
    },

    clear() {
      this.items = {}
      this.note = ''
      this.persist()
    },

    setOrderInfo(patch) {
      if (patch.mealDate) this.mealDate = patch.mealDate
      if (patch.mealType) this.mealType = patch.mealType
      if (patch.diners != null) {
        this.diners = Math.max(1, Math.min(30, Number(patch.diners) || 1))
      }
      if (patch.note !== undefined) this.note = patch.note
      this.persist()
    },

    /**
     * 提交订单
     * @param {object} opts { memberId, autoConfirm }
     */
    async submit({ memberId, autoConfirm = false } = {}) {
      if (this.isEmpty) throw new Error('购物车是空的')

      const items = Object.entries(this.items).map(([, v]) => {
        const base = {
          portions: v.portions || 1,
          member_id: v.memberId || memberId || undefined,
          note: v.note || undefined
        }
        // 未建档的新菜：走 new_recipe，后端在同一事务里先建菜谱再入单
        return v.newRecipe
          ? { ...base, new_recipe: v.newRecipe }
          : { ...base, recipe_id: Number(v.recipe.id) }
      })

      const payload = {
        member_id: memberId,
        meal_date: this.mealDate,
        meal_type: this.mealType,
        diners: this.diners,
        note: this.note || undefined,
        auto_confirm: autoConfirm,
        items
      }

      const res = await api.orders.create(payload)
      this.clear()
      return res?.order || res
    }
  }
})

/** 按当前时刻猜一个默认餐次 */
function guessMealType() {
  const h = new Date().getHours()
  if (h < 10) return 'breakfast'
  if (h < 15) return 'lunch'
  if (h < 21) return 'dinner'
  return 'snack'
}
