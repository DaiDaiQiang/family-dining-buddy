//! 数据库行模型与 API 响应结构。
//!
//! 类型与 `01_schema.sql` 严格对应：
//! - `BIGINT UNSIGNED` → `u64`，`INT UNSIGNED` → `u32`
//! - `SMALLINT UNSIGNED` → `u16`，`TINYINT UNSIGNED` → `u8`
//! - `TINYINT(1)` → `bool`，`DECIMAL` → `rust_decimal::Decimal`
//! - `DATETIME` → `NaiveDateTime`，`DATE` → `NaiveDate`，`ENUM` → `String`

use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::FromRow;

// =====================================================================
//  成员
// =====================================================================

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Member {
    pub id: u64,
    pub nickname: String,
    pub avatar_emoji: String,
    /// 头像图片地址（/api/uploads/xxx），为空时用 avatar_emoji
    pub avatar_url: Option<String>,
    pub theme_color: String,
    pub role: String,
    pub taste_note: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub gender: Option<String>,
    pub dislikes: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// =====================================================================
//  食材与标签
// =====================================================================

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Ingredient {
    pub id: u64,
    pub name: String,
    pub category: String,
    pub unit: String,
    pub emoji: String,
    pub is_staple: bool,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Tag {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub sort_order: i32,
}

// =====================================================================
//  菜谱
// =====================================================================

/// 列表卡片（不含步骤与食材明细）
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RecipeCard {
    pub id: u64,
    pub name: String,
    pub category: String,
    pub cuisine: String,
    pub difficulty: String,
    pub total_minutes: u16,
    pub servings: u8,
    pub cover_emoji: String,
    pub cover_gradient: String,
    pub cover_url: Option<String>,
    pub description: Option<String>,
    pub calories: Option<u16>,
    pub is_active: bool,
    pub cook_count: u32,
    pub avg_rating: Decimal,
    pub tag_names: Option<String>,
    pub favorite_count: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RecipeStep {
    pub id: u64,
    pub recipe_id: u64,
    pub step_no: u16,
    pub content: String,
    pub duration_sec: Option<u16>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RecipeIngredientRow {
    pub id: u64,
    pub ingredient_id: u64,
    pub name: String,
    pub emoji: String,
    pub category: String,
    pub quantity: Decimal,
    pub unit: String,
    pub group_name: String,
    pub is_optional: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngredientGroup {
    pub group_name: String,
    pub items: Vec<RecipeIngredientRow>,
}

/// 菜谱详情
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RecipeBase {
    pub id: u64,
    pub name: String,
    pub category: String,
    pub cuisine: String,
    pub difficulty: String,
    pub prep_minutes: u16,
    pub cook_minutes: u16,
    pub total_minutes: u16,
    pub servings: u8,
    pub cover_emoji: String,
    pub cover_gradient: String,
    pub cover_url: Option<String>,
    pub description: Option<String>,
    pub tips: Option<String>,
    pub calories: Option<u16>,
    pub is_active: bool,
    pub cook_count: u32,
    pub avg_rating: Decimal,
    pub created_by: Option<u64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecipeDetail {
    #[serde(flatten)]
    pub base: RecipeBase,
    pub creator_name: Option<String>,
    pub tags: Vec<Tag>,
    pub steps: Vec<RecipeStep>,
    pub ingredient_groups: Vec<IngredientGroup>,
    pub favorite_members: Vec<Member>,
}

// =====================================================================
//  订单
// =====================================================================

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct OrderSummary {
    pub id: u64,
    pub order_no: String,
    pub member_id: u64,
    pub member_nickname: String,
    pub member_avatar: String,
    pub meal_date: NaiveDate,
    pub meal_type: String,
    pub status: String,
    pub diners: u8,
    pub note: Option<String>,
    pub served_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub dish_count: i64,
    pub total_portions: i64,
    pub dish_names: Option<String>,
    pub cover_emojis: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct OrderItemView {
    pub id: u64,
    pub recipe_id: Option<u64>,
    pub recipe_snapshot: String,
    pub portions: u8,
    pub rating: Option<u8>,
    pub item_status: String,
    pub note: Option<String>,
    pub sort_order: i32,
    pub recipe_name: Option<String>,
    pub category: Option<String>,
    pub difficulty: Option<String>,
    pub total_minutes: Option<u16>,
    pub calories: Option<u16>,
    pub cover_emoji: Option<String>,
    pub cover_gradient: Option<String>,
    pub ordered_by: Option<String>,
    pub ordered_by_avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ShoppingItemView {
    pub id: u64,
    pub ingredient_id: Option<u64>,
    pub name_snapshot: String,
    pub emoji: String,
    pub category: String,
    pub total_quantity: Decimal,
    pub unit: String,
    pub is_staple: bool,
    pub is_purchased: bool,
    pub recipe_names: Option<String>,
}

/// 采购清单按分类分组后的结构
#[derive(Debug, Clone, Serialize)]
pub struct ShoppingGroup {
    pub category: String,
    pub items: Vec<ShoppingItemView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShoppingList {
    /// 需要采购的（按分类分组）
    pub groups: Vec<ShoppingGroup>,
    /// 家中常备的，前端折叠展示
    pub staples: Vec<ShoppingItemView>,
    pub total_kinds: usize,
    pub purchased_kinds: usize,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct OrderBase {
    pub id: u64,
    pub order_no: String,
    pub member_id: u64,
    pub meal_date: NaiveDate,
    pub meal_type: String,
    pub status: String,
    pub diners: u8,
    pub note: Option<String>,
    pub served_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderDetail {
    #[serde(flatten)]
    pub base: OrderBase,
    pub member_nickname: String,
    pub member_avatar: String,
    pub items: Vec<OrderItemView>,
    pub shopping_list: ShoppingList,
    pub total_portions: i64,
    pub total_calories: i64,
    pub total_cook_minutes: i64,
    pub avg_rating: Option<Decimal>,
}

// =====================================================================
//  统计
// =====================================================================

#[derive(Debug, Clone, Serialize)]
pub struct Overview {
    pub recipe_total: i64,
    pub recipe_active: i64,
    pub member_total: i64,
    pub ingredient_total: i64,
    pub order_total: i64,
    pub order_today: i64,
    pub order_pending: i64,
    pub dishes_served: i64,
    pub avg_rating: Option<Decimal>,
    pub top_category: Option<CategoryCount>,
    pub favorite_dish: Option<PopularDish>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CategoryCount {
    pub category: String,
    pub recipe_count: i64,
    pub order_count: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PopularDish {
    pub id: u64,
    pub name: String,
    pub cover_emoji: String,
    pub cover_gradient: String,
    pub category: String,
    pub order_count: i64,
    pub total_portions: i64,
    pub avg_rating: Option<Decimal>,
}

// 成员口味统计以 serde_json::Value 动态返回，无需固定行结构。

// =====================================================================
//  阶段 1 · 家庭管理底座（账本 / 快递 / 健康 / 同步）
// =====================================================================

/// 账本收支记录
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct LedgerEntry {
    pub id: u64,
    pub member_id: u64,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub entry_type: String,
    pub category: String,
    pub amount: Decimal,
    pub entry_date: NaiveDate,
    pub note: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// 快递取件码
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Parcel {
    pub id: u64,
    pub pickup_code: String,
    pub courier_code: String,
    pub courier_name: Option<String>,
    pub station_type: Option<String>,
    pub station: Option<String>,
    pub shelf_no: Option<String>,
    pub tracking_no: Option<String>,
    pub note: Option<String>,
    pub recipient_member_id: Option<u64>,
    pub recipient_name: Option<String>,
    pub arrived_date: NaiveDate,
    pub status: String,
    pub picked_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// 成员健康档案（一成员一行）
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct HealthProfile {
    pub id: u64,
    pub member_id: u64,
    pub gender: Option<String>,
    pub last_period_start: Option<NaiveDate>,
    pub cycle_length: u8,
    pub period_length: u8,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// 成员同步数据（JSON blob 整包）
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MemberDataRow {
    pub id: u64,
    pub member_id: u64,
    pub data_key: String,
    pub payload: serde_json::Value,
    pub version: u32,
    pub updated_at: NaiveDateTime,
}

// =====================================================================
//  阶段 2 · 家庭档案 + 站内提醒
// =====================================================================

/// 站内提醒（经期推算 / 快递超时 / 后续扩展）
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Reminder {
    pub id: u64,
    pub member_id: Option<u64>,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub reminder_type: String,
    pub title: String,
    pub due_at: NaiveDateTime,
    pub status: String,
    pub payload: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// =====================================================================
//  系统版本更新记录（个人中心 · 版本详情）
// =====================================================================

/// 版本更新记录（每次发版追加一条）
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SystemVersion {
    pub id: u64,
    /// 版本号，如 1.0
    pub version: String,
    /// 更新主题（可选）
    pub title: Option<String>,
    /// 更新内容
    pub content: String,
    pub released_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}
