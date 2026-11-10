//! 下厨房菜谱导入：粘贴分享链接 → 抓取解析 → 预览 → 确认建档。
//!
//! 只支持「单个链接手动导入」的低频场景，因此不引入代理池 / 并发等
//! 批量爬虫手段，仅带常规 UA + Referer 访问移动版详情页（反爬更松）。

use actix_web::{web, HttpResponse};
use scraper::{Html, Selector};
use serde::Deserialize;

use crate::db::AppState;
use crate::dto::CreateRecipeReq;
use crate::error::ApiError;
use crate::routes::recipes::{build_recipe_detail, insert_recipe_in_tx};
use crate::util::{ok, ok_with_message};

/// 允许解析的域名白名单（防止把本接口当 SSRF 代理去扫内网）
const ALLOWED_HOSTS: [&str; 2] = ["xiachufang.com", "www.xiachufang.com"];

/// 抓取时的浏览器 UA
const USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) \
AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";

#[derive(Debug, Deserialize)]
pub struct ImportUrlReq {
    /// 下厨房菜谱分享链接
    pub url: String,
}

/// 预览接口返回的结构：前端据此渲染可编辑表单
#[derive(Debug, serde::Serialize)]
pub struct PreviewData {
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub source_url: String,
    pub ingredients: Vec<PreviewIngredient>,
    pub steps: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct PreviewIngredient {
    pub name: String,
    /// 用量原始文本，如「600克」「适量」，供预览展示
    pub amount: String,
    /// 数值化后的用量（可选，如 600.0）
    pub quantity: Option<f64>,
    /// 数值化后的单位（可选，如「克」）
    pub unit: Option<String>,
}

/// 从 URL 中提取菜谱数字 ID
fn extract_recipe_id(url: &str) -> Result<u64, ApiError> {
    // 兼容 /recipe/104218740/ 与 /recipe/104218740（末尾有无斜杠皆可）
    let id = url
        .rsplit('/')
        .find(|seg| !seg.is_empty())
        .and_then(|seg| seg.parse::<u64>().ok())
        .ok_or_else(|| ApiError::BadRequest("链接里没找到菜谱编号，请确认是下厨房菜谱分享链接".into()))?;
    Ok(id)
}

/// 校验域名白名单并归一化成移动版 URL
fn to_mobile_url(url: &str) -> Result<String, ApiError> {
    let trimmed = url.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .ok_or_else(|| ApiError::BadRequest("链接需以 http:// 或 https:// 开头".into()))?;

    // host 部分：截取第一个 '/' 之前
    let host = without_scheme.split('/').next().unwrap_or("");

    if !ALLOWED_HOSTS.contains(&host) {
        return Err(ApiError::BadRequest(format!(
            "仅支持导入「下厨房」的菜谱链接（xiachufang.com），当前链接为：{host}"
        )));
    }

    // 归一化为移动版路径，并把 www. 去掉
    let path_and_query = without_scheme
        .strip_prefix(host)
        .unwrap_or(without_scheme);
    Ok(format!("https://m.xiachufang.com{path_and_query}"))
}

/// 抓取下厨房移动版详情页 HTML
async fn fetch_html(client: &reqwest::Client, url: &str) -> Result<String, ApiError> {
    let resp = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::REFERER, "https://m.xiachufang.com/")
        .header(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9")
        .send()
        .await
        .map_err(|e| ApiError::BadRequest(format!("抓取下厨房页面失败：{e}")))?;

    let status = resp.status();

    // 下厨房对数据中心/非真人请求会 302 重定向到人机验证码页（humancheck_captcha）
    if status.is_redirection() {
        let loc = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if loc.contains("humancheck") || loc.contains("captcha") {
            return Err(ApiError::BadRequest(
                "下厨房对当前网络触发了人机验证，暂时无法自动抓取。请在手机/电脑浏览器里打开该菜谱，手动复制「用料」与「步骤」到新建菜谱即可。".into(),
            ));
        }
    }

    if !status.is_success() {
        return Err(ApiError::BadRequest(format!(
            "下厨房返回了异常状态（HTTP {status}），可能被限流或链接失效，请稍后再试"
        )));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| ApiError::BadRequest(format!("读取下厨房页面内容失败：{e}")))?;

    // 兜底：即便状态码 200，也可能返回的是验证码页 HTML
    if body.contains("humancheck") || body.contains("captcha") {
        return Err(ApiError::BadRequest(
            "下厨房对当前网络触发了人机验证，暂时无法自动抓取。请在浏览器里打开该菜谱，手动复制「用料」与「步骤」到新建菜谱即可。".into(),
        ));
    }

    Ok(body)
}

/// 解析 HTML：标题 / 简介 / 封面 / 用料 / 步骤
fn parse_recipe(html: &str, source_url: &str) -> Result<PreviewData, ApiError> {
    let doc = Html::parse_document(html);

    // 标题：移动版常见于 h1 或 .recipe-title / .info h1
    let name = first_text(&doc, &["h1", ".recipe-title", ".title", ".info h1"])?
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if name.is_none() {
        return Err(ApiError::BadRequest(
            "未能从页面解析出菜名，可能是下厨房页面结构已更新，请改用手动建档".into(),
        ));
    }
    let name = name.unwrap();

    // 封面：主图（仅作预览展示，不落库，落库走 emoji 兜底避免防盗链裂图）
    let cover_url = first_attr(&doc, &["img.recipe-cover", ".cover img", ".pic img"], "src");

    // 简介：下厨房常有「食材介绍 / 作者的话」，取第一个段落
    let description = first_text(&doc, &[".desc", ".intro", ".recipe-desc"])?;

    // 用料：形如「五花肉 600克」或「盐 适量」
    // 移动版常见 .ings 下的 li，或表格里的名称/用量两列
    let mut ingredients = Vec::new();
    let ing_selector = Selector::parse(
        ".ings li, .ingredients li, .recipeIngs li, .ings td, .ingredients td",
    )
    .ok();
    if let Some(sel) = ing_selector {
        for node in doc.select(&sel) {
            let text = node.text().collect::<Vec<_>>().concat();
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            // 拆成「名称 + 用量」：用量通常是末尾的数字+单位，或「适量/少许」
            if let Some((name, amount)) = split_ingredient(text) {
                let (quantity, unit) = parse_amount(&amount);
                ingredients.push(PreviewIngredient {
                    name,
                    amount,
                    quantity,
                    unit,
                });
            } else {
                // 拆不出来的，整段当名称、用量留空
                ingredients.push(PreviewIngredient {
                    name: text.to_string(),
                    amount: String::new(),
                    quantity: None,
                    unit: None,
                });
            }
        }
    }

    // 步骤：移动版常见 .steps 下的 li / p，带序号
    let mut steps = Vec::new();
    let step_selector = Selector::parse(".steps li, .steps p, .recipeStep li, .step li").ok();
    if let Some(sel) = step_selector {
        for node in doc.select(&sel) {
            let text = node.text().collect::<Vec<_>>().concat();
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            steps.push(text.to_string());
        }
    }

    if ingredients.is_empty() && steps.is_empty() {
        return Err(ApiError::BadRequest(
            "未能解析出用料或步骤，可能是下厨房页面结构已更新，请改用手动建档".into(),
        ));
    }

    Ok(PreviewData {
        name,
        description,
        cover_url,
        source_url: source_url.to_string(),
        ingredients,
        steps,
    })
}

/// 从一组 CSS 选择器里取第一个非空文本
fn first_text(doc: &Html, selectors: &[&str]) -> Result<Option<String>, ApiError> {
    for sel_str in selectors {
        let Ok(sel) = Selector::parse(sel_str) else {
            continue;
        };
        if let Some(node) = doc.select(&sel).next() {
            let text = node.text().collect::<Vec<_>>().concat();
            let text = text.trim().to_string();
            if !text.is_empty() {
                return Ok(Some(text));
            }
        }
    }
    Ok(None)
}

/// 取第一个匹配节点的指定属性
fn first_attr(doc: &Html, selectors: &[&str], attr: &str) -> Option<String> {
    for sel_str in selectors {
        let Ok(sel) = Selector::parse(sel_str) else {
            continue;
        };
        if let Some(node) = doc.select(&sel).next() {
            if let Some(v) = node.value().attr(attr) {
                let v = v.trim().to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// 把「五花肉 600克」拆成（名称=五花肉，用量=600克）
/// 识别末尾的「数字+单位」或「适量/少许/少量/若干」等模糊量词
fn split_ingredient(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    // 模糊量词结尾
    const VAGUE: [&str; 8] = ["适量", "少许", "少量", "若干", "按需", "随口味", "酌量", "少许即可"];
    for v in VAGUE {
        if text.ends_with(v) {
            let name = text[..text.len() - v.len()].trim().to_string();
            if !name.is_empty() {
                return Some((name, v.to_string()));
            }
        }
    }

    // 数字+单位结尾：从末尾找数字段（支持小数、分数 1/2、半角½）
    let chars: Vec<char> = text.chars().collect();
    let mut num_start = chars.len();
    let mut seen_digit = false;
    for (i, c) in chars.iter().enumerate().rev() {
        if c.is_ascii_digit() || *c == '.' || *c == '/' || *c == '½' {
            num_start = i;
            seen_digit = true;
        } else if seen_digit {
            break;
        }
    }
    if seen_digit && num_start > 0 && num_start < chars.len() {
        let name = chars[..num_start].iter().collect::<String>().trim().to_string();
        let amount = chars[num_start..].iter().collect::<String>().trim().to_string();
        if !name.is_empty() {
            return Some((name, amount));
        }
    }

    None
}

/// 尝试把「600克」拆成 quantity=600.0 + unit=克；拆不出返回 (None, None)
fn parse_amount(amount: &str) -> (Option<f64>, Option<String>) {
    let amount = amount.trim();
    if amount.is_empty() {
        return (None, None);
    }
    // 纯数字
    if let Ok(q) = amount.parse::<f64>() {
        return (Some(q), None);
    }
    // 数字开头，后面跟单位
    let mut num_end = 0;
    for (i, c) in amount.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            num_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if num_end > 0 {
        let q = amount[..num_end].parse::<f64>().ok();
        let unit = amount[num_end..].trim().to_string();
        if let Some(q) = q {
            return (Some(q), if unit.is_empty() { None } else { Some(unit) });
        }
    }
    (None, None)
}

/// 抓取并解析，返回预览数据（不落库）
async fn fetch_and_parse(client: &reqwest::Client, raw_url: &str) -> Result<PreviewData, ApiError> {
    // 校验并保留原始链接用于回显
    let _ = extract_recipe_id(raw_url)?;
    let mobile_url = to_mobile_url(raw_url)?;
    let html = fetch_html(client, &mobile_url).await?;
    parse_recipe(&html, raw_url)
}

/// 构建共享的 HTTP 客户端（带超时，不自动跟随重定向以便识别人机验证）
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("构建 HTTP 客户端失败")
}

/// POST /api/recipes/import/preview —— 抓取并解析，返回预览（不落库）
pub async fn import_preview(
    body: web::Json<ImportUrlReq>,
) -> Result<HttpResponse, ApiError> {
    let client = http_client();
    let preview = fetch_and_parse(&client, &body.url).await?;
    Ok(ok(preview))
}

/// POST /api/recipes/import —— 接收用户确认（可编辑）后的数据并落库
///
/// 前端把预览里可编辑的字段（菜名 / 分类 / 菜系 / 难度 / 用料 / 步骤）回传，
/// 后端复用 `insert_recipe_in_tx` 完成建档，保证与手工建档走同一套校验与逻辑。
pub async fn import_recipe(
    state: web::Data<AppState>,
    body: web::Json<CreateRecipeReq>,
) -> Result<HttpResponse, ApiError> {
    let pool = state.db()?;
    let req = body.into_inner();

    let mut tx = pool.begin().await?;
    let (id, final_name) = insert_recipe_in_tx(&mut *tx, &req).await?;
    tx.commit().await?;

    let requested = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let message = match requested {
        Some(r) if r != final_name => format!("菜名「{r}」已存在，已自动命名为「{final_name}」"),
        _ => "导入成功，菜谱已建档".to_string(),
    };

    Ok(ok_with_message(message, build_recipe_detail(pool, id).await?))
}
