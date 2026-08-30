pub mod appfile;
pub mod config;
pub mod downloader;
pub mod instance;
pub mod log;
pub mod profile;
pub mod runtime;
pub mod skin;
pub mod util;

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock,
    },
};

use crate::{
    config::MainConfig,
    downloader::{
        deserializer,
        minecraft::{DownloadFinished, MinecraftDownloader, DOWNLOAD_FINISHED_EVENT},
        net,
        provider::{get_minecraft_version_paged, VersionMode, VER_ALL},
    },
    util::theme_color_to_hex,
};
use futures_util::lock::Mutex;
use system_theme::SystemTheme;
use tauri::Emitter;
use tauri_plugin_tracing::LevelFilter;

/// 进行中的下载取消标志，key 为版本号
static DOWNLOAD_CANCELS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[tauri::command]
async fn get_system_color() -> anyhow::Result<String, String> {
    let theme = SystemTheme::new().map_err(|e| format!("获取系统主题失败: {}", e))?;
    let accent_color = theme_color_to_hex(
        theme
            .get_accent()
            .map_err(|e| format!("未找到系统强调色: {}", e))?,
    );
    tracing::debug!("系统强调色: {}", accent_color);
    Ok(accent_color)
}

/// 获取我的世界版本列表，默认一页 20 个版本，返回第 1 页
#[tauri::command]
async fn get_minecraft_version(
    size: Option<u32>,
    page: Option<u32>,
    version_mode: Option<VersionMode>,
) -> Result<deserializer::VersionManifest, tauri::Error> {
    let result = get_minecraft_version_paged(
        size.unwrap_or(20),
        page.unwrap_or(1),
        version_mode.unwrap_or(VersionMode::ALL),
    )
    .await?;
    Ok(result)
}

/// 获取 1.21.1 的版本 JSON
#[tauri::command]
async fn get_version() -> Result<deserializer::VersionContent, tauri::Error> {
    // 获取 1.21.1 版本 JSON URL
    let provider = VER_ALL.lock().await.ver.as_ref().ok_or_else(|| tauri::Error::Anyhow(anyhow::anyhow!("获取不到 version_manifest")))?.clone();
    for version in &provider.versions {
        if version.id == "1.21.1" {
            match net::fetch_and_parse_json::<deserializer::VersionContent>(
                &version.url.to_owned(),
            )
            .await
            {
                Ok(data) => return Ok(data),
                Err(e) => {
                    return Err(tauri::Error::Anyhow(anyhow::anyhow!("获取 1.21.1.json 失败: {}", e)));
                }
            };
        }
    }
    return Err(tauri::Error::Anyhow(anyhow::anyhow!("未找到 1.21.1 (这是一个未预料的错误！)")));
}

/// 全局 Toast 事件名，前端通过 listen("toast", ...) 被动接收
pub const TOAST_EVENT: &str = "toast";

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ToastPayload {
    pub level: String,
    pub title: String,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
}

/// 下载指定版本的全部 Minecraft 文件并建立实例，全程通过 minecraft-download-progress 事件向前端推送进度
#[tauri::command]
async fn download_minecraft_version(
    app: tauri::AppHandle,
    version_id: String,
    instance_name: Option<String>,
    instance_config: Option<instance::InstanceConfig>,
) -> Result<(), String> {
    let _ = app.emit(
        TOAST_EVENT,
        &ToastPayload {
            level: "info".into(),
            title: format!("开始下载 {}", version_id),
            message: None,
            kind: Some("download".into()),
            version_id: Some(version_id.clone()),
        },
    );

    let cancel = Arc::new(AtomicBool::new(false));
    DOWNLOAD_CANCELS.lock().await.insert(version_id.clone(), cancel.clone());

    let config = MainConfig::get().await;
    let defaults = config.instance_defaults();
    let raw_dir = instance_name.clone().unwrap_or_else(|| version_id.clone());
    let dir_name = {
        let s = instance::sanitize_dir_name(&raw_dir);
        if s.is_empty() { version_id.clone() } else { s }
    };
    let downloader = MinecraftDownloader::new(app.clone(), config, cancel.clone(), dir_name.clone());
    let result = downloader.download_version(&version_id).await;
    let cancelled = cancel.load(Ordering::Relaxed);
    DOWNLOAD_CANCELS.lock().await.remove(&version_id);

    let (success, error, toast) = if cancelled {
        (
            false,
            Some("下载已取消".into()),
            ToastPayload {
                level: "warning".into(),
                title: format!("下载已取消 {}", version_id),
                message: Some("下载已取消".into()),
                kind: Some("download".into()),
                version_id: Some(version_id.clone()),
            },
        )
    } else {
        match result {
            Ok(_) => {
                if let Err(e) = instance::create(&version_id, instance_name, instance_config, &defaults, &dir_name) {
                    tracing::warn!("创建实例失败: {}", e);
                }
                (
                    true,
                    None,
                    ToastPayload {
                        level: "success".into(),
                        title: format!("下载完成 {}", version_id),
                        message: Some("下载完成".into()),
                        kind: Some("download".into()),
                        version_id: Some(version_id.clone()),
                    },
                )
            }
            Err(e) => {
                tracing::error!("下载 {} 失败: {}", version_id, e);
                (
                    false,
                    Some(e.to_string()),
                    ToastPayload {
                        level: "error".into(),
                        title: format!("下载失败 {}", version_id),
                        message: Some(e.to_string()),
                        kind: Some("download".into()),
                        version_id: Some(version_id.clone()),
                    },
                )
            }
        }
    };
    let _ = app.emit(TOAST_EVENT, &toast);
    let _ = app.emit(
        DOWNLOAD_FINISHED_EVENT,
        &DownloadFinished {
            version_id: version_id.clone(),
            success,
            error: error.clone(),
        },
    );
    match error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// 取消指定版本的下载
#[tauri::command]
async fn cancel_minecraft_download(version_id: String) -> Result<(), String> {
    let map = DOWNLOAD_CANCELS.lock().await;
    if let Some(flag) = map.get(&version_id) {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

/// 获取全部已创建的 Minecraft 实例
#[tauri::command]
async fn list_instances() -> Result<Vec<instance::MinecraftInstance>, String> {
    instance::list().map_err(|e| e.to_string())
}

/// 获取单个实例的完整信息（含配置），不存在返回 null
#[tauri::command]
async fn get_instance(version_id: String) -> Result<Option<instance::InstanceInfo>, String> {
    instance::get(&version_id).map_err(|e| e.to_string())
}

/// 更新实例的名称与配置
#[tauri::command]
async fn update_instance(
    version_id: String,
    name: Option<String>,
    config: Option<instance::InstanceConfig>,
) -> Result<instance::InstanceInfo, String> {
    let defaults = MainConfig::get().await.instance_defaults();
    instance::update(&version_id, name, config, &defaults).map_err(|e| e.to_string())
}

/// 列出全部游戏档案
#[tauri::command]
async fn list_game_profiles() -> Result<Vec<profile::GameProfile>, String> {
    profile::list().map_err(|e| e.to_string())
}

/// 获取单个游戏档案，不存在返回 null
#[tauri::command]
async fn get_game_profile(id: String) -> Result<Option<profile::GameProfile>, String> {
    profile::get(&id).map_err(|e| e.to_string())
}

/// 创建游戏档案（目前仅实现离线登录，其它登录方式留入口）
#[tauri::command]
async fn create_game_profile(
    auth_type: String,
    username: Option<String>,
) -> Result<profile::GameProfile, String> {
    profile::create(&auth_type, username.as_deref()).map_err(|e| e.to_string())
}

/// 删除游戏档案（若为默认/当前档案则同步清空）
#[tauri::command]
async fn delete_game_profile(id: String) -> Result<(), String> {
    profile::delete(&id).await.map_err(|e| e.to_string())
}

/// 获取当前激活的游戏档案
#[tauri::command]
async fn get_current_profile() -> Result<Option<profile::GameProfile>, String> {
    Ok(profile::get_current().await)
}

/// 获取玩家皮肤的脸部头像（Base64 data URL），查不到皮肤/玩家时返回 null
#[tauri::command]
async fn get_profile_avatar(username: String) -> Result<Option<String>, String> {
    skin::get_profile_avatar(&username)
        .await
        .map_err(|e| e.to_string())
}

/// 设置默认游戏档案（并立即切换当前档案）；None/空串表示清除
#[tauri::command]
async fn set_default_profile(profile_id: Option<String>) -> Result<(), String> {
    let id = profile_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let current = match &id {
        Some(id) => Some(
            profile::get(id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "游戏档案不存在".to_string())?,
        ),
        None => None,
    };
    MainConfig::update(|config| {
        config.default_profile_id = id;
    })
    .await
    .map_err(|e| e.to_string())?;
    profile::set_current(current).await;
    Ok(())
}

/// 获取最后一次启动的实例记录（可能为 null）
#[tauri::command]
async fn get_last_launched_instance() -> Result<Option<runtime::LastLaunchedInstance>, String> {
    runtime::get_last_launched().await.map_err(|e| e.to_string())
}

/// 记录最后一次启动的实例（供未来启动模块调用）
#[tauri::command]
async fn record_last_launched_instance(
    version_id: String,
    name: String,
    dir: String,
) -> Result<(), String> {
    runtime::record_last_launched(runtime::LastLaunchedInstance {
        version_id,
        name,
        dir,
    })
    .await
    .map_err(|e| e.to_string())
}

/// 获取实例目录下的自定义图标（data URL），不存在返回 null
#[tauri::command]
async fn get_instance_icon(dir_name: String) -> Result<Option<String>, String> {
    instance::get_instance_icon(&dir_name).map_err(|e| e.to_string())
}

/// 获取当前主配置
#[tauri::command]
async fn get_main_config() -> Result<MainConfig, String> {
    Ok(MainConfig::get().await)
}

/// 主配置的局部更新字段
#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct MainConfigUpdate {
    accent_color: Option<u32>,
    download_concurrency: Option<u32>,
    mirror_url: Option<String>,
    default_width: Option<u32>,
    default_height: Option<u32>,
    default_jvm_args: Option<Vec<String>>,
    default_game_args: Option<Vec<String>>,
    default_launch_command_prefix: Option<Vec<String>>,
    default_launch_command_suffix: Option<Vec<String>>,
    default_profile_id: Option<String>,
}

/// 修改主配置并持久化，仅更新传入的字段
#[tauri::command]
async fn set_main_config(update: MainConfigUpdate) -> Result<MainConfig, String> {
    MainConfig::update(|config| {
        if let Some(color) = update.accent_color {
            config.accent_color = color;
        }
        if let Some(concurrency) = update.download_concurrency {
            config.download_concurrency = concurrency.max(1);
        }
        if let Some(mirror) = update.mirror_url {
            config.mirror_url = if mirror.trim().is_empty() {
                None
            } else {
                Some(mirror.trim().to_string())
            };
        }
        if let Some(width) = update.default_width {
            config.default_width = width.max(1);
        }
        if let Some(height) = update.default_height {
            config.default_height = height.max(1);
        }
        if let Some(args) = update.default_jvm_args {
            config.default_jvm_args = args;
        }
        if let Some(args) = update.default_game_args {
            config.default_game_args = args;
        }
        if let Some(args) = update.default_launch_command_prefix {
            config.default_launch_command_prefix = args;
        }
        if let Some(args) = update.default_launch_command_suffix {
            config.default_launch_command_suffix = args;
        }
        if let Some(profile) = update.default_profile_id {
            config.default_profile_id = if profile.trim().is_empty() {
                None
            } else {
                Some(profile.trim().to_string())
            };
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_tracing::Builder::new()
                .with_max_level(LevelFilter::DEBUG)
                .build(),
        )
        .setup(|_app| {
            // 启动时从本地缓存填充版本清单，避免离线时无法获取；并确保生成默认 config.json、切换默认游戏档案
            tauri::async_runtime::spawn(async {
                crate::downloader::provider::load_local_manifest().await;
                let _ = MainConfig::get().await;
                if let Err(e) = profile::init_current().await {
                    tracing::warn!("初始化当前游戏档案失败: {}", e);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_color,
            get_minecraft_version, get_version,
            download_minecraft_version,
            cancel_minecraft_download,
            list_instances, get_instance, update_instance,
            list_game_profiles, get_game_profile, create_game_profile, delete_game_profile,
            get_current_profile, set_default_profile,
            get_profile_avatar,
            get_last_launched_instance, record_last_launched_instance,
            get_instance_icon,
            get_main_config, set_main_config
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 框架时遇到错误");
}
