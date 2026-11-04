//! 统一错误类型与 HTTP 响应映射。

use actix_web::{http::StatusCode, HttpResponse};
use serde_json::json;
use std::fmt;

/// 数据库未就绪时返回给前端的引导文案
pub const DB_HINT: &str = "数据库尚未连接。请先安装 MySQL 8.0，依次执行 database/01_schema.sql 与 \
database/02_seed_data.sql，然后在 backend/.env 中配置 DATABASE_URL 并重启后端服务。";

#[derive(Debug)]
pub enum ApiError {
    /// 数据库连接池不可用（服务启动时未能连上 MySQL）
    DbUnavailable(String),
    /// 请求参数不合法
    BadRequest(String),
    /// 未登录或令牌失效
    Unauthorized(String),
    /// 已登录但无权限
    Forbidden(String),
    /// 资源不存在
    NotFound(String),
    /// 唯一约束冲突等业务冲突
    Conflict(String),
    /// 服务端内部错误
    Internal(String),
}

impl ApiError {
    pub fn code(&self) -> i32 {
        match self {
            ApiError::DbUnavailable(_) => 5030,
            ApiError::BadRequest(_) => 4000,
            ApiError::Unauthorized(_) => 4010,
            ApiError::Forbidden(_) => 4030,
            ApiError::NotFound(_) => 4040,
            ApiError::Conflict(_) => 4090,
            ApiError::Internal(_) => 5000,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::DbUnavailable(m) => write!(f, "{m}"),
            ApiError::BadRequest(m) => write!(f, "{m}"),
            ApiError::Unauthorized(m) => write!(f, "{m}"),
            ApiError::Forbidden(m) => write!(f, "{m}"),
            ApiError::NotFound(m) => write!(f, "{m}"),
            ApiError::Conflict(m) => write!(f, "{m}"),
            ApiError::Internal(m) => write!(f, "服务器内部错误：{m}"),
        }
    }
}

impl actix_web::ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::DbUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(json!({
            "code": self.code(),
            "message": self.to_string(),
            "data": null,
        }))
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => ApiError::NotFound("记录不存在".into()),
            sqlx::Error::PoolTimedOut => ApiError::DbUnavailable(
                "数据库连接池繁忙或数据库不可达，请检查 MySQL 服务是否正在运行。".into(),
            ),
            sqlx::Error::PoolClosed => ApiError::DbUnavailable("数据库连接池已关闭。".into()),
            sqlx::Error::Database(db) => {
                let sqlstate = db.code().unwrap_or_default().to_string();
                let msg = db.message().to_string();

                // 23000 = 完整性约束违反（含唯一键冲突 1062）
                if sqlstate == "23000" {
                    if msg.contains("uk_recipe_name") || msg.contains("recipe.name") {
                        return ApiError::Conflict("菜名已存在，请换一个名字".into());
                    }
                    if msg.contains("uk_member_nickname") {
                        return ApiError::Conflict("该昵称已被占用".into());
                    }
                    if msg.contains("uk_tag_name") {
                        return ApiError::Conflict("标签已存在".into());
                    }
                    if msg.contains("uk_order_no") {
                        return ApiError::Conflict("订单号冲突，请重试".into());
                    }
                    if msg.to_lowercase().contains("foreign key") {
                        return ApiError::BadRequest(format!("关联数据不存在或正被引用：{msg}"));
                    }
                    return ApiError::Conflict(format!("数据冲突：{msg}"));
                }

                // 存储过程 SIGNAL 抛出的业务错误
                if sqlstate == "45000" {
                    return ApiError::BadRequest(msg);
                }

                // 42S02 = 表/视图不存在，通常是尚未导入建表脚本
                if sqlstate == "42S02" || msg.contains("doesn't exist") {
                    return ApiError::DbUnavailable(format!(
                        "数据表或视图不存在，请先执行 database/01_schema.sql 与 \
                         database/02_seed_data.sql。原始信息：{msg}"
                    ));
                }

                ApiError::Internal(msg)
            }
            other => ApiError::Internal(other.to_string()),
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::BadRequest(format!("请求体不是合法 JSON：{err}"))
    }
}

impl From<std::num::ParseIntError> for ApiError {
    fn from(err: std::num::ParseIntError) -> Self {
        ApiError::BadRequest(format!("数字参数解析失败：{err}"))
    }
}
