//! 鉴权：每人密码（argon2 哈希）+ JWT 令牌 + /api 保护中间件
//!
//! 约定：
//! - `member.password_hash` 为 NULL 表示未设置密码，允许免密登录（渐进迁移，家人零门槛）
//! - 令牌放 `Authorization: Bearer <token>`，有效期 AUTH_EXPIRE_DAYS（默认 30 天）
//! - 公开路径：/api/health、/api/meta、/api/auth/members、/api/auth/login、GET /api/uploads/*（<img> 无法带请求头）
//! - Claims 注入请求扩展，处理器可用 `web::ReqData<Claims>` 取当前登录人

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::Method,
    middleware::Next,
    web, Error, HttpMessage,
};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

/// JWT 声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 成员ID
    pub sub: u64,
    /// 角色：admin / member / child
    pub role: String,
    /// 过期时间（unix 秒）
    pub exp: usize,
}

// =====================================================================
//  密码哈希（argon2）
// =====================================================================

/// 生成密码哈希
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;

    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(format!("密码哈希失败：{e}")))
}

/// 校验密码；哈希损坏一律视为不通过
pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;

    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

// =====================================================================
//  JWT
// =====================================================================

/// 签发令牌
pub fn issue_token(
    member_id: u64,
    role: &str,
    secret: &str,
    expire_days: i64,
) -> Result<String, ApiError> {
    use jsonwebtoken::{encode, EncodingKey, Header};

    let exp = chrono::Utc::now().timestamp() + expire_days.max(1) * 86_400;
    let claims = Claims {
        sub: member_id,
        role: role.to_string(),
        exp: exp as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("令牌签发失败：{e}")))
}

/// 解析并校验令牌（含过期）
pub fn parse_token(token: &str, secret: &str) -> Result<Claims, ApiError> {
    use jsonwebtoken::{decode, DecodingKey, Validation};

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| ApiError::Unauthorized("登录已过期或令牌无效，请重新登录".into()))
}

// =====================================================================
//  中间件
// =====================================================================

/// 从 `Authorization: Bearer <token>` 提取令牌
fn bearer_token(req: &ServiceRequest) -> Option<String> {
    req.headers()
        .get(actix_web::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 是否公开路径（无需登录）
fn is_public(path: &str, method: &Method) -> bool {
    if *method == Method::OPTIONS {
        return true;
    }
    if path == "/api/health" || path == "/api/meta" {
        return true;
    }
    // 登录入口公开（登录页成员卡片 + 登录）；改密/重置/me 需要令牌
    if path == "/api/auth/members" || path == "/api/auth/login" {
        return true;
    }
    // 封面图片经 <img> 标签加载，带不了请求头
    *method == Method::GET && path.starts_with("/api/uploads/")
}

/// /api 鉴权中间件：校验 JWT 并把 Claims 注入请求扩展
pub async fn auth_middleware<B: MessageBody>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<B>, Error> {
    if is_public(req.path(), req.method()) {
        return next.call(req).await;
    }

    let state = req.app_data::<web::Data<crate::db::AppState>>();
    let secret = state.map(|s| s.auth_secret.clone()).unwrap_or_default();

    let claims = match bearer_token(&req) {
        Some(token) => parse_token(&token, &secret)?,
        None => return Err(ApiError::Unauthorized("未登录，请先登录".into()).into()),
    };

    req.extensions_mut().insert(claims);
    next.call(req).await
}

/// 从请求扩展取当前登录人（供处理器使用）
pub fn current_claims(req: &actix_web::HttpRequest) -> Result<Claims, ApiError> {
    req.extensions()
        .get::<Claims>()
        .cloned()
        .ok_or_else(|| ApiError::Unauthorized("未登录，请先登录".into()))
}
