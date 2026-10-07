use super::{audio_dir, trigger_server_sync};
use crate::{db, notes, profiles, server_sync, sync_log, AudioKeyState};

#[tauri::command]
pub(crate) fn list_notes(db: tauri::State<db::Db>) -> Result<Vec<notes::NoteDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    notes::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn list_note_groups(
    db: tauri::State<db::Db>,
) -> Result<Vec<notes::NoteGroupDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    notes::list_groups(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn create_note_group(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    name: String,
) -> Result<notes::NoteGroupDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let group = notes::create_group(&mut conn, &mut clock, &profile_id, &name)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(group)
}

#[tauri::command]
pub(crate) fn create_note(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    body: String,
    group_id: Option<String>,
) -> Result<notes::NoteDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let note = notes::create(
        &mut conn,
        &mut clock,
        &profile_id,
        &body,
        group_id.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(note)
}

#[tauri::command]
pub(crate) fn move_note_to_group(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
    group_id: Option<String>,
) -> Result<notes::NoteDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let note = notes::move_to_group(&mut conn, &mut clock, &profile_id, &id, group_id.as_deref())
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(note)
}

#[tauri::command]
pub(crate) fn update_note(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
    body: String,
) -> Result<notes::NoteDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let note = notes::update_body(&mut conn, &mut clock, &profile_id, &id, &body)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(note)
}

#[tauri::command]
pub(crate) fn delete_note(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<(), String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let dir = audio_dir(&profiles_state);
    notes::delete(&mut conn, &mut clock, &profile_id, &dir, &id)?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(())
}

#[tauri::command]
pub(crate) fn create_audio_note(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    audio_key_state: tauri::State<AudioKeyState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    audio_base64: String,
    group_id: Option<String>,
) -> Result<notes::NoteDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let dir = audio_dir(&profiles_state);
    let audio_key = audio_key_state.0.lock().map_err(|e| e.to_string())?;
    let note = notes::create_audio(
        &mut conn,
        &mut clock,
        &profile_id,
        &dir,
        audio_key.as_deref(),
        &audio_base64,
        group_id.as_deref(),
    )?;
    drop(audio_key);
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(note)
}

#[tauri::command]
pub(crate) async fn get_note_audio(
    db: tauri::State<'_, db::Db>,
    audio_key_state: tauri::State<'_, AudioKeyState>,
    profiles_state: tauri::State<'_, profiles::ProfilesState>,
    id: String,
) -> Result<String, String> {
    let dir = audio_dir(&profiles_state);
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let audio_key = audio_key_state.0.lock().map_err(|e| e.to_string())?.clone();
    let filename = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        notes::audio_filename(&conn, &id)?
    };
    server_sync::ensure_audio_blob_downloaded(
        &db,
        &profile_id,
        &dir,
        audio_key.as_deref(),
        &filename,
    )
    .await?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    notes::read_audio(&conn, &dir, audio_key.as_deref(), &id)
}
