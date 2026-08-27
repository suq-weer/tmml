use std::sync::LazyLock;

use crate::appfile::dirs;
use anyhow::Result;
use futures_util::lock::Mutex;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct MainConfig {
    pub accent_color: u32,
    #[serde(default)]
    pub download_concurrency: u32,
    #[serde(default)]
    pub mirror_url: Option<String>,
}

impl Default for MainConfig {
    fn default() -> Self {
        Self {
            accent_color: 0x4a92cb,
            download_concurrency: 16,
            mirror_url: None,
        }
    }
}

struct ConfigState {
    config: MainConfig,
    loaded: bool,
}

static STATE: LazyLock<Mutex<ConfigState>> =
    LazyLock::new(|| Mutex::new(ConfigState { config: MainConfig::default(), loaded: false }));

fn config_file() -> Result<std::path::PathBuf> {
    Ok(dirs::config()?.join("config.json"))
}

fn load_from_disk() -> Result<MainConfig> {
    let path = config_file()?;
    if !path.exists() {
        return Ok(MainConfig::default());
    }
    let data = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&data)?)
}

fn save_to_disk(config: &MainConfig) -> Result<()> {
    let path = config_file()?;
    let data = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, data)?;
    Ok(())
}

impl MainConfig {
    /// 获取当前配置（首次调用时会从磁盘读取并缓存）
    pub async fn get() -> MainConfig {
        let mut guard = STATE.lock().await;
        if !guard.loaded {
            match load_from_disk() {
                Ok(config) => guard.config = config,
                Err(e) => tracing::warn!("读取配置文件失败，使用默认配置: {}", e),
            }
            guard.loaded = true;
        }
        guard.config.clone()
    }

    /// 修改配置并持久化到磁盘
    pub async fn update(f: impl FnOnce(&mut MainConfig)) -> Result<MainConfig> {
        let mut guard = STATE.lock().await;
        if !guard.loaded {
            match load_from_disk() {
                Ok(config) => guard.config = config,
                Err(e) => tracing::warn!("读取配置文件失败，使用默认配置: {}", e),
            }
            guard.loaded = true;
        }
        f(&mut guard.config);
        save_to_disk(&guard.config)?;
        Ok(guard.config.clone())
    }
}
