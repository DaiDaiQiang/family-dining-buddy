//! 登录鉴权路由（/api/auth/*，全部公开，除改密/重置外无需令牌）

use actix_web::{web, HttpResponse, HttpRequest};
use serde::Deserialize;

use crate::auth::{current_claims, hash_password, issue_token, verify_password};
use crate::db::AppState;
use crate::error::ApiError;
use crate::models::Member;
use crate::util::{ok, ok_with_message};

fn clean(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// 密码长度校验（家庭内网：4~64 位，不强制复杂度）
fn validate_password(pwd: &str, field: &str) -> Result<String, ApiError> {
    let p = pwd.trim().to_string();
    if p.chars().count() < 4 {
        return Err(ApiError::BadRequest(format!("{field} 至少 4 个字符")));
    }
    if p.chars().count() > 64 {
        return Err(ApiError::BadRequest(format!("{field} 不能超过 64 个字符")));
    }
    Ok(p)
}

// =====================================================================
//  登录页成员卡片（公开）
// =====================================================================

#[derive(sqlx::FromRow, serde::Serialize)]
struct LoginMember {
    id: u64,
    nickname: String,
    avatar_emoji: String,
    avatar_url: Option<String>,
    theme_color: String,
    role: String,
    has_password: bool,
}

/// GET /api/auth/members —— 登录页头像卡片（是否已设密码）
pub async fn login_members(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let list: Vec<LoginMember> = sqlx::query_as(
        "SELECT id, nickname, avatar_emoji, avatar_url, theme_color, role, \
                (password_hash IS NOT NULL) AS has_password \
         FROM member WHERE is_active = 1 ORDER BY sort_order, id",
    )
    .fetch_all(pool)
    .await?;
    Ok(ok(list))
}

// =====================================================================
//  登录
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct LoginReq {
    pub member_id: Option<u64>,
    pub nickname: Option<String>,
    pub password: Option<String>,
}

/// POST /api/auth/login
/// 未设密码的成员免密登录（渐进迁移）；已设密码必须校验
pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let req = body.into_inner();

    #[derive(sqlx::FromRow)]
    struct AuthRow {
        id: u64,
        nickname: String,
        avatar_emoji: String,
        avatar_url: Option<String>,
        theme_color: String,
        role: String,
        taste_note: Option<String>,
        birthday: Option<chrono::NaiveDate>,
        gender: Option<String>,
        dislikes: Option<String>,
        is_active: bool,
        sort_order: i32,
        created_at: chrono::NaiveDateTime,
        updated_at: chrono::NaiveDateTime,
        password_hash: Option<String>,
    }

    let row: Option<AuthRow> = if let Some(id) = req.member_id {
        sqlx::query_as("SELECT * FROM member WHERE id = ? AND is_active = 1")
            .bind(id)
            .fetch_optional(pool)
            .await?
    } else if let Some(nick) = clean(req.nickname.as_deref()) {
        sqlx::query_as("SELECT * FROM member WHERE nickname = ? AND is_active = 1")
            .bind(nick)
            .fetch_optional(pool)
            .await?
    } else {
        return Err(ApiError::BadRequest("请选择要登录的成员".into()));
    };

    let row = row.ok_or_else(|| ApiError::NotFound("成员不存在或已停用".into()))?;

    match &row.password_hash {
        None => { /* 未设密码：免密登录 */ }
        Some(hash) => {
            let pwd = req.password.unwrap_or_default();
            if !verify_password(&pwd, hash) {
                return Err(ApiError::Unauthorized("密码不正确".into()));
            }
        }
    }

    let token = issue_token(
        row.id,
        &row.role,
        &state.auth_secret,
        state.token_expire_days,
    )?;

    let member = Member {
        id: row.id,
        nickname: row.nickname,
        avatar_emoji: row.avatar_emoji,
        avatar_url: row.avatar_url,
        theme_color: row.theme_color,
        role: row.role,
        taste_note: row.taste_note,
        birthday: row.birthday,
        gender: row.gender,
        dislikes: row.dislikes,
        is_active: row.is_active,
        sort_order: row.sort_order,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(ok(serde_json::json!({
        "token": token,
        "member": member,
        "password_set": row.password_hash.is_some(),
        "expires_days": state.token_expire_days,
    })))
}

// =====================================================================
//  当前登录人 / 改密 / 重置
// =====================================================================

/// GET /api/auth/me —— 当前登录人信息
pub async fn me(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let claims = current_claims(&req)?;
    let pool = state.db()?;

    let member: Option<Member> = sqlx::query_as("SELECT * FROM member WHERE id = ?")
        .bind(claims.sub)
        .fetch_optional(pool)
        .await?;
    let member = member.ok_or_else(|| ApiError::NotFound("成员不存在".into()))?;

    let (has_password,): (bool,) =
        sqlx::query_as("SELECT (password_hash IS NOT NULL) FROM member WHERE id = ?")
            .bind(claims.sub)
            .fetch_one(pool)
            .await?;

    Ok(ok(serde_json::json!({
        "member": member,
        "password_set": has_password,
    })))
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordReq {
    pub old_password: Option<String>,
    pub new_password: String,
}

/// PUT /api/auth/password —— 修改自己的密码（未设密码时可直接设置）
pub async fn change_password(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ChangePasswordReq>,
) -> Result<HttpResponse, ApiError> {
    let claims = current_claims(&req)?;
    let pool = state.db()?;
    let new_pwd = validate_password(&body.new_password, "新密码")?;

    let (hash,): (Option<String>,) =
        sqlx::query_as("SELECT password_hash FROM member WHERE id = ?")
            .bind(claims.sub)
            .fetch_one(pool)
            .await?;

    if let Some(old_hash) = hash {
        let old_pwd = body.old_password.clone().unwrap_or_default();
        if !verify_password(&old_pwd, &old_hash) {
            return Err(ApiError::Unauthorized("原密码不正确".into()));
        }
    }

    let hashed = hash_password(&new_pwd)?;
    sqlx::query("UPDATE member SET password_hash = ? WHERE id = ?")
        .bind(&hashed)
        .bind(claims.sub)
        .execute(pool)
        .await?;

    Ok(ok_with_message("密码已更新", serde_json::json!({ "id": claims.sub })))
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordReq {
    pub member_id: u64,
    pub new_password: String,
}

/// POST /api/auth/reset-password —— admin 重置他人密码
pub async fn reset_password(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ResetPasswordReq>,
) -> Result<HttpResponse, ApiError> {
    let claims = current_claims(&req)?;
    if claims.role != "admin" {
        return Err(ApiError::Forbidden("只有掌勺人可以重置家人密码".into()));
    }

    let pool = state.db()?;
    let new_pwd = validate_password(&body.new_password, "新密码")?;

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM member WHERE id = ?")
        .bind(body.member_id)
        .fetch_one(pool)
        .await?;
    if count == 0 {
        return Err(ApiError::NotFound(format!("成员 {} 不存在", body.member_id)));
    }

    let hashed = hash_password(&new_pwd)?;
    sqlx::query("UPDATE member SET password_hash = ? WHERE id = ?")
        .bind(&hashed)
        .bind(body.member_id)
        .execute(pool)
        .await?;

    Ok(ok_with_message(
        "密码已重置",
        serde_json::json!({ "id": body.member_id }),
    ))
}
