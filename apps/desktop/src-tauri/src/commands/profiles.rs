use crate::{android_vault_key, db, profiles, sync_log, AudioKeyState};

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
    app: &tauri::AppHandle,
    db: &tauri::State<db::Db>,
    hlc_state: &tauri::State<sync_log::HlcClockState>,
    audio_key_state: &tauri::State<AudioKeyState>,
    data_dir: &std::path::Path,
    path: &std::path::Path,
    keyring_user: &str,
) -> Result<(), String> {
    let android_key_hex = android_vault_key::resolve_for_platform(app, data_dir, keyring_user)?;
    let new_conn = db::open(path, keyring_user, android_key_hex.as_deref())?;
    let new_device_id = sync_log::ensure_device_identity(&new_conn)?;
    let new_clock =
        sync_log::HlcClock::load(&new_conn, new_device_id).map_err(|e| e.to_string())?;
    // db::vault_key_for_audio не существует на Android вообще (см. db/vault_key.rs) —
    // раздельные #[cfg]-ветки, а не runtime match, иначе Android-сборка не
    // компилируется: символ должен существовать независимо от того, какая
    // ветка реально выполнится.
    #[cfg(target_os = "android")]
    let new_audio_key = android_key_hex;
    #[cfg(not(target_os = "android"))]
    let new_audio_key = match android_key_hex {
        Some(key) => Some(key),
        None => db::vault_key_for_audio(keyring_user)?,
    };

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
    app: &tauri::AppHandle,
    db: &tauri::State<db::Db>,
    hlc_state: &tauri::State<sync_log::HlcClockState>,
    audio_key_state: &tauri::State<AudioKeyState>,
    state: &tauri::State<profiles::ProfilesState>,
    id: &str,
) -> Result<(), String> {
    let (path, keyring_user) = profiles::vault_location(state, id)?;
    let data_dir = profiles::data_dir(state).to_path_buf();
    install_vault(
        app,
        db,
        hlc_state,
        audio_key_state,
        &data_dir,
        &path,
        &keyring_user,
    )?;
    profiles::set_active(state, id)?;
    log::info!("активный профиль переключён");
    Ok(())
}

#[tauri::command]
pub(crate) fn list_profiles(
    state: tauri::State<profiles::ProfilesState>,
) -> Result<profiles::ProfilesResponse, String> {
    profiles::list(&state)
}

// Раздел 15 ТЗ + разбор ревью: vault открывается ДО того, как профиль
// попадёт в profiles.json. Если install_vault упадёт (например, сбой
// keyring), команда завершится ошибкой, но ничего не запишется на диск —
// значит, в списке профилей не останется "осиротевшей" записи, на которую
// невозможно переключиться и невозможно удалить.
// Tauri injects AppHandle and State<_> parameters by type and matches the
// remaining arguments by name from the JS invoke payload, so the signature
// cannot be collapsed into a parameter struct without breaking the frontend.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub(crate) fn create_profile(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    audio_key_state: tauri::State<AudioKeyState>,
    state: tauri::State<profiles::ProfilesState>,
    display_name: String,
    email: String,
    password: String,
) -> Result<profiles::ProfilesResponse, String> {
    let (pending, vault_path) = profiles::prepare_create(&state, &display_name, &email, &password)?;
    let data_dir = profiles::data_dir(&state).to_path_buf();
    install_vault(
        &app,
        &db,
        &hlc_state,
        &audio_key_state,
        &data_dir,
        &vault_path,
        pending.keyring_user(),
    )?;
    profiles::commit_create(&state, pending)?;
    log::info!("создан новый профиль, vault установлен");
    profiles::list(&state)
}

#[tauri::command]
pub(crate) fn configure_active_account(
    state: tauri::State<profiles::ProfilesState>,
    display_name: String,
    email: String,
    password: String,
) -> Result<profiles::ProfilesResponse, String> {
    profiles::configure_active_account(&state, &display_name, &email, &password)
}

#[tauri::command]
pub(crate) fn logout_account(
    state: tauri::State<profiles::ProfilesState>,
) -> Result<profiles::ProfilesResponse, String> {
    profiles::lock_session(&state)
}

#[tauri::command]
pub(crate) fn switch_active_profile(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    audio_key_state: tauri::State<AudioKeyState>,
    state: tauri::State<profiles::ProfilesState>,
    id: String,
    password: String,
) -> Result<profiles::ProfilesResponse, String> {
    profiles::verify_account_password(&state, &id, &password)?;
    switch_vault(&app, &db, &hlc_state, &audio_key_state, &state, &id)?;
    profiles::unlock_account(&state, &id, &password)?;
    profiles::list(&state)
}
