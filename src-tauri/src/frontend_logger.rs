use tracing::{debug, error, info, warn};

#[tauri::command]
pub fn info_f(text: &str) {
    info!("{}", text)
}

#[tauri::command]
pub fn debug_f(text: &str) {
    debug!("{}", text);
}

#[tauri::command]
pub fn warn_f(text: &str) {
    warn!("{}", text)
}

#[tauri::command]
pub fn error_f(text: &str) {
    error!("{}", text)
}
