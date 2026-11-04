//! 系统版本更新记录（个人中心 · 版本详情）
//!
//! - GET  /api/versions   版本列表（按发布时间倒序，最新在前）
//! - POST /api/versions   追加一条更新记录（仅掌勺人 admin）
//!
//! 每次系统更新追加一条记录（版本号 + 更新内容）；当前版本取列表第一条。

use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::auth::current_claims;
use crate::db::AppState;
use crate::error::ApiError;
use crate::models::SystemVersion;
use crate::util::{ok, ok_with_message, require_text};

fn clean(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// 版本号格式：数字与点组成（如 1.0 / 1.2.3），1~32 字符
fn validate_version_no(v: &str) -> Result<String, ApiError> {
    let t = v.trim();
    let valid = !t.is_empty()
        && t.chars().count() <= 32
        && t.chars().next().is_some_and(|c| c.is_ascii_digit())
        && t.chars().all(|c| c.is_ascii_digit() || c == '.');
    if valid {
        Ok(t.to_string())
    } else {
        Err(ApiError::BadRequest(
            "版本号需为数字与点组成的形式，如 1.0 或 1.2.3".into(),
        ))
    }
}

// =====================================================================
//  列表
// =====================================================================

/// GET /api/versions —— 版本列表（最新在前）
pub async fn list_versions(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let _claims = current_claims(&req)?;
    let pool = state.db()?;

    let list: Vec<SystemVersion> = sqlx::query_as(
        "SELECT * FROM system_version ORDER BY released_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(ok(list))
}

// =====================================================================
//  追加更新记录
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct CreateVersionReq {
    /// 版本号，如 1.1
    pub version: Option<String>,
    /// 更新主题（可选）
    pub title: Option<String>,
    /// 更新内容
    pub content: Option<String>,
    /// 发布时间（可选，缺省当前时间）
    pub released_at: Option<chrono::NaiveDateTime>,
}

/// POST /api/versions —— 追加一条更新记录（仅掌勺人）
pub async fn create_version(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateVersionReq>,
) -> Result<HttpResponse, ApiError> {
    let claims = current_claims(&req)?;
    if claims.role != "admin" {
        return Err(ApiError::Forbidden("只有掌勺人可以记录版本更新".into()));
    }
    let pool = state.db()?;
    let r = body.into_inner();

    let version = validate_version_no(&r.version.unwrap_or_default())?;
    let content = require_text(&r.content, "更新内容", 5000)?;
    let title = clean(r.title.as_deref());
    if let Some(t) = &title {
        if t.chars().count() > 128 {
            return Err(ApiError::BadRequest("更新主题长度不能超过 128 个字符".into()));
        }
    }

    // 版本号唯一（库内也有唯一键兜底）
    let (dup,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM system_version WHERE version = ?")
        .bind(&version)
        .fetch_one(pool)
        .await?;
    if dup > 0 {
        return Err(ApiError::Conflict(format!("版本 {version} 已有更新记录")));
    }

    let row = sqlx::query(
        "INSERT INTO system_version (version, title, content, released_at) \
         VALUES (?, ?, ?, COALESCE(?, NOW()))",
    )
    .bind(&version)
    .bind(&title)
    .bind(&content)
    .bind(r.released_at)
    .execute(pool)
    .await?;

    let saved: SystemVersion =
        sqlx::query_as("SELECT * FROM system_version WHERE id = ?")
            .bind(row.last_insert_id())
            .fetch_one(pool)
            .await?;
    Ok(ok_with_message("版本记录已追加", saved))
}
