//! 数据库连接池与应用状态。

use sqlx::mysql::{MySqlConnection, MySqlPool, MySqlPoolOptions};
use std::time::Duration;
use tracing::{info, warn};

use crate::config::AppConfig;
use crate::error::{ApiError, DB_HINT};

/// 全局应用状态。
///
/// `pool` 为 `None` 表示进程启动时完全无法建立连接池；
/// 此时所有数据库接口返回 503 并附带配置引导，服务本身仍可提供健康检查。
#[derive(Clone)]
pub struct AppState {
    pub pool: Option<MySqlPool>,
    /// JWT 签名密钥（AUTH_SECRET）
    pub auth_secret: String,
    /// 令牌有效期（天）
    pub token_expire_days: i64,
}

impl AppState {
    pub fn new(pool: Option<MySqlPool>, config: &AppConfig) -> Self {
        Self {
            pool,
            auth_secret: config.auth_secret.clone(),
            token_expire_days: config.auth_expire_days,
        }
    }

    /// 取得连接池，不可用时返回带引导文案的 503 错误
    pub fn db(&self) -> Result<&MySqlPool, ApiError> {
        self.pool
            .as_ref()
            .ok_or_else(|| ApiError::DbUnavailable(DB_HINT.to_string()))
    }
}

/// 按配置建立 MySQL 连接池，带重试。
///
/// 连接池创建成功即返回（即便首次 ping 失败），这样 MySQL 之后启动时
/// 无需重启后端也能自动恢复；ping 失败只记录告警。
pub async fn build_pool(config: &AppConfig) -> Option<MySqlPool> {
    let retries = config.connect_retries.max(1);

    for attempt in 1..=retries {
        let build_result = MySqlPoolOptions::new()
            .max_connections(config.max_connections.max(1))
            .min_connections(0)
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(&config.database_url)
            .await;

        match build_result {
            Ok(pool) => match ping(&pool).await {
                Ok(()) => {
                    info!("MySQL 连接成功（第 {attempt} 次尝试）");
                    return Some(pool);
                }
                Err(err) => {
                    warn!("连接池已建立但连通性检查失败：{err}");
                    warn!("服务将以「数据库未就绪」模式启动，MySQL 恢复后接口会自动可用。");
                    return Some(pool);
                }
            },
            Err(err) => {
                warn!("第 {attempt}/{retries} 次连接 MySQL 失败：{err}");
                if attempt < retries {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    warn!("无法建立 MySQL 连接池，服务以离线模式启动。{DB_HINT}");
    None
}

/// 连通性检查。
///
/// 用 `SELECT 1` 而非驱动内置 ping，以兼容不同 sqlx 版本；
/// 同时能真实验证连接是否可用（池里有空闲连接不代表 MySQL 还活着）。
pub async fn ping(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    let (_one,): (i32,) = sqlx::query_as("SELECT 1").fetch_one(pool).await?;
    Ok(())
}

/// 生成订单号：`YYYYMMDD-{餐次首字母}-{三位序号}`
///
/// 在调用方事务内执行，序号取当日同餐次已有订单数 + 1；
/// 若仍撞唯一键（并发下可能），由调用方整体重试。
pub async fn next_order_no(
    tx: &mut MySqlConnection,
    meal_date: chrono::NaiveDate,
    meal_type: &str,
) -> Result<String, ApiError> {
    let letter = match meal_type {
        "breakfast" => "B",
        "lunch" => "L",
        "dinner" => "D",
        "snack" => "S",
        _ => "X",
    };

    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM meal_order WHERE meal_date = ? AND meal_type = ?")
            .bind(meal_date)
            .bind(meal_type)
            .fetch_one(&mut *tx)
            .await?;

    Ok(format!(
        "{}-{}-{:03}",
        meal_date.format("%Y%m%d"),
        letter,
        count + 1
    ))
}
