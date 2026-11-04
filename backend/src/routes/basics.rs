//! 成员、食材、标签三个基础字典模块。

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::db::AppState;
use crate::dto::{
    validate_gender, validate_role, CreateIngredientReq, CreateMemberReq, CreateTagReq,
    UpdateIngredientReq, UpdateMemberReq,
};
use crate::error::ApiError;
use crate::models::*;
use crate::util::{
    normalize_paging, ok, ok_with_message, require_text, validate_color, Page,
};

fn clean(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

// =====================================================================
//  成员
// =====================================================================

/// GET /api/members?include_inactive=true
pub async fn list_members(
    state: web::Data<AppState>,
    query: web::Query<MemberListQuery>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let sql = if query.include_inactive == Some(true) {
        "SELECT * FROM member ORDER BY sort_order, id"
    } else {
        "SELECT * FROM member WHERE is_active = 1 ORDER BY sort_order, id"
    };
    let list: Vec<Member> = sqlx::query_as(sql).fetch_all(pool).await?;
    Ok(ok(list))
}

#[derive(Debug, Deserialize)]
pub struct MemberListQuery {
    pub include_inactive: Option<bool>,
}

/// GET /api/members/{id}
pub async fn get_member(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();
    let member: Member = sqlx::query_as("SELECT * FROM member WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("成员 {id} 不存在")))?;
    Ok(ok(member))
}

/// POST /api/members
pub async fn create_member(
    state: web::Data<AppState>,
    body: web::Json<CreateMemberReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let req = body.into_inner();

    let nickname = require_text(&req.nickname, "昵称", 32)?;
    let avatar = clean(req.avatar_emoji.as_deref()).unwrap_or_else(|| "🙂".into());
    let color = match clean(req.theme_color.as_deref()) {
        Some(c) => validate_color(&c, "主题色")?,
        None => "#FF8A65".into(),
    };
    let role = clean(req.role.as_deref()).unwrap_or_else(|| "member".into());
    validate_role(&role)?;
    if let Some(g) = clean(req.gender.as_deref()) {
        validate_gender(&g)?;
    }

    let result = sqlx::query(
        "INSERT INTO member (nickname, avatar_emoji, avatar_url, theme_color, role, taste_note, birthday, gender, dislikes, sort_order) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&nickname)
    .bind(&avatar)
    .bind(clean(req.avatar_url.as_deref()))
    .bind(&color)
    .bind(&role)
    .bind(clean(req.taste_note.as_deref()))
    .bind(req.birthday)
    .bind(clean(req.gender.as_deref()))
    .bind(clean(req.dislikes.as_deref()))
    .bind(req.sort_order.unwrap_or(0))
    .execute(pool)
    .await?;

    let id = result.last_insert_id();
    let member: Member = sqlx::query_as("SELECT * FROM member WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(ok_with_message("成员已添加", member))
}

/// PUT /api/members/{id}
pub async fn update_member(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<UpdateMemberReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();
    let req = body.into_inner();

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM member WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    if count == 0 {
        return Err(ApiError::NotFound(format!("成员 {id} 不存在")));
    }

    if let Some(role) = clean(req.role.as_deref()) {
        validate_role(&role)?;
    }
    if let Some(Some(g)) = req.gender.as_ref() {
        validate_gender(g)?;
    }
    let color = req
        .theme_color
        .as_deref()
        .map(|c| validate_color(c, "主题色"))
        .transpose()?;

    // 生日/性别/忌口/头像图：? = 1 时取新值（可为 NULL 清空），否则保留原值
    let birthday_set = req.birthday.is_some();
    let gender_set = req.gender.is_some();
    let dislikes_set = req.dislikes.is_some();
    let avatar_url_set = req.avatar_url.is_some();

    sqlx::query(
        "UPDATE member SET \
            nickname = COALESCE(?, nickname), \
            avatar_emoji = COALESCE(?, avatar_emoji), \
            avatar_url = IF(? = 1, ?, avatar_url), \
            theme_color = COALESCE(?, theme_color), \
            role = COALESCE(?, role), \
            taste_note = IF(? IS NULL, taste_note, ?), \
            birthday = IF(? = 1, ?, birthday), \
            gender = IF(? = 1, ?, gender), \
            dislikes = IF(? = 1, ?, dislikes), \
            sort_order = COALESCE(?, sort_order), \
            is_active = COALESCE(?, is_active) \
         WHERE id = ?",
    )
    .bind(clean(req.nickname.as_deref()))
    .bind(clean(req.avatar_emoji.as_deref()))
    .bind(avatar_url_set)
    .bind(req.avatar_url.flatten().and_then(|u| clean(Some(&u))))
    .bind(color)
    .bind(clean(req.role.as_deref()))
    .bind(req.taste_note.as_deref())
    .bind(req.taste_note.as_deref())
    .bind(birthday_set)
    .bind(req.birthday.flatten())
    .bind(gender_set)
    .bind(req.gender.flatten().and_then(|g| clean(Some(&g))))
    .bind(dislikes_set)
    .bind(req.dislikes.flatten().and_then(|d| clean(Some(&d))))
    .bind(req.sort_order)
    .bind(req.is_active)
    .bind(id)
    .execute(pool)
    .await?;

    let member: Member = sqlx::query_as("SELECT * FROM member WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(ok_with_message("成员已更新", member))
}

/// DELETE /api/members/{id} —— 停用（有历史订单的成员不允许物理删除）
pub async fn delete_member(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();

    let (order_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM meal_order WHERE member_id = ?")
            .bind(id)
            .fetch_one(pool)
            .await?;

    if order_count > 0 {
        // 存在历史订单，只能停用，避免订单外键悬空
        sqlx::query("UPDATE member SET is_active = 0 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        return Ok(ok_with_message(
            "该成员有历史点餐记录，已停用而非删除",
            serde_json::json!({ "id": id, "disabled": true }),
        ));
    }

    let result = sqlx::query("DELETE FROM member WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("成员 {id} 不存在")));
    }
    Ok(ok_with_message(
        "成员已删除",
        serde_json::json!({ "id": id, "disabled": false }),
    ))
}

// =====================================================================
//  食材
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct IngredientQuery {
    pub keyword: Option<String>,
    pub category: Option<String>,
    /// only_staple=true 只看常备；no_staple=true 只看需采购
    pub only_staple: Option<bool>,
    pub no_staple: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// GET /api/ingredients
pub async fn list_ingredients(
    state: web::Data<AppState>,
    query: web::Query<IngredientQuery>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let q = query.into_inner();
    let (page, page_size, offset) = normalize_paging(q.page, q.page_size);

    let mut where_sql = String::from(" WHERE is_active = 1");
    let mut binds: Vec<String> = Vec::new();

    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        where_sql.push_str(" AND name LIKE ?");
        binds.push(format!("%{kw}%"));
    }
    if let Some(c) = q
        .category
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "全部")
    {
        where_sql.push_str(" AND category = ?");
        binds.push(c.to_string());
    }
    if q.only_staple == Some(true) {
        where_sql.push_str(" AND is_staple = 1");
    } else if q.no_staple == Some(true) {
        where_sql.push_str(" AND is_staple = 0");
    }

    let count_sql = format!("SELECT COUNT(*) FROM ingredient{where_sql}");
    let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
    for b in &binds {
        cq = cq.bind(b);
    }
    let (total,): (i64,) = cq.fetch_one(pool).await?;

    if total == 0 {
        return Ok(ok(Page::<Ingredient>::empty(page, page_size)));
    }

    let list_sql = format!(
        "SELECT * FROM ingredient{where_sql} ORDER BY FIELD(category,'蔬菜','肉禽','水产','蛋奶','豆制品','主食','干货','乳品','水果','烘焙','调味','饮品','其他'), category, name LIMIT ? OFFSET ?"
    );
    let mut lq = sqlx::query_as::<_, Ingredient>(&list_sql);
    for b in &binds {
        lq = lq.bind(b);
    }
    let list = lq.bind(page_size).bind(offset).fetch_all(pool).await?;

    Ok(ok(Page::new(list, total, page, page_size)))
}

/// GET /api/ingredients/categories —— 食材分类（含数量）
pub async fn list_ingredient_categories(
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;

    #[derive(sqlx::FromRow, serde::Serialize)]
    struct Row {
        name: String,
        count: i64,
    }
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT category AS name, COUNT(*) AS count FROM ingredient WHERE is_active = 1 \
         GROUP BY category ORDER BY count DESC, category ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(ok(rows))
}

/// POST /api/ingredients
pub async fn create_ingredient(
    state: web::Data<AppState>,
    body: web::Json<CreateIngredientReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let req = body.into_inner();

    let name = require_text(&req.name, "食材名", 64)?;
    let category = clean(req.category.as_deref()).unwrap_or_else(|| "其他".into());
    let unit = clean(req.unit.as_deref()).unwrap_or_else(|| "克".into());
    let emoji = clean(req.emoji.as_deref()).unwrap_or_else(|| "🥬".into());

    let result = sqlx::query(
        "INSERT INTO ingredient (name, category, unit, emoji, is_staple) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&name)
    .bind(&category)
    .bind(&unit)
    .bind(&emoji)
    .bind(req.is_staple.unwrap_or(false))
    .execute(pool)
    .await?;

    let id = result.last_insert_id();
    let ing: Ingredient = sqlx::query_as("SELECT * FROM ingredient WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(ok_with_message("食材已添加", ing))
}

/// PUT /api/ingredients/{id}
pub async fn update_ingredient(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<UpdateIngredientReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();
    let req = body.into_inner();

    sqlx::query(
        "UPDATE ingredient SET \
            name = COALESCE(?, name), category = COALESCE(?, category), \
            unit = COALESCE(?, unit), emoji = COALESCE(?, emoji), \
            is_staple = COALESCE(?, is_staple), is_active = COALESCE(?, is_active) \
         WHERE id = ?",
    )
    .bind(clean(req.name.as_deref()))
    .bind(clean(req.category.as_deref()))
    .bind(clean(req.unit.as_deref()))
    .bind(clean(req.emoji.as_deref()))
    .bind(req.is_staple)
    .bind(req.is_active)
    .bind(id)
    .execute(pool)
    .await?;

    let ing: Ingredient = sqlx::query_as("SELECT * FROM ingredient WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("食材 {id} 不存在")))?;
    Ok(ok_with_message("食材已更新", ing))
}

/// DELETE /api/ingredients/{id}
/// 被菜谱引用时改为停用（recipe_ingredient 外键为 RESTRICT）
pub async fn delete_ingredient(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();

    let (ref_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM recipe_ingredient WHERE ingredient_id = ?")
            .bind(id)
            .fetch_one(pool)
            .await?;

    if ref_count > 0 {
        sqlx::query("UPDATE ingredient SET is_active = 0 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        return Ok(ok_with_message(
            format!("该食材被 {ref_count} 处菜谱使用，已停用而非删除"),
            serde_json::json!({ "id": id, "disabled": true }),
        ));
    }

    let result = sqlx::query("DELETE FROM ingredient WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("食材 {id} 不存在")));
    }
    Ok(ok_with_message(
        "食材已删除",
        serde_json::json!({ "id": id, "disabled": false }),
    ))
}

// =====================================================================
//  标签
// =====================================================================

/// GET /api/tags
pub async fn list_tags(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;

    #[derive(sqlx::FromRow, serde::Serialize)]
    struct TagWithCount {
        id: u64,
        name: String,
        color: String,
        sort_order: i32,
        recipe_count: i64,
    }

    let tags: Vec<TagWithCount> = sqlx::query_as(
        "SELECT t.id, t.name, t.color, t.sort_order, \
                (SELECT COUNT(*) FROM recipe_tag rt JOIN recipe r ON r.id = rt.recipe_id \
                  WHERE rt.tag_id = t.id AND r.is_active = 1) AS recipe_count \
         FROM tag t ORDER BY t.sort_order, t.id",
    )
    .fetch_all(pool)
    .await?;

    Ok(ok(tags))
}

/// POST /api/tags
pub async fn create_tag(
    state: web::Data<AppState>,
    body: web::Json<CreateTagReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let req = body.into_inner();

    let name = require_text(&req.name, "标签名", 32)?;
    let color = match clean(req.color.as_deref()) {
        Some(c) => validate_color(&c, "标签颜色")?,
        None => "#FF7043".into(),
    };

    let result = sqlx::query("INSERT INTO tag (name, color, sort_order) VALUES (?, ?, ?)")
        .bind(&name)
        .bind(&color)
        .bind(req.sort_order.unwrap_or(0))
        .execute(pool)
        .await?;

    let id = result.last_insert_id();
    let tag: Tag = sqlx::query_as("SELECT * FROM tag WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(ok_with_message("标签已添加", tag))
}

/// DELETE /api/tags/{id}
pub async fn delete_tag(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let id = path.into_inner();

    let result = sqlx::query("DELETE FROM tag WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("标签 {id} 不存在")));
    }
    Ok(ok_with_message(
        "标签已删除",
        serde_json::json!({ "id": id }),
    ))
}
