use std::{io::Cursor, time::Duration};

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use tokio::fs;

use crate::appfile::dirs;

/// Mojang API：通过玩家名获取玩家的 Mojang UUID
async fn lookup_mojang_uuid(username: &str) -> Result<Option<String>> {
    let url = format!(
        "https://api.mojang.com/users/profiles/minecraft/{}",
        username
    );
    let resp = http_client()
        .get(&url)
        .send()
        .await
        .context("请求 Mojang 玩家 UUID API 失败")?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        bail!("Mojang 玩家 UUID API 返回状态码 {}", resp.status());
    }
    let text = resp.text().await.context("读取 Mojang UUID 响应失败")?;
    let parsed: UuidLookup = serde_json::from_str(&text).context("解析 Mojang UUID 响应失败")?;
    tracing::info!("正版账号 {} 的 UUID: {}", username, parsed.id);
    Ok(Some(parsed.id))
}

/// 通过 Mojang UUID 获取玩家皮肤纹理 URL
async fn lookup_skin_url(mojang_uuid: &str) -> Result<Option<String>> {
    let url = format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{}",
        mojang_uuid
    );
    let resp = http_client()
        .get(&url)
        .send()
        .await
        .context("请求 Mojang 会话 API 失败")?;
    if resp.status() == reqwest::StatusCode::NO_CONTENT || resp.status().is_client_error() {
        return Ok(None);
    }
    if !resp.status().is_success() {
        bail!("Mojang 会话 API 返回状态码 {}", resp.status());
    }
    let text = resp.text().await.context("读取 Mojang 会话响应失败")?;
    let parsed: SessionProfile = serde_json::from_str(&text).context("解析 Mojang 会话响应失败")?;
    for prop in parsed.properties {
        if prop.name != "textures" {
            continue;
        }
        let decoded = general_purpose::STANDARD
            .decode(prop.value)
            .context("解码皮肤纹理 Base64 失败")?;
        let textures: TexturePayload =
            serde_json::from_slice(&decoded).context("解析皮肤纹理 JSON 失败")?;
        if let Some(skin) = textures.textures.skin {
            return Ok(Some(normalize_url(&skin.url)));
        }
    }
    Ok(None)
}

/// 下载皮肤 PNG 原图并持久化到本地缓存目录
async fn download_bytes(url: &str) -> Result<Vec<u8>> {
    let resp = http_client()
        .get(url)
        .send()
        .await
        .context("下载皮肤图片失败")?;
    if !resp.status().is_success() {
        bail!("皮肤图片下载失败：状态码 {}", resp.status());
    }
    Ok(resp.bytes().await.context("读取皮肤图片数据失败")?.to_vec())
}

/// 从 Minecraft 皮肤 PNG 中裁出脸部（坐标 8,8 到 15,15，含端点，原点在左上角），输出 8x8 RGBA PNG
fn crop_face(png_bytes: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::ALPHA | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().context("解析皮肤 PNG 失败")?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).context("解码皮肤 PNG 失败")?;
    let width = info.width as usize;
    let height = info.height as usize;
    if width < 16 || height < 16 {
        bail!("皮肤尺寸过小（{}x{}），无法裁剪脸部", width, height);
    }
    if info.color_type != png::ColorType::Rgba {
        bail!("皮肤不是 RGBA 格式，无法裁剪脸部");
    }
    let bpp = 4usize;
    let mut face = Vec::with_capacity(8 * 8 * bpp);
    for y in 8..16 {
        for x in 8..16 {
            let src = (y * width + x) * bpp;
            face.extend_from_slice(&buf[src..src + bpp]);
        }
    }

    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 8, 8);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().context("创建皮肤头像 PNG 失败")?;
        writer
            .write_image_data(&face)
            .context("编码皮肤头像 PNG 失败")?;
    }
    Ok(out)
}

fn normalize_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        format!("https://{}", rest)
    } else {
        url.to_string()
    }
}

fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .expect("构建 HTTP 客户端失败")
}

/// 获取玩家皮肤的脸部头像（8x8 PNG 的 Base64 data URL）。
/// 玩家不存在于 Mojang、没有自定义皮肤或发生可忽略的错误时返回 None。
/// 皮肤原图会按 Mojang UUID 缓存到本地缓存文件夹。
pub async fn get_profile_avatar(username: &str) -> Result<Option<String>> {
    if username.is_empty() || username.len() > 16 {
        tracing::debug!(username, "玩家名不符合 Mojang 规范，跳过皮肤头像");
        return Ok(None);
    }
    let Some(uuid) = lookup_mojang_uuid(username).await? else {
        tracing::debug!(username, "玩家不存在于 Mojang");
        return Ok(None);
    };
    let Some(skin_url) = lookup_skin_url(&uuid).await? else {
        tracing::debug!(uuid, "玩家没有自定义皮肤");
        return Ok(None);
    };

    let cache_dir = dirs::config()?.join(".cache").join("skins");
    fs::create_dir_all(&cache_dir).await?;
    let skin_path = cache_dir.join(format!("{}.png", uuid));

    if !fs::try_exists(&skin_path).await? {
        let bytes = download_bytes(&skin_url).await?;
        fs::write(&skin_path, &bytes).await?;
        tracing::debug!(path = ?skin_path, "已缓存玩家皮肤");
    }

    let png_bytes = fs::read(&skin_path).await?;
    let face = crop_face(&png_bytes)?;
    let data = general_purpose::STANDARD.encode(&face);
    Ok(Some(format!("data:image/png;base64,{}", data)))
}

#[derive(serde::Deserialize)]
struct UuidLookup {
    id: String,
}

#[derive(serde::Deserialize)]
struct SessionProfile {
    #[serde(default)]
    properties: Vec<SessionProperty>,
}

#[derive(serde::Deserialize)]
struct SessionProperty {
    name: String,
    value: String,
}

#[derive(serde::Deserialize)]
struct TexturePayload {
    #[serde(default)]
    textures: TextureSet,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct TextureSet {
    #[serde(default)]
    skin: Option<TextureEntry>,
}

#[derive(serde::Deserialize)]
struct TextureEntry {
    url: String,
}
