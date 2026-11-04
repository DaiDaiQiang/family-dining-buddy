//! 统计模块：首页概览、菜品排行、成员口味偏好。

use actix_web::{web, HttpResponse};
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::db::AppState;
use crate::error::ApiError;
use crate::models::*;
use crate::util::ok;

/// GET /api/stats/overview —— 首页概览
pub async fn overview(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let today = chrono::Local::now().date_naive();

    let (recipe_total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM recipe")
        .fetch_one(pool)
        .await?;
    let (recipe_active,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM recipe WHERE is_active = 1")
        .fetch_one(pool)
        .await?;
    let (member_total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM member WHERE is_active = 1")
        .fetch_one(pool)
        .await?;
    let (ingredient_total,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ingredient WHERE is_active = 1")
            .fetch_one(pool)
            .await?;
    let (order_total,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM meal_order WHERE status <> 'cancelled'")
            .fetch_one(pool)
            .await?;
    let (order_today,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM meal_order WHERE meal_date = ? AND status <> 'cancelled'",
    )
    .bind(today)
    .fetch_one(pool)
    .await?;
    let (order_pending,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM meal_order WHERE status IN ('draft','confirmed')",
    )
    .fetch_one(pool)
    .await?;
    let (dishes_served,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM order_item oi JOIN meal_order o ON o.id = oi.order_id \
         WHERE o.status = 'served'",
    )
    .fetch_one(pool)
    .await?;

    let avg_rating: Option<Decimal> = sqlx::query_scalar(
        "SELECT ROUND(AVG(rating),2) FROM order_item oi \
         JOIN meal_order o ON o.id = oi.order_id \
         WHERE oi.rating IS NOT NULL AND o.status <> 'cancelled'",
    )
    .fetch_optional(pool)
    .await?
    .flatten();

    // 点得最多的分类
    let top_category: Option<CategoryCount> = sqlx::query_as(
        "SELECT r.category AS category, \
                (SELECT COUNT(*) FROM recipe x WHERE x.category = r.category AND x.is_active = 1) AS recipe_count, \
                COUNT(*) AS order_count \
         FROM order_item oi \
         JOIN meal_order o ON o.id = oi.order_id \
         JOIN recipe r ON r.id = oi.recipe_id \
         WHERE o.status <> 'cancelled' AND o.status <> 'draft' \
         GROUP BY r.category ORDER BY order_count DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    let favorite_dish: Option<PopularDish> = sqlx::query_as(
        "SELECT r.id, r.name, r.cover_emoji, r.cover_gradient, r.category, \
                COUNT(*) AS order_count, CAST(SUM(oi.portions) AS SIGNED) AS total_portions, \
                ROUND(AVG(oi.rating),2) AS avg_rating \
         FROM order_item oi \
         JOIN meal_order o ON o.id = oi.order_id \
         JOIN recipe r ON r.id = oi.recipe_id \
         WHERE o.status <> 'cancelled' AND o.status <> 'draft' \
         GROUP BY r.id, r.name, r.cover_emoji, r.cover_gradient, r.category \
         ORDER BY order_count DESC, avg_rating DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(ok(Overview {
        recipe_total,
        recipe_active,
        member_total,
        ingredient_total,
        order_total,
        order_today,
        order_pending,
        dishes_served,
        avg_rating,
        top_category,
        favorite_dish,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PopularQuery {
    pub limit: Option<i64>,
    pub category: Option<String>,
    /// days=N 时只统计最近 N 天
    pub days: Option<i64>,
}

/// GET /api/stats/popular —— 菜品排行
pub async fn popular(
    state: web::Data<AppState>,
    query: web::Query<PopularQuery>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let limit = query.limit.unwrap_or(10).clamp(1, 50);

    let since: Option<NaiveDate> = query
        .days
        .filter(|d| *d > 0)
        .map(|d| chrono::Local::now().date_naive() - chrono::Duration::days(d - 1));

    let category = query
        .category
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "全部");

    let mut cond = String::from(" WHERE o.status <> 'cancelled' AND o.status <> 'draft'");
    let mut cat_bind: Option<String> = None;
    let mut since_bind: Option<NaiveDate> = None;

    if let Some(c) = category {
        cond.push_str(" AND r.category = ?");
        cat_bind = Some(c.to_string());
    }
    if let Some(s) = since {
        cond.push_str(" AND o.meal_date >= ?");
        since_bind = Some(s);
    }

    let sql = format!(
        "SELECT r.id, r.name, r.cover_emoji, r.cover_gradient, r.category, \
                COUNT(*) AS order_count, CAST(SUM(oi.portions) AS SIGNED) AS total_portions, \
                ROUND(AVG(oi.rating),2) AS avg_rating \
         FROM order_item oi \
         JOIN meal_order o ON o.id = oi.order_id \
         JOIN recipe r ON r.id = oi.recipe_id{cond} \
         GROUP BY r.id, r.name, r.cover_emoji, r.cover_gradient, r.category \
         ORDER BY order_count DESC, avg_rating DESC \
         LIMIT ?"
    );

    let mut q = sqlx::query_as::<_, PopularDish>(&sql);
    if let Some(c) = cat_bind {
        q = q.bind(c);
    }
    if let Some(s) = since_bind {
        q = q.bind(s);
    }
    let list = q.bind(limit).fetch_all(pool).await?;

    Ok(ok(list))
}

/// GET /api/stats/categories —— 分类分布（菜谱数 + 点单数）
pub async fn categories(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let list: Vec<CategoryCount> = sqlx::query_as(
        "SELECT c.category AS category, \
                (SELECT COUNT(*) FROM recipe r WHERE r.category = c.category AND r.is_active = 1) AS recipe_count, \
                (SELECT COUNT(*) FROM order_item oi JOIN meal_order o ON o.id = oi.order_id \
                   JOIN recipe r2 ON r2.id = oi.recipe_id \
                  WHERE r2.category = c.category AND o.status <> 'cancelled' AND o.status <> 'draft') AS order_count \
         FROM (SELECT DISTINCT category FROM recipe) c \
         ORDER BY order_count DESC, recipe_count DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(ok(list))
}

/// GET /api/stats/members/{id}/taste —— 单个成员的口味画像
pub async fn member_taste(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let member_id = path.into_inner();

    let member: Member = sqlx::query_as("SELECT * FROM member WHERE id = ?")
        .bind(member_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("成员 {member_id} 不存在")))?;

    // 分类偏好
    let by_category: Vec<(String, i64, Option<Decimal>)> = sqlx::query_as(
        "SELECT r.category, COUNT(*), ROUND(AVG(oi.rating),2) \
         FROM order_item oi \
         JOIN meal_order o ON o.id = oi.order_id \
         JOIN recipe r ON r.id = oi.recipe_id \
         WHERE oi.member_id = ? AND o.status <> 'cancelled' AND o.status <> 'draft' \
         GROUP BY r.category ORDER BY COUNT(*) DESC",
    )
    .bind(member_id)
    .fetch_all(pool)
    .await?;

    // 常点的菜
    let top_dishes: Vec<PopularDish> = sqlx::query_as(
        "SELECT r.id, r.name, r.cover_emoji, r.cover_gradient, r.category, \
                COUNT(*) AS order_count, CAST(SUM(oi.portions) AS SIGNED) AS total_portions, \
                ROUND(AVG(oi.rating),2) AS avg_rating \
         FROM order_item oi \
         JOIN meal_order o ON o.id = oi.order_id \
         JOIN recipe r ON r.id = oi.recipe_id \
         WHERE oi.member_id = ? AND o.status <> 'cancelled' AND o.status <> 'draft' \
         GROUP BY r.id, r.name, r.cover_emoji, r.cover_gradient, r.category \
         ORDER BY order_count DESC, avg_rating DESC LIMIT 5",
    )
    .bind(member_id)
    .fetch_all(pool)
    .await?;

    // 收藏
    let favorites: Vec<(u64, String, String, String)> = sqlx::query_as(
        "SELECT r.id, r.name, r.cover_emoji, r.cover_gradient \
         FROM member_favorite f JOIN recipe r ON r.id = f.recipe_id \
         WHERE f.member_id = ? ORDER BY f.created_at DESC LIMIT 10",
    )
    .bind(member_id)
    .fetch_all(pool)
    .await?;

    let (order_count, total_portions, avg_rating, last_at): (i64, i64, Option<Decimal>, Option<NaiveDate>) =
        sqlx::query_as(
            "SELECT COUNT(*), COALESCE(CAST(SUM(oi.portions) AS SIGNED),0), ROUND(AVG(oi.rating),2), MAX(o.meal_date) \
             FROM order_item oi JOIN meal_order o ON o.id = oi.order_id \
             WHERE oi.member_id = ? AND o.status <> 'cancelled'",
        )
        .bind(member_id)
        .fetch_one(pool)
        .await?;

    Ok(ok(serde_json::json!({
        "member": member,
        "summary": {
            "order_count": order_count,
            "total_portions": total_portions,
            "avg_rating": avg_rating,
            "last_ordered_at": last_at,
        },
        "by_category": by_category.iter().map(|(c, n, r)| serde_json::json!({
            "category": c, "order_count": n, "avg_rating": r
        })).collect::<Vec<_>>(),
        "top_dishes": top_dishes,
        "favorites": favorites.iter().map(|(id, name, emoji, gradient)| serde_json::json!({
            "id": id, "name": name, "cover_emoji": emoji, "cover_gradient": gradient
        })).collect::<Vec<_>>(),
    })))
}

/// GET /api/stats/members —— 全家口味排行
pub async fn member_ranking(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;

    #[derive(sqlx::FromRow, serde::Serialize)]
    struct Row {
        member_id: u64,
        nickname: String,
        avatar_emoji: String,
        theme_color: String,
        order_count: i64,
        total_portions: i64,
        avg_rating: Option<Decimal>,
        last_ordered_at: Option<NaiveDate>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT m.id AS member_id, m.nickname, m.avatar_emoji, m.theme_color, \
                COUNT(oi.id) AS order_count, \
                COALESCE(CAST(SUM(oi.portions) AS SIGNED),0) AS total_portions, \
                ROUND(AVG(oi.rating),2) AS avg_rating, \
                MAX(o.meal_date) AS last_ordered_at \
         FROM member m \
         LEFT JOIN order_item oi ON oi.member_id = m.id \
         LEFT JOIN meal_order o ON o.id = oi.order_id AND o.status <> 'cancelled' \
         WHERE m.is_active = 1 \
         GROUP BY m.id, m.nickname, m.avatar_emoji, m.theme_color, m.sort_order \
         ORDER BY order_count DESC, m.sort_order",
    )
    .fetch_all(pool)
    .await?;

    // 每位成员的最爱分类与常点菜
    let mut result = Vec::new();
    for r in rows {
        let top_category: Option<(String,)> = sqlx::query_as(
            "SELECT r.category FROM order_item oi \
             JOIN meal_order o ON o.id = oi.order_id \
             JOIN recipe r ON r.id = oi.recipe_id \
             WHERE oi.member_id = ? AND o.status <> 'cancelled' AND o.status <> 'draft' \
             GROUP BY r.category ORDER BY COUNT(*) DESC LIMIT 1",
        )
        .bind(r.member_id)
        .fetch_optional(pool)
        .await?;

        let dish_names: Vec<(String,)> = sqlx::query_as(
            "SELECT oi.recipe_snapshot FROM order_item oi \
             JOIN meal_order o ON o.id = oi.order_id \
             WHERE oi.member_id = ? AND o.status <> 'cancelled' AND o.status <> 'draft' \
             GROUP BY oi.recipe_snapshot ORDER BY COUNT(*) DESC LIMIT 3",
        )
        .bind(r.member_id)
        .fetch_all(pool)
        .await?;

        result.push(serde_json::json!({
            "member_id": r.member_id,
            "nickname": r.nickname,
            "avatar_emoji": r.avatar_emoji,
            "theme_color": r.theme_color,
            "order_count": r.order_count,
            "total_portions": r.total_portions,
            "avg_rating": r.avg_rating,
            "last_ordered_at": r.last_ordered_at,
            "top_category": top_category.map(|t| t.0),
            "top_dishes": dish_names.iter().map(|d| d.0.clone()).collect::<Vec<_>>(),
        }));
    }

    Ok(ok(result))
}

/// GET /api/stats/calendar?year=2026&month=9 —— 月历点餐记录
pub async fn calendar(
    state: web::Data<AppState>,
    query: web::Query<CalendarQuery>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let now = chrono::Local::now().date_naive();
    let year = query.year.unwrap_or(now.year()).clamp(2000, 2100);
    let month = query.month.unwrap_or(now.month()).clamp(1, 12);

    let start = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| ApiError::BadRequest("日期无效".into()))?;
    let end = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };

    #[derive(sqlx::FromRow, serde::Serialize)]
    struct Day {
        meal_date: NaiveDate,
        order_count: i64,
        dish_names: Option<String>,
    }

    let days: Vec<Day> = sqlx::query_as(
        "SELECT o.meal_date, COUNT(DISTINCT o.id) AS order_count, \
                (SELECT GROUP_CONCAT(DISTINCT oi.recipe_snapshot ORDER BY oi.sort_order SEPARATOR '、') \
                   FROM order_item oi JOIN meal_order o2 ON o2.id = oi.order_id \
                  WHERE o2.meal_date = o.meal_date AND o2.status <> 'cancelled' \
                    AND oi.item_status <> 'skipped') AS dish_names \
         FROM meal_order o \
         WHERE o.meal_date >= ? AND o.meal_date < ? AND o.status <> 'cancelled' \
         GROUP BY o.meal_date ORDER BY o.meal_date",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;

    Ok(ok(serde_json::json!({
        "year": year,
        "month": month,
        "days": days,
        "active_days": days.len(),
    })))
}

#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
}

/// GET /api/stats/next-suggestion —— 「今晚吃什么」智能建议
///
/// 规则（家庭场景够用且可解释）：
/// 1. 优先推最近 7 天没吃过的菜
/// 2. 在候选里按评分与点单次数加权
/// 3. 按建议荤素搭配取 1 荤 1 素 1 汤
pub async fn next_suggestion(
    state: web::Data<AppState>,
    query: web::Query<SuggestQuery>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let count = query.count.unwrap_or(4).clamp(1, 10);
    let avoid_days = query.avoid_days.unwrap_or(7).clamp(0, 60);

    let since = chrono::Local::now().date_naive() - chrono::Duration::days(avoid_days as i64);

    let candidates: Vec<RecipeCard> = sqlx::query_as(&format!(
        "SELECT {cols} FROM recipe r \
         WHERE r.is_active = 1 \
           AND NOT EXISTS ( \
             SELECT 1 FROM order_item oi JOIN meal_order o ON o.id = oi.order_id \
              WHERE oi.recipe_id = r.id AND o.meal_date >= ? AND o.status <> 'cancelled') \
         ORDER BY (r.avg_rating * 2 + r.cook_count) DESC, RAND() LIMIT ?",
        cols = "r.id, r.name, r.category, r.cuisine, r.difficulty, r.total_minutes, r.servings, \
                r.cover_emoji, r.cover_gradient, r.cover_url, r.description, r.calories, \
                r.is_active, r.cook_count, r.avg_rating, \
                (SELECT GROUP_CONCAT(t.name ORDER BY t.sort_order SEPARATOR ',') \
                   FROM recipe_tag rt JOIN tag t ON t.id = rt.tag_id WHERE rt.recipe_id = r.id) AS tag_names, \
                (SELECT COUNT(*) FROM member_favorite f WHERE f.recipe_id = r.id) AS favorite_count"
    ))
    .bind(since)
    .bind(count * 4)
    .fetch_all(pool)
    .await?;

    // 荤素汤搭配
    let main: Vec<&RecipeCard> = candidates
        .iter()
        .filter(|c| matches!(c.category.as_str(), "家常菜" | "主食"))
        .take(2)
        .collect();
    let veg: Vec<&RecipeCard> = candidates
        .iter()
        .filter(|c| matches!(c.category.as_str(), "凉菜"))
        .take(1)
        .collect();
    let soup: Vec<&RecipeCard> = candidates
        .iter()
        .filter(|c| c.category == "汤羹")
        .take(1)
        .collect();

    let mut picked: Vec<&RecipeCard> = Vec::new();
    picked.extend(main);
    picked.extend(veg);
    picked.extend(soup);
    for c in candidates.iter() {
        if picked.len() >= count as usize {
            break;
        }
        if !picked.iter().any(|p| p.id == c.id) {
            picked.push(c);
        }
    }

    let suggestions: Vec<&RecipeCard> = picked.into_iter().take(count as usize).collect();

    // 候选为空时（比如最近天天点同样的菜）退化为纯随机
    let final_list: Vec<RecipeCard> = if suggestions.is_empty() {
        sqlx::query_as(
            "SELECT r.id, r.name, r.category, r.cuisine, r.difficulty, r.total_minutes, \
                    r.servings, r.cover_emoji, r.cover_gradient, r.cover_url, r.description, \
                    r.calories, r.is_active, r.cook_count, r.avg_rating, \
                    (SELECT GROUP_CONCAT(t.name ORDER BY t.sort_order SEPARATOR ',') \
                       FROM recipe_tag rt JOIN tag t ON t.id = rt.tag_id WHERE rt.recipe_id = r.id) AS tag_names, \
                    (SELECT COUNT(*) FROM member_favorite f WHERE f.recipe_id = r.id) AS favorite_count \
             FROM recipe r WHERE r.is_active = 1 ORDER BY RAND() LIMIT ?",
        )
        .bind(count)
        .fetch_all(pool)
        .await?
    } else {
        suggestions.into_iter().cloned().collect()
    };

    Ok(ok(serde_json::json!({
        "avoid_days": avoid_days,
        "reason": format!("已避开最近 {avoid_days} 天吃过的菜"),
        "recipes": final_list,
    })))
}

#[derive(Debug, Deserialize)]
pub struct SuggestQuery {
    pub count: Option<i64>,
    /// 避开最近 N 天吃过的菜，默认 7 天
    pub avoid_days: Option<i64>,
}
