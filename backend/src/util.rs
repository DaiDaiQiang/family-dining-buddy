//! 通用工具：统一响应封装、分页结构、参数校验。

use actix_web::HttpResponse;
use serde::Serialize;
use serde_json::json;

use crate::error::ApiError;

/// 统一成功响应：`{ code: 0, message: "ok", data }`
pub fn ok<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "code": 0,
        "message": "ok",
        "data": data,
    }))
}

/// 带自定义提示的成功响应
pub fn ok_with_message<T: Serialize>(message: impl AsRef<str>, data: T) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "code": 0,
        "message": message.as_ref(),
        "data": data,
    }))
}

/// 分页包装
#[derive(Debug, Serialize)]
pub struct Page<T: Serialize> {
    pub list: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T: Serialize> Page<T> {
    pub fn new(list: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };
        Self {
            list,
            total,
            page,
            page_size,
            total_pages,
        }
    }

    pub fn empty(page: i64, page_size: i64) -> Self {
        Self::new(Vec::new(), 0, page, page_size)
    }
}

/// 归一化分页参数：page ≥ 1，page_size ∈ [1, 100]
pub fn normalize_paging(page: Option<i64>, page_size: Option<i64>) -> (i64, i64, i64) {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;
    (page, page_size, offset)
}

/// 校验非空字符串（去首尾空格后不为空且不超过上限）
pub fn require_text(value: &Option<String>, field: &str, max_len: usize) -> Result<String, ApiError> {
    let trimmed = value
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::BadRequest(format!("{field} 不能为空")))?;

    if trimmed.chars().count() > max_len {
        return Err(ApiError::BadRequest(format!(
            "{field} 长度不能超过 {max_len} 个字符"
        )));
    }
    Ok(trimmed)
}

/// 校验十六进制颜色 `#RRGGBB`
pub fn validate_color(color: &str, field: &str) -> Result<String, ApiError> {
    let c = color.trim();
    let valid = c.len() == 7
        && c.starts_with('#')
        && c[1..].chars().all(|ch| ch.is_ascii_hexdigit());
    if valid {
        Ok(c.to_uppercase())
    } else {
        Err(ApiError::BadRequest(format!(
            "{field} 必须是 #RRGGBB 格式，例如 #FF8A65"
        )))
    }
}

/// 校验枚举取值
pub fn validate_enum(value: &str, allowed: &[&str], field: &str) -> Result<(), ApiError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "{field} 只能是 {} 之一",
            allowed.join(" / ")
        )))
    }
}

/// 校验数值区间
pub fn validate_range<T>(value: T, min: T, max: T, field: &str) -> Result<(), ApiError>
where
    T: PartialOrd + std::fmt::Display + Copy,
{
    if value < min || value > max {
        Err(ApiError::BadRequest(format!(
            "{field} 需在 {min} ~ {max} 之间，当前为 {value}"
        )))
    } else {
        Ok(())
    }
}

/// 允许的菜谱分类（与建表脚本保持一致）
pub const CATEGORIES: [&str; 8] = [
    "家常菜", "汤羹", "主食", "凉菜", "甜点", "饮品", "烘焙", "其他",
];

/// 允许的菜系
pub const CUISINES: [&str; 8] = [
    "中餐", "川菜", "粤菜", "湘菜", "鲁菜", "西餐", "日韩", "东南亚",
];

/// 难度取值
pub const DIFFICULTIES: [&str; 3] = ["easy", "medium", "hard"];

/// 餐次取值
pub const MEAL_TYPES: [&str; 4] = ["breakfast", "lunch", "dinner", "snack"];

/// 订单状态取值
pub const ORDER_STATUS: [&str; 5] = ["draft", "confirmed", "cooking", "served", "cancelled"];

/// 食材分组取值
pub const INGREDIENT_GROUPS: [&str; 4] = ["主料", "辅料", "调味料", "其他"];

/// 成员角色取值
pub const MEMBER_ROLES: [&str; 3] = ["admin", "member", "child"];
pub const GENDERS: [&str; 3] = ["female", "male", "other"];
pub const REMINDER_STATUS: [&str; 3] = ["pending", "done", "dismissed"];
