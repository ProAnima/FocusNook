mod alerts;
mod audio_crypto;
mod blob_crypto;
mod config;
mod db;
mod diagnostics;
mod notes;
mod oauth;
mod plan_items;
mod profiles;
mod reminders;
mod server_sync;
mod sync;
mod sync_blobs;
mod sync_log;
mod sync_status;
mod sync_tokens;
mod window_state;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(desktop)]
use tauri::image::Image;
#[cfg(desktop)]
use tauri::menu::{Menu, MenuItem};
#[cfg(desktop)]
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, PhysicalPosition};
#[cfg(desktop)]
use tauri_plugin_autostart::MacosLauncher;
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_reminder_alarm::{CancelRequest, ReminderAlarmExt, ScheduleRequest};

#[cfg(desktop)]
const DEFAULT_SHORTCUT: &str = "ctrl+shift+v";
#[cfg(desktop)]
const FALLBACK_SHORTCUT: &str = "ctrl+alt+space";
const BOUNDS_SETTLE_MS: i64 = 150;
const WINDOW_STATE_SETTLE_MS: i64 = 300;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShortcutStatus {
    shortcut: String,
    is_fallback: bool,
}

struct AppState {
    layer_front: AtomicBool,
    shortcut_status: Mutex<Option<ShortcutStatus>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateAudioReminderRequest {
    title: String,
    trigger_at_utc: String,
    audio_base64: String,
}

// Ключ для шифрования аудиофайлов текущего профиля (см. audio_crypto.rs) —
// None на Android, где своего Keystore-эквивалента пока нет (раздел 26).
// Отдельное managed-состояние, а не поле внутри Db: это не про соединение с
// SQLite, и добавление сюда не должно трогать все места, где уже
// используется db.0.lock() как Connection напрямую.
struct AudioKeyState(Mutex<Option<String>>);

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// Единая точка правды: и клик по кнопке, и глобальный хоткей идут сюда,
// поэтому front/back в Rust и в UI никогда не расходятся (было замечание
// ревью: раньше хоткей полагался на round-trip через ещё не готовый webview).
fn toggle_layer(app: &tauri::AppHandle) -> Option<bool> {
    let window = app.get_webview_window("main")?;
    let state = app.state::<AppState>();
    let next = !state.layer_front.load(Ordering::SeqCst);
    // always-on-top — оконная концепция desktop-платформ, на Android нет
    // отдельных перекрывающихся окон (одна Activity/WebView на приложение).
    #[cfg(desktop)]
    window.set_always_on_top(next).ok()?;
    state.layer_front.store(next, Ordering::SeqCst);
    let _ = window.emit("layer-changed", next);
    Some(next)
}

#[tauri::command]
fn toggle_overlay_layer(app: tauri::AppHandle) -> Result<bool, String> {
    toggle_layer(&app).ok_or_else(|| "overlay window not found".to_string())
}

#[tauri::command]
fn get_shortcut_status(state: tauri::State<AppState>) -> Option<ShortcutStatus> {
    state.shortcut_status.lock().ok()?.clone()
}

// UI прячет desktop-специфичные элементы (always-on-top переключатель) на
// платформах, где им нет соответствия — раздел 11 ТЗ, Android-путь другой.
#[tauri::command]
fn is_desktop_platform() -> bool {
    cfg!(desktop)
}

// Раздел 15 ТЗ: у каждого профиля свой vault-файл и свой ключ в keychain.
// Переключение профиля = закрыть текущее соединение и открыть другое —
// managed-состояние Db остаётся тем же объектом, меняется только Connection
// внутри его Mutex, поэтому commands.ts не нужно ничего знать про это.
//
// ВАЖНО (раздел 9 ТЗ, Iteration 2): device_id и состояние HLC-часов теперь
// тоже per-profile (живут как таблицы в vault-{id}.db, см. sync_log.rs) — при
// переключении профиля HlcClock обязан пересоздаться для НОВОГО vault, а не
// только Connection. Иначе после смены профиля операции продолжали бы
// помечаться device_id и счётчиком СТАРОГО профиля поверх БД нового.
// Открывает vault по указанному пути/keyring-имени и подставляет его в
// managed-состояние (Db, HlcClock) — общая часть switch_vault и
// create_profile. Не трогает profiles.json — это ответственность вызывающей
// стороны (для create_profile порядок важен: profiles.json пишется только
// после того, как install_vault здесь отработал без ошибки).
fn install_vault(
    db: &tauri::State<db::Db>,
    hlc_state: &tauri::State<sync_log::HlcClockState>,
    audio_key_state: &tauri::State<AudioKeyState>,
    path: &std::path::Path,
    keyring_user: &str,
) -> Result<(), String> {
    let new_conn = db::open(path, keyring_user)?;
    let new_device_id = sync_log::ensure_device_identity(&new_conn)?;
    let new_clock =
        sync_log::HlcClock::load(&new_conn, new_device_id).map_err(|e| e.to_string())?;
    let new_audio_key = db::vault_key_for_audio(keyring_user)?;

    let mut conn_guard = db.0.lock().map_err(|e| e.to_string())?;
    *conn_guard = new_conn;
    drop(conn_guard);

    let mut clock_guard = hlc_state.0.lock().map_err(|e| e.to_string())?;
    *clock_guard = new_clock;
    drop(clock_guard);

    let mut audio_key_guard = audio_key_state.0.lock().map_err(|e| e.to_string())?;
    *audio_key_guard = new_audio_key;
    Ok(())
}

fn switch_vault(
    db: &tauri::State<db::Db>,
    hlc_state: &tauri::State<sync_log::HlcClockState>,
    audio_key_state: &tauri::State<AudioKeyState>,
    state: &tauri::State<profiles::ProfilesState>,
    id: &str,
) -> Result<(), String> {
    let (path, keyring_user) = profiles::vault_location(state, id)?;
    install_vault(db, hlc_state, audio_key_state, &path, &keyring_user)?;
    profiles::set_active(state, id)
}

#[tauri::command]
fn list_profiles(
    state: tauri::State<profiles::ProfilesState>,
) -> Result<profiles::ProfilesResponse, String> {
    profiles::list(&state)
}

// Раздел 15 ТЗ + разбор ревью: vault открывается ДО того, как профиль
// попадёт в profiles.json. Если install_vault упадёт (например, сбой
// keyring), команда завершится ошибкой, но ничего не запишется на диск —
// значит, в списке профилей не останется "осиротевшей" записи, на которую
// невозможно переключиться и невозможно удалить.
#[tauri::command]
fn create_profile(
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    audio_key_state: tauri::State<AudioKeyState>,
    state: tauri::State<profiles::ProfilesState>,
    display_name: String,
) -> Result<profiles::ProfilesResponse, String> {
    let (pending, vault_path) = profiles::prepare_create(&state, &display_name)?;
    install_vault(
        &db,
        &hlc_state,
        &audio_key_state,
        &vault_path,
        pending.keyring_user(),
    )?;
    let created = profiles::commit_create(&state, pending)?;
    profiles::set_active(&state, &created.id)?;
    profiles::list(&state)
}

#[tauri::command]
fn switch_active_profile(
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    audio_key_state: tauri::State<AudioKeyState>,
    state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<profiles::ProfilesResponse, String> {
    switch_vault(&db, &hlc_state, &audio_key_state, &state, &id)?;
    profiles::list(&state)
}

#[tauri::command]
fn list_plan_items(
    db: tauri::State<db::Db>,
    plan_date: String,
) -> Result<Vec<plan_items::PlanItemDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    plan_items::list(&conn, &plan_date).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_plan_items_range(
    db: tauri::State<db::Db>,
    start_date: String,
    end_date: String,
) -> Result<Vec<plan_items::PlanItemDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    plan_items::list_range(&conn, &start_date, &end_date).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_plan_item(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    title: String,
    plan_date: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::create(&mut conn, &mut clock, &profile_id, &title, &plan_date)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
fn toggle_plan_item_done(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::toggle_done(&mut conn, &mut clock, &profile_id, &id)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
fn cycle_plan_item_progress(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::cycle_progress(&mut conn, &mut clock, &profile_id, &id)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
fn toggle_plan_item_deferred(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::toggle_deferred(&mut conn, &mut clock, &profile_id, &id)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
fn move_plan_item_to_date(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
    plan_date: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::move_to_date(&mut conn, &mut clock, &profile_id, &id, &plan_date)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
fn roll_over_pending_plan_items(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    target_date: String,
) -> Result<usize, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let moved = plan_items::roll_over_pending(&mut conn, &mut clock, &profile_id, &target_date)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    if moved > 0 {
        trigger_server_sync(&app);
    }
    Ok(moved)
}

#[tauri::command]
fn delete_plan_item(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<(), String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    plan_items::delete(&mut conn, &mut clock, &profile_id, &id).map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(())
}

#[tauri::command]
fn list_notes(db: tauri::State<db::Db>) -> Result<Vec<notes::NoteDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    notes::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_note_groups(db: tauri::State<db::Db>) -> Result<Vec<notes::NoteGroupDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    notes::list_groups(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_note_group(
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
fn create_note(
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
fn move_note_to_group(
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
fn update_note(
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
fn delete_note(
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

fn audio_dir(profiles_state: &tauri::State<profiles::ProfilesState>) -> std::path::PathBuf {
    profiles::data_dir(profiles_state).join("audio")
}

#[tauri::command]
fn create_audio_note(
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
fn get_note_audio(
    db: tauri::State<db::Db>,
    audio_key_state: tauri::State<AudioKeyState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<String, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let audio_key = audio_key_state.0.lock().map_err(|e| e.to_string())?;
    notes::read_audio(
        &conn,
        &audio_dir(&profiles_state),
        audio_key.as_deref(),
        &id,
    )
}

// Раздел 19 ТЗ: "user export diagnostics bundle без пользовательского
// содержимого" — пишем JSON-файл в data_dir и возвращаем путь, чтобы
// фронтенд мог показать пользователю, куда сохранилось (без файлового
// save-диалога — новой Tauri-зависимости ради этого не заводили).
#[tauri::command]
fn export_diagnostics(
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
fn sync_readiness_status(
    db: tauri::State<db::Db>,
    profiles_state: tauri::State<profiles::ProfilesState>,
) -> Result<sync_status::SyncReadinessStatus, String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    sync_status::build(&conn, &profile_id)
}

fn trigger_server_sync(app: &tauri::AppHandle) {
    server_sync::spawn_best_effort(app.clone());
}

#[tauri::command]
fn list_reminders(db: tauri::State<db::Db>) -> Result<Vec<reminders::ReminderDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    reminders::list(&conn).map_err(|e| e.to_string())
}

// Настоящее срабатывание на Android идёт через AlarmManager (плагин
// reminder-alarm), а не через опрос alerts::spawn_scheduler — тот выключен на
// Android (#[cfg(desktop)] в setup()), потому что процесс приложения может
// быть убит системой в фоне, а системный alarm это переживает (раздел 11 ТЗ).
// На десктопе плагин — no-op (см. plugins/tauri-plugin-reminder-alarm/src/desktop.rs),
// поэтому вызов ниже безопасен без cfg(target_os) на каждом месте.
fn schedule_android_alarm(app: &tauri::AppHandle, reminder: &reminders::ReminderDto) {
    let Some(trigger_at_millis) = reminders::parse_trigger_millis(&reminder.trigger_at_utc) else {
        eprintln!(
            "reminder-alarm: не удалось разобрать trigger_at_utc: {}",
            reminder.trigger_at_utc
        );
        return;
    };
    if let Err(err) = app.reminder_alarm().schedule_exact_alarm(ScheduleRequest {
        id: reminder.id.clone(),
        title: reminder.title.clone(),
        trigger_at_millis,
    }) {
        eprintln!("reminder-alarm: не удалось запланировать alarm: {err}");
    }
    if let Err(err) = app.reminder_alarm().ensure_notification_permission() {
        eprintln!("reminder-alarm: не удалось запросить разрешение на уведомления: {err}");
    }
}

fn cancel_android_alarm(app: &tauri::AppHandle, id: &str) {
    if let Err(err) = app
        .reminder_alarm()
        .cancel_alarm(CancelRequest { id: id.to_string() })
    {
        eprintln!("reminder-alarm: не удалось отменить alarm: {err}");
    }
}

#[tauri::command]
fn create_reminder(
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
fn create_audio_reminder(
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
fn get_current_alert(state: tauri::State<alerts::AlertState>) -> Option<reminders::ReminderDto> {
    alerts::current_alert(&state)
}

#[tauri::command]
fn get_reminder_audio(
    db: tauri::State<db::Db>,
    audio_key_state: tauri::State<AudioKeyState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<String, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let audio_key = audio_key_state.0.lock().map_err(|e| e.to_string())?;
    reminders::read_audio(
        &conn,
        &audio_dir(&profiles_state),
        audio_key.as_deref(),
        &id,
    )
}

#[tauri::command]
fn acknowledge_reminder(
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
fn snooze_reminder(
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
fn delete_reminder(
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

#[cfg(desktop)]
fn toggle_window_visibility(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    if visible {
        let _ = window.hide();
    } else {
        // show() одного недостаточно: Windows не гарантирует передачу фокуса
        // окну, у которого его не было — без set_focus() оно "показывается",
        // но остаётся позади активного окна и выглядит так, будто ничего не произошло.
        let _ = window.show();
        let _ = window.set_focus();
    }
}

// Раздел 10 ТЗ: "ограничить координаты рабочей областью экрана".
fn clamp_to_monitor(window: &tauri::WebviewWindow) {
    let (Ok(Some(monitor)), Ok(size), Ok(position)) = (
        window.current_monitor(),
        window.outer_size(),
        window.outer_position(),
    ) else {
        return;
    };

    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let min_x = monitor_pos.x;
    let min_y = monitor_pos.y;
    let max_x = (monitor_pos.x + monitor_size.width as i32 - size.width as i32).max(min_x);
    let max_y = (monitor_pos.y + monitor_size.height as i32 - size.height as i32).max(min_y);

    let clamped_x = position.x.clamp(min_x, max_x);
    let clamped_y = position.y.clamp(min_y, max_y);

    if clamped_x != position.x || clamped_y != position.y {
        let _ = window.set_position(PhysicalPosition::new(clamped_x, clamped_y));
    }
}

// set_position нельзя дёргать синхронно из WindowEvent::Moved — на Windows это
// происходит внутри нативного модального drag-цикла ОС, и конкурирующий
// SetWindowPos оттуда просто ломает перетаскивание. Поэтому только запоминаем
// момент последнего Moved, а поправляем позицию отдельным потоком после того,
// как движение затихло на BOUNDS_SETTLE_MS — уже вне drag-цикла ОС.
fn spawn_bounds_watcher(app: tauri::AppHandle, last_moved: Arc<AtomicI64>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(BOUNDS_SETTLE_MS as u64));
        let last = last_moved.load(Ordering::SeqCst);
        if last == 0 || now_millis() - last < BOUNDS_SETTLE_MS {
            continue;
        }
        last_moved.store(0, Ordering::SeqCst);
        let app_for_main = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(window) = app_for_main.get_webview_window("main") {
                clamp_to_monitor(&window);
            }
        });
    });
}

fn spawn_window_state_watcher(
    app: tauri::AppHandle,
    data_dir: PathBuf,
    last_window_state_change: Arc<AtomicI64>,
) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(WINDOW_STATE_SETTLE_MS as u64));
        let last = last_window_state_change.load(Ordering::SeqCst);
        if last == 0 || now_millis() - last < WINDOW_STATE_SETTLE_MS {
            continue;
        }
        last_window_state_change.store(0, Ordering::SeqCst);
        let app_for_main = app.clone();
        let data_dir_for_main = data_dir.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(window) = app_for_main.get_webview_window("main") {
                let _ = window_state::save(&window, &data_dir_for_main);
            }
        });
    });
}

// Пробуем основной хоткей, при конфликте — запасной (раздел 10 ТЗ, риск конфликта с paste-without-formatting).
#[cfg(desktop)]
fn register_layer_shortcut(app: &tauri::AppHandle) -> Result<&'static str, String> {
    let manager = app.global_shortcut();

    let default: Shortcut = DEFAULT_SHORTCUT.parse().map_err(|e| format!("{e}"))?;
    if manager.register(default).is_ok() {
        return Ok(DEFAULT_SHORTCUT);
    }

    let fallback: Shortcut = FALLBACK_SHORTCUT.parse().map_err(|e| format!("{e}"))?;
    manager.register(fallback).map_err(|e| format!("{e}"))?;
    Ok(FALLBACK_SHORTCUT)
}

// Статус хранится в state и отдаётся по запросу (get_shortcut_status), а не
// через emit из setup(): emit туда, где ещё никто не слушает, теряется молча
// (замечание ревью — React мог не успеть подписаться до этого момента).
#[cfg(desktop)]
fn store_shortcut_status(app: &tauri::AppHandle, active: &str) {
    let status = ShortcutStatus {
        shortcut: active.to_string(),
        is_fallback: active != DEFAULT_SHORTCUT,
    };
    if let Ok(mut guard) = app.state::<AppState>().shortcut_status.lock() {
        *guard = Some(status);
    }
}

// Раздел 10 ТЗ: закрытие по умолчанию прячет в tray, реально выходит только
// пункт трея "Выход".
#[cfg(desktop)]
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let icon = Image::new(include_bytes!("../icons/tray-icon.rgba"), 64, 64);

    let show_hide = MenuItem::with_id(app, "show_hide", "Показать/скрыть", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_hide, &quit])?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show_hide" => toggle_window_visibility(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Обычный клик по самой иконке — как в большинстве трей-приложений,
            // не только через пункт меню "Показать/скрыть".
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window_visibility(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

#[allow(clippy::expect_used)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let last_moved = Arc::new(AtomicI64::new(0));
    let last_moved_for_event = last_moved.clone();
    let last_window_state_change = Arc::new(AtomicI64::new(0));
    let last_window_state_change_for_event = last_window_state_change.clone();

    #[cfg_attr(mobile, allow(unused_mut))]
    let mut builder = tauri::Builder::default();

    // Без иконки в таскбаре (skipTaskbar: true) пользователь легко забывает,
    // что приложение уже запущено и висит в трее, и может случайно поднять
    // второй процесс — а два процесса, одновременно пишущие в один vault.db
    // и в один и тот же ключ в OS keychain, это прямой путь к "file is not
    // a database". Второй запуск теперь просто поднимает существующее окно.
    // Autostart, global shortcut, single-instance и tray — desktop-понятия без
    // мобильного аналога (раздел 11 ТЗ уже описывает Android-путь отдельно
    // через нотификации/alarm, не через эти плагины). Без cfg(desktop) сборка
    // под Android либо не компилируется, либо падает в setup() в рантайме.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
        builder = builder.plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ));
        builder = builder.plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_layer(app);
                    }
                })
                .build(),
        );
    }

    builder
        .manage(AppState {
            layer_front: AtomicBool::new(true),
            shortcut_status: Mutex::new(None),
        })
        .manage(alerts::AlertState::default())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_reminder_alarm::init())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(move |window, event| match event {
            tauri::WindowEvent::Moved(_) => {
                last_moved_for_event.store(now_millis(), Ordering::SeqCst);
                if window.label() == "main" {
                    last_window_state_change_for_event.store(now_millis(), Ordering::SeqCst);
                }
            }
            tauri::WindowEvent::Resized(_) if window.label() == "main" => {
                last_window_state_change_for_event.store(now_millis(), Ordering::SeqCst);
            }
            // Только главное окно прячется в tray при закрытии — иначе
            // alert-окно "закрывалось" бы, просто скрываясь, и блокировало
            // показ следующего напоминания из очереди (см. alerts.rs).
            tauri::WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .setup(move |app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            window_state::apply(app.handle(), &data_dir);
            if let Some(window) = app.get_webview_window("main") {
                clamp_to_monitor(&window);
            }

            let profiles_state = profiles::init(&data_dir)?;
            let active_id = profiles::list(&profiles_state)?.active_profile_id;
            let (vault_path, keyring_user) = profiles::vault_location(&profiles_state, &active_id)?;
            let conn = db::open(&vault_path, &keyring_user)?;

            // Раздел 9 ТЗ, Iteration 2: device_id/HLC — per-profile (см.
            // sync_log.rs), поэтому загружаются из того же vault, что и conn,
            // а не заводятся отдельно на уровне приложения.
            let device_id = sync_log::ensure_device_identity(&conn)?;
            let clock = sync_log::HlcClock::load(&conn, device_id)?;
            let audio_key = db::vault_key_for_audio(&keyring_user)?;

            app.manage(db::Db(Mutex::new(conn)));
            app.manage(sync_log::HlcClockState(Mutex::new(clock)));
            app.manage(AudioKeyState(Mutex::new(audio_key)));
            app.manage(profiles_state);
            // Раздел 14 ТЗ, sync — client_id/secret владелец продукта
            // регистрирует и вписывает сам (см. config.rs); отсутствие файла
            // или отдельного провайдера в нём — нормальное состояние, не
            // блокирует обычную работу приложения без sync.
            app.manage(config::load(&data_dir));
            server_sync::spawn_best_effort(app.handle().clone());

            spawn_bounds_watcher(app.handle().clone(), last_moved.clone());
            spawn_window_state_watcher(
                app.handle().clone(),
                data_dir.clone(),
                last_window_state_change.clone(),
            );

            #[cfg(desktop)]
            {
                // На Android напоминания срабатывают через системный AlarmManager
                // (плагин reminder-alarm) — он переживает смерть процесса, в
                // отличие от этого опроса. На десктопе процесс приложения жив,
                // пока оно "запущено" (висит в трее), так что опрос уместен.
                alerts::spawn_scheduler(app.handle().clone());
                setup_tray(app.handle())?;

                let handle = app.handle().clone();
                match register_layer_shortcut(&handle) {
                    Ok(active) => store_shortcut_status(&handle, active),
                    Err(err) => eprintln!("Не удалось зарегистрировать глобальный хоткей: {err}"),
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            toggle_overlay_layer,
            get_shortcut_status,
            is_desktop_platform,
            list_profiles,
            create_profile,
            switch_active_profile,
            list_plan_items,
            list_plan_items_range,
            create_plan_item,
            toggle_plan_item_done,
            cycle_plan_item_progress,
            toggle_plan_item_deferred,
            move_plan_item_to_date,
            roll_over_pending_plan_items,
            delete_plan_item,
            list_notes,
            list_note_groups,
            create_note_group,
            create_note,
            move_note_to_group,
            update_note,
            create_audio_note,
            get_note_audio,
            delete_note,
            export_diagnostics,
            sync_readiness_status,
            list_reminders,
            create_reminder,
            create_audio_reminder,
            get_current_alert,
            get_reminder_audio,
            acknowledge_reminder,
            snooze_reminder,
            delete_reminder,
            sync::start_provider_auth,
            sync::connection_status,
            sync::disconnect_provider,
            server_sync::server_sync_status,
            server_sync::connect_server_sync,
            server_sync::connect_default_server_sync,
            server_sync::register_server_account,
            server_sync::login_server_account,
            server_sync::disconnect_server_sync,
            server_sync::sync_server_now
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
