//! 家庭点餐系统 · 后端入口
//!
//! 技术栈：actix-web 4 + sqlx 0.8（MySQL 8.0）
//!
//! 启动后：
//!   - API  : http://localhost:8080/api/...
//!   - 前端 : http://localhost:8080/  （当 STATIC_DIR 指向 Vue 构建产物时）

mod auth;
mod config;
mod db;
mod dto;
mod error;
mod models;
mod routes;
mod util;

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use tracing::{info, warn};

use config::AppConfig;
use db::{build_pool, ping, AppState};
use error::{ApiError, DB_HINT};

/// 前端单页应用入口文件内容（启动时读入内存，避免每次请求都读盘）
type IndexHtml = Arc<String>;

#[actix_web::main]
async fn main() -> io::Result<()> {
    // 加载 .env（文件不存在时静默跳过，全部走内置默认值）
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let config = AppConfig::from_env();

    info!("==========================================");
    info!("  家庭点餐系统 · Family Dining API v{}", env!("CARGO_PKG_VERSION"));
    info!("==========================================");
    info!("数据库：{}", config.masked_database_url());

    let pool = build_pool(&config).await;
    match &pool {
        Some(p) => match ping(p).await {
            Ok(()) => info!("数据库状态：连通正常"),
            Err(e) => warn!("数据库状态：连接池已建立但当前不可用（{e}），接口会在 MySQL 恢复后自动可用"),
        },
        None => {
            warn!("数据库状态：未建立连接池，服务以离线模式启动");
            warn!("{DB_HINT}");
        }
    }

    let bind_addr = format!("{}:{}", config.host, config.port);

    // 仅当 dist 目录与 index.html 都真实存在时才托管前端
    let static_dir: Option<PathBuf> = config
        .static_dir
        .as_ref()
        .filter(|d| d.join("index.html").is_file())
        .cloned();

    let index_html: IndexHtml = match &static_dir {
        Some(dir) => {
            let html = std::fs::read_to_string(dir.join("index.html")).unwrap_or_default();
            info!("前端静态资源：{} -> http://{bind_addr}/", dir.display());
            Arc::new(html)
        }
        None => {
            match &config.static_dir {
                Some(d) => warn!(
                    "STATIC_DIR（{}）下未找到 index.html，暂不托管前端；\
                     请先到 frontend 目录执行 npm run build",
                    d.display()
                ),
                None => info!("前端静态资源：已关闭（纯 API 模式）"),
            }
            Arc::new(String::new())
        }
    };

    info!("监听地址  ：http://{bind_addr}");
    info!("健康检查  ：http://{bind_addr}/api/health");
    info!("接口字典  ：http://{bind_addr}/api/meta");
    info!("==========================================");

    let state = AppState::new(pool, &config);
    let cors_origin = config.cors_origin.clone();
    let upload_dir = config.upload_dir.clone();
    let serve_frontend = static_dir.is_some();

    HttpServer::new(move || {
        let upload_dir = upload_dir.clone();
        // error_handler 需返回 actix_web::Error，由 ResponseError 转换而来
        let json_cfg = web::JsonConfig::default()
            .limit(4 * 1024 * 1024)
            .error_handler(|err, _req| {
                ApiError::BadRequest(format!("请求体解析失败：{err}")).into()
            });
        let path_cfg = web::PathConfig::default()
            .error_handler(|err, _req| ApiError::BadRequest(format!("路径参数无效：{err}")).into());
        let query_cfg = web::QueryConfig::default()
            .error_handler(|err, _req| ApiError::BadRequest(format!("查询参数无效：{err}")).into());

        let mut app = App::new()
            .app_data(web::Data::new(state.clone()))
            .app_data(json_cfg)
            .app_data(path_cfg)
            .app_data(query_cfg)
            .app_data(web::PayloadConfig::default().limit(4 * 1024 * 1024))
            .wrap(build_cors(&cors_origin))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::new("%r -> %s %Dms"))
            .wrap(middleware::NormalizePath::trim())
            .configure(move |cfg| routes::configure(cfg, upload_dir.clone()))
            .default_service(web::route().to(not_found));

        if let Some(dir) = &static_dir {
            let html: IndexHtml = index_html.clone();
            app = app.service(
                actix_files::Files::new("/", dir.clone())
                    .index_file("index.html")
                    // 静态文件找不到时回退到 index.html，以支持前端 history 路由
                    .default_handler(move |req: ServiceRequest| {
                        let html = Arc::clone(&html);
                        async move {
                            let (http_req, _payload) = req.into_parts();
                            if http_req.path().starts_with("/api") {
                                return Ok(ServiceResponse::new(http_req, api_not_found()));
                            }
                            let res = HttpResponse::Ok()
                                .content_type("text/html; charset=utf-8")
                                .body(Arc::try_unwrap(html).unwrap_or_else(|arc| (*arc).clone()));
                            Ok(ServiceResponse::new(http_req, res))
                        }
                    }),
            );
        } else {
            app = app.route("/", web::get().to(api_index));
        }

        let _ = serve_frontend;
        app
    })
    .bind(&bind_addr)?
    .workers(worker_count())
    .run()
    .await
}

// =====================================================================
//  CORS
// =====================================================================

/// 构建跨域中间件。
///
/// - `*` 或空值：允许任意来源（家庭内网场景够用）
/// - 逗号分隔列表：逐个加入白名单，非法项跳过并告警
///
/// 说明：actix-cors 0.7 的 `Cors` 本身就是中间件，没有 `build()`；
/// `allowed_origin` 对非法值会 panic，因此这里先做格式校验。
fn build_cors(origin: &str) -> Cors {
    let trimmed = origin.trim();

    let mut cors = Cors::default()
        .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
            actix_web::http::header::CONTENT_TYPE,
            actix_web::http::header::ACCEPT,
            actix_web::http::header::AUTHORIZATION,
        ])
        .supports_credentials()
        .max_age(3600);

    if trimmed == "*" || trimmed.is_empty() {
        // allow_any_origin 与 supports_credentials 互斥，需重新构建
        return Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::ACCEPT,
                actix_web::http::header::AUTHORIZATION,
            ])
            .max_age(3600);
    }

    let mut added = 0usize;
    for o in trimmed.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if !(o.starts_with("http://") || o.starts_with("https://")) {
            warn!("CORS 来源「{o}」格式非法（需以 http:// 或 https:// 开头），已跳过");
            continue;
        }
        cors = cors.allowed_origin(o);
        added += 1;
    }

    if added == 0 {
        warn!("CORS 白名单为空，已回退为允许任意来源");
        return build_cors("*");
    }

    info!("CORS 白名单：已加入 {added} 个来源");
    cors
}

// =====================================================================
//  兜底响应
// =====================================================================

fn worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().min(4).max(2))
        .unwrap_or(2)
}

/// 未托管前端时的根路径首页
async fn api_index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            r#"<!DOCTYPE html>
<html lang="zh-CN"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>家庭点餐系统</title>
<style>
body{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;
font-family:system-ui,-apple-system,"PingFang SC","Microsoft YaHei",sans-serif;
background:linear-gradient(135deg,#FFF3E0,#FFE0EC)}
.card{background:#fff;border-radius:24px;padding:40px 36px;max-width:460px;width:90%;
box-shadow:0 20px 60px rgba(0,0,0,.08);text-align:center}
h1{margin:0 0 8px;font-size:22px;color:#333}
p{margin:8px 0;color:#777;font-size:14px;line-height:1.8;text-align:left}
code{background:#FFF3E0;padding:2px 7px;border-radius:6px;color:#E65100;font-size:13px}
a{display:inline-block;margin-top:18px;padding:10px 24px;border-radius:999px;
background:linear-gradient(135deg,#FF9A8B,#FF6A88);color:#fff;text-decoration:none;font-size:14px}
</style></head><body><div class="card">
<div style="font-size:52px">🍲</div>
<h1>家庭点餐系统 API 已启动</h1>
<p>接口地址：<code>http://localhost:8080/api</code></p>
<p>健康检查：<code>GET /api/health</code>　查看数据库是否连通</p>
<p>接口字典：<code>GET /api/meta</code>　分类、餐次、难度等下拉选项</p>
<p>前端尚未部署：进入 <code>frontend</code> 目录执行 <code>npm run build</code>，
并把环境变量 <code>STATIC_DIR</code> 指向 <code>frontend/dist</code>，重启后端即可在同端口访问页面。</p>
<a href="/api/health">查看健康检查</a>
</div></body></html>"#,
        )
}

fn api_not_found() -> HttpResponse {
    HttpResponse::NotFound().json(serde_json::json!({
        "code": 4040,
        "message": "接口不存在",
        "data": null
    }))
}

async fn not_found(req: HttpRequest) -> HttpResponse {
    if req.path().starts_with("/api") {
        return api_not_found();
    }
    api_not_found()
}
