import { defineStore } from 'pinia'
import { api } from '@/api'
import { toast } from '@/composables/useToast'

/**
 * 菜谱库状态：列表、筛选条件、详情缓存
 */
export const useRecipeStore = defineStore('recipe', {
  state: () => ({
    list: [],
    total: 0,
    page: 1,
    pageSize: 20,
    loading: false,
    loaded: false,
    filters: {
      keyword: '',
      category: '',
      cuisine: '',
      difficulty: '',
      tag: '',
      sort: 'rating'
    },
    facets: { categories: [], cuisines: [], tags: [] },
    /** 详情缓存：{ [id]: RecipeDetail } */
    detailCache: {},
    /** 当前成员的收藏集合 */
    favoriteIds: new Set()
  }),

  getters: {
    hasActiveFilter: (s) =>
      !!(s.filters.keyword || s.filters.category || s.filters.cuisine ||
         s.filters.difficulty || s.filters.tag),
    isEmpty: (s) => s.loaded && !s.loading && s.list.length === 0,
    totalPages: (s) => Math.max(1, Math.ceil(s.total / s.pageSize))
  },

  actions: {
    async fetchList({ append = false } = {}) {
      this.loading = true
      try {
        if (!append) this.page = 1
        const res = await api.recipes.list({
          ...this.filters,
          page: this.page,
          page_size: this.pageSize
        })
        const rows = res?.list || res || []
        this.list = append ? [...this.list, ...rows] : rows
        this.total = res?.total ?? rows.length
        this.loaded = true
      } catch (e) {
        toast.error(e.message || '菜谱加载失败')
        if (!append) { this.list = []; this.total = 0 }
      } finally {
        this.loading = false
      }
    },

    async loadMore() {
      if (this.loading) return
      if (this.list.length >= this.total) return
      this.page += 1
      await this.fetchList({ append: true })
    },

    async fetchFacets() {
      try {
        this.facets = await api.recipes.facets()
      } catch (e) {
        console.warn('加载筛选项失败', e)
      }
    },

    setFilter(patch) {
      Object.assign(this.filters, patch)
    },

    resetFilters() {
      this.filters = { keyword: '', category: '', cuisine: '', difficulty: '', tag: '', sort: this.filters.sort }
    },

    async fetchDetail(id) {
      if (this.detailCache[id]) return this.detailCache[id]
      const detail = await api.recipes.detail(id)
      this.detailCache[id] = detail
      // 同步列表里的卡片，保证评分/次数一致
      const idx = this.list.findIndex((r) => r.id === Number(id))
      if (idx >= 0) {
        this.list[idx] = { ...this.list[idx], ...detail }
      }
      return detail
    },

    invalidate(id) {
      if (id) delete this.detailCache[id]
      else this.detailCache = {}
    },

    async createRecipe(data) {
      const res = await api.recipes.create(data)
      this.invalidate()
      await Promise.all([this.fetchList(), this.fetchFacets()])
      return res
    },

    async updateRecipe(id, data) {
      const res = await api.recipes.update(id, data)
      this.invalidate(id)
      await this.fetchList()
      return res
    },

    async removeRecipe(id, permanent = false) {
      const res = await api.recipes.remove(id, permanent)
      this.invalidate(id)
      this.list = this.list.filter((r) => r.id !== Number(id))
      this.total = Math.max(0, this.total - 1)
      return res
    },

    async toggleFavorite(recipeId, memberId) {
      const res = await api.recipes.toggleFavorite(recipeId, memberId)
      if (res?.favorited) this.favoriteIds.add(Number(recipeId))
      else this.favoriteIds.delete(Number(recipeId))
      const card = this.list.find((r) => r.id === Number(recipeId))
      if (card) {
        card.favorite_count = Math.max(0, (card.favorite_count || 0) + (res?.favorited ? 1 : -1))
      }
      const detail = this.detailCache[recipeId]
      if (detail) delete this.detailCache[recipeId]
      return res
    },

    markFavorites(ids) {
      this.favoriteIds = new Set((ids || []).map(Number))
    },

    isFavorite(id) {
      return this.favoriteIds.has(Number(id))
    }
  }
})
