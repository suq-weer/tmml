//! NeoForge 模块：版本目录 + 安装入口。

pub mod catalog;
pub mod installer;
pub mod processor;
pub mod profile;

use anyhow::Result;

use crate::loader::{patch::VersionPatch, InstallContext};

/// 拉取某 MC 版本可用的 NeoForge 版本
pub async fn list_versions(minecraft_version: &str) -> Result<Vec<crate::loader::LoaderVersion>> {
    catalog::list_versions(minecraft_version).await
}

/// 安装 NeoForge，返回待合并的启动补丁
pub async fn install(
    minecraft_version: &str,
    loader_version: &str,
    ctx: &InstallContext,
) -> Result<VersionPatch> {
    installer::install(minecraft_version, loader_version, ctx).await
}
