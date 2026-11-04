//! 应用配置：从环境变量（.env）读取。

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub cors_origin: String,
    pub static_dir: Option<PathBuf>,
    /// 封面图片上传的本地存储目录
    pub upload_dir: PathBuf,
    pub max_connections: u32,
    pub connect_retries: u32,

    /// 快递100 授权 key（物流轨迹查询用，留空表示未启用）
    pub kuaidi100_key: String,
    /// 快递100 公司编号 customer（与 key 成对下发）
    pub kuaidi100_customer: String,

    /// JWT 签名密钥（不建 .env 时用内置默认值，仅限内网）
    pub auth_secret: String,
    /// 令牌有效期（天）
    pub auth_expire_days: i64,
}

fn env_str(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    env_str(key, "").parse::<T>().ok().unwrap_or(default)
}

impl AppConfig {
    pub fn from_env() -> Self {
        let static_raw = env_str("STATIC_DIR", "../frontend/dist");
        let static_dir = if static_raw.eq_ignore_ascii_case("none") || static_raw == "-" {
            None
        } else {
            Some(PathBuf::from(static_raw))
        };

        Self {
            database_url: env_str(
                "DATABASE_URL",
                "mysql://root:password@127.0.0.1:3306/family_dining",
            ),
            host: env_str("HOST", "0.0.0.0"),
            port: env_parse("PORT", 8080u16),
            cors_origin: env_str("CORS_ORIGIN", "*"),
            static_dir,
            upload_dir: PathBuf::from(env_str("UPLOAD_DIR", "./uploads")),
            max_connections: env_parse("DB_MAX_CONNECTIONS", 10u32),
            connect_retries: env_parse("DB_CONNECT_RETRIES", 3u32),
            kuaidi100_key: env_str("KUAIDI100_KEY", ""),
            kuaidi100_customer: env_str("KUAIDI100_CUSTOMER", ""),
            auth_secret: env_str("AUTH_SECRET", "family-dining-local-secret-change-me"),
            auth_expire_days: env_parse("AUTH_EXPIRE_DAYS", 30i64),
        }
    }

    /// 快递100 是否已配置完整；未配置时物流查询应返回「未启用」而非报错
    ///
    /// 预留给后期物流轨迹查询使用，当前尚无调用方。
    #[allow(dead_code)]
    pub fn kuaidi100_enabled(&self) -> bool {
        !self.kuaidi100_key.is_empty() && !self.kuaidi100_customer.is_empty()
    }

    /// 隐藏密码后的连接串，用于日志输出
    pub fn masked_database_url(&self) -> String {
        match self.database_url.split_once("://") {
            Some((scheme, rest)) => match rest.split_once('@') {
                Some((cred, host)) => match cred.split_once(':') {
                    Some((user, _)) => format!("{scheme}://{user}:****@{host}"),
                    None => format!("{scheme}://{cred}@{host}"),
                },
                None => self.database_url.clone(),
            },
            None => self.database_url.clone(),
        }
    }
}
