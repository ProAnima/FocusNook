mod alarms;
mod alerts;
mod android_vault_key;
mod audio_crypto;
mod blob_crypto;
#[cfg(feature = "cloud-providers")]
mod cloud_sync;
mod commands;
mod config;
mod db;
mod diagnostics;
mod notes;
#[cfg(feature = "cloud-providers")]
mod oauth;
mod plan_items;
mod profiles;
mod reminders;
mod server_sync;
mod shell;
#[cfg(feature = "cloud-providers")]
mod sync;
mod sync_blobs;
mod sync_log;
mod sync_snapshot;
mod sync_status;
#[cfg(feature = "cloud-providers")]
mod sync_tokens;
mod window_state;

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;
#[cfg(desktop)]
use tauri_plugin_autostart::MacosLauncher;
#[cfg(desktop)]
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg(target_os = "android")]
use alarms::restore_android_alarms;
use shell::{clamp_to_monitor, spawn_bounds_watcher, spawn_window_state_watcher};
#[cfg(desktop)]
use shell::{
    register_layer_shortcut, setup_tray, spawn_initial_window_state_reapply, store_shortcut_status,
    toggle_layer, DESKTOP_WEBVIEW_ZOOM,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShortcutStatus {
    pub(crate) shortcut: String,
    pub(crate) is_fallback: bool,
}

pub(crate) struct AppState {
    pub(crate) layer_front: AtomicBool,
    pub(crate) shortcut_status: Mutex<Option<ShortcutStatus>>,
}

// Ключ для шифрования аудиофайлов текущего профиля (см. audio_crypto.rs) —
// None на Android, где своего Keystore-эквивалента пока нет (раздел 26).
// Отдельное managed-состояние, а не поле внутри Db: это не про соединение с
// SQLite, и добавление сюда не должно трогать все места, где уже
// используется db.0.lock() как Connection напрямую.
pub(crate) struct AudioKeyState(pub(crate) Mutex<Option<String>>);

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Плагин логирования (`log` + `tauri-plugin-log`).
///
/// Куда пишется: stdout (на Android это Logcat; тег записи — путь модуля, например
/// `desktop_lib::server_sync::engine`) и
/// ротируемый файл `focusnook.log` в каталоге логов приложения (`app_log_dir`:
/// на Windows `%LOCALAPPDATA%\<identifier>\logs`, на Android
/// `logs` в приватном каталоге данных приложения). Хранится текущий файл и до 5
/// старых по ~1 МБ.
///
/// Уровень: Info в release, Debug в debug-сборках. Чтобы поднять уровень,
/// запустите приложение с переменной окружения `FOCUSNOOK_LOG=debug` (или
/// `trace`/`info`/`warn`/`error`/`off`); на Android переменную задать нельзя —
/// там действует уровень сборки. Шумные зависимости (tao, wry, hyper,
/// reqwest, rustls и т.п.) всегда ограничены уровнем Warn.
///
/// В журнал нельзя писать содержимое задач/заметок/напоминаний, e-mail, имена,
/// токены и ключи — только идентификаторы и счётчики. JS-API логирования не
/// подключено (в capabilities нет `log:*`).
fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

    let default_level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    let level = std::env::var("FOCUSNOOK_LOG")
        .ok()
        .and_then(|value| value.trim().parse::<log::LevelFilter>().ok())
        .unwrap_or(default_level);

    let mut builder = tauri_plugin_log::Builder::new()
        .clear_targets()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir {
                file_name: Some("focusnook".to_string()),
            }),
        ])
        .rotation_strategy(RotationStrategy::KeepSome(5))
        .max_file_size(1_000_000)
        .level(level);
    for noisy in [
        "tao",
        "wry",
        "hyper",
        "hyper_util",
        "reqwest",
        "rustls",
        "h2",
        "tracing",
        "tokio_util",
        "tiny_http",
        "mio",
    ] {
        builder = builder.level_for(noisy, log::LevelFilter::Warn);
    }
    builder.build()
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

    // Логгер ставится первым, чтобы остальные плагины и setup() уже писали в него.
    builder = builder.plugin(log_plugin());

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

    builder = builder
        .manage(AppState {
            layer_front: AtomicBool::new(true),
            shortcut_status: Mutex::new(None),
        })
        .manage(alerts::AlertState::default())
        .plugin(tauri_plugin_store::Builder::new().build());

    #[cfg(feature = "cloud-providers")]
    {
        builder = builder.plugin(tauri_plugin_google_auth::init());
    }

    builder
        .plugin(tauri_plugin_secure_storage::init())
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
            tauri::WindowEvent::ScaleFactorChanged { scale_factor, .. }
                if window.label() == "main" =>
            {
                // Windows can retain the old physical size when a frameless
                // window crosses onto a monitor with another DPI. Restore the
                // persisted logical size before the transient size is saved.
                if let (Ok(data_dir), Some(webview_window)) = (
                    window.app_handle().path().app_data_dir(),
                    window.app_handle().get_webview_window(window.label()),
                ) {
                    #[cfg(desktop)]
                    let _ = webview_window.set_zoom(DESKTOP_WEBVIEW_ZOOM);
                    window_state::reapply_size_after_scale_change(
                        &webview_window,
                        &data_dir,
                        *scale_factor,
                    );
                }
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
            log::info!(
                "FocusNook {} запущен ({}/{})",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                std::env::consts::ARCH
            );
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            window_state::apply(app.handle(), &data_dir);
            #[cfg(desktop)]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_zoom(DESKTOP_WEBVIEW_ZOOM);
            }
            #[cfg(desktop)]
            spawn_initial_window_state_reapply(app.handle().clone(), data_dir.clone());
            if let Some(window) = app.get_webview_window("main") {
                clamp_to_monitor(&window);
            }

            let profiles_state = profiles::init(&data_dir)?;
            let active_id = profiles::list(&profiles_state)?.active_profile_id;
            let (vault_path, keyring_user) = profiles::vault_location(&profiles_state, &active_id)?;
            let android_key_hex =
                android_vault_key::resolve_for_platform(app.handle(), &data_dir, &keyring_user)?;
            let conn = db::open(&vault_path, &keyring_user, android_key_hex.as_deref())?;
            #[cfg(target_os = "android")]
            restore_android_alarms(app.handle(), &conn);

            // Раздел 9 ТЗ, Iteration 2: device_id/HLC — per-profile (см.
            // sync_log.rs), поэтому загружаются из того же vault, что и conn,
            // а не заводятся отдельно на уровне приложения.
            let device_id = sync_log::ensure_device_identity(&conn)?;
            let clock = sync_log::HlcClock::load(&conn, device_id)?;
            // db::vault_key_for_audio не существует на Android вообще (см.
            // db/vault_key.rs) — раздельные #[cfg]-ветки, а не runtime match, иначе
            // Android-сборка не компилируется.
            #[cfg(target_os = "android")]
            let audio_key = android_key_hex;
            #[cfg(not(target_os = "android"))]
            let audio_key = match android_key_hex {
                Some(key) => Some(key),
                None => db::vault_key_for_audio(&keyring_user)?,
            };

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
            server_sync::spawn_server_event_listener(app.handle().clone());
            server_sync::spawn_periodic_best_effort(app.handle().clone());
            #[cfg(feature = "cloud-providers")]
            {
                cloud_sync::spawn_best_effort(app.handle().clone());
                cloud_sync::spawn_periodic_best_effort(app.handle().clone());
            }

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
                    Err(err) => log::warn!("Не удалось зарегистрировать глобальный хоткей: {err}"),
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::overlay::toggle_overlay_layer,
            commands::overlay::get_shortcut_status,
            commands::overlay::is_desktop_platform,
            commands::profiles::list_profiles,
            commands::profiles::create_profile,
            commands::profiles::configure_active_account,
            commands::profiles::logout_account,
            commands::profiles::switch_active_profile,
            commands::plan_items::list_plan_items,
            commands::plan_items::list_plan_items_range,
            commands::plan_items::create_plan_item,
            commands::plan_items::toggle_plan_item_done,
            commands::plan_items::cycle_plan_item_progress,
            commands::plan_items::toggle_plan_item_deferred,
            commands::plan_items::toggle_plan_item_long_running,
            commands::plan_items::move_plan_item_to_date,
            commands::plan_items::roll_over_pending_plan_items,
            commands::plan_items::delete_plan_item,
            commands::notes::list_notes,
            commands::notes::list_note_groups,
            commands::notes::create_note_group,
            commands::notes::create_note,
            commands::notes::move_note_to_group,
            commands::notes::update_note,
            commands::notes::create_audio_note,
            commands::notes::get_note_audio,
            commands::notes::delete_note,
            commands::diagnostics::export_diagnostics,
            commands::diagnostics::sync_readiness_status,
            commands::reminders::list_reminders,
            commands::reminders::create_reminder,
            commands::reminders::create_audio_reminder,
            commands::reminders::get_current_alert,
            commands::reminders::get_reminder_audio,
            commands::reminders::acknowledge_reminder,
            commands::reminders::snooze_reminder,
            commands::reminders::delete_reminder,
            server_sync::account::server_sync_status,
            server_sync::account::connect_server_sync,
            server_sync::account::connect_default_server_sync,
            server_sync::account_auth::set_account_sync_enabled,
            server_sync::account_auth::register_server_account,
            server_sync::account_auth::login_server_account,
            server_sync::account::delete_server_account,
            server_sync::account::disconnect_server_sync,
            server_sync::engine::sync_server_now,
            server_sync::engine::request_server_sync,
            commands::overlay::open_privacy_policy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
