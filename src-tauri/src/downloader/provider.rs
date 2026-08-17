use std::sync::LazyLock;

use futures_util::lock::Mutex;


use crate::downloader::{deserializer::{self, FOOL_VERSIONS, SingleVersion, VersionManifest}, net::fetch_and_parse_json, urls::VERSION_MANIFEST};

struct VersionManifestProvider {
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
    FOOL
}

static VER_ALL: LazyLock<Mutex<VersionManifestProvider>> = LazyLock::new(|| {
    Mutex::new(VersionManifestProvider::default())
});

pub async fn get_minecraft_version_paged(
    size_u: u32, 
    page_u: u32, 
    version_mode: VersionMode
) -> Result<VersionManifest, String> {
    // 1. 参数校验
    if size_u == 0 || page_u == 0 {
        return Err("{}".to_owned());
    }

    // 2. 获取缓存数据
    let mut provider = VER_ALL.lock().await;
    if provider.ver.is_none() {
        let url = VERSION_MANIFEST;
        let result = fetch_and_parse_json::<deserializer::VersionManifest>(url).await.expect("{}");
        provider.ver = Some(result);
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
            },
        })
        .collect();

    // 5. 计算分页索引（基于筛选后的长度）
    let start = (page_u - 1) as usize * size_u as usize;
    let end = start + size_u as usize;
    let len = filtered_versions.len(); 

    if start >= len {
        return Err("{}".to_owned());
    }

    let actual_end = end.min(len);

    // 6. 返回分页结果
    Ok(VersionManifest {
        latest: mani.latest,
        versions: filtered_versions[start..actual_end].to_vec()
    })
}
