//! 成员健康档案（经期周期）——家庭共享视图（首页提醒卡需要全家视角）
//!
//! - GET /api/health/profiles            全部档案
//! - PUT /api/health/profiles/{member_id} 按成员保存（无则自动建行）

use actix_web::{web, HttpRequest, HttpResponse};

use crate::auth::current_claims;
use crate::db::AppState;
use crate::dto::SaveHealthReq;
use crate::error::ApiError;
use crate::models::HealthProfile;
use crate::util::{ok, ok_with_message, validate_range};

fn clean(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// GET /api/health/profiles
pub async fn list_profiles(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let list: Vec<HealthProfile> =
        sqlx::query_as("SELECT * FROM health_profile ORDER BY member_id").fetch_all(pool).await?;
    Ok(ok(list))
}

/// PUT /api/health/profiles/{member_id}
///
/// gender 接受 female / male / other / unknown（unknown 存 NULL）；
/// last_period_start 传空字符串表示清空。
pub async fn save_profile(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<SaveHealthReq>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;
    let member_id = path.into_inner();
    let r = body.into_inner();

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM member WHERE id = ?")
        .bind(member_id)
        .fetch_one(pool)
        .await?;
    if count == 0 {
        return Err(ApiError::NotFound(format!("成员 {member_id} 不存在")));
    }

    let gender_db: Option<String> = match clean(r.gender.as_deref()) {
        Some(g) => match g.as_str() {
            "female" | "male" | "other" => Some(g),
            "unknown" => None,
            _ => {
                return Err(ApiError::BadRequest(
                    "性别只能是 female / male / other / unknown".into(),
                ))
            }
        },
        None => None,
    };
    if let Some(c) = r.cycle_length {
        validate_range(c, 15u8, 60u8, "周期长度")?;
    }
    if let Some(p) = r.period_length {
        validate_range(p, 1u8, 15u8, "经期长度")?;
    }

    sqlx::query("INSERT IGNORE INTO health_profile (member_id) VALUES (?)")
        .bind(member_id)
        .execute(pool)
        .await?;

    // 整包替换语义：前端每次保存都带完整档案，未传字段按清空/默认处理，
    // 这样「清空周期」（last_period_start=NULL）与「性别改回 unknown」才能生效
    sqlx::query(
        "UPDATE health_profile SET \
            gender = ?, last_period_start = ?, \
            cycle_length = ?, period_length = ? \
         WHERE member_id = ?",
    )
    .bind(gender_db)
    .bind(r.last_period_start)
    .bind(r.cycle_length.unwrap_or(28))
    .bind(r.period_length.unwrap_or(5))
    .bind(member_id)
    .execute(pool)
    .await?;

    let profile: HealthProfile = sqlx::query_as("SELECT * FROM health_profile WHERE member_id = ?")
        .bind(member_id)
        .fetch_one(pool)
        .await?;
    Ok(ok_with_message("健康档案已保存", profile))
}
