//! 耗材维护与消耗统计（阶段 3）
//!
//! - GET    /api/consumables?status=          耗材档案列表（active / inactive / 空=启用）
//! - POST   /api/consumables                  新建耗材
//! - PUT    /api/consumables/{id}             更新耗材（含手动调整库存）
//! - DELETE /api/consumables/{id}             停用/删除耗材
//! - GET    /api/consumables/low              低库存清单（stock <= low_threshold，供滚动栏）
//! - POST   /api/consumables/{id}/consumption 登记一次消耗（自动扣减库存）
//! - GET    /api/consumables/{id}/consumption 某耗材的消耗记录
//! - GET    /api/consumables/stats            消耗统计（累计/本月/近30天逐日）
//!
//! 低库存提醒：由 reminder 生成引擎统一扫描（payload.source=consumable-low:{id}），
//! 在首页滚动栏与提醒中心展示。

use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::auth::current_claims;
use crate::db::AppState;
use crate::dto::{AddConsumptionReq, CreateConsumableReq, UpdateConsumableReq};
use crate::error::ApiError;
use crate::models::{Consumable, ConsumableLog, ConsumableStats, DailyConsumption};
use crate::util::{ok, ok_with_message, require_text, validate_enum};

use rust_decimal::Decimal;

fn clean(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn dec(v: Option<f64>) -> Option<Decimal> {
    v.and_then(|f| Decimal::try_from(f).ok())
}

// =====================================================================
//  档案列表
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct ConsumableQuery {
    /// active / inactive / 空=启用
    pub status: Option<String>,
}

/// GET /api/consumables?status=
pub async fn list_consumables(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<ConsumableQuery>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;

    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT * FROM consumable WHERE is_active = ",
    );
    if clean(query.status.as_deref()).as_deref() == Some("inactive") {
        qb.push_bind(false);
    } else {
        qb.push_bind(true);
    }
    qb.push(" ORDER BY FIELD(category,'清洁','纸品','调味料','个护','其他'), name ASC");

    let list = qb.build_query_as::<Consumable>().fetch_all(pool).await?;
    Ok(ok(list))
}

/// GET /api/consumables/low —— 低库存清单（库存 <= 预警阈值 且阈值 > 0）
pub async fn list_low_stock(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;

    let list: Vec<Consumable> = sqlx::query_as(
        "SELECT * FROM consumable \
         WHERE is_active = 1 AND low_threshold > 0 AND stock <= low_threshold \
         ORDER BY (stock / low_threshold) ASC, name ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(ok(list))
}

// =====================================================================
//  新建 / 更新 / 删除
// =====================================================================

/// POST /api/consumables
pub async fn create_consumable(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateConsumableReq>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let r = body.into_inner();

    let name = require_text(&r.name, "耗材名称", 64)?;
    let category = clean(r.category.as_deref()).unwrap_or_else(|| "其他".into());
    validate_enum(&category, &crate::dto::CONSUMABLE_CATEGORIES, "耗材分类")?;
    let unit = clean(r.unit.as_deref()).unwrap_or_else(|| "个".into());
    if unit.chars().count() > 16 {
        return Err(ApiError::BadRequest("单位不能超过 16 个字符".into()));
    }
    let emoji = clean(r.emoji.as_deref()).unwrap_or_else(|| "🧴".into());
    if emoji.chars().count() > 8 {
        return Err(ApiError::BadRequest("图标不能超过 8 个字符".into()));
    }
    let stock = dec(r.stock).unwrap_or(Decimal::ZERO);
    if stock < Decimal::ZERO {
        return Err(ApiError::BadRequest("库存量不能为负数".into()));
    }
    let low_threshold = dec(r.low_threshold).unwrap_or(Decimal::ZERO);
    if low_threshold < Decimal::ZERO {
        return Err(ApiError::BadRequest("预警阈值不能为负数".into()));
    }
    if let Some(n) = r.note.as_deref() {
        if n.chars().count() > 255 {
            return Err(ApiError::BadRequest("备注不能超过 255 个字符".into()));
        }
    }

    let result = sqlx::query(
        "INSERT INTO consumable (name, category, unit, emoji, stock, low_threshold, note) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&name)
    .bind(&category)
    .bind(&unit)
    .bind(&emoji)
    .bind(stock)
    .bind(low_threshold)
    .bind(clean(r.note.as_deref()))
    .execute(pool)
    .await?;

    let id = result.last_insert_id();
    let row: Consumable = sqlx::query_as("SELECT * FROM consumable WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(ok_with_message("耗材已添加", row))
}

/// PUT /api/consumables/{id}
pub async fn update_consumable(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<UpdateConsumableReq>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let id = path.into_inner();
    let r = body.into_inner();

    if let Some(n) = r.name.as_deref() {
        if n.trim().is_empty() || n.trim().chars().count() > 64 {
            return Err(ApiError::BadRequest("耗材名称需为 1~64 个字符".into()));
        }
    }
    if let Some(c) = clean(r.category.as_deref()) {
        validate_enum(&c, &crate::dto::CONSUMABLE_CATEGORIES, "耗材分类")?;
    }
    if let Some(u) = r.unit.as_deref() {
        if u.trim().chars().count() > 16 {
            return Err(ApiError::BadRequest("单位不能超过 16 个字符".into()));
        }
    }
    if let Some(s) = r.stock {
        if s < 0.0 {
            return Err(ApiError::BadRequest("库存量不能为负数".into()));
        }
    }
    if let Some(t) = r.low_threshold {
        if t < 0.0 {
            return Err(ApiError::BadRequest("预警阈值不能为负数".into()));
        }
    }

    sqlx::query(
        "UPDATE consumable SET \
            name = COALESCE(?, name), \
            category = COALESCE(?, category), \
            unit = COALESCE(?, unit), \
            emoji = COALESCE(?, emoji), \
            stock = COALESCE(?, stock), \
            low_threshold = COALESCE(?, low_threshold), \
            note = IF(? IS NULL, note, ?), \
            is_active = COALESCE(?, is_active) \
         WHERE id = ?",
    )
    .bind(clean(r.name.as_deref()))
    .bind(clean(r.category.as_deref()))
    .bind(clean(r.unit.as_deref()))
    .bind(clean(r.emoji.as_deref()))
    .bind(dec(r.stock))
    .bind(dec(r.low_threshold))
    .bind(r.note.as_deref())
    .bind(r.note.as_deref())
    .bind(r.is_active)
    .bind(id)
    .execute(pool)
    .await?;

    let row: Consumable = sqlx::query_as("SELECT * FROM consumable WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("耗材 {id} 不存在")))?;
    Ok(ok_with_message("耗材已更新", row))
}

/// DELETE /api/consumables/{id} —— 软删除（is_active=0），保留消耗记录
pub async fn delete_consumable(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let id = path.into_inner();

    let result = sqlx::query("UPDATE consumable SET is_active = 0 WHERE id = ? AND is_active = 1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("耗材 {id} 不存在或已停用")));
    }
    Ok(ok_with_message(
        "耗材已停用",
        serde_json::json!({ "id": id }),
    ))
}

// =====================================================================
//  消耗登记
// =====================================================================

/// POST /api/consumables/{id}/consumption —— 登记一次消耗并扣减库存
pub async fn add_consumption(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<AddConsumptionReq>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let id = path.into_inner();
    let r = body.into_inner();

    let qty = Decimal::try_from(r.quantity)
        .map_err(|_| ApiError::BadRequest("消耗量不是有效数字".into()))?;
    if qty <= Decimal::ZERO {
        return Err(ApiError::BadRequest("消耗量必须是正数".into()));
    }

    // 确认耗材存在且启用
    let row: Consumable = sqlx::query_as("SELECT * FROM consumable WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("耗材 {id} 不存在")))?;
    if !row.is_active {
        return Err(ApiError::BadRequest("该耗材已停用，无法登记消耗".into()));
    }

    // 登记人昵称快照
    let mut member_name: Option<String> = None;
    if let Some(mid) = r.member_id {
        let nick: Option<(String,)> =
            sqlx::query_as("SELECT nickname FROM member WHERE id = ?")
                .bind(mid)
                .fetch_optional(pool)
                .await?;
        member_name = nick.map(|(n,)| n);
    }

    let occurred_at = r.occurred_at.unwrap_or_else(|| chrono::Local::now().naive_local());

    // 扣减库存（下不设防，允许扣到 0；不主动负数，负数由调用方判断是否合理）
    let new_stock = row.stock - qty;

    let result = sqlx::query(
        "INSERT INTO consumable_log (consumable_id, quantity, member_id, member_name, note, occurred_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(qty)
    .bind(r.member_id)
    .bind(member_name)
    .bind(clean(r.note.as_deref()))
    .bind(occurred_at)
    .execute(pool)
    .await?;

    sqlx::query("UPDATE consumable SET stock = ? WHERE id = ?")
        .bind(new_stock)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(ok_with_message(
        format!("已登记消耗 {} {}", qty, row.unit),
        serde_json::json!({ "id": result.last_insert_id(), "stock": new_stock }),
    ))
}

// =====================================================================
//  消耗记录与统计
// =====================================================================

/// GET /api/consumables/{id}/consumption —— 某耗材的消耗记录
pub async fn list_consumption(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let id = path.into_inner();

    let list: Vec<ConsumableLog> = sqlx::query_as(
        "SELECT l.id, l.consumable_id, c.name AS consumable_name, c.unit, c.emoji, \
                l.quantity, l.member_id, l.member_name, l.note, l.occurred_at \
         FROM consumable_log l \
         JOIN consumable c ON c.id = l.consumable_id \
         WHERE l.consumable_id = ? \
         ORDER BY l.occurred_at DESC, l.id DESC \
         LIMIT 200",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    Ok(ok(list))
}

/// GET /api/consumables/stats —— 全量消耗统计
pub async fn stats(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;

    #[derive(sqlx::FromRow)]
    struct Agg {
        total_count: i64,
        total_quantity: Decimal,
        month_count: i64,
        month_quantity: Decimal,
        last30_quantity: Decimal,
    }
    let agg: Agg = sqlx::query_as(
        "SELECT \
            COUNT(*) AS total_count, \
            COALESCE(SUM(quantity), 0) AS total_quantity, \
            COUNT(CASE WHEN occurred_at >= DATE_FORMAT(CURDATE(), '%Y-%m-01') THEN 1 END) AS month_count, \
            COALESCE(SUM(CASE WHEN occurred_at >= DATE_FORMAT(CURDATE(), '%Y-%m-01') THEN quantity ELSE 0 END), 0) AS month_quantity, \
            COALESCE(SUM(CASE WHEN occurred_at >= DATE_SUB(CURDATE(), INTERVAL 29 DAY) THEN quantity ELSE 0 END), 0) AS last30_quantity \
         FROM consumable_log",
    )
    .fetch_one(pool)
    .await?;

    // 近 30 天逐日消耗（含零值日，前端画折线更连贯）
    let daily_rows: Vec<DailyConsumption> = sqlx::query_as(
        "SELECT d AS day, COALESCE(t.quantity, 0) AS quantity \
         FROM ( \
           SELECT DATE_SUB(CURDATE(), INTERVAL n DAY) AS d \
           FROM (SELECT 0 n UNION SELECT 1 UNION SELECT 2 UNION SELECT 3 UNION SELECT 4 \
                 UNION SELECT 5 UNION SELECT 6 UNION SELECT 7 UNION SELECT 8 UNION SELECT 9 \
                 UNION SELECT 10 UNION SELECT 11 UNION SELECT 12 UNION SELECT 13 UNION SELECT 14 \
                 UNION SELECT 15 UNION SELECT 16 UNION SELECT 17 UNION SELECT 18 UNION SELECT 19 \
                 UNION SELECT 20 UNION SELECT 21 UNION SELECT 22 UNION SELECT 23 UNION SELECT 24 \
                 UNION SELECT 25 UNION SELECT 26 UNION SELECT 27 UNION SELECT 28 UNION SELECT 29) x \
         ) days \
         LEFT JOIN ( \
           SELECT DATE(occurred_at) AS dd, SUM(quantity) AS quantity \
           FROM consumable_log \
           WHERE occurred_at >= DATE_SUB(CURDATE(), INTERVAL 29 DAY) \
           GROUP BY DATE(occurred_at) \
         ) t ON t.dd = days.d \
         ORDER BY days.d ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(ok(ConsumableStats {
        total_count: agg.total_count,
        total_quantity: agg.total_quantity,
        month_count: agg.month_count,
        month_quantity: agg.month_quantity,
        last30_quantity: agg.last30_quantity,
        daily: daily_rows,
    }))
}
