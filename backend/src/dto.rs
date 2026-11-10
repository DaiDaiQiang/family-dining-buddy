//! 请求体 DTO 与参数校验。

use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;

use crate::error::ApiError;
use crate::util::{
    require_text, validate_enum, validate_range, CATEGORIES, CUISINES, DIFFICULTIES,
    GENDERS, INGREDIENT_GROUPS, MEAL_TYPES, MEMBER_ROLES, ORDER_STATUS,
};

// =====================================================================
//  成员
// =====================================================================

/// 区分「字段缺失（不修改）」与「显式 null（清空）」的双层 Option 反序列化
fn de_some_option<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(d)?))
}

#[derive(Debug, Deserialize)]
pub struct CreateMemberReq {
    pub nickname: Option<String>,
    pub avatar_emoji: Option<String>,
    pub avatar_url: Option<String>,
    pub theme_color: Option<String>,
    pub role: Option<String>,
    pub taste_note: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub gender: Option<String>,
    pub dislikes: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberReq {
    pub nickname: Option<String>,
    pub avatar_emoji: Option<String>,
    /// None=不修改 / Some(None)=清空（回落 emoji 头像）/ Some(Some(url))=设置
    #[serde(default, deserialize_with = "de_some_option")]
    pub avatar_url: Option<Option<String>>,
    pub theme_color: Option<String>,
    pub role: Option<String>,
    pub taste_note: Option<String>,
    /// None=不修改 / Some(None)=清空 / Some(Some(d))=设置
    #[serde(default, deserialize_with = "de_some_option")]
    pub birthday: Option<Option<NaiveDate>>,
    #[serde(default, deserialize_with = "de_some_option")]
    pub gender: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_some_option")]
    pub dislikes: Option<Option<String>>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
}

// =====================================================================
//  食材
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct CreateIngredientReq {
    pub name: Option<String>,
    pub category: Option<String>,
    pub unit: Option<String>,
    pub emoji: Option<String>,
    pub is_staple: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIngredientReq {
    pub name: Option<String>,
    pub category: Option<String>,
    pub unit: Option<String>,
    pub emoji: Option<String>,
    pub is_staple: Option<bool>,
    pub is_active: Option<bool>,
}

// =====================================================================
//  标签
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct CreateTagReq {
    pub name: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i32>,
}

// =====================================================================
//  菜谱
// =====================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeIngredientReq {
    pub ingredient_id: Option<u64>,
    /// 直接给食材名时自动建档（家庭场景常见：临时想到一种新食材）
    pub name: Option<String>,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub group_name: Option<String>,
    pub is_optional: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeStepReq {
    pub content: Option<String>,
    pub duration_sec: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRecipeReq {
    pub name: Option<String>,
    pub category: Option<String>,
    pub cuisine: Option<String>,
    pub difficulty: Option<String>,
    pub prep_minutes: Option<u16>,
    pub cook_minutes: Option<u16>,
    pub servings: Option<u8>,
    pub cover_emoji: Option<String>,
    pub cover_gradient: Option<String>,
    pub cover_url: Option<String>,
    pub description: Option<String>,
    pub tips: Option<String>,
    pub calories: Option<u16>,
    pub is_active: Option<bool>,
    pub created_by: Option<u64>,
    /// 标签名数组，不存在则自动创建
    pub tags: Option<Vec<String>>,
    /// 食材用量数组
    pub ingredients: Option<Vec<RecipeIngredientReq>>,
    /// 做法步骤：可传字符串数组，也可传对象数组
    pub steps: Option<Vec<StepInput>>,
}

/// 步骤既支持 `"切块"` 也支持 `{ "content": "切块", "duration_sec": 60 }`
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StepInput {
    Text(String),
    Object(RecipeStepReq),
}

impl StepInput {
    pub fn into_parts(self) -> (String, Option<u16>) {
        match self {
            StepInput::Text(t) => (t, None),
            StepInput::Object(o) => (o.content.unwrap_or_default(), o.duration_sec),
        }
    }
}

/// 校验菜谱写入参数，返回归一化后的字段元组。
/// 创建与更新共用，未提供的字段在更新时保持原值由 SQL 层用 COALESCE 处理。
#[allow(clippy::type_complexity)]
pub fn validate_recipe_fields(
    req: &CreateRecipeReq,
) -> Result<
    (
        Option<String>,
        String,
        String,
        String,
        u16,
        u16,
        u8,
        String,
        String,
    ),
    ApiError,
> {
    let name = match req.name.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(n) if n.chars().count() > 64 => {
            return Err(ApiError::BadRequest("菜名不能超过 64 个字符".into()))
        }
        Some(n) => Some(n.to_string()),
        None => None,
    };

    let category = req
        .category
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("家常菜")
        .to_string();
    validate_enum(&category, &CATEGORIES, "分类")?;

    let cuisine = req
        .cuisine
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("中餐")
        .to_string();
    validate_enum(&cuisine, &CUISINES, "菜系")?;

    let difficulty = req
        .difficulty
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("easy")
        .to_string();
    validate_enum(&difficulty, &DIFFICULTIES, "难度")?;

    let prep_minutes = req.prep_minutes.unwrap_or(10);
    validate_range(prep_minutes, 0, 600, "备菜时长")?;

    let cook_minutes = req.cook_minutes.unwrap_or(15);
    validate_range(cook_minutes, 0, 600, "烹饪时长")?;

    let servings = req.servings.unwrap_or(2);
    validate_range(servings, 1u8, 20u8, "份数")?;

    let cover_emoji = req
        .cover_emoji
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("🍲")
        .to_string();

    let cover_gradient = match req.cover_gradient.as_deref().map(str::trim).filter(|s| !s.is_empty())
    {
        Some(g) => {
            if g.chars().count() > 64 || !g.starts_with("linear-gradient") {
                return Err(ApiError::BadRequest(
                    "封面渐变需以 linear-gradient 开头，例如 linear-gradient(135deg,#FF9A8B,#FF6A88)".into(),
                ));
            }
            g.to_string()
        }
        None => "linear-gradient(135deg,#FF9A8B,#FF6A88)".to_string(),
    };

    Ok((
        name,
        category,
        cuisine,
        difficulty,
        prep_minutes,
        cook_minutes,
        servings,
        cover_emoji,
        cover_gradient,
    ))
}

/// 校验单条食材用量
pub fn validate_ingredient_entry(item: &RecipeIngredientReq) -> Result<(), ApiError> {
    if item.ingredient_id.is_none() {
        require_text(&item.name, "食材名", 64)?;
    }
    if let Some(q) = item.quantity {
        if !(0.0..=100_000.0).contains(&q) {
            return Err(ApiError::BadRequest(format!(
                "食材用量需在 0 ~ 100000 之间，当前为 {q}"
            )));
        }
    }
    if let Some(g) = item.group_name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        validate_enum(g, &INGREDIENT_GROUPS, "食材分组")?;
    }
    Ok(())
}

// =====================================================================
//  订单
// =====================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct OrderItemReq {
    pub recipe_id: Option<u64>,
    pub portions: Option<u8>,
    pub member_id: Option<u64>,
    pub note: Option<String>,
    /// 点餐时顺手建档：传入完整菜谱定义，与 recipe_id 二选一。
    /// 系统会在同一事务内先建菜谱再加入订单，前端无需两次请求。
    pub new_recipe: Option<Box<CreateRecipeReq>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderReq {
    pub member_id: Option<u64>,
    /// 缺省为今天
    pub meal_date: Option<NaiveDate>,
    pub meal_type: Option<String>,
    pub diners: Option<u8>,
    pub note: Option<String>,
    /// true 时创建后直接确认并生成采购清单
    pub auto_confirm: Option<bool>,
    pub items: Option<Vec<OrderItemReq>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrderReq {
    pub meal_date: Option<NaiveDate>,
    pub meal_type: Option<String>,
    pub status: Option<String>,
    pub diners: Option<u8>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrderItemReq {
    pub portions: Option<u8>,
    pub rating: Option<u8>,
    pub item_status: Option<String>,
    pub note: Option<String>,
    pub member_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateShoppingItemReq {
    pub is_purchased: Option<bool>,
}

pub const ITEM_STATUS: [&str; 4] = ["pending", "cooking", "done", "skipped"];

/// 校验订单创建参数
pub fn validate_order(req: &CreateOrderReq) -> Result<(NaiveDate, String, u8), ApiError> {
    let meal_date = req.meal_date.unwrap_or_else(|| chrono::Local::now().date_naive());

    let meal_type = req
        .meal_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("dinner")
        .to_string();
    validate_enum(&meal_type, &MEAL_TYPES, "餐次")?;

    let diners = req.diners.unwrap_or(2);
    validate_range(diners, 1u8, 30u8, "就餐人数")?;

    if let Some(n) = req.note.as_deref() {
        if n.chars().count() > 512 {
            return Err(ApiError::BadRequest("订单备注不能超过 512 个字符".into()));
        }
    }

    let items = req.items.as_ref().filter(|v| !v.is_empty()).ok_or_else(|| {
        ApiError::BadRequest("至少要选一道菜才能下单".into())
    })?;

    for it in items {
        match (it.recipe_id, it.new_recipe.as_ref()) {
            (Some(0), _) => {
                return Err(ApiError::BadRequest("recipe_id 不能为 0".into()))
            }
            (Some(_), _) => {}
            (None, Some(new)) => {
                // 点餐时顺手建档：菜名必填，其余走菜谱校验
                require_text(&new.name, "新建菜谱的菜名", 64)?;
            }
            (None, None) => {
                return Err(ApiError::BadRequest(
                    "订单明细需提供 recipe_id，或提供 new_recipe 以便顺手建档".into(),
                ))
            }
        }
        if let Some(p) = it.portions {
            validate_range(p, 1u8, 20u8, "份数")?;
        }
        if let Some(n) = it.note.as_deref() {
            if n.chars().count() > 255 {
                return Err(ApiError::BadRequest("单项备注不能超过 255 个字符".into()));
            }
        }
    }

    Ok((meal_date, meal_type, diners))
}

/// 校验订单状态流转
pub fn validate_status(status: &str) -> Result<(), ApiError> {
    validate_enum(status, &ORDER_STATUS, "订单状态")
}

/// 校验菜品出菜状态
pub fn validate_item_status(status: &str) -> Result<(), ApiError> {
    validate_enum(status, &ITEM_STATUS, "出菜状态")
}

/// 校验成员角色
pub fn validate_role(role: &str) -> Result<(), ApiError> {
    validate_enum(role, &MEMBER_ROLES, "家庭角色")
}

pub fn validate_gender(gender: &str) -> Result<(), ApiError> {
    validate_enum(gender, &GENDERS, "性别")
}

// =====================================================================
//  阶段 1 · 账本 / 快递 / 健康 / 同步
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct CreateLedgerReq {
    /// 缺省取当前登录人
    pub member_id: Option<u64>,
    /// income / expense
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub category: Option<String>,
    pub amount: f64,
    /// 缺省今天
    pub entry_date: Option<NaiveDate>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLedgerReq {
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub category: Option<String>,
    pub amount: Option<f64>,
    pub entry_date: Option<NaiveDate>,
    pub note: Option<String>,
}

pub const LEDGER_TYPES: [&str; 2] = ["income", "expense"];
/// 与前端 stores/ledger.js 的 LEDGER_CATEGORIES 取值保持一致
pub const LEDGER_CATEGORIES: [&str; 12] = [
    "grocery",
    "shopping",
    "food_out",
    "daily",
    "transport",
    "medical",
    "child",
    "housing",
    "entertain",
    "salary",
    "bonus",
    "other",
];

#[derive(Debug, Deserialize)]
pub struct CreateParcelReq {
    pub pickup_code: String,
    pub courier_code: Option<String>,
    pub courier_name: Option<String>,
    pub station_type: Option<String>,
    pub station: Option<String>,
    pub shelf_no: Option<String>,
    pub tracking_no: Option<String>,
    pub note: Option<String>,
    pub recipient_member_id: Option<u64>,
    pub recipient_name: Option<String>,
    /// 缺省今天
    pub arrived_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateParcelReq {
    pub pickup_code: Option<String>,
    pub courier_code: Option<String>,
    pub courier_name: Option<String>,
    pub station_type: Option<String>,
    pub station: Option<String>,
    pub shelf_no: Option<String>,
    pub tracking_no: Option<String>,
    pub note: Option<String>,
    pub recipient_member_id: Option<u64>,
    pub recipient_name: Option<String>,
    pub arrived_date: Option<NaiveDate>,
    /// pending / picked
    pub status: Option<String>,
}

pub const PARCEL_STATUS: [&str; 2] = ["pending", "picked"];

#[derive(Debug, Deserialize)]
pub struct SaveHealthReq {
    pub gender: Option<String>,
    pub last_period_start: Option<NaiveDate>,
    pub cycle_length: Option<u8>,
    pub period_length: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct PutSyncReq {
    /// 整包 JSON（学习模块等）
    pub payload: serde_json::Value,
    /// 乐观版本号：与库中不一致时拒绝覆盖（可省略，省略则直接覆盖）
    pub expected_version: Option<u32>,
}

// =====================================================================
//  阶段 3 · 耗材维护与消耗统计
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct CreateConsumableReq {
    pub name: Option<String>,
    pub category: Option<String>,
    pub unit: Option<String>,
    pub emoji: Option<String>,
    pub stock: Option<f64>,
    pub low_threshold: Option<f64>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConsumableReq {
    pub name: Option<String>,
    pub category: Option<String>,
    pub unit: Option<String>,
    pub emoji: Option<String>,
    pub stock: Option<f64>,
    pub low_threshold: Option<f64>,
    pub note: Option<String>,
    pub is_active: Option<bool>,
}

/// 消耗登记：consumable_id 或 name 二选一（name 用于快捷补记时自动匹配）
#[derive(Debug, Deserialize)]
pub struct AddConsumptionReq {
    pub consumable_id: Option<u64>,
    pub name: Option<String>,
    /// 本次消耗量，正数
    pub quantity: f64,
    pub member_id: Option<u64>,
    pub note: Option<String>,
    /// 缺省为当前时间
    pub occurred_at: Option<NaiveDateTime>,
}

/// 耗材分类取值（与 util 保持一致）
pub const CONSUMABLE_CATEGORIES: [&str; 5] = ["清洁", "纸品", "调味料", "个护", "其他"];

