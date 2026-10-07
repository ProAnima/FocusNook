use super::audio_dir;
use super::trigger_server_sync;
use crate::alarms::{cancel_android_alarm, schedule_android_alarm};
use crate::{alerts, db, profiles, reminders, server_sync, sync_log, AudioKeyState};
use serde::Deserialize;
use tauri::Emitter;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateAudioReminderRequest {
    title: String,
    trigger_at_utc: String,
    audio_base64: String,
}

#[tauri::command]
pub(crate) fn list_reminders(
    db: tauri::State<db::Db>,
) -> Result<Vec<reminders::ReminderDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    reminders::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn create_reminder(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    title: String,
    trigger_at_utc: String,
) -> Result<reminders::ReminderDto, String> {
    let reminder = {
        let mut conn = db.0.lock().map_err(|e| e.to_string())?;
        let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
        let profile_id = profiles::active_profile_id(&profiles_state)?;
        reminders::create(&mut conn, &mut clock, &profile_id, &title, &trigger_at_utc)
            .map_err(|e| e.to_string())?
    };
    schedule_android_alarm(&app, &reminder);
    trigger_server_sync(&app);
    Ok(reminder)
}

#[tauri::command]
pub(crate) fn create_audio_reminder(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    audio_key_state: tauri::State<AudioKeyState>,
    request: CreateAudioReminderRequest,
) -> Result<reminders::ReminderDto, String> {
    let reminder = {
        let mut conn = db.0.lock().map_err(|e| e.to_string())?;
        let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
        let profile_id = profiles::active_profile_id(&profiles_state)?;
        let audio_key = audio_key_state.0.lock().map_err(|e| e.to_string())?;
        reminders::create_audio(
            &mut conn,
            &mut clock,
            reminders::CreateAudioReminder {
                profile_id: &profile_id,
                audio_dir: &audio_dir(&profiles_state),
                audio_key: audio_key.as_deref(),
                title: &request.title,
                trigger_at_utc: &request.trigger_at_utc,
                base64_data: &request.audio_base64,
            },
        )?
    };
    schedule_android_alarm(&app, &reminder);
    trigger_server_sync(&app);
    Ok(reminder)
}

#[tauri::command]
pub(crate) fn get_current_alert(
    state: tauri::State<alerts::AlertState>,
) -> Option<reminders::ReminderDto> {
    alerts::current_alert(&state)
}

#[tauri::command]
pub(crate) async fn get_reminder_audio(
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
        reminders::audio_filename(&conn, &id)?
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
    reminders::read_audio(&conn, &dir, audio_key.as_deref(), &id)
}

#[tauri::command]
pub(crate) fn acknowledge_reminder(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<(), String> {
    {
        let dir = audio_dir(&profiles_state);
        let mut conn = db.0.lock().map_err(|e| e.to_string())?;
        let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
        let profile_id = profiles::active_profile_id(&profiles_state)?;
        reminders::delete(&mut conn, &mut clock, &profile_id, &dir, &id)?;
    }
    cancel_android_alarm(&app, &id);
    let _ = app.emit("reminders-changed", ());
    alerts::resolve_current_alert(&app);
    trigger_server_sync(&app);
    Ok(())
}

#[tauri::command]
pub(crate) fn snooze_reminder(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
    new_trigger_at_utc: String,
) -> Result<(), String> {
    let reminder = {
        let mut conn = db.0.lock().map_err(|e| e.to_string())?;
        let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
        let profile_id = profiles::active_profile_id(&profiles_state)?;
        reminders::reschedule(&mut conn, &mut clock, &profile_id, &id, &new_trigger_at_utc)
            .map_err(|e| e.to_string())?
    };
    schedule_android_alarm(&app, &reminder);
    let _ = app.emit("reminders-changed", ());
    alerts::resolve_current_alert(&app);
    trigger_server_sync(&app);
    Ok(())
}

#[tauri::command]
pub(crate) fn delete_reminder(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<(), String> {
    {
        let dir = audio_dir(&profiles_state);
        let mut conn = db.0.lock().map_err(|e| e.to_string())?;
        let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
        let profile_id = profiles::active_profile_id(&profiles_state)?;
        reminders::delete(&mut conn, &mut clock, &profile_id, &dir, &id)?;
    }
    cancel_android_alarm(&app, &id);
    let _ = app.emit("reminders-changed", ());
    trigger_server_sync(&app);
    Ok(())
}
