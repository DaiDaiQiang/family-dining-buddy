//! 封面图片上传
//!
//! - `POST /api/uploads`          接收 JSON `{ data }`，data 为 dataURL 或纯 base64；
//!                                解码校验后写入 uploads 目录，返回可直接存入
//!                                recipe.cover_url 的相对地址。
//! - `GET  /api/uploads/{name}`   读取已上传的图片。
//!
//! 设计约束：不引入 multipart / base64 等额外依赖，base64 手写解码；
//! 体积上限与 main.rs 的 JsonConfig limit（2MB）匹配，前端负责压缩图片。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::error::ApiError;
use crate::util::ok;

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// 解码后图片字节上限（与前端 1080px JPEG 压缩后的典型体积匹配）
const MAX_IMAGE_BYTES: usize = 1_400_000;

#[derive(Deserialize)]
pub struct UploadReq {
    /// 形如 "data:image/jpeg;base64,xxx" 的 dataURL，或纯 base64 字符串
    pub data: String,
}

/// POST /api/uploads
pub async fn create(
    req: web::Json<UploadReq>,
    dir: web::Data<PathBuf>,
) -> Result<HttpResponse, ApiError> {
    let raw = req.data.trim();
    let b64 = match raw.find(',') {
        Some(i) if raw.starts_with("data:") => &raw[i + 1..],
        _ => raw,
    };
    // base64 膨胀约 4/3，先按字符数粗限，避免无谓解码
    if b64.len() > MAX_IMAGE_BYTES * 2 {
        return Err(ApiError::BadRequest("图片过大，请先压缩后再上传".into()));
    }

    let bytes = base64_decode(b64)
        .ok_or_else(|| ApiError::BadRequest("图片数据无效（base64 解析失败）".into()))?;

    if bytes.len() < 12 {
        return Err(ApiError::BadRequest("图片内容过小，疑似无效文件".into()));
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ApiError::BadRequest(format!(
            "图片不能超过 {}MB",
            MAX_IMAGE_BYTES / 1024 / 1024 + if MAX_IMAGE_BYTES % (1024 * 1024) > 0 { 1 } else { 0 }
        )));
    }

    let ext = detect_ext(&bytes)
        .ok_or_else(|| ApiError::BadRequest("仅支持 JPG / PNG / GIF / WebP 图片".into()))?;

    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = format!(
        "{}-{:04}-{:x}.{}",
        chrono::Local::now().format("%Y%m%d%H%M%S"),
        seq % 10000,
        nanos_seed(),
        ext
    );

    let dir_path = dir.get_ref();
    tokio::fs::create_dir_all(dir_path)
        .await
        .map_err(|e| ApiError::Internal(format!("创建上传目录失败：{e}")))?;
    tokio::fs::write(dir_path.join(&name), &bytes)
        .await
        .map_err(|e| ApiError::Internal(format!("保存图片失败：{e}")))?;

    Ok(ok(serde_json::json!({
        "url": format!("/api/uploads/{name}"),
        "size": bytes.len(),
    })))
}

/// GET /api/uploads/{name}
pub async fn get(
    name: web::Path<String>,
    dir: web::Data<PathBuf>,
) -> Result<HttpResponse, ApiError> {
    let name = name.into_inner();
    if !is_safe_name(&name) {
        return Err(ApiError::BadRequest("文件名不合法".into()));
    }
    let path = dir.get_ref().join(&name);
    if !path.is_file() {
        return Err(ApiError::NotFound("图片不存在".into()));
    }
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| ApiError::Internal(format!("读取图片失败：{e}")))?;

    let content_type = match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "image/jpeg",
    };

    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .append_header(("Cache-Control", "public, max-age=31536000, immutable"))
        .body(bytes))
}

/// 仅允许「时间戳-序号-随机.扩展名」形态的文件名，杜绝路径穿越
fn is_safe_name(s: &str) -> bool {
    let ext_ok = [".jpg", ".png", ".gif", ".webp"].iter().any(|e| s.ends_with(e));
    !s.is_empty()
        && s.len() <= 80
        && ext_ok
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        && !s.contains("..")
}

fn nanos_seed() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0)
}

/// 按魔数识别真实图片格式，返回存储扩展名
fn detect_ext(b: &[u8]) -> Option<&'static str> {
    if b.len() >= 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF {
        return Some("jpg");
    }
    if b.len() >= 8 && b[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return Some("png");
    }
    if b.len() >= 6 && &b[..3] == b"GIF" && b[3] == b'8' {
        return Some("gif");
    }
    if b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return Some("webp");
    }
    None
}

/// 标准 base64 解码（宽松：忽略换行与空白，容忍缺失 padding）
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;

    for &c in input.as_bytes() {
        if c == b'=' {
            break;
        }
        if c.is_ascii_whitespace() {
            continue;
        }
        let val = TABLE.iter().position(|&t| t == c)? as u32;
        acc = (acc << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}
