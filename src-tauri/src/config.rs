use std::sync::LazyLock;

use crate::appfile::dirs;
use crate::instance::InstanceConfig;
use anyhow::Result;
use futures_util::lock::Mutex;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct MainConfig {
    pub accent_color: u32,
    #[serde(default)]
    pub download_concurrency: u32,
    #[serde(default)]
    pub mirror_url: Option<String>,
    /// 全局默认实例分辨率宽
    #[serde(default)]
    pub default_width: u32,
    /// 全局默认实例分辨率高
    #[serde(default)]
    pub default_height: u32,
    #[serde(default)]
    pub default_jvm_args: Vec<String>,
    #[serde(default)]
    pub default_game_args: Vec<String>,
    #[serde(default)]
    pub default_launch_command_prefix: Vec<String>,
    #[serde(default)]
    pub default_launch_command_suffix: Vec<String>,
    /// 默认游戏档案 id（启动时自动切换到此档案；空表示未设置）
    #[serde(default)]
    pub default_profile_id: Option<String>,
}

impl Default for MainConfig {
    fn default() -> Self {
        Self {
            accent_color: 0x4a92cb,
            download_concurrency: 16,
            mirror_url: None,
            default_width: 800,
            default_height: 600,
            default_jvm_args: Vec::new(),
            default_game_args: Vec::new(),
            default_launch_command_prefix: Vec::new(),
            default_launch_command_suffix: Vec::new(),
            default_profile_id: None,
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
        // 首次启动无配置文件时，直接生成默认配置
        let defaults = MainConfig::default();
        save_to_disk(&defaults)?;
        return Ok(defaults);
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

    /// 由全局默认配置构造实例默认配置（作为实例配置的托底）
    pub fn instance_defaults(&self) -> InstanceConfig {
        InstanceConfig {
            launch_command_prefix: self.default_launch_command_prefix.clone(),
            launch_command_suffix: self.default_launch_command_suffix.clone(),
            jvm_args: self.default_jvm_args.clone(),
            game_args: self.default_game_args.clone(),
            width: Some(self.default_width),
            height: Some(self.default_height),
        }
    }
}
