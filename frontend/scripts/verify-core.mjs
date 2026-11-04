/**
 * 核心算法自检：采购清单的「按份数换算 + 跨菜合并去重」
 *
 * 直接 import 前端离线数据的真实实现来跑，不依赖后端与端口。
 * 运行：npm run verify   或   node scripts/verify-core.mjs
 *
 * 后端由 SQL 视图 v_order_shopping_preview + 存储过程 sp_generate_shopping_list
 * 实现等价逻辑，本脚本用于快速回归「换算/合并」这一最易错的核心规则。
 */
import { computeShoppingList, mockOrders, mockRecipes } from '../src/api/mock.js'

let pass = 0, fail = 0
const ok = (cond, msg) => { if (cond) { pass++; console.log('  ✓', msg) } else { fail++; console.error('  ✗', msg) } }
const flatten = (l) => [...l.groups.flatMap((g) => g.items), ...l.staples]

const order = mockOrders.find((o) => o.id === 1)
console.log('订单1 菜品：', order.items.map((i) => `${i.recipe_snapshot}×${i.portions}`).join('、'))
const list = computeShoppingList(order)
const all = flatten(list)

console.log('\n[测试1] 采购清单非空且按分类分组')
ok(all.length > 0, `聚合出 ${all.length} 种食材`)
ok(list.groups.every((g) => g.items.length > 0), '每个分组都非空')

console.log('\n[测试2] 份数换算（红烧肉×2，默认3人份 → 五花肉 600g×2/3=400g）')
const pork = all.find((x) => x.name_snapshot === '五花肉')
ok(!!pork, '找到「五花肉」条目')
ok(pork && Math.abs(pork.total_quantity - 400) < 0.01, `五花肉用量 = ${pork?.total_quantity}g（期望 400）`)

console.log('\n[测试3] 跨菜合并（同食材同单位累加）')
const r1 = mockRecipes.find((r) => r.name === '番茄炒蛋')      // 西红柿 300g / 2人份
const r2 = mockRecipes.find((r) => r.name === '番茄蛋花汤')    // 西红柿 250g / 3人份
const merged = computeShoppingList({
  items: [
    { recipe_id: r1.id, recipe_snapshot: '番茄炒蛋', portions: 1, item_status: 'pending' },
    { recipe_id: r2.id, recipe_snapshot: '番茄蛋花汤', portions: 1, item_status: 'pending' }
  ]
})
const tomato = flatten(merged).find((x) => x.name_snapshot === '西红柿')
ok(!!tomato, '找到「西红柿」条目')
// 300×1/2 + 250×1/3 = 150 + 83.33 = 233.33
ok(tomato && Math.abs(tomato.total_quantity - 233.33) < 0.1, `跨菜合并 = ${tomato?.total_quantity}g（期望≈233.33）`)
ok(tomato && tomato.recipe_names.includes('番茄炒蛋') && tomato.recipe_names.includes('番茄蛋花汤'), `来源标注 = ${tomato?.recipe_names}`)

console.log('\n[测试4] 常备食材分流到 staples，不进采购分组')
const eggS = list.staples.find((x) => x.name_snapshot === '鸡蛋')
const eggG = list.groups.flatMap((g) => g.items).find((x) => x.name_snapshot === '鸡蛋')
ok(!!eggS && !eggG, '鸡蛋（常备）只在 staples')

console.log('\n[测试5] 同名食材按单位区分，不错误相加')
const potato = all.filter((x) => x.name_snapshot === '土豆')
ok(potato.length <= 1 || new Set(potato.map((p) => p.unit)).size === potato.length, '土豆按单位区分统计')

console.log('\n[测试6] 标记 skipped 的菜品不计入采购清单')
const skipRecipe = mockRecipes.find((r) => r.name === '红烧肉')
const skipList = computeShoppingList({
  items: [{ recipe_id: skipRecipe.id, recipe_snapshot: '红烧肉', portions: 2, item_status: 'skipped' }]
})
ok(flatten(skipList).length === 0, `全部 skipped 时清单为空（实际 ${flatten(skipList).length} 项）`)

console.log(`\n========== 结果：${pass} 通过 / ${fail} 失败 ==========`)
process.exit(fail > 0 ? 1 : 0)
