use futures_util::lock::{Mutex};
use std::{
    path::{PathBuf},
    sync::LazyLock,
};
use tokio::fs;
use tracing::{info, warn};
use anyhow::bail;
use crate::{
    appfile::{dirs::{self}, file}, downloader::{
        deserializer::{self, FOOL_VERSIONS, SingleVersion, VersionManifest}, net::fetch_and_parse_json, urls::VERSION_MANIFEST,
    },
};

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

pub async fn get_minecraft_version_paged(
    size_u: u32,
    page_u: u32,
    version_mode: VersionMode,
) -> anyhow::Result<VersionManifest> {
    // 1. 参数校验
    if size_u == 0 || page_u == 0 {
        bail!("参数错误: size_u 和 page_u 必须大于 0");
    }

    // 2. 获取数据
    let mut provider = VER_ALL.lock().await;
    let url: &str = VERSION_MANIFEST;
    let result = match fetch_and_parse_json::<deserializer::VersionManifest>(url).await {
        Ok(data) => data,
        Err(e) => {
            bail!("获取 version_manifest 失败: {}", e);
        }
    };
    provider.ver = Some(result);
    let path = PathBuf::new()
        .join(dirs::dot_minecraft()?)
        .join("version_manifest.json");
    if let Some(ref manifest) = provider.ver {
        let json_data = match serde_json::to_string_pretty(manifest) {
            Ok(data) => data,
            Err(e) => {
                bail!("VersionManifest 序列化失败: {}", e);
            }
        };
        if let Err(e) = fs::write(&path, json_data).await {
            bail!("写入磁盘失败: {}", e);
        } else {
            info!(path = ?path, "已成功写入 version_manifest.json");
        }
    }

    // 3. 克隆数据并处理
    let mani = provider.ver.clone().unwrap();
    let versions: Vec<SingleVersion> = mani.versions;

    // 4. 根据 version_mode 进行筛选
    let filtered_versions: Vec<SingleVersion> = versions
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

    // 5. 计算分页索引（基于筛选后的长度）
    let start: usize = (page_u - 1) as usize * size_u as usize;
    let end: usize = start + size_u as usize;
    let len: usize = filtered_versions.len();

    if start >= len {
        bail!("页数大于版本数");
    }

    let actual_end: usize = end.min(len);

    // 6. 返回分页结果
    Ok(VersionManifest {
        latest: mani.latest,
        versions: filtered_versions[start..actual_end].to_vec(),
    })
}
