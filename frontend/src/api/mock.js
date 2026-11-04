/**
 * 离线 Mock 数据
 *
 * 后端不可用（未装 MySQL / 未启动服务）时，前端自动降级到这里，
 * 保证界面与交互完整可演示。写操作只改内存，刷新即还原。
 */

const now = new Date()
const dateStr = (d) => d.toISOString().slice(0, 10)
const today = dateStr(now)
const dayAgo = (n) => {
  const d = new Date(now)
  d.setDate(d.getDate() - n)
  return dateStr(d)
}

export const mockMembers = [
  { id: 1, nickname: '爸爸', avatar_emoji: '👨', theme_color: '#4A90D9', role: 'admin', taste_note: '爱吃辣，不吃香菜', is_active: true, sort_order: 1 },
  { id: 2, nickname: '妈妈', avatar_emoji: '👩', theme_color: '#D4537E', role: 'admin', taste_note: '少油少盐，爱吃汤羹', is_active: true, sort_order: 2 },
  { id: 3, nickname: '哥哥', avatar_emoji: '🧑', theme_color: '#7B61EE', role: 'member', taste_note: '无肉不欢', is_active: true, sort_order: 3 },
  { id: 4, nickname: '妹妹', avatar_emoji: '👧', theme_color: '#D1915A', role: 'child', taste_note: '不吃青椒，爱吃甜的', is_active: true, sort_order: 4 },
  { id: 5, nickname: '奶奶', avatar_emoji: '👵', theme_color: '#26A69A', role: 'member', taste_note: '清淡软烂，忌辛辣', is_active: true, sort_order: 5 }
]

export const mockTags = [
  { id: 1, name: '下饭', color: '#C2703D', sort_order: 1 },
  { id: 2, name: '快手菜', color: '#26A69A', sort_order: 2 },
  { id: 3, name: '低卡', color: '#66BB6A', sort_order: 3 },
  { id: 4, name: '宝宝爱吃', color: '#FFA726', sort_order: 4 },
  { id: 5, name: '无辣', color: '#42A5F5', sort_order: 5 },
  { id: 6, name: '宴客菜', color: '#AB47BC', sort_order: 6 },
  { id: 7, name: '高蛋白', color: '#EF5350', sort_order: 7 },
  { id: 8, name: '素食', color: '#8BC34A', sort_order: 8 },
  { id: 9, name: '开胃', color: '#FFCA28', sort_order: 9 },
  { id: 10, name: '汤水', color: '#29B6F6', sort_order: 10 }
]

const G = [
  ['#FF512F', '#F09819'], ['#11998E', '#38EF7D'], ['#CB2D3E', '#EF473A'],
  ['#8E2DE2', '#4A00E0'], ['#2193B0', '#6DD5ED'], ['#56AB2F', '#A8E063'],
  ['#F7971E', '#FFD200'], ['#36D1DC', '#5B86E5'], ['#4E342E', '#8D6E63'],
  ['#FBD3E9', '#FBB03B'], ['#C0392B', '#8E44AD'], ['#00B09B', '#96C93D']
]
const grad = (i) => `linear-gradient(135deg,${G[i % G.length][0]},${G[i % G.length][1]})`

export const mockRecipes = [
  { id: 1, name: '红烧肉', category: '家常菜', cuisine: '中餐', difficulty: 'medium', prep_minutes: 20, cook_minutes: 55, total_minutes: 75, servings: 3, cover_emoji: '🍖', cover_gradient: grad(3), description: '肥而不腻、入口即化的经典硬菜，冰糖炒色是关键', tips: '炒糖色时全程小火，冒鱼眼泡立刻下肉', calories: 520, cook_count: 12, avg_rating: 4.8, tag_names: '下饭,宴客菜,高蛋白', steps: ['五花肉冷水下锅，加姜片料酒焯水，切成 3cm 方块', '小火炒冰糖至琥珀色冒鱼眼泡', '下肉块快速翻炒上色，加姜八角桂皮', '烹入料酒、生抽、老抽，加没过肉面的热水', '小火加盖焖 45 分钟', '大火收汁至浓稠，撒葱花'], ingredients: [['五花肉', 600, '克', '主料'], ['冰糖', 30, '克', '调味料'], ['生姜', 15, '克', '辅料'], ['生抽', 20, '毫升', '调味料']] },
  { id: 2, name: '番茄炒蛋', category: '家常菜', cuisine: '中餐', difficulty: 'easy', prep_minutes: 8, cook_minutes: 7, total_minutes: 15, servings: 2, cover_emoji: '🍅', cover_gradient: grad(0), description: '国民下饭菜，酸甜多汁，十分钟搞定', tips: '番茄先煸出沙再加糖，汁水浓稠才挂得住蛋', calories: 180, cook_count: 24, avg_rating: 4.9, tag_names: '下饭,快手菜,宝宝爱吃,无辣', steps: ['番茄划十字烫水去皮切块，鸡蛋加盐打散', '热油滑炒蛋液至八分凝固盛出', '余油煸炒番茄出沙，加糖和盐', '倒回鸡蛋轻推，撒葱花出锅'], ingredients: [['鸡蛋', 3, '个', '主料'], ['西红柿', 300, '克', '主料'], ['白糖', 8, '克', '调味料']] },
  { id: 3, name: '宫保鸡丁', category: '家常菜', cuisine: '川菜', difficulty: 'medium', prep_minutes: 18, cook_minutes: 12, total_minutes: 30, servings: 2, cover_emoji: '🌶️', cover_gradient: grad(2), description: '鸡丁滑嫩、花生酥脆，糊辣荔枝味', tips: '糖醋比例 2:1，花生米最后放才脆', calories: 360, cook_count: 9, avg_rating: 4.6, tag_names: '下饭,宴客菜,高蛋白', steps: ['鸡胸切丁用盐料酒蛋清淀粉腌 15 分钟', '调碗汁：糖醋生抽淀粉清水', '滑炒鸡丁至变色捞出', '炒香干辣椒花椒豆瓣酱出红油', '下葱姜蒜末，倒回鸡丁', '淋碗汁收浓，关火拌花生米'], ingredients: [['鸡胸肉', 300, '克', '主料'], ['花生米', 60, '克', '辅料'], ['青椒', 80, '克', '辅料']] },
  { id: 4, name: '麻婆豆腐', category: '家常菜', cuisine: '川菜', difficulty: 'medium', prep_minutes: 12, cook_minutes: 15, total_minutes: 27, servings: 2, cover_emoji: '🍲', cover_gradient: grad(8), description: '麻辣烫香嫩，五块钱做出饭店水准', tips: '豆腐先用淡盐水焯烫定型，翻动用推不用铲', calories: 240, cook_count: 7, avg_rating: 4.5, tag_names: '下饭,素食', steps: ['豆腐切块淡盐水泡 10 分钟', '肉末炒散加豆瓣酱炒红油', '加姜蒜末辣椒面，倒高汤', '下豆腐轻推煮 5 分钟', '分两次勾芡', '撒花椒面和蒜苗末'], ingredients: [['嫩豆腐', 400, '克', '主料'], ['猪里脊', 100, '克', '主料'], ['郫县豆瓣酱', 20, '克', '调味料']] },
  { id: 5, name: '可乐鸡翅', category: '家常菜', cuisine: '中餐', difficulty: 'easy', prep_minutes: 10, cook_minutes: 25, total_minutes: 35, servings: 3, cover_emoji: '🍗', cover_gradient: grad(6), description: '甜咸焦香，小朋友最爱的入门硬菜', tips: '收汁时不停翻面，可乐含糖高易糊锅', calories: 410, cook_count: 15, avg_rating: 4.9, tag_names: '宝宝爱吃,快手菜', steps: ['鸡翅划刀焯水沥干', '煎至两面金黄', '倒可乐没过鸡翅加生抽老抽', '中小火焖 15 分钟', '大火收汁翻面至浓稠'], ingredients: [['鸡翅中', 600, '克', '主料'], ['可乐', 330, '毫升', '调味料'], ['生姜', 15, '克', '辅料']] },
  { id: 7, name: '青椒土豆丝', category: '家常菜', cuisine: '中餐', difficulty: 'easy', prep_minutes: 12, cook_minutes: 8, total_minutes: 20, servings: 2, cover_emoji: '🥔', cover_gradient: grad(5), description: '爽脆开胃的家常素菜之王', tips: '土豆丝泡水去淀粉，炒出来根根分明', calories: 150, cook_count: 18, avg_rating: 4.4, tag_names: '下饭,快手菜,低卡,素食', steps: ['土豆切丝泡水洗去淀粉', '青椒切丝蒜切片', '爆香蒜片干辣椒', '大火快炒 2 分钟加白醋', '放青椒丝炒 30 秒调味'], ingredients: [['土豆', 300, '克', '主料'], ['青椒', 100, '克', '辅料'], ['大蒜', 10, '克', '辅料']] },
  { id: 8, name: '蒜蓉西兰花', category: '家常菜', cuisine: '中餐', difficulty: 'easy', prep_minutes: 10, cook_minutes: 6, total_minutes: 16, servings: 2, cover_emoji: '🥦', cover_gradient: grad(1), description: '翠绿爽脆，低卡高纤维的快手菜', tips: '焯水加盐和几滴油，颜色保持翠绿', calories: 95, cook_count: 11, avg_rating: 4.3, tag_names: '快手菜,低卡,素食', steps: ['西兰花掰小朵洗净', '沸水加油盐焯 1 分钟捞出', '爆香蒜末', '下西兰花快炒加蚝油'], ingredients: [['西兰花', 300, '克', '主料'], ['大蒜', 20, '克', '辅料'], ['蚝油', 8, '毫升', '调味料']] },
  { id: 9, name: '清蒸鲈鱼', category: '家常菜', cuisine: '粤菜', difficulty: 'medium', prep_minutes: 15, cook_minutes: 18, total_minutes: 33, servings: 3, cover_emoji: '🐟', cover_gradient: grad(4), description: '原汁原味的宴客菜，鱼肉嫩如豆腐', tips: '水开后上锅，一斤鱼蒸 8 分钟虚蒸 2 分钟', calories: 210, cook_count: 6, avg_rating: 4.7, tag_names: '低卡,宴客菜,高蛋白,无辣', steps: ['鲈鱼处理干净划刀，料酒盐腌 10 分钟', '盘底铺姜葱，鱼身放上', '水开大火蒸 8 分钟虚蒸 2 分钟', '倒掉腥水铺新鲜葱姜丝', '淋蒸鱼豉油浇热油'], ingredients: [['鲈鱼', 1, '条', '主料'], ['生姜', 20, '克', '辅料'], ['小葱', 30, '克', '辅料']] },
  { id: 15, name: '番茄蛋花汤', category: '汤羹', cuisine: '中餐', difficulty: 'easy', prep_minutes: 8, cook_minutes: 10, total_minutes: 18, servings: 3, cover_emoji: '🍲', cover_gradient: grad(0), description: '红黄相间的国民汤品，清爽解腻', tips: '蛋液在汤微沸时画圈淋入，静置 3 秒再推', calories: 80, cook_count: 20, avg_rating: 4.6, tag_names: '快手菜,汤水,无辣,开胃', steps: ['番茄切块，鸡蛋打散加清水', '炒软番茄加糖盐', '加水煮 5 分钟', '画圈淋蛋液静置后轻推', '淋香油撒葱花'], ingredients: [['西红柿', 250, '克', '主料'], ['鸡蛋', 2, '个', '主料'], ['香油', 5, '毫升', '调味料']] },
  { id: 16, name: '玉米排骨汤', category: '汤羹', cuisine: '中餐', difficulty: 'medium', prep_minutes: 15, cook_minutes: 60, total_minutes: 75, servings: 4, cover_emoji: '🌽', cover_gradient: grad(9), description: '清甜滋补，汤色奶白不油腻', tips: '一次加足热水，中途加水会破坏汤味', calories: 220, cook_count: 8, avg_rating: 4.8, tag_names: '汤水,高蛋白,宴客菜', steps: ['排骨焯水撇沫温水冲洗', '砂锅加足热水放排骨姜片烧开', '小火炖 40 分钟', '加玉米胡萝卜炖 20 分钟', '加盐调味撒葱花'], ingredients: [['猪排骨', 500, '克', '主料'], ['甜玉米', 2, '根', '主料'], ['胡萝卜', 150, '克', '辅料']] },
  { id: 19, name: '扬州炒饭', category: '主食', cuisine: '中餐', difficulty: 'medium', prep_minutes: 12, cook_minutes: 10, total_minutes: 22, servings: 2, cover_emoji: '🍚', cover_gradient: grad(6), description: '粒粒分明、配料丰富的经典炒饭', tips: '用隔夜冷饭，下锅前抓散；锅要够热', calories: 480, cook_count: 14, avg_rating: 4.5, tag_names: '快手菜,宝宝爱吃', steps: ['米饭抓散，鸡蛋打散，配料切丁', '滑炒鸡蛋至半凝固倒入米饭', '中大火翻炒压散裹蛋液', '下配料丁炒匀', '加盐生抽白胡椒粉撒葱花'], ingredients: [['大米', 300, '克', '主料'], ['鸡蛋', 3, '个', '主料'], ['基围虾', 100, '克', '辅料'], ['豌豆', 60, '克', '辅料']] },
  { id: 24, name: '拍黄瓜', category: '凉菜', cuisine: '中餐', difficulty: 'easy', prep_minutes: 10, cook_minutes: 0, total_minutes: 10, servings: 3, cover_emoji: '🥒', cover_gradient: grad(7), description: '清脆爽口的夏日开胃凉菜', tips: '刀背拍裂再切段更易吸味，腌 10 分钟', calories: 45, cook_count: 16, avg_rating: 4.4, tag_names: '快手菜,低卡,素食,开胃', steps: ['黄瓜拍裂切段', '加盐腌 10 分钟倒掉水', '蒜切末小米椒切圈', '调料汁：蒜末生抽醋糖香油辣椒油', '浇汁拌匀冷藏'], ingredients: [['黄瓜', 400, '克', '主料'], ['大蒜', 20, '克', '辅料'], ['香醋', 20, '毫升', '调味料']] },
  { id: 27, name: '芒果班戟', category: '甜点', cuisine: '东南亚', difficulty: 'medium', prep_minutes: 30, cook_minutes: 15, total_minutes: 45, servings: 4, cover_emoji: '🥭', cover_gradient: grad(9), description: '薄皮裹住奶油与芒果，清甜不腻', tips: '面糊过筛两次更细腻，小火摊皮', calories: 310, cook_count: 5, avg_rating: 4.9, tag_names: '宝宝爱吃,宴客菜', steps: ['面粉糖蛋牛奶黄油混合过筛两次', '小火摊薄饼皮晾凉', '淡奶油加糖打发', '芒果切长条', '饼皮包奶油芒果四边折叠', '收口朝下冷藏 30 分钟对半切开'], ingredients: [['芒果', 400, '克', '主料'], ['面粉', 100, '克', '主料'], ['纯牛奶', 250, '毫升', '主料'], ['淡奶油', 200, '毫升', '主料']] },
  { id: 28, name: '香蕉牛奶昔', category: '甜点', cuisine: '西餐', difficulty: 'easy', prep_minutes: 5, cook_minutes: 3, total_minutes: 8, servings: 2, cover_emoji: '🍌', cover_gradient: grad(11), description: '两分钟搞定的健康饮品，早餐神器', tips: '香蕉提前冷冻，奶昔更浓稠不用加冰', calories: 180, cook_count: 10, avg_rating: 4.7, tag_names: '快手菜,宝宝爱吃,开胃', steps: ['香蕉去皮切段', '香蕉牛奶蜂蜜放料理机', '高速搅打 40 秒', '倒入杯中撒燕麦装饰'], ingredients: [['香蕉', 2, '根', '主料'], ['纯牛奶', 400, '毫升', '主料'], ['蜂蜜', 15, '克', '调味料']] }
]

// 食材 emoji 与分类映射（需在 mockRecipes 初始化前定义，否则触发暂时性死区）
const EMOJI_MAP = {
  五花肉: '🥓', 鸡蛋: '🥚', 西红柿: '🍅', 土豆: '🥔', 青椒: '🫑', 鸡胸肉: '🍗', 鸡翅中: '🍗',
  嫩豆腐: '🍥', 西兰花: '🥦', 鲈鱼: '🐟', 甜玉米: '🌽', 胡萝卜: '🥕', 猪排骨: '🍖', 大米: '🍚',
  基围虾: '🦐', 黄瓜: '🥒', 芒果: '🥭', 香蕉: '🍌', 面粉: '🌾', 纯牛奶: '🥛', 淡奶油: '🥛',
  花生米: '🥜', 猪里脊: '🥩', 大蒜: '🧄', 生姜: '🫚', 小葱: '🌿', 豌豆: '🫛', 挂面: '🍜',
  白糖: '🍬', 生抽: '🫙', 蚝油: '🦪', 香油: '🫗', 香醋: '🫙', 蜂蜜: '🍯', 可乐: '🥤',
  郫县豆瓣酱: '🌶️', 冰糖: '🍬'
}
const emojiOf = (n) => EMOJI_MAP[n] || '🥬'
const categoryOf = (g) => (g === '调味料' ? '调味' : '蔬菜')

// 统一补齐列表卡片需要的字段
mockRecipes.forEach((r) => {
  r.cover_url = null
  r.is_active = true
  r.favorite_count = Math.floor(r.cook_count / 3)
  r.cuisine = r.cuisine || '中餐'
  r.steps = (r.steps || []).map((content, i) => ({ id: r.id * 100 + i, recipe_id: r.id, step_no: i + 1, content, duration_sec: null }))
  r.ingredient_groups = groupIngredients(r.id, r.ingredients || [])
  r.tags = (r.tag_names ? r.tag_names.split(',') : []).map((n) => {
    const t = mockTags.find((x) => x.name === n.trim())
    return t || { id: 0, name: n.trim(), color: '#C2703D', sort_order: 99 }
  })
  r.favorite_members = []
  r.creator_name = '妈妈'
  r.created_at = `${dayAgo(30)} 10:00:00`
  r.updated_at = `${dayAgo(2)} 18:30:00`
})

function groupIngredients(recipeId, list) {
  const order = ['主料', '辅料', '调味料', '其他']
  const map = new Map()
  list.forEach((it, i) => {
    const [name, quantity, unit, group] = it
    const g = group || '主料'
    if (!map.has(g)) map.set(g, [])
    map.get(g).push({
      id: recipeId * 1000 + i,
      ingredient_id: i + 1,
      name,
      emoji: emojiOf(name),
      category: categoryOf(g),
      quantity,
      unit,
      group_name: g,
      is_optional: false,
      sort_order: i + 1
    })
  })
  return order.filter((g) => map.has(g)).map((g) => ({ group_name: g, items: map.get(g) }))
}

const STAPLES = new Set(['大米', '挂面', '面粉', '白糖', '生抽', '蚝油', '香油', '香醋', '蜂蜜', '鸡蛋', '大蒜', '生姜', '小葱', '花生米'])

export const mockIngredients = (() => {
  const map = new Map()
  mockRecipes.forEach((r) => {
    r.ingredient_groups.forEach((g) => {
      g.items.forEach((it) => {
        if (!map.has(it.name)) {
          map.set(it.name, {
            id: map.size + 1,
            name: it.name,
            category: categoryOf(it.group_name),
            unit: it.unit,
            emoji: it.emoji,
            is_staple: STAPLES.has(it.name),
            is_active: true,
            created_at: `${dayAgo(60)} 09:00:00`
          })
        }
      })
    })
  })
  return [...map.values()]
})()

export const mockOrders = [
  {
    id: 1, order_no: `${dayAgo(4).replace(/-/g, '')}-D-001`, member_id: 1, meal_date: dayAgo(4),
    meal_type: 'dinner', status: 'served', diners: 4, note: '周末家庭聚餐',
    served_at: `${dayAgo(4)} 18:30:00`, created_at: `${dayAgo(4)} 16:00:00`,
    items: [
      { recipe_id: 1, recipe_snapshot: '红烧肉', portions: 2, member_id: 1, rating: 5, item_status: 'done' },
      { recipe_id: 2, recipe_snapshot: '番茄炒蛋', portions: 1, member_id: 4, rating: 4, item_status: 'done' },
      { recipe_id: 8, recipe_snapshot: '蒜蓉西兰花', portions: 1, member_id: 2, rating: 5, item_status: 'done' },
      { recipe_id: 16, recipe_snapshot: '玉米排骨汤', portions: 1, member_id: 5, rating: 5, item_status: 'done' }
    ]
  },
  {
    id: 2, order_no: `${dayAgo(2).replace(/-/g, '')}-L-001`, member_id: 2, meal_date: dayAgo(2),
    meal_type: 'lunch', status: 'served', diners: 3, note: '奶奶要清淡',
    served_at: `${dayAgo(2)} 12:15:00`, created_at: `${dayAgo(2)} 10:30:00`,
    items: [
      { recipe_id: 9, recipe_snapshot: '清蒸鲈鱼', portions: 1, member_id: 5, rating: 5, item_status: 'done' },
      { recipe_id: 7, recipe_snapshot: '青椒土豆丝', portions: 1, member_id: 2, rating: 4, item_status: 'done' },
      { recipe_id: 15, recipe_snapshot: '番茄蛋花汤', portions: 1, member_id: 2, rating: 5, item_status: 'done' }
    ]
  },
  {
    id: 3, order_no: `${dayAgo(1).replace(/-/g, '')}-D-001`, member_id: 3, meal_date: dayAgo(1),
    meal_type: 'dinner', status: 'cooking', diners: 4, note: '妹妹不吃青椒',
    served_at: null, created_at: `${dayAgo(1)} 17:00:00`,
    items: [
      { recipe_id: 3, recipe_snapshot: '宫保鸡丁', portions: 1, member_id: 3, rating: null, item_status: 'cooking', note: '不放青椒' },
      { recipe_id: 4, recipe_snapshot: '麻婆豆腐', portions: 1, member_id: 1, rating: null, item_status: 'pending' },
      { recipe_id: 19, recipe_snapshot: '扬州炒饭', portions: 1, member_id: 3, rating: null, item_status: 'pending' }
    ]
  },
  {
    id: 4, order_no: `${today.replace(/-/g, '')}-D-001`, member_id: 4, meal_date: today,
    meal_type: 'dinner', status: 'draft', diners: 5, note: '今晚吃什么草稿',
    served_at: null, created_at: `${today} 09:00:00`,
    items: [
      { recipe_id: 5, recipe_snapshot: '可乐鸡翅', portions: 2, member_id: 4, rating: null, item_status: 'pending' },
      { recipe_id: 24, recipe_snapshot: '拍黄瓜', portions: 1, member_id: 2, rating: null, item_status: 'pending' },
      { recipe_id: 28, recipe_snapshot: '香蕉牛奶昔', portions: 1, member_id: 4, rating: null, item_status: 'pending' }
    ]
  }
].map((o, i) => {
  o.id = i + 1
  o.items = o.items.map((it, j) => ({
    ...it,
    id: o.id * 100 + j + 1,
    order_id: o.id,
    sort_order: j + 1,
    note: it.note || null,
    recipe_name: it.recipe_snapshot
  }))
  o.updated_at = o.created_at
  return o
})

export const mockOverview = {
  recipe_total: mockRecipes.length,
  recipe_active: mockRecipes.length,
  member_total: mockMembers.length,
  ingredient_total: mockIngredients.length,
  order_total: mockOrders.filter((o) => o.status !== 'cancelled').length,
  order_today: mockOrders.filter((o) => o.meal_date === today && o.status !== 'cancelled').length,
  order_pending: mockOrders.filter((o) => ['draft', 'confirmed'].includes(o.status)).length,
  dishes_served: mockOrders.filter((o) => o.status === 'served').reduce((s, o) => s + o.items.length, 0),
  avg_rating: 4.7,
  top_category: { category: '家常菜', recipe_count: 7, order_count: 8 },
  favorite_dish: { id: 2, name: '番茄炒蛋', cover_emoji: '🍅', cover_gradient: grad(0), category: '家常菜', order_count: 24, total_portions: 26, avg_rating: 4.9 }
}

/** 依据订单实时聚合采购清单（复刻数据库视图 v_order_shopping_preview 的逻辑） */
export function computeShoppingList(order) {
  const map = new Map()
  order.items.forEach((it) => {
    if (it.item_status === 'skipped') return
    const recipe = mockRecipes.find((r) => r.id === it.recipe_id)
    if (!recipe) return
    recipe.ingredient_groups.forEach((g) => {
      g.items.forEach((ing) => {
        if (ing.is_optional) return
        const key = `${ing.name}|${ing.unit}`
        if (!map.has(key)) {
          const dict = mockIngredients.find((x) => x.name === ing.name)
          map.set(key, {
            id: map.size + 1,
            ingredient_id: dict?.id ?? null,
            name_snapshot: ing.name,
            emoji: ing.emoji,
            category: ing.category,
            total_quantity: 0,
            unit: ing.unit,
            is_staple: dict?.is_staple ?? false,
            is_purchased: false,
            recipe_names: []
          })
        }
        const slot = map.get(key)
        slot.total_quantity += (ing.quantity * it.portions) / (recipe.servings || 1)
        if (!slot.recipe_names.includes(it.recipe_snapshot)) slot.recipe_names.push(it.recipe_snapshot)
      })
    })
  })

  const items = [...map.values()].map((x) => ({
    ...x,
    total_quantity: Math.round(x.total_quantity * 100) / 100,
    recipe_names: x.recipe_names.join('、')
  }))

  const ORDER = ['蔬菜', '肉禽', '水产', '蛋奶', '豆制品', '主食', '干货', '乳品', '水果', '烘焙', '调味', '饮品', '其他']
  const need = items.filter((x) => !x.is_staple).sort((a, b) => ORDER.indexOf(a.category) - ORDER.indexOf(b.category) || a.name_snapshot.localeCompare(b.name_snapshot, 'zh'))
  const staples = items.filter((x) => x.is_staple)

  const groups = []
  need.forEach((x) => {
    const last = groups[groups.length - 1]
    if (last && last.category === x.category) last.items.push(x)
    else groups.push({ category: x.category, items: [x] })
  })

  return {
    groups,
    staples,
    total_kinds: items.length,
    purchased_kinds: items.filter((x) => x.is_purchased).length
  }
}

export const mockMeta = {
  categories: ['家常菜', '汤羹', '主食', '凉菜', '甜点', '饮品', '烘焙', '其他'],
  cuisines: ['中餐', '川菜', '粤菜', '湘菜', '鲁菜', '西餐', '日韩', '东南亚'],
  difficulties: [
    { value: 'easy', label: '简单', emoji: '🟢' },
    { value: 'medium', label: '中等', emoji: '🟡' },
    { value: 'hard', label: '挑战', emoji: '🔴' }
  ],
  meal_types: [
    { value: 'breakfast', label: '早餐', emoji: '🌅' },
    { value: 'lunch', label: '午餐', emoji: '☀️' },
    { value: 'dinner', label: '晚餐', emoji: '🌙' },
    { value: 'snack', label: '加餐', emoji: '🍪' }
  ],
  order_status: [
    { value: 'draft', label: '草稿' },
    { value: 'confirmed', label: '已确认' },
    { value: 'cooking', label: '烹饪中' },
    { value: 'served', label: '已开饭' },
    { value: 'cancelled', label: '已取消' }
  ],
  ingredient_groups: ['主料', '辅料', '调味料', '其他'],
  member_roles: [
    { value: 'admin', label: '掌勺人' },
    { value: 'member', label: '家庭成员' },
    { value: 'child', label: '小朋友' }
  ],
  cover_presets: [
    { emoji: '🍖', gradient: grad(3) }, { emoji: '🍅', gradient: grad(0) },
    { emoji: '🌶️', gradient: grad(2) }, { emoji: '🍲', gradient: grad(8) },
    { emoji: '🍗', gradient: grad(6) }, { emoji: '🥔', gradient: grad(5) },
    { emoji: '🥦', gradient: grad(1) }, { emoji: '🐟', gradient: grad(4) },
    { emoji: '🍚', gradient: grad(9) }, { emoji: '🥒', gradient: grad(7) },
    { emoji: '🥭', gradient: grad(9) }, { emoji: '🍮', gradient: grad(8) }
  ]
}
