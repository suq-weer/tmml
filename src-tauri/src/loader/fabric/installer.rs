//! Fabric Loader 安装：直接消费 Fabric Meta API 生成启动补丁。
//!
//! 无需 Java processor，按文档 §4：libraries = common + server + intermediary + fabric-loader，
//! 之后统一走「下载库 + 合并写回」流程。

use anyhow::{bail, Context, Result};

use crate::downloader::deserializer::{Artifact, LibrariesDownloads, OnceLibraries};
use crate::loader::{
    download::{sha1_hex_file, RepoArtifact},
    maven::split_coordinate,
    patch::VersionPatch,
    InstallContext,
};

use super::meta::{FabricLaunchMeta, FabricLibrary};

pub const FABRIC_MAVEN: &str = "https://maven.fabricmc.net/";
pub const CLIENT_MAIN: &str = "net.minecraft.client.main.Main";

/// 获取指定 MC + Loader 组合的启动元数据
async fn fetch_launch_meta(
    minecraft_version: &str,
    loader_version: &str,
) -> Result<FabricLaunchMeta> {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}",
        minecraft_version, loader_version
    );
    let bytes = crate::loader::download::fetch_bytes(&url).await?;
    serde_json::from_slice(&bytes)
        .context("解析 Fabric Launch Meta 失败（可能是该 MC 版本不支持此 Loader 组合）")
}

/// 把 meta 库条目转成下载描述（url 为仓库根地址）
fn repo_artifact(lib: &FabricLibrary) -> Result<RepoArtifact> {
    let coord = split_coordinate(&lib.name).map_err(|e| anyhow::anyhow!("{}", e.0))?;
    let rel = coord.relative_path();
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    Ok(RepoArtifact {
        rel_path: rel,
        bases: vec![],
        exact_url: Some(format!("{}/{}", lib.url.trim_end_matches('/'), rel_str)),
        sha1: lib.sha1.clone(),
        size: lib.size,
    })
}

/// 下载一批库并追加为 OnceLibraries；跳过与原版重复的项
async fn append_libraries(
    ctx: &InstallContext,
    libs: &[FabricLibrary],
    libraries: &mut Vec<OnceLibraries>,
    seen: &mut std::collections::HashSet<String>,
) -> Result<()> {
    for lib in libs {
        if !seen.insert(lib.name.clone()) {
            continue;
        }
        if ctx.base.libraries.iter().any(|b| b.name == lib.name) {
            continue;
        }
        let artifact = repo_artifact(lib)?;
        let path = crate::loader::download::ensure_artifact(&artifact, &ctx.libraries_dir).await?;
        let sha1 = match &lib.sha1 {
            Some(s) => s.clone(),
            None => sha1_hex_file(&path)?,
        };
        let size = match lib.size {
            Some(s) => s,
            None => std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
        };
        libraries.push(OnceLibraries {
            name: lib.name.clone(),
            rules: None,
            downloads: LibrariesDownloads {
                artifact: Artifact {
                    path: artifact.rel_path.to_string_lossy().replace('\\', "/"),
                    sha1,
                    size,
                    url: artifact.exact_url.unwrap_or_default(),
                },
            },
            classifiers: None,
        });
    }
    Ok(())
}

/// 追加一个坐标库（intermediary / fabric-loader），基于 maven 仓库根
async fn append_maven_library(
    ctx: &InstallContext,
    maven_name: &str,
    repo_base: &str,
    libraries: &mut Vec<OnceLibraries>,
    seen: &mut std::collections::HashSet<String>,
) -> Result<()> {
    if !seen.insert(maven_name.to_string()) {
        return Ok(());
    }
    if ctx.base.libraries.iter().any(|b| b.name == maven_name) {
        return Ok(());
    }
    let coord = split_coordinate(maven_name).map_err(|e| anyhow::anyhow!("{}", e.0))?;
    let rel = coord.relative_path();
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    let artifact = RepoArtifact {
        rel_path: rel,
        bases: vec![],
        exact_url: Some(format!("{}/{}", repo_base.trim_end_matches('/'), rel_str)),
        sha1: None,
        size: None,
    };
    let path = crate::loader::download::ensure_artifact(&artifact, &ctx.libraries_dir).await?;
    let sha1 = sha1_hex_file(&path)?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    libraries.push(OnceLibraries {
        name: maven_name.to_string(),
        rules: None,
        downloads: LibrariesDownloads {
            artifact: Artifact {
                path: rel_str,
                sha1,
                size,
                url: artifact.exact_url.unwrap_or_default(),
            },
        },
        classifiers: None,
    });
    Ok(())
}

/// 生成 patch：下载各库并写成标准 OnceLibraries
pub async fn build_patch(
    minecraft_version: &str,
    loader_version: &str,
    ctx: &InstallContext,
) -> Result<VersionPatch> {
    if ctx.base.main_class != CLIENT_MAIN {
        bail!("Fabric 只能安装到纯净原版上（当前入口不是 net.minecraft.client.main.Main）");
    }

    let meta = fetch_launch_meta(minecraft_version, loader_version).await?;
    let main_class = meta
        .launcher_meta
        .main_class
        .client()
        .ok_or_else(|| anyhow::anyhow!("Fabric Meta 缺少客户端 mainClass"))?
        .to_string();

    let mut libraries: Vec<OnceLibraries> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // common + server（与文档/HMCL 行为一致）
    append_libraries(
        ctx,
        &meta.launcher_meta.libraries.common,
        &mut libraries,
        &mut seen,
    )
    .await?;
    append_libraries(
        ctx,
        &meta.launcher_meta.libraries.server,
        &mut libraries,
        &mut seen,
    )
    .await?;

    // 追加 intermediary + fabric-loader
    append_maven_library(
        ctx,
        &meta.intermediary.maven,
        FABRIC_MAVEN,
        &mut libraries,
        &mut seen,
    )
    .await?;
    append_maven_library(
        ctx,
        &meta.loader.maven,
        FABRIC_MAVEN,
        &mut libraries,
        &mut seen,
    )
    .await?;

    // 旧版 LaunchWrapper 兼容参数
    let mut game_args = Vec::new();
    if let Some(wrapper) = &meta.launcher_meta.launchwrapper {
        if let Some(tweaker) = wrapper.client_tweaker() {
            use crate::downloader::deserializer::StringOrArgument;
            game_args.push(StringOrArgument::String("--tweakClass".to_string()));
            game_args.push(StringOrArgument::String(tweaker.to_string()));
        }
    }

    Ok(VersionPatch {
        main_class,
        libraries,
        jvm_args: Vec::new(),
        game_args,
        java_major: None,
    })
}
