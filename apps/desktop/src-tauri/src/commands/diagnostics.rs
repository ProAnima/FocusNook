use crate::{alerts, db, diagnostics, profiles, sync_status};

// Раздел 19 ТЗ: "user export diagnostics bundle без пользовательского
// содержимого" — пишем JSON-файл в data_dir и возвращаем путь, чтобы
// фронтенд мог показать пользователю, куда сохранилось (без файлового
// save-диалога — новой Tauri-зависимости ради этого не заводили).
#[tauri::command]
pub(crate) fn export_diagnostics(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    alert_state: tauri::State<alerts::AlertState>,
) -> Result<String, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let profiles_response = profiles::list(&profiles_state)?;
    let bundle = diagnostics::build(
        &conn,
        &app.package_info().version.to_string(),
        profiles_response.profiles.len(),
        &profiles_response.active_profile_id,
        &alert_state,
    )?;
    let json = serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())?;
    let filename = format!(
        "diagnostics-{}.json",
        bundle.generated_at.replace([':', ' '], "-")
    );
    let path = profiles::data_dir(&profiles_state).join(filename);
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub(crate) fn sync_readiness_status(
    db: tauri::State<db::Db>,
    profiles_state: tauri::State<profiles::ProfilesState>,
) -> Result<sync_status::SyncReadinessStatus, String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    sync_status::build(&conn, &profile_id)
}
