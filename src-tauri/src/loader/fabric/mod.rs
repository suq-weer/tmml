//! Fabric 模块：版本目录 + 安装入口。

pub mod api;
pub mod installer;
pub mod meta;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::loader::LoaderVersion;

/// 版本目录响应里只取 loader 概要（该接口按 loader build 由新到旧返回）
#[derive(Deserialize)]
struct LoaderListEnvelope {
    loader: LoaderMetaInfo,
}

#[derive(Deserialize)]
struct LoaderMetaInfo {
    version: String,
    #[serde(default)]
    stable: bool,
}

/// 拉取指定 MC 版本可用的 Fabric Loader 版本列表
pub async fn list_versions(minecraft_version: &str) -> Result<Vec<LoaderVersion>> {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}",
        minecraft_version
    );
    let bytes = crate::loader::download::fetch_bytes(&url).await?;
    let entries: Vec<LoaderListEnvelope> =
        serde_json::from_slice(&bytes).context("解析 Fabric Loader 版本列表失败")?;
    Ok(entries
        .into_iter()
        .map(|e| LoaderVersion {
            version: e.loader.version,
            stable: e.loader.stable,
        })
        .collect())
}
