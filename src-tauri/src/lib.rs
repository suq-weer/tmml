pub mod downloader;
pub mod frontend_logger;
pub mod log;
pub mod util;

use crate::{downloader::{deserializer::VersionManifest, provider::{VersionMode, get_minecraft_version_paged}}, util::theme_color_to_hex};
use frontend_logger::{debug_f, error_f, info_f, warn_f};
use system_theme::SystemTheme;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn get_system_color() -> Result<String, String> {
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
async fn get_minecraft_version(size: Option<u32>, page: Option<u32>, version_mode: Option<VersionMode>) -> Result<VersionManifest, String> {
    let result = get_minecraft_version_paged(size.unwrap_or(20), page.unwrap_or(1), version_mode.unwrap_or(VersionMode::ALL)).await.unwrap();
    return Ok(result);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_system_color,
            get_minecraft_version,
            info_f, warn_f, debug_f, error_f
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 框架时遇到错误……");
}
