//! 站内提醒中心（阶段 2）——经期推算 + 快递超时两类生成，纯站内通知
//!
//! - GET  /api/reminders?status=&type=     列表（默认 pending，按提醒时间升序）
//! - POST /api/reminders/sync               重新生成（幂等：按 payload.source 去重）
//! - PUT  /api/reminders/{id}               完成 / 忽略 / 推迟（改 due_at）
//!
//! 生成规则：
//! - 经期推算：health_profile.last_period_start + cycle_length 天（滚动到下一个未来日期）
//! - 快递超时：status=pending 且到件已满 2 天未取
//! 幂等去重：每条生成提醒带 payload.source（health-period:{member_id}:{date} / parcel-overdue:{id}），
//! 同 source 只保留一条；用户已 done/dismissed 的 source 不再重建。

use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::auth::current_claims;
use crate::db::AppState;
use crate::error::ApiError;
use crate::models::Reminder;
use crate::util::{ok, ok_with_message, validate_enum, REMINDER_STATUS};

fn clean(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

// =====================================================================
//  列表
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct ReminderQuery {
    /// pending / done / dismissed / 空=pending
    pub status: Option<String>,
    /// health / parcel / bill / device / other / 空=全部
    #[serde(rename = "type")]
    pub reminder_type: Option<String>,
}

/// GET /api/reminders?status=&type=
pub async fn list_reminders(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<ReminderQuery>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;

    let status = clean(query.status.as_deref()).unwrap_or_else(|| "pending".into());
    validate_enum(&status, &REMINDER_STATUS, "提醒状态")?;
    if let Some(t) = clean(query.reminder_type.as_deref()) {
        validate_enum(
            &t,
            &["health", "parcel", "bill", "device", "consumable", "other"],
            "提醒类型",
        )?;
    }

    // 动态条件（WHERE 1=1 模式，避免绑定顺序问题）
    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT * FROM reminder WHERE status = ",
    );
    qb.push_bind(&status);
    if let Some(t) = clean(query.reminder_type.as_deref()) {
        qb.push(" AND type = ");
        qb.push_bind(t);
    }
    qb.push(" ORDER BY due_at ASC, id ASC");

    let list: Vec<Reminder> = qb.build_query_as().fetch_all(pool).await?;
    Ok(ok(list))
}

// =====================================================================
//  状态操作：完成 / 忽略 / 推迟
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct UpdateReminderReq {
    /// done / dismissed / pending（pending 用于撤销误操作）
    pub status: Option<String>,
    /// 推迟：新的提醒时间（ISO 8601，如 2026-10-25T09:00:00）
    pub due_at: Option<chrono::NaiveDateTime>,
}

/// PUT /api/reminders/{id}
pub async fn update_reminder(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<UpdateReminderReq>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let id = path.into_inner();
    let r = body.into_inner();

    if r.status.is_none() && r.due_at.is_none() {
        return Err(ApiError::BadRequest("没有需要更新的内容".into()));
    }
    if let Some(s) = clean(r.status.as_deref()) {
        validate_enum(&s, &REMINDER_STATUS, "提醒状态")?;
    }

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reminder WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    if count == 0 {
        return Err(ApiError::NotFound(format!("提醒 {id} 不存在")));
    }

    sqlx::query(
        "UPDATE reminder SET \
            status = COALESCE(?, status), \
            due_at = COALESCE(?, due_at) \
         WHERE id = ?",
    )
    .bind(clean(r.status.as_deref()))
    .bind(r.due_at)
    .bind(id)
    .execute(pool)
    .await?;

    let row: Reminder = sqlx::query_as("SELECT * FROM reminder WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(ok_with_message("提醒已更新", row))
}

// =====================================================================
//  生成引擎（幂等）
// =====================================================================

/// 一条待生成提醒的描述
struct Wanted {
    source: String,
    member_id: Option<u64>,
    reminder_type: String,
    title: String,
    due_at: chrono::NaiveDateTime,
    payload: serde_json::Value,
}

/// POST /api/reminders/sync —— 重新生成经期/快递两类提醒
pub async fn sync_reminders(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;

    let wanted = build_wanted(pool).await?;
    let wanted_sources: std::collections::HashSet<&str> =
        wanted.iter().map(|w| w.source.as_str()).collect();

    // 现存的「生成类」提醒（payload.source 非空）
    #[derive(sqlx::FromRow)]
    struct Existing {
        id: u64,
        status: String,
        source: Option<String>,
    }
    let existing: Vec<Existing> = sqlx::query_as(
        "SELECT id, status, CAST(JSON_UNQUOTE(JSON_EXTRACT(payload, '$.source')) AS CHAR) AS source \
         FROM reminder WHERE payload IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut created = 0u32;
    let mut expired = 0u32;

    // 1) 条件已消失的 pending 生成提醒 → 自动忽略（如快递已取、经期日期已滚动）
    for e in &existing {
        if e.status == "pending" {
            if let Some(src) = e.source.as_deref() {
                if !wanted_sources.contains(src) {
                    sqlx::query("UPDATE reminder SET status = 'dismissed' WHERE id = ?")
                        .bind(e.id)
                        .execute(pool)
                        .await?;
                    expired += 1;
                }
            }
        }
    }

    // 2) 缺失的 source → 新建（已存在任何状态的同 source 都跳过，尊重用户操作）
    let known: std::collections::HashSet<String> = existing
        .iter()
        .filter_map(|e| e.source.clone())
        .collect();
    for w in &wanted {
        if known.contains(&w.source) {
            continue;
        }
        sqlx::query(
            "INSERT INTO reminder (member_id, type, title, due_at, status, payload) \
             VALUES (?, ?, ?, ?, 'pending', ?)",
        )
        .bind(w.member_id)
        .bind(&w.reminder_type)
        .bind(&w.title)
        .bind(w.due_at)
        .bind(&w.payload)
        .execute(pool)
        .await?;
        created += 1;
    }

    Ok(ok_with_message(
        format!("同步完成：新增 {created} 条，自动清理 {expired} 条"),
        serde_json::json!({ "created": created, "expired": expired }),
    ))
}

/// 收集两类提醒的「应有状态」
async fn build_wanted(pool: &sqlx::MySqlPool) -> Result<Vec<Wanted>, ApiError> {
    let mut wanted: Vec<Wanted> = Vec::new();
    let today = chrono::Local::now().date_naive();

    // ---- 经期推算 ----
    #[derive(sqlx::FromRow)]
    struct CycleRow {
        member_id: u64,
        nickname: String,
        last_period_start: chrono::NaiveDate,
        cycle_length: u8,
    }
    let cycles: Vec<CycleRow> = sqlx::query_as(
        "SELECT hp.member_id, m.nickname, hp.last_period_start, hp.cycle_length \
         FROM health_profile hp \
         JOIN member m ON m.id = hp.member_id AND m.is_active = 1 \
         WHERE hp.last_period_start IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    for c in cycles {
        // 从上次开始日按周期滚动，取第一个 >= 今天的预测日
        let mut next = c.last_period_start;
        let cycle = i64::from(c.cycle_length.max(15));
        let mut guard = 0;
        while next < today && guard < 200 {
            next += chrono::Duration::days(cycle);
            guard += 1;
        }
        let due_at = next
            .and_hms_opt(8, 0, 0)
            .ok_or_else(|| ApiError::Internal("提醒时间构造失败".into()))?;
        let source = format!("health-period:{}:{}", c.member_id, next);
        wanted.push(Wanted {
            source,
            member_id: Some(c.member_id),
            reminder_type: "health".into(),
            title: format!("「{}」经期预计开始（{}）", c.nickname, next.format("%m-%d")),
            due_at,
            payload: serde_json::json!({
                "source": format!("health-period:{}:{}", c.member_id, next),
                "kind": "period_predicted",
                "predicted_date": next.to_string(),
            }),
        });
    }

    // ---- 快递超时未取（到件满 2 天）----
    #[derive(sqlx::FromRow)]
    struct ParcelRow {
        id: u64,
        arrived_date: chrono::NaiveDate,
        recipient_name: Option<String>,
    }
    let parcels: Vec<ParcelRow> = sqlx::query_as(
        "SELECT id, arrived_date, recipient_name \
         FROM parcel \
         WHERE status = 'pending' AND arrived_date <= DATE_SUB(CURDATE(), INTERVAL 2 DAY)",
    )
    .fetch_all(pool)
    .await?;

    for p in parcels {
        let who = p.recipient_name.clone().unwrap_or_else(|| "家人".into());
        let days = (today - p.arrived_date).num_days().max(2);
        let due_date = p.arrived_date + chrono::Duration::days(2);
        let due_at = due_date
            .and_hms_opt(9, 0, 0)
            .ok_or_else(|| ApiError::Internal("提醒时间构造失败".into()))?;
        let source = format!("parcel-overdue:{}", p.id);
        wanted.push(Wanted {
            source: source.clone(),
            member_id: None,
            reminder_type: "parcel".into(),
            title: format!("{who}的快递到件 {days} 天还没取"),
            due_at,
            payload: serde_json::json!({
                "source": source,
                "kind": "parcel_overdue",
                "parcel_id": p.id,
                "days_overdue": days,
            }),
        });
    }

    // ---- 耗材低库存（阶段 3）：库存 <= 预警阈值 且阈值 > 0 ----
    #[derive(sqlx::FromRow)]
    struct LowStockRow {
        id: u64,
        name: String,
        unit: String,
        emoji: String,
        stock: rust_decimal::Decimal,
        low_threshold: rust_decimal::Decimal,
    }
    let low_stock: Vec<LowStockRow> = sqlx::query_as(
        "SELECT id, name, unit, emoji, stock, low_threshold \
         FROM consumable \
         WHERE is_active = 1 AND low_threshold > 0 AND stock <= low_threshold",
    )
    .fetch_all(pool)
    .await?;

    for c in low_stock {
        let due_at = today
            .and_hms_opt(10, 0, 0)
            .ok_or_else(|| ApiError::Internal("提醒时间构造失败".into()))?;
        let source = format!("consumable-low:{}", c.id);
        let text = if c.stock <= rust_decimal::Decimal::ZERO {
            format!("「{}」已用完，需尽快补充", c.name)
        } else {
            format!("「{}」仅剩 {} {}（预警线 {}）", c.name, c.stock, c.unit, c.low_threshold)
        };
        wanted.push(Wanted {
            source: source.clone(),
            member_id: None,
            reminder_type: "consumable".into(),
            title: text,
            due_at,
            payload: serde_json::json!({
                "source": source,
                "kind": "consumable_low",
                "consumable_id": c.id,
                "stock": c.stock,
                "low_threshold": c.low_threshold,
            }),
        });
    }

    Ok(wanted)
}
