//! 家庭账本（收支记录）

use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Datelike;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::auth::current_claims;
use crate::db::AppState;
use crate::dto::{CreateLedgerReq, UpdateLedgerReq, LEDGER_CATEGORIES, LEDGER_TYPES};
use crate::error::ApiError;
use crate::models::LedgerEntry;
use crate::util::{ok, ok_with_message, require_text, validate_enum, validate_range};

fn clean(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn to_amount(v: f64, field: &str) -> Result<Decimal, ApiError> {
    if !v.is_finite() {
        return Err(ApiError::BadRequest(format!("{field} 不是有效数字")));
    }
    let d = Decimal::try_from(v).map_err(|_| ApiError::BadRequest(format!("{field} 超出范围")))?;
    if d < Decimal::ZERO {
        return Err(ApiError::BadRequest(format!("{field} 不能为负数")));
    }
    Ok(d.round_dp(2))
}

/// 解析 `month=2026-10` → (起始日, 下月起始日)；未传月份返回 None（不限日期）
fn month_range(month: Option<&str>) -> Result<Option<(chrono::NaiveDate, chrono::NaiveDate)>, ApiError> {
    let trimmed = month.map(str::trim).filter(|s| !s.is_empty());
    let Some(s) = trimmed else {
        return Ok(None);
    };
    let today = chrono::Local::now().date_naive();
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err(ApiError::BadRequest("month 格式应为 YYYY-MM，例如 2026-10".into()));
    }
    let y: i32 = parts[0].parse()?;
    let m: u32 = parts[1].parse()?;
    if !(1..=12).contains(&m) {
        return Err(ApiError::BadRequest("月份需在 1~12 之间".into()));
    }
    let _ = today;
    let start = chrono::NaiveDate::from_ymd_opt(y, m, 1)
        .ok_or_else(|| ApiError::BadRequest("无效的月份".into()))?;
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let next = chrono::NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
    Ok(Some((start, next)))
}

#[derive(Debug, Deserialize)]
pub struct LedgerQuery {
    pub month: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub category: Option<String>,
    pub member_id: Option<u64>,
}

/// GET /api/ledger?month=2026-10&type=&category=&member_id=
pub async fn list_entries(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<LedgerQuery>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let q = query.into_inner();

    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new("SELECT * FROM ledger_entry WHERE 1=1");
    if let Some((start, next)) = month_range(q.month.as_deref())? {
        qb.push(" AND entry_date >= ").push_bind(start);
        qb.push(" AND entry_date < ").push_bind(next);
    }
    if let Some(t) = clean(q.entry_type.as_deref()) {
        validate_enum(&t, &LEDGER_TYPES, "收支类型")?;
        qb.push(" AND type = ").push_bind(t);
    }
    if let Some(c) = clean(q.category.as_deref()) {
        validate_enum(&c, &LEDGER_CATEGORIES, "账本分类")?;
        qb.push(" AND category = ").push_bind(c);
    }
    if let Some(m) = q.member_id {
        qb.push(" AND member_id = ").push_bind(m);
    }
    qb.push(" ORDER BY entry_date DESC, id DESC");

    let list = qb.build_query_as::<LedgerEntry>().fetch_all(pool).await?;
    Ok(ok(list))
}

#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    pub month: Option<String>,
}

/// GET /api/ledger/summary?month=2026-10 —— 月度汇总
pub async fn summary(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<SummaryQuery>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    // 汇总默认当月
    let (start, next) = month_range(query.month.as_deref())?.unwrap_or_else(|| {
        let today = chrono::Local::now().date_naive();
        let start = today.with_day(1).unwrap_or(today);
        let (ny, nm) = if start.month() == 12 {
            (start.year() + 1, 1)
        } else {
            (start.year(), start.month() + 1)
        };
        (start, chrono::NaiveDate::from_ymd_opt(ny, nm, 1).unwrap())
    });

    #[derive(sqlx::FromRow)]
    struct Totals {
        income: Decimal,
        expense: Decimal,
    }
    let totals: Totals = sqlx::query_as(
        "SELECT \
            COALESCE(SUM(CASE WHEN type='income'  THEN amount END), 0) AS income, \
            COALESCE(SUM(CASE WHEN type='expense' THEN amount END), 0) AS expense \
         FROM ledger_entry WHERE entry_date >= ? AND entry_date < ?",
    )
    .bind(start)
    .bind(next)
    .fetch_one(pool)
    .await?;

    #[derive(sqlx::FromRow, serde::Serialize)]
    struct CategoryRow {
        category: String,
        total: Decimal,
    }
    let by_category: Vec<CategoryRow> = sqlx::query_as(
        "SELECT category, SUM(amount) AS total FROM ledger_entry \
         WHERE entry_date >= ? AND entry_date < ? AND type = 'expense' \
         GROUP BY category ORDER BY total DESC",
    )
    .bind(start)
    .bind(next)
    .fetch_all(pool)
    .await?;

    Ok(ok(serde_json::json!({
        "month": start.format("%Y-%m").to_string(),
        "income": totals.income,
        "expense": totals.expense,
        "balance": totals.income - totals.expense,
        "by_category": by_category,
    })))
}

/// POST /api/ledger
pub async fn create_entry(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateLedgerReq>,
) -> Result<HttpResponse, ApiError> {
    let claims = current_claims(&req)?;
    let pool = state.db()?;
    let r = body.into_inner();

    let entry_type = clean(r.entry_type.as_deref()).unwrap_or_else(|| "expense".into());
    validate_enum(&entry_type, &LEDGER_TYPES, "收支类型")?;
    let category = require_text(&r.category, "分类", 32)?;
    validate_enum(&category, &LEDGER_CATEGORIES, "账本分类")?;
    let amount = to_amount(r.amount, "金额")?;
    validate_range(r.amount, 0.0, 1_000_000_000.0, "金额")?;
    let entry_date = r
        .entry_date
        .unwrap_or_else(|| chrono::Local::now().date_naive());
    if let Some(n) = r.note.as_deref() {
        if n.chars().count() > 255 {
            return Err(ApiError::BadRequest("备注不能超过 255 个字符".into()));
        }
    }

    let member_id = r.member_id.unwrap_or(claims.sub);
    let result = sqlx::query(
        "INSERT INTO ledger_entry (member_id, type, category, amount, entry_date, note) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(member_id)
    .bind(&entry_type)
    .bind(&category)
    .bind(&amount)
    .bind(entry_date)
    .bind(clean(r.note.as_deref()))
    .execute(pool)
    .await?;

    let id = result.last_insert_id();
    let entry: LedgerEntry = sqlx::query_as("SELECT * FROM ledger_entry WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(ok_with_message("账目已记录", entry))
}

/// PUT /api/ledger/{id}
pub async fn update_entry(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<UpdateLedgerReq>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let id = path.into_inner();
    let r = body.into_inner();

    if let Some(t) = clean(r.entry_type.as_deref()) {
        validate_enum(&t, &LEDGER_TYPES, "收支类型")?;
    }
    if let Some(c) = clean(r.category.as_deref()) {
        validate_enum(&c, &LEDGER_CATEGORIES, "账本分类")?;
    }
    let amount = r.amount.map(|v| to_amount(v, "金额")).transpose()?;
    if let Some(n) = r.note.as_deref() {
        if n.chars().count() > 255 {
            return Err(ApiError::BadRequest("备注不能超过 255 个字符".into()));
        }
    }

    sqlx::query(
        "UPDATE ledger_entry SET \
            type = COALESCE(?, type), category = COALESCE(?, category), \
            amount = COALESCE(?, amount), entry_date = COALESCE(?, entry_date), \
            note = IF(? IS NULL, note, ?) \
         WHERE id = ?",
    )
    .bind(clean(r.entry_type.as_deref()))
    .bind(clean(r.category.as_deref()))
    .bind(amount)
    .bind(r.entry_date)
    .bind(r.note.as_deref())
    .bind(r.note.as_deref())
    .bind(id)
    .execute(pool)
    .await?;

    let entry: LedgerEntry = sqlx::query_as("SELECT * FROM ledger_entry WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("账目 {id} 不存在")))?;
    Ok(ok_with_message("账目已更新", entry))
}

/// DELETE /api/ledger/{id}
pub async fn delete_entry(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let id = path.into_inner();

    let result = sqlx::query("DELETE FROM ledger_entry WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("账目 {id} 不存在")));
    }
    Ok(ok_with_message(
        "账目已删除",
        serde_json::json!({ "id": id }),
    ))
}
