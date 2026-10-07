use crate::{shell, AppState, ShortcutStatus};
use tauri_plugin_opener::OpenerExt;

const PRIVACY_POLICY_URL: &str = "https://focus.proanima.net/privacy";

#[tauri::command]
pub(crate) fn open_privacy_policy(app: tauri::AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(PRIVACY_POLICY_URL, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn toggle_overlay_layer(app: tauri::AppHandle) -> Result<bool, String> {
    shell::toggle_layer(&app).ok_or_else(|| "overlay window not found".to_string())
}

#[tauri::command]
pub(crate) fn get_shortcut_status(state: tauri::State<AppState>) -> Option<ShortcutStatus> {
    state.shortcut_status.lock().ok()?.clone()
}

// UI прячет desktop-специфичные элементы (always-on-top переключатель) на
// платформах, где им нет соответствия — раздел 11 ТЗ, Android-путь другой.
#[tauri::command]
pub(crate) fn is_desktop_platform() -> bool {
    cfg!(desktop)
}
