pub mod appfile;
pub mod config;
pub mod downloader;
pub mod log;
pub mod util;

use crate::{
    downloader::{
        deserializer, net,
        provider::{get_minecraft_version_paged, VersionMode, VER_ALL},
    },
    util::theme_color_to_hex,
};
use system_theme::SystemTheme;
use tauri_plugin_tracing::LevelFilter;

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
        .invoke_handler(tauri::generate_handler![
            get_system_color,
            get_minecraft_version, get_version
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 框架时遇到错误");
}
