//! 模组加载器安装模块。
//!
//! 统一入口 `install_loader`：完成加载器 patch 的生成、运行时库下载、
//! 与原版 version JSON 的合并写回（另存 `.vanilla.json` 原版备份）。
//!
//! NeoForge 走「installer + Java processor」；Fabric 走 Fabric Meta API；
//! Fabric API 是可选的普通模组，放入实例 mods 目录。

pub mod download;
pub mod fabric;
pub mod install_ctx;
pub mod maven;
pub mod neoforge;
pub mod patch;

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub use install_ctx::InstallContext;

/// 支持的加载器类型
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    NeoForge,
    Fabric,
}

impl LoaderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LoaderKind::NeoForge => "neoforge",
            LoaderKind::Fabric => "fabric",
        }
    }
}

/// 前端提交的加载器安装请求
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoaderRequest {
    pub kind: LoaderKind,
    pub version: String,
    #[serde(default)]
    pub with_fabric_api: bool,
}

/// 加载器版本目录条目
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

/// 列出某 MC 版本可用的加载器版本
pub async fn list_loader_versions(
    kind: LoaderKind,
    minecraft_version: &str,
) -> Result<Vec<LoaderVersion>> {
    match kind {
        LoaderKind::NeoForge => neoforge::list_versions(minecraft_version).await,
        LoaderKind::Fabric => fabric::list_versions(minecraft_version).await,
    }
}

/// 安装加载器：生成补丁 -> 合并写回 -> （可选）安装 Fabric API。
///
/// 安装完成后游戏可直接通过现有的 `<version>.json` 启动。
pub async fn install_loader(
    kind: LoaderKind,
    minecraft_version: &str,
    loader_version: &str,
    with_fabric_api: bool,
    ctx: &InstallContext,
) -> Result<()> {
    let patch = match kind {
        LoaderKind::NeoForge => neoforge::install(minecraft_version, loader_version, ctx).await,
        LoaderKind::Fabric => {
            fabric::installer::build_patch(minecraft_version, loader_version, ctx).await
        }
    };

    // 处理器/下载用的临时目录不再需要（无论后续成败都清理，避免残留）
    let temp = ctx.temp_dir();
    if temp.exists() {
        let _ = std::fs::remove_dir_all(&temp);
    }
    let patch = patch?;

    let merged = patch::merge(&ctx.base, &patch);
    let (_, _) = patch::persist(&ctx.game_dir, &ctx.base.id, &ctx.base, &merged)?;

    if with_fabric_api && kind == LoaderKind::Fabric {
        fabric::api::install_fabric_api(minecraft_version, ctx).await?;
    }

    tracing::info!(
        loader = kind.as_str(),
        loader_version,
        mc = minecraft_version,
        "模组加载器安装完成"
    );
    Ok(())
}

/// 便捷：从实例配置取出 java 可执行路径
pub fn pick_java_bin(java_path: &Option<String>) -> String {
    java_path
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "java".to_string())
}
