//! NeoForge 安装流程编排：
//!
//! ```text
//! 下载 installer.jar
//!   -> install_profile.json
//!   -> 预下载安装期 Maven 依赖
//!   -> Java processors（仅客户端，串行 + 输出校验）
//!   -> version.json（mainClass/arguments/libraries）
//!   -> 下载运行时库并生成 patch
//! ```

use anyhow::{anyhow, bail, Context, Result};
use std::{io::Read, path::PathBuf};

use crate::downloader::deserializer::{Arguments, Artifact, OnceLibraries};
use crate::downloader::minecraft::{DownloadPhase, PhaseProgress};
use crate::loader::{
    download::{sha1_hex_file, RepoArtifact},
    maven::relative_path_of,
    neoforge::{
        catalog::{is_legacy_1201, mc_supports_neoforge},
        processor::ProcessorRunner,
        profile::{InstallProfile, ProfileArtifact, ProfileLibrary},
    },
    patch::VersionPatch,
    InstallContext,
};

use serde::Deserialize;

/// 官方制品仓库候选（按顺序回退）
pub const NEOFORGE_REPOS: &[&str] = &[
    "https://maven.neoforged.net/releases",
    "https://maven.minecraftforge.net/",
    "https://libraries.minecraft.net/",
];

/// installer jar 的官方地址
fn installer_url(minecraft_version: &str, loader_version: &str) -> String {
    if is_legacy_1201(minecraft_version) {
        format!(
            "https://maven.neoforged.net/releases/net/neoforged/forge/{}/forge-{}-installer.jar",
            loader_version, loader_version
        )
    } else {
        format!(
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
            loader_version, loader_version
        )
    }
}

fn read_zip_entry(path: &std::path::Path, entry_name: &str) -> Result<Vec<u8>> {
    let file =
        std::fs::File::open(path).with_context(|| format!("打开 {} 失败", path.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("解析 zip 失败")?;
    let normalized: String = entry_name.trim_start_matches('/').replace('\\', "/");
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.name().replace('\\', "/") == normalized {
            let mut data = Vec::new();
            entry.read_to_end(&mut data)?;
            return Ok(data);
        }
    }
    bail!("zip 内未找到 {}（{}）", entry_name, path.display())
}

async fn parse_install_profile(installer: &std::path::Path) -> Result<InstallProfile> {
    let data = read_zip_entry(installer, "install_profile.json")?;
    serde_json::from_slice(&data).context("解析 install_profile.json 失败")
}

/// 预下载安装期依赖（有 downloads 声明的制品）
async fn predownload_libraries(
    ctx: &InstallContext,
    libraries: &[ProfileLibrary],
    progress: &PhaseProgress,
) -> Result<()> {
    for lib in libraries {
        let Some(artifact) = lib.artifact() else {
            continue;
        };
        let artifact = RepoArtifact {
            rel_path: PathBuf::from(&artifact.path),
            bases: vec![],
            exact_url: artifact.url.clone().filter(|u| !u.is_empty()),
            sha1: artifact.sha1.clone(),
            size: artifact.size,
        };
        // 精确 url 缺失时按仓库回退
        let mut artifact = artifact;
        if artifact.exact_url.is_none() {
            artifact.bases = NEOFORGE_REPOS.iter().map(|s| s.to_string()).collect();
        }
        crate::loader::download::ensure_artifact(&artifact, &ctx.libraries_dir, Some(progress))
            .await?;
    }
    Ok(())
}

/// 运行时库兜底描述（用于日志）
fn describe_lib(lib: &OnceLibraries) -> String {
    lib.name.clone()
}

/// 确保一条运行时库落盘，并返回补齐 sha1/size/url 的标准 OnceLibraries
async fn ensure_runtime_library(
    ctx: &InstallContext,
    lib: &OnceLibraries,
    progress: &PhaseProgress,
) -> Result<OnceLibraries> {
    let artifact = match &lib.downloads.artifact {
        a if a.path.is_empty() => {
            let rel = relative_path_of(&lib.name)
                .ok_or_else(|| anyhow!("无法解析库坐标: {}", lib.name))?;
            ProfileArtifact {
                path: rel.to_string_lossy().replace('\\', "/"),
                sha1: None,
                size: None,
                url: None,
            }
        }
        a => ProfileArtifact {
            path: a.path.clone(),
            sha1: if a.sha1.is_empty() {
                None
            } else {
                Some(a.sha1.clone())
            },
            size: if a.size == 0 { None } else { Some(a.size) },
            url: if a.url.is_empty() {
                None
            } else {
                Some(a.url.clone())
            },
        },
    };

    let mut repo = RepoArtifact {
        rel_path: PathBuf::from(&artifact.path),
        bases: Vec::new(),
        exact_url: artifact.url.clone().filter(|u| !u.is_empty()),
        sha1: artifact.sha1.clone(),
        size: artifact.size,
    };
    if repo.exact_url.is_none() {
        repo.bases = NEOFORGE_REPOS.iter().map(|s| s.to_string()).collect();
    }

    let path = crate::loader::download::ensure_artifact(&repo, &ctx.libraries_dir, Some(progress))
        .await
        .map_err(|e| anyhow!("下载 NeoForge 运行时库 {} 失败: {}", describe_lib(lib), e))?;
    let sha1 = sha1_hex_file(&path)?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    let display_url = artifact
        .url
        .unwrap_or_else(|| format!("{}/{}", NEOFORGE_REPOS[0], artifact.path));

    let mut out = lib.clone();
    out.downloads.artifact = Artifact {
        path: artifact.path,
        sha1,
        size,
        url: display_url,
    };
    Ok(out)
}

#[derive(Deserialize)]
struct LoaderVersionJson {
    #[serde(rename = "mainClass", default)]
    main_class: Option<String>,
    #[serde(default)]
    arguments: Option<Arguments>,
    #[serde(default)]
    libraries: Vec<OnceLibraries>,
    #[serde(rename = "javaVersion", default)]
    java_version: Option<JavaVersionLite>,
}

#[derive(Deserialize, Default)]
struct JavaVersionLite {
    #[serde(rename = "majorVersion", default)]
    major_version: Option<u64>,
}

/// NeoForge 安装：执行 processors、读取 version.json、下载运行时库并生成 patch
pub async fn install(
    minecraft_version: &str,
    loader_version: &str,
    ctx: &InstallContext,
) -> Result<VersionPatch> {
    if ctx.base.main_class != "net.minecraft.client.main.Main" {
        bail!("NeoForge 只能安装到纯净原版上（当前入口不是 net.minecraft.client.main.Main）");
    }
    if !mc_supports_neoforge(minecraft_version) {
        bail!("NeoForge 需要 Minecraft 1.20.1 及以上的正式版");
    }

    let temp = ctx.temp_dir();
    if temp.exists() {
        std::fs::remove_dir_all(&temp).context("清理 loader 临时目录失败")?;
    }
    std::fs::create_dir_all(&temp).context("创建 loader 临时目录失败")?;
    let installer_path = temp.join("installer.jar");

    // 1. 下载 installer
    let url = installer_url(minecraft_version, loader_version);
    let installer_progress = ctx.progress(DownloadPhase::LoaderInstaller, 1, 0);
    crate::loader::download::download_file(&url, &installer_path, None, Some(&installer_progress))
        .await?;
    installer_progress.emit(String::new(), 0, 0, 0, true, false);

    // 2. 解析 install_profile.json
    let profile = parse_install_profile(&installer_path).await?;
    if let Some(declared) = &profile.minecraft {
        if declared != minecraft_version {
            bail!(
                "installer 面向 Minecraft {}，与所选 {} 不一致",
                declared,
                minecraft_version
            );
        }
    }

    // 3. 预下载安装期 Maven 依赖（installertools / srg / mappings 等）
    tracing::info!(
        "预下载 NeoForge 安装期依赖（{} 项）",
        profile.libraries.len()
    );
    let install_lib_count = profile
        .libraries
        .iter()
        .filter(|l| l.artifact().is_some())
        .count() as u64;
    let install_lib_bytes: u64 = profile
        .libraries
        .iter()
        .filter_map(|l| l.artifact().and_then(|a| a.size))
        .sum();
    let predownload_progress = ctx.progress(
        DownloadPhase::LoaderLibraries,
        install_lib_count,
        install_lib_bytes,
    );
    predownload_libraries(ctx, &profile.libraries, &predownload_progress).await?;
    predownload_progress.emit(String::new(), 0, 0, 0, true, false);

    // 4. 客户端副本 + 执行 processor
    let client_copy = temp.join("minecraft-client.jar");
    std::fs::copy(&ctx.client_jar, &client_copy)
        .with_context(|| format!("复制原版客户端失败: {}", ctx.client_jar.display()))?;
    let mut runner = ProcessorRunner::new(ctx, &profile, &installer_path, &temp, &client_copy)?;
    tracing::info!("开始执行 NeoForge 安装 processor");
    runner.run_all().await?;

    // 5. 读取 version.json 生成 patch
    let version_path = profile.json.as_deref().unwrap_or("/version.json");
    let bytes = read_zip_entry(&installer_path, version_path)?;
    let loader_json: LoaderVersionJson =
        serde_json::from_slice(&bytes).context("解析 installer version.json 失败")?;
    let main_class = loader_json
        .main_class
        .ok_or_else(|| anyhow!("installer version.json 缺少 mainClass"))?;

    let runtime_bytes: u64 = loader_json
        .libraries
        .iter()
        .filter_map(|l| {
            let a = &l.downloads.artifact;
            if a.size > 0 {
                Some(a.size)
            } else {
                None
            }
        })
        .sum();
    let runtime_progress = ctx.progress(
        DownloadPhase::LoaderLibraries,
        loader_json.libraries.len() as u64,
        runtime_bytes,
    );
    let mut libraries = Vec::new();
    for lib in loader_json.libraries {
        let normalized = ensure_runtime_library(ctx, &lib, &runtime_progress).await?;
        libraries.push(normalized);
    }
    runtime_progress.emit(String::new(), 0, 0, 0, true, false);

    let java_major = loader_json
        .java_version
        .as_ref()
        .and_then(|j| j.major_version);

    let (jvm_args, game_args) = match loader_json.arguments {
        Some(Arguments { jvm, game }) => (jvm, game),
        None => (Vec::new(), Vec::new()),
    };

    tracing::info!(
        mc = minecraft_version,
        loader = loader_version,
        "NeoForge 安装完成（{} 个运行时库）",
        libraries.len()
    );
    Ok(VersionPatch {
        main_class,
        libraries,
        jvm_args,
        game_args,
        java_major,
    })
}
