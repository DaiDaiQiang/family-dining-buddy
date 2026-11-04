//! 菜谱模块：浏览、搜索、详情、增删改、收藏、随机推荐。

use actix_web::{web, HttpResponse};
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::{MySql, MySqlConnection, MySqlPool, QueryBuilder};

use crate::db::AppState;
use crate::dto::{
    validate_ingredient_entry, validate_recipe_fields, CreateRecipeReq, RecipeIngredientReq,
    StepInput,
};
use crate::error::ApiError;
use crate::models::*;
use crate::util::{normalize_paging, ok, ok_with_message, Page};

/// 食材分组固定顺序
const GROUP_ORDER: [&str; 4] = ["主料", "辅料", "调味料", "其他"];

/// 新建标签时循环使用的调色板
const TAG_PALETTE: [&str; 10] = [
    "#FF7043", "#26A69A", "#66BB6A", "#FFA726", "#42A5F5", "#AB47BC", "#EF5350", "#8BC34A",
    "#FFCA28", "#29B6F6",
];

/// 去首尾空格，空串归一为 None（用于可选文本字段）
fn clean_text(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// 记录是否存在
async fn exists(pool_or_tx: &MySqlPool, sql: &str, id: u64) -> Result<bool, ApiError> {
    let (count,): (i64,) = sqlx::query_as(sql).bind(id).fetch_one(pool_or_tx).await?;
    Ok(count > 0)
}

async fn exists_in_tx(
    tx: &mut MySqlConnection,
    sql: &str,
    id: u64,
) -> Result<bool, ApiError> {
    let (count,): (i64,) = sqlx::query_as(sql).bind(id).fetch_one(&mut *tx).await?;
    Ok(count > 0)
}

// =====================================================================
//  查询参数
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct RecipeQuery {
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub cuisine: Option<String>,
    pub difficulty: Option<String>,
    pub tag: Option<String>,
    /// 排序：rating / popular / fastest / name / newest
    pub sort: Option<String>,
    /// 只看该成员收藏（需配合 member_id）
    pub member_id: Option<u64>,
    /// true 时连下架菜谱一起返回
    pub include_inactive: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RandomQuery {
    pub limit: Option<i64>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    /// true = 物理删除；默认 false = 下架（历史订单仍保留菜名快照）
    pub permanent: Option<bool>,
}

fn sort_fragment(sort: &str) -> &'static str {
    match sort {
        "popular" => "ORDER BY r.cook_count DESC, r.avg_rating DESC, r.id ASC",
        "fastest" => "ORDER BY r.total_minutes ASC, r.id ASC",
        "name" => "ORDER BY r.name ASC",
        "newest" => "ORDER BY r.created_at DESC, r.id DESC",
        _ => "ORDER BY r.avg_rating DESC, r.cook_count DESC, r.id ASC",
    }
}

/// 把「全部」与空串视为不过滤
fn opt_filter(v: &Option<String>) -> Option<&str> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "全部")
}

fn apply_filters<'q>(qb: &mut QueryBuilder<'q, MySql>, q: &'q RecipeQuery) {
    if q.include_inactive != Some(true) {
        qb.push(" AND r.is_active = 1");
    }
    if let Some(kw) = q
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let pattern = format!("%{kw}%");
        qb.push(" AND (r.name LIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR r.description LIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR r.tips LIKE ");
        qb.push_bind(pattern);
        qb.push(")");
    }
    if let Some(c) = opt_filter(&q.category) {
        qb.push(" AND r.category = ");
        qb.push_bind(c);
    }
    if let Some(c) = opt_filter(&q.cuisine) {
        qb.push(" AND r.cuisine = ");
        qb.push_bind(c);
    }
    if let Some(d) = opt_filter(&q.difficulty) {
        qb.push(" AND r.difficulty = ");
        qb.push_bind(d);
    }
    if let Some(t) = q.tag.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        qb.push(
            " AND EXISTS (SELECT 1 FROM recipe_tag rt JOIN tag t ON t.id = rt.tag_id \
             WHERE rt.recipe_id = r.id AND t.name = ",
        );
        qb.push_bind(t);
        qb.push(")");
    }
    if let Some(mid) = q.member_id {
        qb.push(
            " AND EXISTS (SELECT 1 FROM member_favorite mf \
             WHERE mf.recipe_id = r.id AND mf.member_id = ",
        );
        qb.push_bind(mid);
        qb.push(")");
    }
}

const CARD_COLUMNS: &str = "r.id, r.name, r.category, r.cuisine, r.difficulty, r.total_minutes, \
    r.servings, r.cover_emoji, r.cover_gradient, r.cover_url, r.description, r.calories, \
    r.is_active, r.cook_count, r.avg_rating, \
    (SELECT GROUP_CONCAT(t.name ORDER BY t.sort_order SEPARATOR ',') \
       FROM recipe_tag rt JOIN tag t ON t.id = rt.tag_id WHERE rt.recipe_id = r.id) AS tag_names, \
    (SELECT COUNT(*) FROM member_favorite f WHERE f.recipe_id = r.id) AS favorite_count";

// =====================================================================
//  列表 / 详情
// =====================================================================

/// GET /api/recipes —— 分页列表，支持关键词、分类、菜系、难度、标签、收藏筛选
pub async fn list_recipes(
    state: web::Data<AppState>,
    query: web::Query<RecipeQuery>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let q = query.into_inner();
    let (page, page_size, offset) = normalize_paging(q.page, q.page_size);

    let mut count_qb: QueryBuilder<MySql> =
        QueryBuilder::new("SELECT COUNT(*) FROM recipe r WHERE 1=1");
    apply_filters(&mut count_qb, &q);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    if total == 0 {
        return Ok(ok(Page::<RecipeCard>::empty(page, page_size)));
    }

    let mut qb: QueryBuilder<MySql> = QueryBuilder::new("SELECT ");
    qb.push(CARD_COLUMNS);
    qb.push(" FROM recipe r WHERE 1=1");
    apply_filters(&mut qb, &q);
    qb.push(" ");
    qb.push(sort_fragment(q.sort.as_deref().unwrap_or("rating")));
    qb.push(" LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let list = qb.build_query_as::<RecipeCard>().fetch_all(pool).await?;

    Ok(ok(Page::new(list, total, page, page_size)))
}

#[derive(sqlx::FromRow, serde::Serialize)]
struct Facet {
    name: String,
    count: i64,
}

/// GET /api/recipes/facets —— 分类 / 菜系 / 标签筛选项（含数量）
pub async fn list_facets(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;

    let categories: Vec<Facet> = sqlx::query_as(
        "SELECT category AS name, COUNT(*) AS count FROM recipe \
         WHERE is_active = 1 GROUP BY category ORDER BY count DESC, category ASC",
    )
    .fetch_all(pool)
    .await?;

    let cuisines: Vec<Facet> = sqlx::query_as(
        "SELECT cuisine AS name, COUNT(*) AS count FROM recipe \
         WHERE is_active = 1 GROUP BY cuisine ORDER BY count DESC, cuisine ASC",
    )
    .fetch_all(pool)
    .await?;

    let tags: Vec<Facet> = sqlx::query_as(
        "SELECT t.name AS name, COUNT(r.id) AS count FROM tag t \
         LEFT JOIN recipe_tag rt ON rt.tag_id = t.id \
         LEFT JOIN recipe r ON r.id = rt.recipe_id AND r.is_active = 1 \
         GROUP BY t.id, t.name, t.sort_order ORDER BY t.sort_order ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(ok(serde_json::json!({
        "categories": categories,
        "cuisines": cuisines,
        "tags": tags,
    })))
}

/// GET /api/recipes/random —— 「不知道吃什么」随机推荐
pub async fn random_recipes(
    state: web::Data<AppState>,
    query: web::Query<RandomQuery>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let limit = query.limit.unwrap_or(4).clamp(1, 12);
    let category = opt_filter(&query.category);

    let list: Vec<RecipeCard> = match category {
        Some(c) => {
            let sql = format!(
                "SELECT {CARD_COLUMNS} FROM recipe r \
                 WHERE r.is_active = 1 AND r.category = ? ORDER BY RAND() LIMIT ?"
            );
            sqlx::query_as(&sql).bind(c).bind(limit).fetch_all(pool).await?
        }
        None => {
            let sql = format!(
                "SELECT {CARD_COLUMNS} FROM recipe r \
                 WHERE r.is_active = 1 ORDER BY RAND() LIMIT ?"
            );
            sqlx::query_as(&sql).bind(limit).fetch_all(pool).await?
        }
    };

    Ok(ok(list))
}

/// GET /api/recipes/{id} —— 详情（步骤 + 食材分组 + 标签 + 收藏人）
pub async fn get_recipe(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();
    Ok(ok(build_recipe_detail(pool, id).await?))
}

/// 组装菜谱详情；订单模块也复用此函数
pub async fn build_recipe_detail(pool: &MySqlPool, id: u64) -> Result<RecipeDetail, ApiError> {
    let base: RecipeBase = sqlx::query_as("SELECT * FROM recipe WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("菜谱 {id} 不存在")))?;

    let creator_name: Option<String> = match base.created_by {
        Some(cid) => sqlx::query_scalar("SELECT nickname FROM member WHERE id = ?")
            .bind(cid)
            .fetch_optional(pool)
            .await?,
        None => None,
    };

    let tags: Vec<Tag> = sqlx::query_as(
        "SELECT t.id, t.name, t.color, t.sort_order FROM tag t \
         JOIN recipe_tag rt ON rt.tag_id = t.id WHERE rt.recipe_id = ? ORDER BY t.sort_order",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let steps: Vec<RecipeStep> =
        sqlx::query_as("SELECT * FROM recipe_step WHERE recipe_id = ? ORDER BY step_no")
            .bind(id)
            .fetch_all(pool)
            .await?;

    let rows: Vec<RecipeIngredientRow> = sqlx::query_as(
        "SELECT ri.id, ri.ingredient_id, i.name, i.emoji, i.category, \
                ri.quantity, COALESCE(NULLIF(ri.unit,''), i.unit) AS unit, \
                ri.group_name, ri.is_optional, ri.sort_order \
         FROM recipe_ingredient ri JOIN ingredient i ON i.id = ri.ingredient_id \
         WHERE ri.recipe_id = ? \
         ORDER BY FIELD(ri.group_name,'主料','辅料','调味料','其他'), ri.sort_order, ri.id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(RecipeDetail {
        base,
        creator_name,
        tags,
        steps,
        ingredient_groups: group_ingredients(rows),
        favorite_members: sqlx::query_as(
            "SELECT m.* FROM member_favorite f JOIN member m ON m.id = f.member_id \
             WHERE f.recipe_id = ? ORDER BY m.sort_order",
        )
        .bind(id)
        .fetch_all(pool)
        .await?,
    })
}

/// 把食材行按「主料 → 辅料 → 调味料 → 其他」分组，未知分组追加在后
fn group_ingredients(rows: Vec<RecipeIngredientRow>) -> Vec<IngredientGroup> {
    let mut groups = Vec::new();
    for g in GROUP_ORDER {
        let items: Vec<RecipeIngredientRow> =
            rows.iter().filter(|r| r.group_name == g).cloned().collect();
        if !items.is_empty() {
            groups.push(IngredientGroup {
                group_name: g.to_string(),
                items,
            });
        }
    }

    let mut extras: Vec<String> = rows
        .iter()
        .map(|r| r.group_name.clone())
        .filter(|g| !GROUP_ORDER.contains(&g.as_str()))
        .collect();
    extras.sort();
    extras.dedup();
    for g in extras {
        let items: Vec<RecipeIngredientRow> =
            rows.iter().filter(|r| r.group_name == g).cloned().collect();
        groups.push(IngredientGroup {
            group_name: g,
            items,
        });
    }

    groups
}

// =====================================================================
//  食材 / 标签 / 步骤 写入（事务内，供建档与下单顺手建档复用）
// =====================================================================

/// 按名称查食材，不存在则自动建档
pub async fn resolve_ingredient(
    tx: &mut MySqlConnection,
    name: &str,
    unit: Option<&str>,
) -> Result<(u64, String), ApiError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("食材名不能为空".into()));
    }
    let unit = clean_text(unit);

    let existing: Option<(u64, String)> =
        sqlx::query_as("SELECT id, unit FROM ingredient WHERE name = ? LIMIT 1")
            .bind(name)
            .fetch_optional(&mut *tx)
            .await?;

    if let Some((id, default_unit)) = existing {
        return Ok((id, unit.unwrap_or(default_unit)));
    }

    let final_unit = unit.unwrap_or_else(|| "克".to_string());
    let result = sqlx::query(
        "INSERT INTO ingredient (name, category, unit, emoji, is_staple) \
         VALUES (?, '其他', ?, '🥬', 0)",
    )
    .bind(name)
    .bind(&final_unit)
    .execute(&mut *tx)
    .await?;

    Ok((result.last_insert_id(), final_unit))
}

/// 按名称查标签，不存在则自动建档（颜色从调色板循环取）
pub async fn resolve_tag(
    tx: &mut MySqlConnection,
    name: &str,
) -> Result<u64, ApiError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("标签名不能为空".into()));
    }
    if name.chars().count() > 32 {
        return Err(ApiError::BadRequest("标签名不能超过 32 个字符".into()));
    }

    let existing: Option<u64> = sqlx::query_scalar("SELECT id FROM tag WHERE name = ?")
        .bind(name)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(id) = existing {
        return Ok(id);
    }

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tag")
        .fetch_one(&mut *tx)
        .await?;
    let color = TAG_PALETTE[(count.max(0) as usize) % TAG_PALETTE.len()];

    let result = sqlx::query("INSERT INTO tag (name, color, sort_order) VALUES (?, ?, ?)")
        .bind(name)
        .bind(color)
        .bind((count + 1) as i32)
        .execute(&mut *tx)
        .await?;

    Ok(result.last_insert_id())
}

/// 全量覆盖写入菜谱食材用量
pub async fn replace_ingredients(
    tx: &mut MySqlConnection,
    recipe_id: u64,
    items: &[RecipeIngredientReq],
) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM recipe_ingredient WHERE recipe_id = ?")
        .bind(recipe_id)
        .execute(&mut *tx)
        .await?;

    for (idx, item) in items.iter().enumerate() {
        validate_ingredient_entry(item)?;

        let (ingredient_id, resolved_unit) = match item.ingredient_id {
            Some(id) if id > 0 => {
                let default_unit: Option<String> =
                    sqlx::query_scalar("SELECT COALESCE(NULLIF(unit,''), '克') FROM ingredient WHERE id = ?")
                        .bind(id)
                        .fetch_optional(&mut *tx)
                        .await?;
                let default_unit = default_unit
                    .ok_or_else(|| ApiError::BadRequest(format!("食材 {id} 不存在")))?;
                let unit = clean_text(item.unit.as_deref()).unwrap_or(default_unit);
                (id, unit)
            }
            _ => {
                let name = item.name.as_deref().unwrap_or_default();
                resolve_ingredient(tx, name, item.unit.as_deref()).await?
            }
        };

        let quantity = Decimal::try_from(item.quantity.unwrap_or(0.0)).unwrap_or(Decimal::ZERO);
        let group = clean_text(item.group_name.as_deref())
            .unwrap_or_else(|| if idx == 0 { "主料" } else { "辅料" }.to_string());

        sqlx::query(
            "INSERT INTO recipe_ingredient \
             (recipe_id, ingredient_id, quantity, unit, group_name, is_optional, sort_order) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(recipe_id)
        .bind(ingredient_id)
        .bind(quantity)
        .bind(&resolved_unit)
        .bind(&group)
        .bind(item.is_optional.unwrap_or(false))
        .bind(item.sort_order.unwrap_or((idx + 1) as i32))
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

/// 全量覆盖写入菜谱步骤
pub async fn replace_steps(
    tx: &mut MySqlConnection,
    recipe_id: u64,
    steps: &[StepInput],
) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM recipe_step WHERE recipe_id = ?")
        .bind(recipe_id)
        .execute(&mut *tx)
        .await?;

    let mut step_no = 0u16;
    for step in steps {
        let (content, duration) = step.clone().into_parts();
        let content = content.trim();
        if content.is_empty() {
            continue;
        }
        if content.chars().count() > 1000 {
            return Err(ApiError::BadRequest(format!(
                "第 {} 步说明不能超过 1000 个字符",
                step_no + 1
            )));
        }
        step_no += 1;
        sqlx::query(
            "INSERT INTO recipe_step (recipe_id, step_no, content, duration_sec) VALUES (?, ?, ?, ?)",
        )
        .bind(recipe_id)
        .bind(step_no)
        .bind(content)
        .bind(duration)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

/// 全量覆盖写入菜谱标签
pub async fn replace_tags(
    tx: &mut MySqlConnection,
    recipe_id: u64,
    tags: &[String],
) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM recipe_tag WHERE recipe_id = ?")
        .bind(recipe_id)
        .execute(&mut *tx)
        .await?;

    for t in tags {
        let tag_id = resolve_tag(tx, t).await?;
        sqlx::query("INSERT IGNORE INTO recipe_tag (recipe_id, tag_id) VALUES (?, ?)")
            .bind(recipe_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
    }
    Ok(())
}

// =====================================================================
//  建档 / 更新 / 删除 / 收藏
// =====================================================================

/// 建档核心逻辑，供 `POST /api/recipes` 与「下单顺手建档」共用。
/// 必须在调用方已开启的事务中执行。
pub async fn insert_recipe_in_tx(
    tx: &mut MySqlConnection,
    req: &CreateRecipeReq,
) -> Result<u64, ApiError> {
    let (
        name,
        category,
        cuisine,
        difficulty,
        prep_minutes,
        cook_minutes,
        servings,
        cover_emoji,
        cover_gradient,
    ) = validate_recipe_fields(req)?;

    let name = name.ok_or_else(|| ApiError::BadRequest("菜名不能为空".into()))?;

    for (label, value) in [
        ("简介", req.description.as_deref()),
        ("小贴士", req.tips.as_deref()),
    ] {
        if let Some(v) = value {
            if v.chars().count() > 1000 {
                return Err(ApiError::BadRequest(format!("{label}不能超过 1000 个字符")));
            }
        }
    }
    if let Some(u) = clean_text(req.cover_url.as_deref()) {
        if u.chars().count() > 512 {
            return Err(ApiError::BadRequest("封面图片地址过长".into()));
        }
        if !(u.starts_with("http://") || u.starts_with("https://") || u.starts_with('/')) {
            return Err(ApiError::BadRequest(
                "封面图片地址需以 http:// 、https:// 或 / 开头".into(),
            ));
        }
    }

    let result = sqlx::query(
        "INSERT INTO recipe \
         (name, category, cuisine, difficulty, prep_minutes, cook_minutes, servings, \
          cover_emoji, cover_gradient, cover_url, description, tips, calories, is_active, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&name)
    .bind(&category)
    .bind(&cuisine)
    .bind(&difficulty)
    .bind(prep_minutes)
    .bind(cook_minutes)
    .bind(servings)
    .bind(&cover_emoji)
    .bind(&cover_gradient)
    .bind(clean_text(req.cover_url.as_deref()))
    .bind(clean_text(req.description.as_deref()))
    .bind(clean_text(req.tips.as_deref()))
    .bind(req.calories)
    .bind(req.is_active.unwrap_or(true))
    .bind(req.created_by)
    .execute(&mut *tx)
    .await?;

    let id = result.last_insert_id();

    if let Some(tags) = req.tags.as_ref() {
        replace_tags(tx, id, tags).await?;
    }
    if let Some(ings) = req.ingredients.as_ref() {
        replace_ingredients(tx, id, ings).await?;
    }
    if let Some(steps) = req.steps.as_ref() {
        replace_steps(tx, id, steps).await?;
    }

    Ok(id)
}

/// POST /api/recipes
pub async fn create_recipe(
    state: web::Data<AppState>,
    body: web::Json<CreateRecipeReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let req = body.into_inner();

    let mut tx = pool.begin().await?;
    let id = insert_recipe_in_tx(&mut *tx, &req).await?;
    tx.commit().await?;

    Ok(ok_with_message(
        "菜谱已建档",
        build_recipe_detail(pool, id).await?,
    ))
}

/// PUT /api/recipes/{id}
///
/// 注意：基础字段（分类、菜系、难度、时长、份数、封面）为全量覆盖；
/// 可选文本字段（简介、小贴士、封面图）传空串或不传即清空。
/// 前端编辑表单总是先拉详情再整体提交，因此不会出现字段意外丢失。
/// `tags` / `ingredients` / `steps` 三个数组仅在请求中出现时才覆盖，
/// 未出现则保持原样，便于「只改个名字」这类轻量更新。
pub async fn update_recipe(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<CreateRecipeReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();
    let req = body.into_inner();

    let (
        name,
        category,
        cuisine,
        difficulty,
        prep_minutes,
        cook_minutes,
        servings,
        cover_emoji,
        cover_gradient,
    ) = validate_recipe_fields(&req)?;

    let mut tx = pool.begin().await?;

    if !exists_in_tx(&mut *tx, "SELECT COUNT(*) FROM recipe WHERE id = ?", id).await? {
        return Err(ApiError::NotFound(format!("菜谱 {id} 不存在")));
    }

    sqlx::query(
        "UPDATE recipe SET \
            name = COALESCE(?, name), category = ?, cuisine = ?, difficulty = ?, \
            prep_minutes = ?, cook_minutes = ?, servings = ?, \
            cover_emoji = ?, cover_gradient = ?, cover_url = ?, description = ?, \
            tips = ?, calories = ?, is_active = COALESCE(?, is_active), \
            created_by = COALESCE(created_by, ?) \
         WHERE id = ?",
    )
    .bind(name.as_deref())
    .bind(&category)
    .bind(&cuisine)
    .bind(&difficulty)
    .bind(prep_minutes)
    .bind(cook_minutes)
    .bind(servings)
    .bind(&cover_emoji)
    .bind(&cover_gradient)
    .bind(clean_text(req.cover_url.as_deref()))
    .bind(clean_text(req.description.as_deref()))
    .bind(clean_text(req.tips.as_deref()))
    .bind(req.calories)
    .bind(req.is_active)
    .bind(req.created_by)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if let Some(tags) = req.tags.as_ref() {
        replace_tags(&mut *tx, id, tags).await?;
    }
    if let Some(ings) = req.ingredients.as_ref() {
        replace_ingredients(&mut *tx, id, ings).await?;
    }
    if let Some(steps) = req.steps.as_ref() {
        replace_steps(&mut *tx, id, steps).await?;
    }

    tx.commit().await?;
    Ok(ok_with_message(
        "菜谱已更新",
        build_recipe_detail(pool, id).await?,
    ))
}

/// DELETE /api/recipes/{id}?permanent=true|false
pub async fn delete_recipe(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    query: web::Query<DeleteQuery>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();

    if !exists(pool, "SELECT COUNT(*) FROM recipe WHERE id = ?", id).await? {
        return Err(ApiError::NotFound(format!("菜谱 {id} 不存在")));
    }

    if query.permanent.unwrap_or(false) {
        // order_item.recipe_id 为 ON DELETE SET NULL，历史订单仍保留 recipe_snapshot
        sqlx::query("DELETE FROM recipe WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(ok_with_message(
            "菜谱已彻底删除",
            serde_json::json!({ "id": id }),
        ))
    } else {
        sqlx::query("UPDATE recipe SET is_active = 0 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(ok_with_message(
            "菜谱已下架，历史订单不受影响",
            serde_json::json!({ "id": id }),
        ))
    }
}

/// POST /api/recipes/{id}/favorite/{member_id} —— 切换收藏状态
pub async fn toggle_favorite(
    state: web::Data<AppState>,
    path: web::Path<(u64, u64)>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let (recipe_id, member_id) = path.into_inner();

    if !exists(pool, "SELECT COUNT(*) FROM recipe WHERE id = ?", recipe_id).await? {
        return Err(ApiError::NotFound(format!("菜谱 {recipe_id} 不存在")));
    }
    if !exists(pool, "SELECT COUNT(*) FROM member WHERE id = ?", member_id).await? {
        return Err(ApiError::NotFound(format!("成员 {member_id} 不存在")));
    }

    let (fav_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM member_favorite WHERE recipe_id = ? AND member_id = ?",
    )
    .bind(recipe_id)
    .bind(member_id)
    .fetch_one(pool)
    .await?;
    let already = fav_count > 0;

    let favorited = if already {
        sqlx::query("DELETE FROM member_favorite WHERE recipe_id = ? AND member_id = ?")
            .bind(recipe_id)
            .bind(member_id)
            .execute(pool)
            .await?;
        false
    } else {
        sqlx::query("INSERT INTO member_favorite (member_id, recipe_id) VALUES (?, ?)")
            .bind(member_id)
            .bind(recipe_id)
            .execute(pool)
            .await?;
        true
    };

    Ok(ok(serde_json::json!({
        "recipe_id": recipe_id,
        "member_id": member_id,
        "favorited": favorited,
    })))
}
