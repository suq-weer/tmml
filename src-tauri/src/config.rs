use std::{sync::LazyLock};
use futures_util::lock::Mutex;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct MainConfig {
    accent_color: u32,
}

impl MainConfig {
    pub fn default() -> Self {
        Self { accent_color: 0x4a92cb }
    }
}

static MAIN_CONFIG: LazyLock<Mutex<MainConfig>> = LazyLock::new(|| Mutex::new(MainConfig::default()));
