pub mod appfile;
pub mod config;
pub mod downloader;
pub mod instance;
pub mod log;
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

/// 下载指定版本的全部 Minecraft 文件，全程通过 minecraft-download-progress 事件向前端推送进度
#[tauri::command]
async fn download_minecraft_version(
    app: tauri::AppHandle,
    version_id: String,
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
    let downloader = MinecraftDownloader::new(app.clone(), config, cancel.clone());
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
                if let Err(e) = instance::create(&version_id) {
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

/// 获取当前主配置
#[tauri::command]
async fn get_main_config() -> Result<MainConfig, String> {
    Ok(MainConfig::get().await)
}

/// 修改主配置并持久化，仅更新传入的字段
#[tauri::command]
async fn set_main_config(
    accent_color: Option<u32>,
    download_concurrency: Option<u32>,
    mirror_url: Option<String>,
) -> Result<MainConfig, String> {
    MainConfig::update(|config| {
        if let Some(color) = accent_color {
            config.accent_color = color;
        }
        if let Some(concurrency) = download_concurrency {
            config.download_concurrency = concurrency.max(1);
        }
        if let Some(mirror) = mirror_url {
            config.mirror_url = if mirror.trim().is_empty() {
                None
            } else {
                Some(mirror.trim().to_string())
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
            // 启动时从本地缓存填充版本清单，避免离线时无法获取
            tauri::async_runtime::spawn(async {
                crate::downloader::provider::load_local_manifest().await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_color,
            get_minecraft_version, get_version,
            download_minecraft_version,
            cancel_minecraft_download,
            list_instances,
            get_main_config, set_main_config
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 框架时遇到错误");
}
