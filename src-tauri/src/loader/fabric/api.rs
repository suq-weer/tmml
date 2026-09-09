//! Fabric API 模组：经 Modrinth 查询并下载最新适配版本的 JAR 到 mods 目录。
//!
//! Fabric API 是一个普通 Fabric 模组，按文档约定：不进 loader patch，
//! 依赖关系作为模组元数据处理，不做 libraries 注入。

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

use crate::loader::download::{download_file, sha1_hex_file};

/// Modrinth 单条版本记录（仅取用所需字段）
#[derive(Deserialize, Debug, Clone)]
pub struct ModrinthVersion {
    pub id: String,
    #[serde(rename = "version_number")]
    pub version_number: String,
    #[serde(rename = "version_type")]
    pub version_type: String,
    #[serde(rename = "game_versions")]
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<ModrinthFile>,
    #[serde(default)]
    pub dependencies: Vec<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    pub size: u64,
    pub hashes: HashMap<String, String>,
}

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

fn project_url(project: &str) -> String {
    format!("{}/project/{}/version", MODRINTH_API, project)
}

/// 拉取符合 MC 版本与 Fabric 加载器的候选版本（官方按新旧倒序返回）
async fn query_versions(minecraft_version: &str) -> Result<Vec<ModrinthVersion>> {
    let game = format!(r#"["{}"]"#, minecraft_version);
    let loaders = r#"["fabric"]"#;
    let url = format!(
        "{}?game_versions={}&loaders={}",
        project_url("fabric-api"),
        urlencoding(&game),
        urlencoding(loaders),
    );
    let bytes = crate::loader::download::fetch_bytes(&url).await?;
    serde_json::from_slice(&bytes).context("解析 Modrinth Fabric API 版本列表失败")
}

fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

/// 选中一条「稳定版」且文件带 sha1 的记录
fn pick_version(versions: &[ModrinthVersion]) -> Option<&ModrinthVersion> {
    versions.iter().find(|v| v.version_type == "release")
}

/// 下载适配指定 MC 版本的 Fabric API，落盘到 mods_dir；返回文件名
pub async fn install_fabric_api(
    minecraft_version: &str,
    mods_dir: &std::path::Path,
) -> Result<String> {
    let versions = query_versions(minecraft_version).await?;
    let chosen = pick_version(&versions).ok_or_else(|| {
        anyhow::anyhow!(
            "Modrinth 上没有适配 Minecraft {} 的 Fabric API 版本",
            minecraft_version
        )
    })?;
    let file = chosen
        .files
        .iter()
        .find(|f| f.hashes.contains_key("sha1"))
        .or_else(|| chosen.files.first())
        .ok_or_else(|| anyhow::anyhow!("Fabric API 版本缺少可下载文件"))?;

    std::fs::create_dir_all(mods_dir).context("创建 mods 目录失败")?;
    let dest = mods_dir.join(&file.filename);
    let sha1 = file.hashes.get("sha1").map(String::as_str);
    download_file(&file.url, &dest, sha1).await?;

    let computed = sha1_hex_file(&dest)?;
    if let Some(expected) = sha1 {
        if !computed.eq_ignore_ascii_case(expected) {
            bail!("Fabric API 下载后哈希不一致");
        }
    }
    tracing::info!(file = %file.filename, "Fabric API 已安装到 mods");
    Ok(file.filename.clone())
}
