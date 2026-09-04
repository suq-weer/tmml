use crate::{
    appfile::{
        dirs::{self},
        file,
    },
    downloader::{
        deserializer::{self, LatestVersion, SingleVersion, VersionManifest, FOOL_VERSIONS},
        net::fetch_and_parse_json,
        urls::VERSION_MANIFEST,
    },
};
use anyhow::{anyhow, bail};
use futures_util::lock::Mutex;
use std::{path::PathBuf, sync::LazyLock};
use tokio::fs;
use tracing::{info, warn};

pub struct VersionManifestProvider {
    pub ver: Option<VersionManifest>,
}

impl VersionManifestProvider {
    pub fn default() -> Self {
        Self { ver: None }
    }
}

/// 我的世界版本类型
#[derive(serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum VersionMode {
    ALL,
    RELEASE,
    SNAPSHOT,
    FOOL,
}

pub static VER_ALL: LazyLock<Mutex<VersionManifestProvider>> =
    LazyLock::new(|| Mutex::new(VersionManifestProvider::default()));

/// 程序启动时从本地 version_manifest.json 读取已缓存的数据并填充 VER_ALL（失败时静默忽略）
pub async fn load_local_manifest() {
    let mut provider = VER_ALL.lock().await;
    if provider.ver.is_some() {
        return;
    }
    let Ok(path) = dirs::dot_minecraft().map(|dir| dir.join("version_manifest.json")) else {
        return;
    };
    if !path.exists() {
        return;
    }
    match file::read_and_parse_json::<deserializer::VersionManifest>(path.clone()) {
        Ok(manifest) => {
            info!(path = ?path, "已从本地加载 version_manifest.json");
            provider.ver = Some(manifest);
        }
        Err(e) => {
            warn!(path = ?path, "读取本地 version_manifest.json 失败: {}", e);
        }
    }
}

/// 分页返回的版本切片及其分页元信息，供前端判断是否还能继续「加载更多」
#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionPage {
    pub latest: LatestVersion,
    /// 本页返回的版本条目
    pub versions: Vec<SingleVersion>,
    /// 当前页码（从 1 开始）
    pub page: u32,
    /// 每页条数
    pub size: u32,
    /// 满足筛选条件的版本总数
    pub total: usize,
    /// 总页数（没有匹配版本时为 0）
    pub total_pages: u32,
    /// 是否还有下一页
    pub has_more: bool,
}

/// 联网拉取最新 version_manifest，写入内存缓存并落盘一份副本
async fn refresh_manifest() -> anyhow::Result<deserializer::VersionManifest> {
    let data = fetch_and_parse_json::<deserializer::VersionManifest>(VERSION_MANIFEST)
        .await
        .map_err(|e| anyhow!("获取 version_manifest 失败: {}", e))?;
    {
        let mut provider = VER_ALL.lock().await;
        provider.ver = Some(data.clone());
    }
    let path = PathBuf::new()
        .join(dirs::dot_minecraft()?)
        .join("version_manifest.json");
    match serde_json::to_string_pretty(&data) {
        Ok(json_data) => {
            if let Err(e) = fs::write(&path, json_data).await {
                warn!("写入 version_manifest.json 失败: {}", e);
            } else {
                info!(path = ?path, "已成功写入 version_manifest.json");
            }
        }
        Err(e) => warn!("VersionManifest 序列化失败: {}", e),
    }
    Ok(data)
}

pub async fn get_minecraft_version_paged(
    size_u: u32,
    page_u: u32,
    version_mode: VersionMode,
) -> anyhow::Result<VersionPage> {
    // 1. 参数校验
    if size_u == 0 || page_u == 0 {
        bail!("参数错误: size 和 page 必须大于 0");
    }

    // 2. 获取数据：优先复用内存缓存，避免「加载更多」时反复联网拉取同一份清单。
    //    若每次分页都基于新下载的清单切片，版本可能在两次请求间插入/移除，
    //    导致前端累加时出现重复或遗漏。
    let manifest = {
        let provider = VER_ALL.lock().await;
        match provider.ver.clone() {
            Some(mani) => mani,
            None => refresh_manifest().await?,
        }
    };

    // 3. 根据 version_mode 进行筛选
    let filtered_versions: Vec<SingleVersion> = manifest
        .versions
        .into_iter()
        .filter(|v| match version_mode {
            VersionMode::ALL => true,
            VersionMode::RELEASE => v.version_type == "release",
            VersionMode::SNAPSHOT => v.version_type == "snapshot",
            VersionMode::FOOL => {
                // 必须是 snapshot 类型，且 id 完全等于数组中的某一项
                v.version_type == "snapshot" && FOOL_VERSIONS.contains(&v.id.as_str())
            }
        })
        .collect();

    // 4. 计算分页索引（基于筛选后的长度）
    let len: usize = filtered_versions.len();
    let total_pages: u32 = if len == 0 {
        0
    } else {
        ((len as u64 + size_u as u64 - 1) / size_u as u64).min(u32::MAX as u64) as u32
    };
    let start: usize = (page_u - 1) as usize * size_u as usize;

    // 请求的页码已超过最后一页：返回空页而非报错，前端据此结束「加载更多」
    if start >= len {
        return Ok(VersionPage {
            latest: manifest.latest,
            versions: Vec::new(),
            page: page_u,
            size: size_u,
            total: len,
            total_pages,
            has_more: false,
        });
    }

    let end: usize = (start + size_u as usize).min(len);

    // 5. 返回分页结果
    Ok(VersionPage {
        latest: manifest.latest,
        versions: filtered_versions[start..end].to_vec(),
        page: page_u,
        size: size_u,
        total: len,
        total_pages,
        has_more: page_u < total_pages,
    })
}
