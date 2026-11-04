//! 成员同步数据（JSON blob 整包存取：学习模块等）
//!
//! 语义：
//! - GET 返回 `{ payload, version }`；无数据时 payload=null、version=0
//! - PUT 全量覆盖；传 expected_version 时做乐观锁（不一致返回 409）

use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::auth::current_claims;
use crate::db::AppState;
use crate::dto::PutSyncReq;
use crate::error::ApiError;
use crate::util::{ok, ok_with_message};

fn validate_key(key: &str) -> Result<String, ApiError> {
    let k = key.trim();
    if k.is_empty() || k.len() > 32 {
        return Err(ApiError::BadRequest("数据域 key 长度需在 1~32 之间".into()));
    }
    if !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(ApiError::BadRequest(
            "数据域 key 只能包含字母、数字、下划线和连字符".into(),
        ));
    }
    Ok(k.to_string())
}

#[derive(Debug, Deserialize)]
pub struct SyncPath {
    pub key: String,
}

/// GET /api/sync/{key}
pub async fn get_sync(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<SyncPath>,
) -> Result<HttpResponse, ApiError> {
    let claims = current_claims(&req)?;
    let pool = state.db()?;
    let key = validate_key(&path.key)?;

    let row: Option<(serde_json::Value, u32)> =
        sqlx::query_as("SELECT payload, version FROM member_data WHERE member_id = ? AND data_key = ?")
            .bind(claims.sub)
            .bind(&key)
            .fetch_optional(pool)
            .await?;

    match row {
        Some((payload, version)) => Ok(ok(serde_json::json!({
            "key": key,
            "payload": payload,
            "version": version,
        }))),
        None => Ok(ok(serde_json::json!({
            "key": key,
            "payload": null,
            "version": 0,
        }))),
    }
}

/// PUT /api/sync/{key} —— 全量覆盖（可选乐观锁）
pub async fn put_sync(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<SyncPath>,
    body: web::Json<PutSyncReq>,
) -> Result<HttpResponse, ApiError> {
    let claims = current_claims(&req)?;
    let pool = state.db()?;
    let key = validate_key(&path.key)?;
    let r = body.into_inner();

    if let Some(expected) = r.expected_version {
        let current: Option<u32> = sqlx::query_scalar(
            "SELECT version FROM member_data WHERE member_id = ? AND data_key = ?",
        )
        .bind(claims.sub)
        .bind(&key)
        .fetch_optional(pool)
        .await?;

        let cur = current.unwrap_or(0);
        if cur != expected {
            return Err(ApiError::Conflict(format!(
                "数据已被其他设备更新（本地版本 {expected}，服务器版本 {cur}），请刷新后重试"
            )));
        }
    }

    sqlx::query(
        "INSERT INTO member_data (member_id, data_key, payload, version) \
         VALUES (?, ?, ?, 1) \
         ON DUPLICATE KEY UPDATE payload = VALUES(payload), version = version + 1",
    )
    .bind(claims.sub)
    .bind(&key)
    .bind(&r.payload)
    .execute(pool)
    .await?;

    let (payload, version): (serde_json::Value, u32) =
        sqlx::query_as("SELECT payload, version FROM member_data WHERE member_id = ? AND data_key = ?")
            .bind(claims.sub)
            .bind(&key)
            .fetch_one(pool)
            .await?;

    Ok(ok_with_message(
        "已保存",
        serde_json::json!({ "key": key, "payload": payload, "version": version }),
    ))
}
