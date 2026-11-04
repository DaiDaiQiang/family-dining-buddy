//! 路由模块聚合。

pub mod auth;
pub mod basics;
pub mod health;
pub mod ledger;
pub mod orders;
pub mod parcel;
pub mod recipes;
pub mod reminder;
pub mod stats;
pub mod sync;
pub mod uploads;
pub mod version;

use actix_web::middleware::from_fn;
use actix_web::web;
use std::path::PathBuf;

use crate::auth::auth_middleware;
use crate::db::{ping, AppState};

/// 注册全部 API 路由到 `/api` 作用域
///
/// 鉴权中间件保护整个作用域（公开路径见 auth::is_public）。
pub fn configure(cfg: &mut web::ServiceConfig, upload_dir: PathBuf) {
    cfg.service(
        web::scope("/api")
            .wrap(from_fn(auth_middleware))
            .app_data(web::Data::new(upload_dir))
            // ---- 健康检查（公开）----
            .route("/health", web::get().to(health))
            .route("/meta", web::get().to(meta))
            // ---- 登录鉴权（公开，改密/重置除外）----
            .route("/auth/members", web::get().to(auth::login_members))
            .route("/auth/login", web::post().to(auth::login))
            .route("/auth/me", web::get().to(auth::me))
            .route("/auth/password", web::put().to(auth::change_password))
            .route(
                "/auth/reset-password",
                web::post().to(auth::reset_password),
            )
            // ---- 封面图片上传（GET 公开）----
            .route("/uploads", web::post().to(uploads::create))
            .route("/uploads/{name}", web::get().to(uploads::get))
            // ---- 家庭账本 ----
            .route("/ledger/summary", web::get().to(ledger::summary))
            .route("/ledger", web::get().to(ledger::list_entries))
            .route("/ledger", web::post().to(ledger::create_entry))
            .route("/ledger/{id}", web::put().to(ledger::update_entry))
            .route("/ledger/{id}", web::delete().to(ledger::delete_entry))
            // ---- 快递取件码（静态段放在 {id} 之前）----
            .route("/parcels/picked", web::delete().to(parcel::clear_picked))
            .route("/parcels", web::get().to(parcel::list_parcels))
            .route("/parcels", web::post().to(parcel::create_parcel))
            .route("/parcels/{id}", web::put().to(parcel::update_parcel))
            .route("/parcels/{id}", web::delete().to(parcel::delete_parcel))
            // ---- 健康档案（家庭共享）----
            .route("/health/profiles", web::get().to(health::list_profiles))
            .route(
                "/health/profiles/{member_id}",
                web::put().to(health::save_profile),
            )
            // ---- 成员同步数据（JSON blob）----
            .route("/sync/{key}", web::get().to(sync::get_sync))
            .route("/sync/{key}", web::put().to(sync::put_sync))
            // ---- 站内提醒（阶段 2，静态段放在 {id} 之前）----
            .route("/reminders/sync", web::post().to(reminder::sync_reminders))
            .route("/reminders", web::get().to(reminder::list_reminders))
            .route("/reminders/{id}", web::put().to(reminder::update_reminder))
            // ---- 系统版本更新记录（个人中心 · 版本详情）----
            .route("/versions", web::get().to(version::list_versions))
            .route("/versions", web::post().to(version::create_version))
            // ---- 统计 ----
            .route("/stats/overview", web::get().to(stats::overview))
            .route("/stats/popular", web::get().to(stats::popular))
            .route("/stats/categories", web::get().to(stats::categories))
            .route("/stats/members", web::get().to(stats::member_ranking))
            .route(
                "/stats/members/{id}/taste",
                web::get().to(stats::member_taste),
            )
            .route("/stats/calendar", web::get().to(stats::calendar))
            .route("/stats/suggestion", web::get().to(stats::next_suggestion))
            // ---- 菜谱（静态段必须放在 {id} 之前）----
            .route("/recipes/facets", web::get().to(recipes::list_facets))
            .route("/recipes/random", web::get().to(recipes::random_recipes))
            .route("/recipes", web::get().to(recipes::list_recipes))
            .route("/recipes", web::post().to(recipes::create_recipe))
            .route("/recipes/{id}", web::get().to(recipes::get_recipe))
            .route("/recipes/{id}", web::put().to(recipes::update_recipe))
            .route("/recipes/{id}", web::delete().to(recipes::delete_recipe))
            .route(
                "/recipes/{id}/favorite/{member_id}",
                web::post().to(recipes::toggle_favorite),
            )
            // ---- 成员 ----
            .route("/members", web::get().to(basics::list_members))
            .route("/members", web::post().to(basics::create_member))
            .route("/members/{id}", web::get().to(basics::get_member))
            .route("/members/{id}", web::put().to(basics::update_member))
            .route("/members/{id}", web::delete().to(basics::delete_member))
            // ---- 食材 ----
            .route(
                "/ingredients/categories",
                web::get().to(basics::list_ingredient_categories),
            )
            .route("/ingredients", web::get().to(basics::list_ingredients))
            .route("/ingredients", web::post().to(basics::create_ingredient))
            .route(
                "/ingredients/{id}",
                web::put().to(basics::update_ingredient),
            )
            .route(
                "/ingredients/{id}",
                web::delete().to(basics::delete_ingredient),
            )
            // ---- 标签 ----
            .route("/tags", web::get().to(basics::list_tags))
            .route("/tags", web::post().to(basics::create_tag))
            .route("/tags/{id}", web::delete().to(basics::delete_tag))
            // ---- 订单（静态段必须放在 {id} 之前）----
            .route("/orders/today", web::get().to(orders::today_orders))
            .route("/orders", web::get().to(orders::list_orders))
            .route("/orders", web::post().to(orders::create_order))
            .route(
                "/orders/items/{item_id}",
                web::put().to(orders::update_order_item),
            )
            .route(
                "/orders/items/{item_id}",
                web::delete().to(orders::delete_order_item),
            )
            .route(
                "/orders/items/{item_id}/rating",
                web::post().to(orders::rate_order_item),
            )
            .route(
                "/orders/shopping/{item_id}",
                web::put().to(orders::update_shopping_item),
            )
            .route("/orders/{id}", web::get().to(orders::get_order))
            .route("/orders/{id}", web::put().to(orders::update_order))
            .route("/orders/{id}", web::delete().to(orders::delete_order))
            .route("/orders/{id}/items", web::post().to(orders::add_order_item))
            .route(
                "/orders/{id}/confirm",
                web::post().to(orders::confirm_order),
            )
            .route("/orders/{id}/cancel", web::post().to(orders::cancel_order))
            .route("/orders/{id}/reorder", web::post().to(orders::reorder))
            .route("/orders/{id}/menu", web::get().to(orders::get_order_menu))
            .route(
                "/orders/{id}/recipes",
                web::get().to(orders::get_order_recipes),
            )
            .route(
                "/orders/{id}/shopping-list",
                web::get().to(orders::get_shopping_list),
            )
            .route(
                "/orders/{id}/shopping-list/reset",
                web::post().to(orders::reset_shopping_list),
            ),
    );
}

/// GET /api/health —— 健康检查
async fn health(state: web::Data<AppState>) -> actix_web::HttpResponse {
    let db_status = match state.pool.as_ref() {
        Some(pool) => match ping(pool).await {
            Ok(()) => "connected",
            Err(e) => {
                tracing::warn!("健康检查：数据库 ping 失败 {e}");
                "error"
            }
        },
        None => "unavailable",
    };

    let status = if db_status == "connected" { "ok" } else { "degraded" };

    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "code": 0,
        "message": "ok",
        "data": {
            "service": "family-dining-api",
            "version": env!("CARGO_PKG_VERSION"),
            "status": status,
            "database": db_status,
            "server_time": chrono::Local::now().to_rfc3339(),
        }
    }))
}

/// GET /api/meta —— 前端下拉选项字典（与数据库 CHECK 约束保持一致）
async fn meta(_state: web::Data<AppState>) -> actix_web::HttpResponse {
    use crate::util::*;

    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "code": 0,
        "message": "ok",
        "data": {
            "categories": CATEGORIES,
            "cuisines": CUISINES,
            "difficulties": [
                { "value": "easy",   "label": "简单", "emoji": "🟢" },
                { "value": "medium", "label": "中等", "emoji": "🟡" },
                { "value": "hard",   "label": "挑战", "emoji": "🔴" }
            ],
            "meal_types": [
                { "value": "breakfast", "label": "早餐", "emoji": "🌅" },
                { "value": "lunch",     "label": "午餐", "emoji": "☀️" },
                { "value": "dinner",    "label": "晚餐", "emoji": "🌙" },
                { "value": "snack",     "label": "加餐", "emoji": "🍪" }
            ],
            "order_status": [
                { "value": "draft",     "label": "草稿" },
                { "value": "confirmed", "label": "已确认" },
                { "value": "cooking",   "label": "烹饪中" },
                { "value": "served",    "label": "已开饭" },
                { "value": "cancelled", "label": "已取消" }
            ],
            "ingredient_groups": INGREDIENT_GROUPS,
            "member_roles": [
                { "value": "admin",  "label": "掌勺人" },
                { "value": "member", "label": "家庭成员" },
                { "value": "child",  "label": "小朋友" }
            ],
            "genders": [
                { "value": "female", "label": "女" },
                { "value": "male",   "label": "男" },
                { "value": "other",  "label": "其他" }
            ],
            "cover_presets": [
                { "emoji": "🍖", "gradient": "linear-gradient(135deg,#8E2DE2,#4A00E0)" },
                { "emoji": "🍅", "gradient": "linear-gradient(135deg,#FF512F,#F09819)" },
                { "emoji": "🌶️", "gradient": "linear-gradient(135deg,#CB2D3E,#EF473A)" },
                { "emoji": "🍲", "gradient": "linear-gradient(135deg,#ED213A,#93291E)" },
                { "emoji": "🍗", "gradient": "linear-gradient(135deg,#F7971E,#FFD200)" },
                { "emoji": "🥔", "gradient": "linear-gradient(135deg,#56AB2F,#A8E063)" },
                { "emoji": "🥦", "gradient": "linear-gradient(135deg,#11998E,#38EF7D)" },
                { "emoji": "🐟", "gradient": "linear-gradient(135deg,#2193B0,#6DD5ED)" },
                { "emoji": "🍚", "gradient": "linear-gradient(135deg,#F3904F,#3B4371)" },
                { "emoji": "🍜", "gradient": "linear-gradient(135deg,#F0A500,#CF7500)" },
                { "emoji": "🥒", "gradient": "linear-gradient(135deg,#36D1DC,#5B86E5)" },
                { "emoji": "🥭", "gradient": "linear-gradient(135deg,#FBD3E9,#FBB03B)" },
                { "emoji": "🍮", "gradient": "linear-gradient(135deg,#3E2723,#795548)" },
                { "emoji": "🍋", "gradient": "linear-gradient(135deg,#F7B733,#FC4A1A)" },
                { "emoji": "🥘", "gradient": "linear-gradient(135deg,#4E342E,#8D6E63)" },
                { "emoji": "🍢", "gradient": "linear-gradient(135deg,#C0392B,#8E44AD)" }
            ]
        }
    }))
}
