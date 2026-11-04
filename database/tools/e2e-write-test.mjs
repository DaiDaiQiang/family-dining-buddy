/**
 * 核心写链路端到端测试（测试完自动清理，恢复原状）
 *
 * 覆盖：下单 → 点餐时顺手建档 → 确认订单生成采购清单 →
 *       追加菜品 → 出菜 → 评分 → 采购勾选 → 取消/删除
 *
 * 全程记录被改动的行，结束时逐项还原并核验，确保不留测试痕迹。
 */
const BASE = 'http://127.0.0.1:8080/api'

async function call(method, path, body) {
  const res = await fetch(BASE + path, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined
  })
  const text = await res.text()
  let json
  try { json = JSON.parse(text) } catch { json = { raw: text } }
  if (!res.ok || json.code !== 0) {
    throw new Error(`${method} ${path} → HTTP ${res.status} code=${json.code} ${json.message || text.slice(0, 200)}`)
  }
  return json.data
}

let pass = 0, fail = 0
const ok = (c, m) => { c ? (pass++, console.log('  ✓', m)) : (fail++, console.error('  ✗', m)) }

// ---------- 快照：测试前的关键状态 ----------
const before = {
  recipeCount: (await call('GET', '/recipes?page_size=1')).total,
  orderCount: (await call('GET', '/orders')).total,
  ingredientCount: (await call('GET', '/ingredients?page_size=1')).total,
  tagCount: (await call('GET', '/tags')).length,
  // 记下将被评分的菜谱原状态
  recipe1: await call('GET', '/recipes/1')
}
console.log(`测试前：菜谱 ${before.recipeCount}，订单 ${before.orderCount}，食材 ${before.ingredientCount}，标签 ${before.tagCount}`)
console.log(`        红烧肉 cook_count=${before.recipe1.cook_count} avg_rating=${before.recipe1.avg_rating}\n`)

let createdOrderId = null
let createdRecipeId = null
let testIngredientId = null
let testTagId = null

try {
  // ---------- 1) 下单：混合「已有菜谱」与「点餐时顺手建档」 ----------
  console.log('[1] 下单（含点餐时顺手建档新菜）')
  const order = await call('POST', '/orders', {
    member_id: 2,
    meal_date: '2026-10-08',
    meal_type: 'dinner',
    diners: 4,
    note: '自动化测试订单（稍后删除）',
    auto_confirm: false,
    items: [
      { recipe_id: 1, portions: 2, member_id: 1 },   // 红烧肉 ×2
      { recipe_id: 2, portions: 1, member_id: 4 },   // 番茄炒蛋 ×1
      {
        // 顺手建档：菜谱库里没有的新菜
        portions: 1,
        member_id: 3,
        note: '现场新记的菜',
        new_recipe: {
          name: '测试菜_自动化勿动',
          category: '家常菜',
          cuisine: '中餐',
          difficulty: 'easy',
          prep_minutes: 5,
          cook_minutes: 10,
          servings: 2,
          cover_emoji: '🧪',
          cover_gradient: 'linear-gradient(135deg,#11998E,#38EF7D)',
          description: '端到端测试用的临时菜谱',
          tags: ['测试标签_勿动'],
          ingredients: [
            { name: '测试食材_勿动', quantity: 100, unit: '克', group_name: '主料' },
            { name: '土豆', quantity: 200, unit: '克', group_name: '主料' }
          ],
          steps: ['第一步：测试', '第二步：测试']
        }
      }
    ]
  })

  createdOrderId = order.order.id
  createdRecipeId = order.created_recipe_ids?.[0]
  ok(!!createdOrderId, `订单已创建 id=${createdOrderId} 单号=${order.order.order_no}`)
  ok(order.order.items.length === 3, `订单含 ${order.order.items.length} 道菜（期望 3）`)
  ok(order.order.status === 'draft', `初始状态为草稿（${order.order.status}）`)

  // ---------- 2) 顺手建档真的入库了 ----------
  console.log('\n[2] 点餐时顺手建档')
  ok(!!createdRecipeId, `新菜谱已建档 id=${createdRecipeId}`)
  const newRecipe = await call('GET', `/recipes/${createdRecipeId}`)
  // 记下测试新建的食材与标签 ID，结束时一并清理
  testIngredientId = newRecipe.ingredient_groups
    .flatMap(g => g.items)
    .find(i => i.name === '测试食材_勿动')?.ingredient_id || null
  testTagId = newRecipe.tags.find(t => t.name === '测试标签_勿动')?.id || null
  ok(newRecipe.name === '测试菜_自动化勿动', `菜名正确：${newRecipe.name}`)
  ok(newRecipe.steps.length === 2, `步骤写入 ${newRecipe.steps.length} 条`)
  ok(newRecipe.ingredient_groups.length === 1 && newRecipe.ingredient_groups[0].items.length === 2,
    `食材写入 ${newRecipe.ingredient_groups[0]?.items.length} 项`)
  ok(newRecipe.tags.some(t => t.name === '测试标签_勿动'), `标签自动创建并关联`)

  const afterCreate = {
    recipeCount: (await call('GET', '/recipes?page_size=1')).total,
    ingredientCount: (await call('GET', '/ingredients?page_size=1')).total,
    tagCount: (await call('GET', '/tags')).length
  }
  ok(afterCreate.recipeCount === before.recipeCount + 1, `菜谱总数 ${before.recipeCount} → ${afterCreate.recipeCount}`)
  ok(afterCreate.ingredientCount === before.ingredientCount + 1, `食材字典新增1种（土豆已存在则复用）：${before.ingredientCount} → ${afterCreate.ingredientCount}`)
  ok(afterCreate.tagCount === before.tagCount + 1, `标签新增1个：${before.tagCount} → ${afterCreate.tagCount}`)

  // ---------- 3) 草稿态：采购清单实时计算 ----------
  console.log('\n[3] 草稿态采购清单（实时计算）')
  const draftList = await call('GET', `/orders/${createdOrderId}/shopping-list`)
  ok(draftList.total_kinds > 0, `草稿态可实时算出 ${draftList.total_kinds} 种食材`)
  // 红烧肉600g×2份÷3人份 = 400g
  const pork = [...draftList.groups.flatMap(g => g.items), ...draftList.staples]
    .find(x => x.name_snapshot === '猪五花肉')
  ok(pork && Math.abs(Number(pork.total_quantity) - 400) < 0.01,
    `份数换算正确：猪五花肉 ${pork?.total_quantity}${pork?.unit}（期望 400克）`)

  // ---------- 4) 确认订单 → 生成并落库采购清单 ----------
  console.log('\n[4] 确认订单（触发存储过程生成采购清单）')
  const confirmed = await call('POST', `/orders/${createdOrderId}/confirm`)
  ok(confirmed.status === 'confirmed', `状态变为 ${confirmed.status}`)
  ok(confirmed.shopping_list.total_kinds > 0, `采购清单已落库 ${confirmed.shopping_list.total_kinds} 项`)

  // cook_count 的口径是「有效订单中被点的总份数」，本单点了红烧肉 ×2 份，
  // 因此确认后应为 before + 2（早先这里误写成 +1，把订单数当成了份数）
  const r1After = await call('GET', '/recipes/1')
  ok(r1After.cook_count === before.recipe1.cook_count + 2,
    `菜谱点单份数累加：${before.recipe1.cook_count} → ${r1After.cook_count}（本单点了 2 份）`)

  // ---------- 5) 追加菜品 → 采购清单应重算 ----------
  console.log('\n[5] 追加菜品并重算采购清单')
  const kindsBefore = confirmed.shopping_list.total_kinds
  await call('POST', `/orders/${createdOrderId}/items`, { recipe_id: 8, portions: 2 })
  const afterAdd = await call('GET', `/orders/${createdOrderId}`)
  ok(afterAdd.items.length === 4, `订单变为 ${afterAdd.items.length} 道菜`)
  ok(afterAdd.shopping_list.total_kinds >= kindsBefore,
    `采购清单重算：${kindsBefore} → ${afterAdd.shopping_list.total_kinds} 项（蒜蓉西兰花已加入）`)

  // ---------- 6) 出菜 + 评分 ----------
  console.log('\n[6] 出菜状态与餐后评分')
  const targetItem = afterAdd.items.find(i => i.recipe_id === 1)
  await call('PUT', `/orders/items/${targetItem.id}`, { item_status: 'done' })
  const afterDone = await call('GET', `/orders/${createdOrderId}`)
  ok(afterDone.items.find(i => i.id === targetItem.id).item_status === 'done', '出菜状态已标记为 done')

  const rateRes = await call('POST', `/orders/items/${targetItem.id}/rating`, { rating: 3 })
  ok(rateRes.rating === 3, `评分提交成功 rating=${rateRes.rating}`)
  ok(typeof rateRes.recipe_avg_rating !== 'undefined', `菜谱平均分已重算为 ${rateRes.recipe_avg_rating}`)

  // ---------- 7) 采购清单勾选 ----------
  console.log('\n[7] 采购清单勾选')
  const shop = await call('GET', `/orders/${createdOrderId}/shopping-list`)
  const firstNeed = shop.groups[0]?.items[0]
  if (firstNeed) {
    const toggled = await call('PUT', `/orders/shopping/${firstNeed.id}`, { is_purchased: true })
    const bought = [...toggled.groups.flatMap(g => g.items), ...toggled.staples].filter(x => x.is_purchased)
    ok(bought.length === 1, `勾选已购买：${firstNeed.name_snapshot}（已购 ${bought.length} 项）`)
  } else {
    ok(false, '未找到可勾选的采购项')
  }

  // ---------- 8) 菜单与做法 ----------
  console.log('\n[8] 菜单卡片与做法串联')
  const menu = await call('GET', `/orders/${createdOrderId}/menu`)
  ok(menu.dish_count === 4, `菜单含 ${menu.dish_count} 道菜`)
  const recs = await call('GET', `/orders/${createdOrderId}/recipes`)
  ok(recs.recipe_count === 4, `做法串联 ${recs.recipe_count} 道菜，累计 ${recs.total_cook_minutes} 分钟`)
  ok(recs.recipes.some(r => r.name === '测试菜_自动化勿动'), '新建档的菜也在做法列表中')

  // ---------- 9) 再来一次（复制订单） ----------
  console.log('\n[9] 照这单再来一次')
  const re = await call('POST', `/orders/${createdOrderId}/reorder`)
  const reorderId = re.order.id
  ok(!!reorderId && reorderId !== createdOrderId, `已生成新草稿 id=${reorderId}`)
  ok(re.order.items.length === 4, `复制了 ${re.order.items.length} 道菜`)
  ok(re.order.status === 'draft', `新单为草稿状态`)
  // 立即清理这个副本
  await call('DELETE', `/orders/${reorderId}`)
  console.log(`  （副本订单 ${reorderId} 已删除）`)

} catch (e) {
  fail++
  console.error('\n✗ 测试过程异常：', e.message)
} finally {
  // ---------- 清理：还原数据库到测试前状态 ----------
  console.log('\n' + '='.repeat(60))
  console.log('  清理测试数据，还原数据库')
  console.log('='.repeat(60))

  // 删除测试订单（级联删除 order_item / shopping_item）
  if (createdOrderId) {
    await call('DELETE', `/orders/${createdOrderId}`).catch(e => console.error('  删除订单失败:', e.message))
    console.log(`  已删除测试订单 ${createdOrderId}（含明细与采购清单）`)
  }
  // 物理删除测试菜谱
  if (createdRecipeId) {
    await call('DELETE', `/recipes/${createdRecipeId}?permanent=true`).catch(e => console.error('  删除菜谱失败:', e.message))
    console.log(`  已删除测试菜谱 ${createdRecipeId}`)
  }
  // 删除测试期间新建的食材与标签。
  // 必须放在删除菜谱之后：recipe_ingredient / recipe_tag 的外键仍指向它们，
  // 先删会被 RESTRICT 拦下（食材）或造成悬挂关联（标签）。
  if (testIngredientId) {
    await call('DELETE', `/ingredients/${testIngredientId}`).catch(e => console.error('  删除测试食材失败:', e.message))
    console.log(`  已删除测试食材 ${testIngredientId}`)
  }
  if (testTagId) {
    await call('DELETE', `/tags/${testTagId}`).catch(e => console.error('  删除测试标签失败:', e.message))
    console.log(`  已删除测试标签 ${testTagId}`)
  }
  // 还原核验：测试期间打过分，删单后统计必须回到原值。
  // 这里用硬断言而非提示 —— 早先正是因为只打印「需人工核对」，
  // 才让「删单不回退统计」的缺陷通过了测试。
  const r1Now = await call('GET', '/recipes/1')
  ok(String(r1Now.avg_rating) === String(before.recipe1.avg_rating),
    `红烧肉 avg_rating 已还原 ${r1Now.avg_rating}（测试前 ${before.recipe1.avg_rating}）`)
  ok(r1Now.cook_count === before.recipe1.cook_count,
    `红烧肉 cook_count 已还原 ${r1Now.cook_count}（测试前 ${before.recipe1.cook_count}）`)

  // 核验还原结果
  const after = {
    recipeCount: (await call('GET', '/recipes?page_size=1')).total,
    orderCount: (await call('GET', '/orders')).total,
    ingredientCount: (await call('GET', '/ingredients?page_size=1')).total,
    tagCount: (await call('GET', '/tags')).length
  }
  console.log('\n  还原核验：')
  ok(after.recipeCount === before.recipeCount, `菜谱数还原 ${after.recipeCount}/${before.recipeCount}`)
  ok(after.orderCount === before.orderCount, `订单数还原 ${after.orderCount}/${before.orderCount}`)
  ok(after.ingredientCount === before.ingredientCount,
    `食材字典还原 ${after.ingredientCount}/${before.ingredientCount}（测试食材已自清理）`)
  ok(after.tagCount === before.tagCount,
    `标签字典还原 ${after.tagCount}/${before.tagCount}（测试标签已自清理）`)
}

console.log('\n' + '='.repeat(60))
console.log(`  写链路测试：${pass} 通过 / ${fail} 失败`)
console.log('='.repeat(60))
process.exit(fail > 0 ? 1 : 0)
