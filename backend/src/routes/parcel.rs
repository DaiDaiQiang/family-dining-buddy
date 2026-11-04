//! 快递取件码

use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::auth::current_claims;
use crate::db::AppState;
use crate::dto::{CreateParcelReq, UpdateParcelReq, PARCEL_STATUS};
use crate::error::ApiError;
use crate::models::Parcel;
use crate::util::{ok, ok_with_message, require_text, validate_enum};

fn clean(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// 收件人姓名快照：优先显式传入，否则按 member_id 查昵称
async fn resolve_recipient(
    pool: &sqlx::MySqlPool,
    member_id: Option<u64>,
    name: Option<String>,
) -> Result<(Option<u64>, Option<String>), ApiError> {
    if let Some(mid) = member_id {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM member WHERE id = ?")
            .bind(mid)
            .fetch_one(pool)
            .await?;
        if count == 0 {
            return Err(ApiError::BadRequest(format!("成员 {mid} 不存在")));
        }
        let snap = match clean(name.as_deref()) {
            Some(n) => Some(n),
            None => {
                let (nick,): (String,) = sqlx::query_as("SELECT nickname FROM member WHERE id = ?")
                    .bind(mid)
                    .fetch_one(pool)
                    .await?;
                Some(nick)
            }
        };
        return Ok((Some(mid), snap));
    }
    Ok((None, clean(name.as_deref())))
}

#[derive(Debug, Deserialize)]
pub struct ParcelQuery {
    /// pending / picked / 空=全部
    pub status: Option<String>,
    pub recipient_member_id: Option<u64>,
}

/// GET /api/parcels?status=&recipient_member_id=
/// 排序：待取沉顶，同状态按到件日期倒序
pub async fn list_parcels(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<ParcelQuery>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let q = query.into_inner();

    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new("SELECT * FROM parcel WHERE 1=1");
    if let Some(s) = clean(q.status.as_deref()) {
        validate_enum(&s, &PARCEL_STATUS, "快递状态")?;
        qb.push(" AND status = ").push_bind(s);
    }
    if let Some(m) = q.recipient_member_id {
        qb.push(" AND recipient_member_id = ").push_bind(m);
    }
    qb.push(" ORDER BY FIELD(status,'pending','picked') , arrived_date DESC, id DESC");

    let list = qb.build_query_as::<Parcel>().fetch_all(pool).await?;
    Ok(ok(list))
}

/// POST /api/parcels
pub async fn create_parcel(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateParcelReq>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let r = body.into_inner();

    let pickup_code = require_text(&Some(r.pickup_code.clone()), "取件码", 64)?;
    let courier_code = clean(r.courier_code.as_deref()).unwrap_or_else(|| "other".into());
    if courier_code.chars().count() > 32 {
        return Err(ApiError::BadRequest("快递公司编码不能超过 32 个字符".into()));
    }
    if let Some(n) = r.note.as_deref() {
        if n.chars().count() > 255 {
            return Err(ApiError::BadRequest("备注不能超过 255 个字符".into()));
        }
    }

    let (recipient_member_id, recipient_name) =
        resolve_recipient(pool, r.recipient_member_id, r.recipient_name.clone()).await?;
    let arrived_date = r
        .arrived_date
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let result = sqlx::query(
        "INSERT INTO parcel (pickup_code, courier_code, courier_name, station_type, station, shelf_no, \
            tracking_no, note, recipient_member_id, recipient_name, arrived_date) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&pickup_code)
    .bind(&courier_code)
    .bind(clean(r.courier_name.as_deref()))
    .bind(clean(r.station_type.as_deref()))
    .bind(clean(r.station.as_deref()))
    .bind(clean(r.shelf_no.as_deref()))
    .bind(clean(r.tracking_no.as_deref()))
    .bind(clean(r.note.as_deref()))
    .bind(recipient_member_id)
    .bind(recipient_name)
    .bind(arrived_date)
    .execute(pool)
    .await?;

    let id = result.last_insert_id();
    let parcel: Parcel = sqlx::query_as("SELECT * FROM parcel WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(ok_with_message("快递已记录", parcel))
}

/// PUT /api/parcels/{id}（含标记已取 / 退回待取）
pub async fn update_parcel(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<UpdateParcelReq>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let id = path.into_inner();
    let r = body.into_inner();

    if let Some(s) = clean(r.status.as_deref()) {
        validate_enum(&s, &PARCEL_STATUS, "快递状态")?;
    }
    if let Some(n) = r.note.as_deref() {
        if n.chars().count() > 255 {
            return Err(ApiError::BadRequest("备注不能超过 255 个字符".into()));
        }
    }

    // 收件人字段仅在显式传了 member_id / name 时才重算快照
    let (recipient_member_id, recipient_name) = if r.recipient_member_id.is_some()
        || r.recipient_name.is_some()
    {
        resolve_recipient(pool, r.recipient_member_id, r.recipient_name.clone()).await?
    } else {
        (None, None)
    };

    // 标记已取时自动记取件时间；退回待取时清空
    let picked_at: Option<chrono::NaiveDateTime> = match clean(r.status.as_deref()).as_deref() {
        Some("picked") => Some(chrono::Local::now().naive_local()),
        Some("pending") => None,
        _ => None,
    };
    let has_status = clean(r.status.as_deref()).is_some();

    sqlx::query(
        "UPDATE parcel SET \
            pickup_code = COALESCE(?, pickup_code), \
            courier_code = COALESCE(?, courier_code), \
            courier_name = COALESCE(?, courier_name), \
            station_type = COALESCE(?, station_type), \
            station = COALESCE(?, station), \
            shelf_no = COALESCE(?, shelf_no), \
            tracking_no = COALESCE(?, tracking_no), \
            note = IF(? IS NULL, note, ?), \
            recipient_member_id = COALESCE(?, recipient_member_id), \
            recipient_name = COALESCE(?, recipient_name), \
            arrived_date = COALESCE(?, arrived_date), \
            status = COALESCE(?, status), \
            picked_at = IF(? = 0, picked_at, ?) \
         WHERE id = ?",
    )
    .bind(clean(r.pickup_code.as_deref()))
    .bind(clean(r.courier_code.as_deref()))
    .bind(clean(r.courier_name.as_deref()))
    .bind(clean(r.station_type.as_deref()))
    .bind(clean(r.station.as_deref()))
    .bind(clean(r.shelf_no.as_deref()))
    .bind(clean(r.tracking_no.as_deref()))
    .bind(r.note.as_deref())
    .bind(r.note.as_deref())
    .bind(recipient_member_id)
    .bind(recipient_name)
    .bind(r.arrived_date)
    .bind(clean(r.status.as_deref()))
    .bind(has_status)
    .bind(picked_at)
    .bind(id)
    .execute(pool)
    .await?;

    let parcel: Parcel = sqlx::query_as("SELECT * FROM parcel WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("快递 {id} 不存在")))?;
    Ok(ok_with_message("快递已更新", parcel))
}

/// DELETE /api/parcels/{id}
pub async fn delete_parcel(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let id = path.into_inner();

    let result = sqlx::query("DELETE FROM parcel WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("快递 {id} 不存在")));
    }
    Ok(ok_with_message(
        "快递已删除",
        serde_json::json!({ "id": id }),
    ))
}

/// DELETE /api/parcels/picked —— 批量清除已取
pub async fn clear_picked(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;

    let result = sqlx::query("DELETE FROM parcel WHERE status = 'picked'")
        .execute(pool)
        .await?;
    Ok(ok_with_message(
        format!("已清除 {} 条已取记录", result.rows_affected()),
        serde_json::json!({ "removed": result.rows_affected() }),
    ))
}
