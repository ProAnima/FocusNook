pub(crate) mod diagnostics;
pub(crate) mod notes;
pub(crate) mod overlay;
pub(crate) mod plan_items;
pub(crate) mod profiles;
pub(crate) mod reminders;

use crate::server_sync;

pub(crate) fn trigger_server_sync(app: &tauri::AppHandle) {
    server_sync::spawn_best_effort(app.clone());
    #[cfg(feature = "cloud-providers")]
    crate::cloud_sync::spawn_best_effort(app.clone());
}

/// Каталог аудио активного профиля; при ошибке — `audio` в каталоге данных.
fn audio_dir(profiles_state: &tauri::State<crate::profiles::ProfilesState>) -> std::path::PathBuf {
    match crate::profiles::audio_dir(profiles_state) {
        Ok(path) => path,
        Err(_) => crate::profiles::data_dir(profiles_state).join("audio"),
    }
}
