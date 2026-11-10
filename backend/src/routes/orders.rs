//! 订单模块：点餐下单、顺手建档、菜单生成、采购清单聚合、出菜与评分。

use actix_web::{web, HttpResponse};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::{MySql, MySqlConnection, MySqlPool, QueryBuilder};

use crate::db::{next_order_no, AppState};
use crate::dto::{
    validate_item_status, validate_order, validate_status, CreateOrderReq, OrderItemReq,
    UpdateOrderItemReq, UpdateOrderReq, UpdateShoppingItemReq,
};
use crate::error::ApiError;
use crate::models::*;
use crate::routes::recipes::{build_recipe_detail, insert_recipe_in_tx};
use crate::util::{normalize_paging, ok, ok_with_message, Page};

fn clean(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

// =====================================================================
//  采购清单构建
// =====================================================================

/// 数据库视图 `v_order_shopping_preview` 的一行
#[derive(Debug, sqlx::FromRow)]
struct PreviewRow {
    ingredient_id: Option<u64>,
    ingredient_name: String,
    emoji: String,
    category: String,
    unit: String,
    total_quantity: Decimal,
    recipe_names: Option<String>,
    is_staple: Option<bool>,
}

/// 按食材分类固定顺序排列
const CATEGORY_ORDER: [&str; 13] = [
    "蔬菜", "肉禽", "水产", "蛋奶", "豆制品", "主食", "干货", "乳品", "水果", "烘焙", "调味",
    "饮品", "其他",
];

fn category_rank(c: &str) -> usize {
    CATEGORY_ORDER
        .iter()
        .position(|x| *x == c)
        .unwrap_or(CATEGORY_ORDER.len())
}

/// 把采购项分组：需采购的按分类归拢，常备的单独一列
fn build_shopping_list(items: Vec<ShoppingItemView>) -> ShoppingList {
    let mut need: Vec<ShoppingItemView> = Vec::new();
    let mut staples: Vec<ShoppingItemView> = Vec::new();

    for it in items {
        if it.is_staple {
            staples.push(it);
        } else {
            need.push(it);
        }
    }

    need.sort_by(|a, b| {
        category_rank(&a.category)
            .cmp(&category_rank(&b.category))
            .then_with(|| a.name_snapshot.cmp(&b.name_snapshot))
    });

    let mut groups: Vec<ShoppingGroup> = Vec::new();
    for it in need {
        match groups.last_mut() {
            Some(g) if g.category == it.category => g.items.push(it),
            _ => groups.push(ShoppingGroup {
                category: it.category.clone(),
                items: vec![it],
            }),
        }
    }

    let purchased = staples
        .iter()
        .filter(|s| s.is_purchased)
        .count()
        + groups
            .iter()
            .flat_map(|g| g.items.iter())
            .filter(|i| i.is_purchased)
            .count();
    let total_kinds = staples.len() + groups.iter().map(|g| g.items.len()).sum::<usize>();

    ShoppingList {
        groups,
        staples,
        total_kinds,
        purchased_kinds: purchased,
    }
}

/// 草稿订单：实时从视图计算，不落库（此时还没确认，采购单随时会变）
async fn shopping_from_view(pool: &MySqlPool, order_id: u64) -> Result<ShoppingList, ApiError> {
    let rows: Vec<PreviewRow> = sqlx::query_as(
        "SELECT ingredient_id, ingredient_name, emoji, category, unit, \
                total_quantity, recipe_names, is_staple \
         FROM v_order_shopping_preview WHERE order_id = ?",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| ShoppingItemView {
            id: 0,
            ingredient_id: r.ingredient_id,
            name_snapshot: r.ingredient_name,
            emoji: r.emoji,
            category: r.category,
            total_quantity: r.total_quantity,
            unit: r.unit,
            is_staple: r.is_staple.unwrap_or(false),
            is_purchased: false,
            recipe_names: r.recipe_names,
        })
        .collect();

    Ok(build_shopping_list(items))
}

/// 已确认订单：读取落库的 `shopping_item`（保留「已购买」勾选状态）
async fn shopping_from_table(pool: &MySqlPool, order_id: u64) -> Result<ShoppingList, ApiError> {
    let items: Vec<ShoppingItemView> = sqlx::query_as(
        "SELECT * FROM shopping_item WHERE order_id = ? ORDER BY is_staple, category, name_snapshot",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;
    Ok(build_shopping_list(items))
}

/// 订单采购清单入口
async fn load_shopping_list(
    pool: &MySqlPool,
    order_id: u64,
    status: &str,
) -> Result<ShoppingList, ApiError> {
    if status == "draft" {
        shopping_from_view(pool, order_id).await
    } else {
        shopping_from_table(pool, order_id).await
    }
}

/// 用 Rust 原生 SQL 生成/刷新采购清单（等价于 `sp_generate_shopping_list`）
async fn regenerate_shopping_list_tx(
    tx: &mut MySqlConnection,
    order_id: u64,
) -> Result<u64, ApiError> {
    sqlx::query("DELETE FROM shopping_item WHERE order_id = ?")
        .bind(order_id)
        .execute(&mut *tx)
        .await?;

    let result = sqlx::query(
        "INSERT INTO shopping_item \
           (order_id, ingredient_id, name_snapshot, emoji, category, \
            total_quantity, unit, is_staple, recipe_names) \
         SELECT p.order_id, p.ingredient_id, p.ingredient_name, p.emoji, p.category, \
                p.total_quantity, p.unit, p.is_staple, p.recipe_names \
           FROM v_order_shopping_preview p \
          WHERE p.order_id = ? \
          ORDER BY p.is_staple ASC, p.category ASC, p.ingredient_name ASC",
    )
    .bind(order_id)
    .execute(&mut *tx)
    .await?;

    Ok(result.rows_affected())
}

// =====================================================================
//  列表与详情
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct OrderListQuery {
    pub status: Option<String>,
    pub meal_type: Option<String>,
    pub member_id: Option<u64>,
    /// 起始日期（含）
    pub from: Option<NaiveDate>,
    /// 截止日期（含）
    pub to: Option<NaiveDate>,
    /// true 只看今天
    pub today: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 把订单筛选条件追加到查询构造器。
///
/// 用 `QueryBuilder` 而非手写 `?` + 绑定数组，可彻底避免
/// 「条件个数与绑定顺序错位」这类隐蔽 bug；计数与列表两次调用结果一致。
fn apply_order_filters(qb: &mut QueryBuilder<'_, MySql>, q: &OrderListQuery) {
    qb.push(" WHERE 1=1");

    if let Some(s) = clean(q.status.as_deref()) {
        qb.push(" AND o.status = ");
        qb.push_bind(s);
    }
    if let Some(t) = clean(q.meal_type.as_deref()) {
        qb.push(" AND o.meal_type = ");
        qb.push_bind(t);
    }
    if let Some(mid) = q.member_id {
        qb.push(" AND o.member_id = ");
        qb.push_bind(mid);
    }

    if q.today == Some(true) {
        qb.push(" AND o.meal_date = ");
        qb.push_bind(chrono::Local::now().date_naive());
    } else {
        if let Some(f) = q.from {
            qb.push(" AND o.meal_date >= ");
            qb.push_bind(f);
        }
        if let Some(t) = q.to {
            qb.push(" AND o.meal_date <= ");
            qb.push_bind(t);
        }
    }
}

/// 订单列表的公共 SELECT 片段（含菜品数、份数、菜名与封面 emoji 聚合）
const ORDER_SUMMARY_COLUMNS: &str = "o.id, o.order_no, o.member_id, \
    m.nickname AS member_nickname, m.avatar_emoji AS member_avatar, \
    o.meal_date, o.meal_type, o.status, o.diners, o.note, o.served_at, o.created_at, \
    (SELECT COUNT(*) FROM order_item oi WHERE oi.order_id = o.id AND oi.item_status <> 'skipped') AS dish_count, \
    (SELECT COALESCE(CAST(SUM(oi.portions) AS SIGNED),0) FROM order_item oi WHERE oi.order_id = o.id AND oi.item_status <> 'skipped') AS total_portions, \
    (SELECT GROUP_CONCAT(oi.recipe_snapshot ORDER BY oi.sort_order SEPARATOR '、') \
       FROM order_item oi WHERE oi.order_id = o.id AND oi.item_status <> 'skipped') AS dish_names, \
    (SELECT GROUP_CONCAT(COALESCE(r.cover_emoji, '🍽️') ORDER BY oi.sort_order SEPARATOR '') \
       FROM order_item oi LEFT JOIN recipe r ON r.id = oi.recipe_id \
      WHERE oi.order_id = o.id AND oi.item_status <> 'skipped') AS cover_emojis";

/// GET /api/orders
pub async fn list_orders(
    state: web::Data<AppState>,
    query: web::Query<OrderListQuery>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let q = query.into_inner();
    let (page, page_size, offset) = normalize_paging(q.page, q.page_size);

    // 先校验枚举，再拼 SQL
    if let Some(s) = clean(q.status.as_deref()) {
        validate_status(&s)?;
    }
    if let Some(t) = clean(q.meal_type.as_deref()) {
        crate::util::validate_enum(&t, &crate::util::MEAL_TYPES, "餐次")?;
    }

    let mut count_qb: QueryBuilder<MySql> =
        QueryBuilder::new("SELECT COUNT(*) FROM meal_order o");
    apply_order_filters(&mut count_qb, &q);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    if total == 0 {
        return Ok(ok(Page::<OrderSummary>::empty(page, page_size)));
    }

    let mut qb: QueryBuilder<MySql> = QueryBuilder::new("SELECT ");
    qb.push(ORDER_SUMMARY_COLUMNS);
    qb.push(" FROM meal_order o JOIN member m ON m.id = o.member_id");
    apply_order_filters(&mut qb, &q);
    qb.push(" ORDER BY o.meal_date DESC, \
             FIELD(o.meal_type,'breakfast','lunch','dinner','snack') DESC, o.id DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let list = qb.build_query_as::<OrderSummary>().fetch_all(pool).await?;

    Ok(ok(Page::new(list, total, page, page_size)))
}

/// GET /api/orders/today —— 今日三餐速览
pub async fn today_orders(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let today = chrono::Local::now().date_naive();

    let list: Vec<OrderSummary> = sqlx::query_as(
        "SELECT o.id, o.order_no, o.member_id, m.nickname AS member_nickname, \
                m.avatar_emoji AS member_avatar, o.meal_date, o.meal_type, o.status, \
                o.diners, o.note, o.served_at, o.created_at, \
                (SELECT COUNT(*) FROM order_item oi WHERE oi.order_id = o.id AND oi.item_status <> 'skipped') AS dish_count, \
                (SELECT COALESCE(CAST(SUM(oi.portions) AS SIGNED),0) FROM order_item oi WHERE oi.order_id = o.id AND oi.item_status <> 'skipped') AS total_portions, \
                (SELECT GROUP_CONCAT(oi.recipe_snapshot ORDER BY oi.sort_order SEPARATOR '、') \
                   FROM order_item oi WHERE oi.order_id = o.id AND oi.item_status <> 'skipped') AS dish_names, \
                (SELECT GROUP_CONCAT(COALESCE(r.cover_emoji,'🍽️') ORDER BY oi.sort_order SEPARATOR '') \
                   FROM order_item oi LEFT JOIN recipe r ON r.id = oi.recipe_id \
                  WHERE oi.order_id = o.id AND oi.item_status <> 'skipped') AS cover_emojis \
         FROM meal_order o JOIN member m ON m.id = o.member_id \
         WHERE o.meal_date = ? \
         ORDER BY FIELD(o.meal_type,'breakfast','lunch','dinner','snack'), o.id DESC",
    )
    .bind(today)
    .fetch_all(pool)
    .await?;

    Ok(ok(serde_json::json!({ "date": today, "orders": list })))
}

/// GET /api/orders/{id} —— 订单详情（含菜单、采购清单、汇总）
pub async fn get_order(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();
    Ok(ok(build_order_detail(pool, id).await?))
}

pub async fn build_order_detail(pool: &MySqlPool, id: u64) -> Result<OrderDetail, ApiError> {
    #[derive(sqlx::FromRow)]
    struct BaseWithMember {
        id: u64,
        order_no: String,
        member_id: u64,
        member_nickname: String,
        member_avatar: String,
        meal_date: NaiveDate,
        meal_type: String,
        status: String,
        diners: u8,
        note: Option<String>,
        served_at: Option<chrono::NaiveDateTime>,
        created_at: chrono::NaiveDateTime,
        updated_at: chrono::NaiveDateTime,
    }

    let row: BaseWithMember = sqlx::query_as(
        "SELECT o.*, m.nickname AS member_nickname, m.avatar_emoji AS member_avatar \
         FROM meal_order o JOIN member m ON m.id = o.member_id WHERE o.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("订单 {id} 不存在")))?;

    let items: Vec<OrderItemView> = sqlx::query_as(
        "SELECT oi.id, oi.recipe_id, oi.recipe_snapshot, oi.portions, oi.rating, oi.item_status, \
                oi.note, oi.sort_order, r.name AS recipe_name, r.category, r.difficulty, \
                r.total_minutes, r.calories, r.cover_emoji, r.cover_gradient, \
                m.nickname AS ordered_by, m.avatar_emoji AS ordered_by_avatar \
         FROM order_item oi \
         LEFT JOIN recipe r ON r.id = oi.recipe_id \
         LEFT JOIN member m ON m.id = oi.member_id \
         WHERE oi.order_id = ? ORDER BY oi.sort_order, oi.id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let shopping_list = load_shopping_list(pool, id, &row.status).await?;

    let active: Vec<&OrderItemView> = items
        .iter()
        .filter(|i| i.item_status != "skipped")
        .collect();

    let total_portions: i64 = active.iter().map(|i| i.portions as i64).sum();
    let total_calories: i64 = active
        .iter()
        .map(|i| i.calories.map(|c| c as i64 * i.portions as i64).unwrap_or(0))
        .sum();
    let total_cook_minutes: i64 = active
        .iter()
        .map(|i| i.total_minutes.map(|t| t as i64).unwrap_or(0))
        .sum();

    let rated: Vec<u8> = active.iter().filter_map(|i| i.rating).collect();
    let avg_rating = if rated.is_empty() {
        None
    } else {
        let sum: u32 = rated.iter().map(|r| *r as u32).sum();
        let avg = (sum as f64) / (rated.len() as f64);
        Some(Decimal::try_from((avg * 100.0).round() / 100.0).unwrap_or(Decimal::ZERO))
    };

    Ok(OrderDetail {
        base: OrderBase {
            id: row.id,
            order_no: row.order_no,
            member_id: row.member_id,
            meal_date: row.meal_date,
            meal_type: row.meal_type,
            status: row.status,
            diners: row.diners,
            note: row.note,
            served_at: row.served_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        },
        member_nickname: row.member_nickname,
        member_avatar: row.member_avatar,
        items,
        shopping_list,
        total_portions,
        total_calories,
        total_cook_minutes,
        avg_rating,
    })
}

// =====================================================================
//  下单
// =====================================================================

/// POST /api/orders
///
/// 支持两种明细：
/// 1. `recipe_id` —— 从菜谱库点已有菜
/// 2. `new_recipe` —— 点餐时顺手建档，先建菜谱再加入订单（同一事务）
///
/// `auto_confirm=true` 时创建后直接确认并生成采购清单。
pub async fn create_order(
    state: web::Data<AppState>,
    body: web::Json<CreateOrderReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let req = body.into_inner();

    let member_id = req
        .member_id
        .ok_or_else(|| ApiError::BadRequest("请选择下单人（member_id）".into()))?;
    let (meal_date, meal_type, diners) = validate_order(&req)?;
    let items: Vec<OrderItemReq> = req.items.clone().unwrap_or_default();

    if !exists(pool, "SELECT COUNT(*) FROM member WHERE id = ?", member_id).await? {
        return Err(ApiError::NotFound(format!("成员 {member_id} 不存在")));
    }

    let mut tx = pool.begin().await?;

    let mut created_recipe_ids: Vec<u64> = Vec::new();
    let mut resolved: Vec<(u64, u8, Option<u64>, Option<String>)> = Vec::new();

    for (idx, item) in items.iter().enumerate() {
        let recipe_id = match (item.recipe_id, item.new_recipe.as_ref()) {
            (Some(id), _) if id > 0 => id,
            (_, Some(new)) => {
                let (rid, _final_name) = insert_recipe_in_tx(&mut *tx, new).await?;
                created_recipe_ids.push(rid);
                rid
            }
            _ => {
                return Err(ApiError::BadRequest(format!(
                    "第 {} 项明细缺少 recipe_id 或 new_recipe",
                    idx + 1
                )))
            }
        };

        if !exists_in_tx(
            &mut *tx,
            "SELECT COUNT(*) FROM recipe WHERE id = ?",
            recipe_id,
        )
        .await?
        {
            return Err(ApiError::NotFound(format!("菜谱 {recipe_id} 不存在")));
        }

        let item_member = match item.member_id {
            Some(mid) if mid > 0 => {
                if !exists_in_tx(&mut *tx, "SELECT COUNT(*) FROM member WHERE id = ?", mid).await? {
                    return Err(ApiError::NotFound(format!("成员 {mid} 不存在")));
                }
                Some(mid)
            }
            _ => Some(member_id),
        };

        resolved.push((
            recipe_id,
            item.portions.unwrap_or(1).clamp(1, 20),
            item_member,
            clean(item.note.as_deref()),
        ));
    }

    if resolved.is_empty() {
        return Err(ApiError::BadRequest("至少要选一道菜才能下单".into()));
    }

    let order_no = next_order_no(&mut *tx, meal_date, &meal_type).await?;
    let initial_status = if req.auto_confirm.unwrap_or(false) {
        "confirmed"
    } else {
        "draft"
    };

    let order_result = sqlx::query(
        "INSERT INTO meal_order (order_no, member_id, meal_date, meal_type, status, diners, note) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&order_no)
    .bind(member_id)
    .bind(meal_date)
    .bind(&meal_type)
    .bind(initial_status)
    .bind(diners)
    .bind(clean(req.note.as_deref()))
    .execute(&mut *tx)
    .await?;

    let order_id = order_result.last_insert_id();

    for (sort, (recipe_id, portions, item_member, note)) in resolved.iter().enumerate() {
        let snapshot: String = sqlx::query_scalar("SELECT name FROM recipe WHERE id = ?")
            .bind(recipe_id)
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO order_item \
             (order_id, recipe_id, recipe_snapshot, portions, member_id, note, sort_order) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(order_id)
        .bind(recipe_id)
        .bind(&snapshot)
        .bind(portions)
        .bind(item_member)
        .bind(note)
        .bind((sort + 1) as i32)
        .execute(&mut *tx)
        .await?;
    }

    if initial_status == "confirmed" {
        let rids = order_recipe_ids(&mut *tx, order_id).await?;
        refresh_recipe_stats(&mut *tx, &rids).await?;
        regenerate_shopping_list_tx(&mut *tx, order_id).await?;
    }

    tx.commit().await?;

    let detail = build_order_detail(pool, order_id).await?;
    let message = if created_recipe_ids.is_empty() {
        "下单成功".to_string()
    } else {
        format!("下单成功，顺手建档了 {} 道新菜", created_recipe_ids.len())
    };

    Ok(ok_with_message(
        &message,
        serde_json::json!({
            "order": detail,
            "created_recipe_ids": created_recipe_ids,
        }),
    ))
}

/// PUT /api/orders/{id} —— 改用餐日期、餐次、人数、备注、状态
pub async fn update_order(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<UpdateOrderReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();
    let req = body.into_inner();

    if let Some(s) = clean(req.status.as_deref()) {
        validate_status(&s)?;
    }
    if let Some(t) = clean(req.meal_type.as_deref()) {
        crate::util::validate_enum(&t, &crate::util::MEAL_TYPES, "餐次")?;
    }
    if let Some(d) = req.diners {
        crate::util::validate_range(d, 1u8, 30u8, "就餐人数")?;
    }

    let current_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM meal_order WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    let current_status =
        current_status.ok_or_else(|| ApiError::NotFound(format!("订单 {id} 不存在")))?;

    sqlx::query(
        "UPDATE meal_order SET \
            meal_date = COALESCE(?, meal_date), meal_type = COALESCE(?, meal_type), \
            status = COALESCE(?, status), diners = COALESCE(?, diners), \
            note = IF(? IS NULL, note, ?), \
            served_at = CASE WHEN ? = 'served' AND served_at IS NULL THEN NOW() ELSE served_at END \
         WHERE id = ?",
    )
    .bind(req.meal_date)
    .bind(clean(req.meal_type.as_deref()))
    .bind(clean(req.status.as_deref()))
    .bind(req.diners)
    .bind(req.note.as_deref())
    .bind(req.note.as_deref())
    .bind(clean(req.status.as_deref()))
    .bind(id)
    .execute(pool)
    .await?;

    // 从草稿转为已确认时，补生成采购清单
    let new_status = clean(req.status.as_deref()).unwrap_or_else(|| current_status.clone());
    if new_status != "draft" && current_status == "draft" {
        let mut tx = pool.begin().await?;
        let rids = order_recipe_ids(&mut *tx, id).await?;
        refresh_recipe_stats(&mut *tx, &rids).await?;
        regenerate_shopping_list_tx(&mut *tx, id).await?;
        tx.commit().await?;
    }

    Ok(ok_with_message("订单已更新", build_order_detail(pool, id).await?))
}

/// POST /api/orders/{id}/confirm —— 确认订单并生成采购清单
///
/// 内部调用存储过程 `sp_confirm_order`；若库中未创建该过程，
/// 自动回退到等价的原生 SQL 实现。
pub async fn confirm_order(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM meal_order WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    if count == 0 {
        return Err(ApiError::NotFound(format!("订单 {id} 不存在")));
    }

    let mut tx = pool.begin().await?;

    // 先记下涉及的菜谱：状态变更会影响统计口径，需在提交前统一重算
    let rids = order_recipe_ids(&mut *tx, id).await?;

    let proc_result = sqlx::query("CALL sp_confirm_order(?)").bind(id).execute(&mut *tx).await;

    match proc_result {
        Ok(_) => {}
        Err(sqlx::Error::Database(db))
            if db.message().contains("sp_confirm_order")
                || db.message().contains("PROCEDURE")
                || db.message().contains("does not exist") =>
        {
            // 存储过程未创建（用户可能只导入了部分脚本），回退到等价的原生 SQL
            tracing::warn!("存储过程 sp_confirm_order 不可用，改用原生 SQL 回退实现");
            sqlx::query(
                "UPDATE meal_order SET status = 'confirmed' WHERE id = ? AND status <> 'cancelled'",
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
            regenerate_shopping_list_tx(&mut *tx, id).await?;
        }
        Err(e) => return Err(e.into()),
    }

    // 权威重算：存储过程 sp_confirm_order 内部用的是 cook_count + 1 增量累加，
    // 而增量方案无法在取消/删除订单时正确回退，会造成长期漂移。
    // 这里无条件按订单明细重算一次，把统计拉回真实值（幂等，重复调用无副作用）。
    refresh_recipe_stats(&mut *tx, &rids).await?;

    tx.commit().await?;

    Ok(ok_with_message(
        "订单已确认，采购清单已生成",
        build_order_detail(pool, id).await?,
    ))
}

/// POST /api/orders/{id}/cancel
pub async fn cancel_order(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();

    let mut tx = pool.begin().await?;

    // 取消前先记下涉及的菜谱，取消后需要把它们从统计口径中剔除
    let rids = order_recipe_ids(&mut *tx, id).await?;

    let result = sqlx::query("UPDATE meal_order SET status = 'cancelled' WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("订单 {id} 不存在")));
    }

    // 已取消的订单不计入统计，重算即可把这些菜的点单数与评分拉回真实值
    refresh_recipe_stats(&mut *tx, &rids).await?;

    tx.commit().await?;

    Ok(ok_with_message(
        "订单已取消",
        serde_json::json!({ "id": id, "status": "cancelled" }),
    ))
}

/// DELETE /api/orders/{id}
pub async fn delete_order(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();

    let mut tx = pool.begin().await?;

    // 必须在删除之前取菜谱 ID：order_item 配置了 ON DELETE CASCADE，
    // 订单删除后明细随之消失，届时再也无法知道该回退哪些菜谱的统计。
    let rids = order_recipe_ids(&mut *tx, id).await?;

    let result = sqlx::query("DELETE FROM meal_order WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("订单 {id} 不存在")));
    }

    refresh_recipe_stats(&mut *tx, &rids).await?;

    tx.commit().await?;

    Ok(ok_with_message(
        "订单已删除",
        serde_json::json!({ "id": id }),
    ))
}

// =====================================================================
//  订单明细：追加、出菜、评分、删除
// =====================================================================

/// POST /api/orders/{id}/items —— 追加一道菜（草稿/已确认均可）
pub async fn add_order_item(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<OrderItemReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let order_id = path.into_inner();
    let req = body.into_inner();

    let order_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM meal_order WHERE id = ?")
            .bind(order_id)
            .fetch_optional(pool)
            .await?;
    let order_status = order_status
        .ok_or_else(|| ApiError::NotFound(format!("订单 {order_id} 不存在")))?;

    if order_status == "cancelled" {
        return Err(ApiError::BadRequest("已取消的订单不能追加菜品".into()));
    }

    let recipe_id = req
        .recipe_id
        .filter(|id| *id > 0)
        .ok_or_else(|| ApiError::BadRequest("请提供 recipe_id".into()))?;

    if !exists(pool, "SELECT COUNT(*) FROM recipe WHERE id = ?", recipe_id).await? {
        return Err(ApiError::NotFound(format!("菜谱 {recipe_id} 不存在")));
    }

    let portions = req.portions.unwrap_or(1).clamp(1, 20);
    if let Some(n) = req.note.as_deref() {
        if n.chars().count() > 255 {
            return Err(ApiError::BadRequest("单项备注不能超过 255 个字符".into()));
        }
    }

    let mut tx = pool.begin().await?;

    let snapshot: String = sqlx::query_scalar("SELECT name FROM recipe WHERE id = ?")
        .bind(recipe_id)
        .fetch_one(&mut *tx)
        .await?;

    let (max_sort,): (Option<i32>,) =
        sqlx::query_as("SELECT MAX(sort_order) FROM order_item WHERE order_id = ?")
            .bind(order_id)
            .fetch_one(&mut *tx)
            .await?;
    let next_sort = max_sort.unwrap_or(0) + 1;

    let result = sqlx::query(
        "INSERT INTO order_item \
         (order_id, recipe_id, recipe_snapshot, portions, member_id, note, sort_order) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(order_id)
    .bind(recipe_id)
    .bind(&snapshot)
    .bind(portions)
    .bind(req.member_id)
    .bind(clean(req.note.as_deref()))
    .bind(next_sort)
    .execute(&mut *tx)
    .await?;

    // 已确认订单追加菜品后需重算采购清单
    if order_status != "draft" {
        regenerate_shopping_list_tx(&mut *tx, order_id).await?;
    }

    tx.commit().await?;

    Ok(ok_with_message(
        format!("已加入「{snapshot}」"),
        serde_json::json!({ "item_id": result.last_insert_id() }),
    ))
}

/// PUT /api/orders/items/{item_id} —— 改份数、评分、出菜状态、备注
pub async fn update_order_item(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<UpdateOrderItemReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let item_id = path.into_inner();
    let req = body.into_inner();

    if let Some(s) = clean(req.item_status.as_deref()) {
        validate_item_status(&s)?;
    }
    if let Some(r) = req.rating {
        if !(1..=5).contains(&r) {
            return Err(ApiError::BadRequest("评分需在 1 ~ 5 之间".into()));
        }
    }
    if let Some(p) = req.portions {
        crate::util::validate_range(p, 1u8, 20u8, "份数")?;
    }

    let row: Option<(u64, String, Option<u8>)> = sqlx::query_as(
        "SELECT oi.order_id, o.status, oi.rating FROM order_item oi \
         JOIN meal_order o ON o.id = oi.order_id WHERE oi.id = ?",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await?;

    let (order_id, order_status, _old_rating) = row
        .ok_or_else(|| ApiError::NotFound(format!("订单明细 {item_id} 不存在")))?;

    if order_status == "cancelled" {
        return Err(ApiError::BadRequest("已取消的订单不能修改".into()));
    }

    let mut tx = pool.begin().await?;

    sqlx::query(
        "UPDATE order_item SET \
            portions = COALESCE(?, portions), rating = IF(? IS NULL, rating, ?), \
            item_status = COALESCE(?, item_status), note = IF(? IS NULL, note, ?), \
            member_id = COALESCE(?, member_id) \
         WHERE id = ?",
    )
    .bind(req.portions)
    .bind(req.rating)
    .bind(req.rating)
    .bind(clean(req.item_status.as_deref()))
    .bind(req.note.as_deref())
    .bind(req.note.as_deref())
    .bind(req.member_id)
    .bind(item_id)
    .execute(&mut *tx)
    .await?;

    // 评分、份数或出菜状态变化都会影响菜谱统计与采购清单
    if req.rating.is_some() || req.portions.is_some() || req.item_status.is_some() {
        let rids = order_recipe_ids(&mut *tx, order_id).await?;
        refresh_recipe_stats(&mut *tx, &rids).await?;
    }

    // 份数或跳过状态变化会影响采购清单
    if order_status != "draft" && (req.portions.is_some() || req.item_status.is_some()) {
        regenerate_shopping_list_tx(&mut *tx, order_id).await?;
    }

    tx.commit().await?;

    Ok(ok_with_message(
        "已更新",
        build_order_detail(pool, order_id).await?,
    ))
}

/// POST /api/orders/items/{item_id}/rating —— 餐后评分（便捷入口）
pub async fn rate_order_item(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<RateBody>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let item_id = path.into_inner();
    let rating = body.rating;
    if !(1..=5).contains(&rating) {
        return Err(ApiError::BadRequest("评分需在 1 ~ 5 之间".into()));
    }

    let row: Option<(u64,)> = sqlx::query_as(
        "SELECT oi.recipe_id FROM order_item oi WHERE oi.id = ? AND oi.recipe_id IS NOT NULL",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await?;

    let (recipe_id,) = row
        .ok_or_else(|| ApiError::NotFound(format!("订单明细 {item_id} 不存在或菜谱已被删除")))?;

    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE order_item SET rating = ? WHERE id = ?")
        .bind(rating)
        .bind(item_id)
        .execute(&mut *tx)
        .await?;

    // 统一走 refresh_recipe_stats：口径与其它路径完全一致
    // （排除 cancelled 与 draft 订单），避免草稿单的评分污染平均分
    refresh_recipe_stats(&mut *tx, &[recipe_id]).await?;

    tx.commit().await?;

    let new_rating: Decimal = sqlx::query_scalar("SELECT avg_rating FROM recipe WHERE id = ?")
        .bind(recipe_id)
        .fetch_one(pool)
        .await?;

    Ok(ok_with_message(
        "评分成功",
        serde_json::json!({
            "item_id": item_id,
            "recipe_id": recipe_id,
            "rating": rating,
            "recipe_avg_rating": new_rating,
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct RateBody {
    pub rating: u8,
}

/// 取出订单涉及的菜谱 ID。
///
/// 删除订单前必须先取，因为 `order_item` 会随订单级联删除，之后就查不到了。
async fn order_recipe_ids(
    tx: &mut MySqlConnection,
    order_id: u64,
) -> Result<Vec<u64>, ApiError> {
    let rows: Vec<(u64,)> = sqlx::query_as(
        "SELECT DISTINCT recipe_id FROM order_item \
         WHERE order_id = ? AND recipe_id IS NOT NULL",
    )
    .bind(order_id)
    .fetch_all(&mut *tx)
    .await?;
    Ok(rows.into_iter().map(|(rid,)| rid).collect())
}

/// 从订单明细**精确重算**菜谱的冗余统计字段（`cook_count` / `avg_rating`）。
///
/// 为什么不用增量 `+1` / `-1`：
/// 增量方案要求每一条会改变统计的路径都记得做反向补偿。实测中
/// 删除订单、取消订单、删除单项菜品都会漏掉回退，导致 `cook_count`
/// 只增不减、`avg_rating` 被已删订单的评分污染，且偏差会长期累积。
/// 重算是幂等的：无论调用多少次、订单如何增删改，结果都等于明细的真实聚合。
///
/// 统计口径（与 `02_seed_data.sql` 回填逻辑一致）：
/// - `cook_count` = 有效订单中该菜被点的**总份数**，排除草稿/已取消、排除 skipped 项
/// - `avg_rating` = 有效订单中该菜所有非空评分的平均值，无评分则为 0
async fn refresh_recipe_stats(
    tx: &mut MySqlConnection,
    recipe_ids: &[u64],
) -> Result<(), ApiError> {
    for rid in recipe_ids {
        sqlx::query(
            "UPDATE recipe SET \
                cook_count = COALESCE(( \
                    SELECT CAST(SUM(oi.portions) AS SIGNED) FROM order_item oi \
                     JOIN meal_order o ON o.id = oi.order_id \
                    WHERE oi.recipe_id = ? \
                      AND o.status NOT IN ('cancelled','draft') \
                      AND oi.item_status <> 'skipped' \
                ), 0), \
                avg_rating = COALESCE(( \
                    SELECT ROUND(AVG(oi.rating),2) FROM order_item oi \
                     JOIN meal_order o ON o.id = oi.order_id \
                    WHERE oi.recipe_id = ? AND oi.rating IS NOT NULL \
                      AND o.status NOT IN ('cancelled','draft') \
                ), 0) \
             WHERE id = ?",
        )
        .bind(rid)
        .bind(rid)
        .bind(rid)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

/// DELETE /api/orders/items/{item_id}
pub async fn delete_order_item(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let item_id = path.into_inner();

    // 取出该明细对应的菜谱，删除后需要重算其统计
    let row: Option<(u64, String, Option<u64>)> = sqlx::query_as(
        "SELECT oi.order_id, o.status, oi.recipe_id FROM order_item oi \
         JOIN meal_order o ON o.id = oi.order_id WHERE oi.id = ?",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await?;

    let (order_id, order_status, item_recipe_id) = row
        .ok_or_else(|| ApiError::NotFound(format!("订单明细 {item_id} 不存在")))?;

    if order_status == "served" {
        return Err(ApiError::BadRequest("已开饭的订单不能删除菜品，可将其标记为跳过".into()));
    }

    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM order_item WHERE id = ?")
        .bind(item_id)
        .execute(&mut *tx)
        .await?;

    // 重算被移除菜品的统计（明细已删除，重算结果自然不含这一项）
    if let Some(rid) = item_recipe_id {
        refresh_recipe_stats(&mut *tx, &[rid]).await?;
    }

    if order_status != "draft" {
        regenerate_shopping_list_tx(&mut *tx, order_id).await?;
    }
    tx.commit().await?;

    Ok(ok_with_message(
        "已从订单移除",
        build_order_detail(pool, order_id).await?,
    ))
}

// =====================================================================
//  采购清单
// =====================================================================

/// GET /api/orders/{id}/shopping-list
pub async fn get_shopping_list(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let order_id = path.into_inner();

    let status: Option<String> = sqlx::query_scalar("SELECT status FROM meal_order WHERE id = ?")
        .bind(order_id)
        .fetch_optional(pool)
        .await?;
    let status = status.ok_or_else(|| ApiError::NotFound(format!("订单 {order_id} 不存在")))?;

    Ok(ok(load_shopping_list(pool, order_id, &status).await?))
}

/// PUT /api/orders/shopping/{item_id} —— 勾选/取消已购买
pub async fn update_shopping_item(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<UpdateShoppingItemReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let item_id = path.into_inner();

    let purchased = body
        .is_purchased
        .ok_or_else(|| ApiError::BadRequest("请提供 is_purchased".into()))?;

    let row: Option<(u64,)> =
        sqlx::query_as("SELECT order_id FROM shopping_item WHERE id = ?")
            .bind(item_id)
            .fetch_optional(pool)
            .await?;
    let order_id = row
        .ok_or_else(|| ApiError::NotFound(format!("采购项 {item_id} 不存在")))?
        .0;

    sqlx::query("UPDATE shopping_item SET is_purchased = ? WHERE id = ?")
        .bind(purchased)
        .bind(item_id)
        .execute(pool)
        .await?;

    let list = shopping_from_table(pool, order_id).await?;
    Ok(ok(list))
}

/// POST /api/orders/{id}/shopping-list/reset —— 清空勾选状态
pub async fn reset_shopping_list(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let order_id = path.into_inner();

    let result = sqlx::query("UPDATE shopping_item SET is_purchased = 0 WHERE order_id = ?")
        .bind(order_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        // 可能采购清单尚未生成（草稿单），尝试实时生成
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM meal_order WHERE id = ?")
                .bind(order_id)
                .fetch_optional(pool)
                .await?;
        if status.is_none() {
            return Err(ApiError::NotFound(format!("订单 {order_id} 不存在")));
        }
    }

    let list = shopping_from_table(pool, order_id).await?;
    Ok(ok_with_message("已清空勾选", list))
}

/// GET /api/orders/{id}/menu —— 这一餐的菜单卡片（打印/分享用）
pub async fn get_order_menu(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let order_id = path.into_inner();
    let detail = build_order_detail(pool, order_id).await?;

    let dishes: Vec<serde_json::Value> = detail
        .items
        .iter()
        .filter(|i| i.item_status != "skipped")
        .map(|i| {
            serde_json::json!({
                "name": i.recipe_snapshot,
                "portions": i.portions,
                "category": i.category,
                "difficulty": i.difficulty,
                "minutes": i.total_minutes,
                "emoji": i.cover_emoji,
                "gradient": i.cover_gradient,
                "note": i.note,
                "ordered_by": i.ordered_by,
                "rating": i.rating,
            })
        })
        .collect();

    Ok(ok(serde_json::json!({
        "order_no": detail.base.order_no,
        "meal_date": detail.base.meal_date,
        "meal_type": detail.base.meal_type,
        "diners": detail.base.diners,
        "note": detail.base.note,
        "status": detail.base.status,
        "host": detail.member_nickname,
        "dish_count": dishes.len(),
        "total_portions": detail.total_portions,
        "total_calories": detail.total_calories,
        "total_cook_minutes": detail.total_cook_minutes,
        "dishes": dishes,
    })))
}

// =====================================================================
//  复制历史订单（「照上次那样再来一顿」）
// =====================================================================

/// POST /api/orders/{id}/reorder —— 按历史订单生成一张新草稿
pub async fn reorder(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let source_id = path.into_inner();

    let source: Option<(u64, u8, Option<String>)> = sqlx::query_as(
        "SELECT member_id, diners, note FROM meal_order WHERE id = ?",
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await?;
    let (member_id, diners, note) = source
        .ok_or_else(|| ApiError::NotFound(format!("订单 {source_id} 不存在")))?;

    let items: Vec<(u64, u8)> = sqlx::query_as(
        "SELECT recipe_id, portions FROM order_item \
         WHERE order_id = ? AND recipe_id IS NOT NULL AND item_status <> 'skipped' \
         ORDER BY sort_order",
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;

    if items.is_empty() {
        return Err(ApiError::BadRequest("原订单没有可复制的菜品".into()));
    }

    let req = CreateOrderReq {
        member_id: Some(member_id),
        meal_date: None,
        meal_type: Some("dinner".to_string()),
        diners: Some(diners),
        note: note.or_else(|| Some("照上次的菜单".to_string())),
        auto_confirm: Some(false),
        items: Some(
            items
                .into_iter()
                .map(|(rid, p)| OrderItemReq {
                    recipe_id: Some(rid),
                    portions: Some(p),
                    member_id: Some(member_id),
                    note: None,
                    new_recipe: None,
                })
                .collect(),
        ),
    };

    // 复用创建逻辑
    create_order(state, web::Json(req)).await
}

/// GET /api/orders/{id}/recipes —— 订单涉及菜谱的完整做法（一键生成「这一餐怎么做」）
pub async fn get_order_recipes(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let order_id = path.into_inner();

    if !exists(pool, "SELECT COUNT(*) FROM meal_order WHERE id = ?", order_id).await? {
        return Err(ApiError::NotFound(format!("订单 {order_id} 不存在")));
    }

    // 注意：MySQL 不允许 DISTINCT 搭配「引用未在 SELECT 列表中的列」的 ORDER BY
    // （errno 3065 ER_FIELD_IN_ORDER_NOT_SELECT），故改用 GROUP BY + MIN(sort_order)：
    // 同样能去重菜谱，并按其在订单中首次出现的顺序排列。
    let recipe_ids: Vec<u64> = sqlx::query_scalar(
        "SELECT recipe_id FROM order_item \
         WHERE order_id = ? AND recipe_id IS NOT NULL AND item_status <> 'skipped' \
         GROUP BY recipe_id ORDER BY MIN(sort_order)",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    let mut recipes = Vec::new();
    for rid in recipe_ids {
        match build_recipe_detail(pool, rid).await {
            Ok(d) => recipes.push(d),
            Err(ApiError::NotFound(_)) => continue,
            Err(e) => return Err(e),
        }
    }

    let total_minutes: i64 = recipes.iter().map(|r| r.base.total_minutes as i64).sum();

    Ok(ok(serde_json::json!({
        "order_id": order_id,
        "recipe_count": recipes.len(),
        "total_cook_minutes": total_minutes,
        "recipes": recipes,
    })))
}

// =====================================================================
//  辅助
// =====================================================================

async fn exists(pool: &MySqlPool, sql: &str, id: u64) -> Result<bool, ApiError> {
    let (count,): (i64,) = sqlx::query_as(sql).bind(id).fetch_one(pool).await?;
    Ok(count > 0)
}

async fn exists_in_tx(
    tx: &mut MySqlConnection,
    sql: &str,
    id: u64,
) -> Result<bool, ApiError> {
    let (count,): (i64,) = sqlx::query_as(sql).bind(id).fetch_one(&mut *tx).await?;
    Ok(count > 0)
}
