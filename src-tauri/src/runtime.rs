use std::{fs, path::PathBuf, sync::LazyLock};

use anyhow::{Context, Result};
use futures_util::lock::Mutex;

use crate::appfile::dirs;

/// 最后一次启动的实例记录
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LastLaunchedInstance {
    pub version_id: String,
    pub name: String,
    /// 实例所在目录名（用于查找实例图标）
    pub dir: String,
}

/// 启动器运行时状态（与 config.json 同级，tmml_runtime.json）
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RuntimeState {
    pub last_launched_instance: Option<LastLaunchedInstance>,
}

struct RuntimeStateCell {
    state: RuntimeState,
    loaded: bool,
}

static STATE: LazyLock<Mutex<RuntimeStateCell>> =
    LazyLock::new(|| Mutex::new(RuntimeStateCell { state: RuntimeState::default(), loaded: false }));

fn runtime_file() -> Result<PathBuf> {
    Ok(dirs::config()?.join("tmml_runtime.json"))
}

fn load_from_disk() -> Result<RuntimeState> {
    let path = runtime_file()?;
    if !path.exists() {
        return Ok(RuntimeState::default());
    }
    let data = fs::read_to_string(&path)?;
    serde_json::from_str(&data).context("解析 tmml_runtime.json 失败")
}

fn save_to_disk(state: &RuntimeState) -> Result<()> {
    let path = runtime_file()?;
    let data = serde_json::to_string_pretty(state)?;
    fs::write(&path, data)?;
    Ok(())
}

async fn ensure_loaded() -> Result<()> {
    let mut guard = STATE.lock().await;
    if !guard.loaded {
        match load_from_disk() {
            Ok(state) => guard.state = state,
            Err(e) => tracing::warn!("读取 tmml_runtime.json 失败: {}", e),
        }
        guard.loaded = true;
    }
    Ok(())
}

/// 获取最后一次启动的实例记录（可能为 None）
pub async fn get_last_launched() -> Result<Option<LastLaunchedInstance>> {
    ensure_loaded().await?;
    let guard = STATE.lock().await;
    Ok(guard.state.last_launched_instance.clone())
}

/// 记录最后一次启动的实例（供未来启动模块调用）
pub async fn record_last_launched(record: LastLaunchedInstance) -> Result<()> {
    let mut guard = STATE.lock().await;
    if !guard.loaded {
        match load_from_disk() {
            Ok(state) => guard.state = state,
            Err(e) => tracing::warn!("读取 tmml_runtime.json 失败: {}", e),
        }
        guard.loaded = true;
    }
    guard.state.last_launched_instance = Some(record);
    save_to_disk(&guard.state)?;
    Ok(())
}
