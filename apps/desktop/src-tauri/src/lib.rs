mod alerts;
mod db;
mod notes;
mod plan_items;
mod reminders;

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, PhysicalPosition};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const DEFAULT_SHORTCUT: &str = "ctrl+shift+v";
const FALLBACK_SHORTCUT: &str = "ctrl+alt+space";
const BOUNDS_SETTLE_MS: i64 = 150;

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

#[tauri::command]
fn list_plan_items(db: tauri::State<db::Db>) -> Result<Vec<plan_items::PlanItemDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    plan_items::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_plan_item(
    db: tauri::State<db::Db>,
    title: String,
) -> Result<plan_items::PlanItemDto, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    plan_items::create(&conn, &title).map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_plan_item_done(
    db: tauri::State<db::Db>,
    id: String,
) -> Result<plan_items::PlanItemDto, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    plan_items::toggle_done(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_notes(db: tauri::State<db::Db>) -> Result<Vec<notes::NoteDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    notes::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_note(db: tauri::State<db::Db>, body: String) -> Result<notes::NoteDto, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    notes::create(&conn, &body).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_reminders(db: tauri::State<db::Db>) -> Result<Vec<reminders::ReminderDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    reminders::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_reminder(
    db: tauri::State<db::Db>,
    title: String,
    trigger_at_utc: String,
) -> Result<reminders::ReminderDto, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    reminders::create(&conn, &title, &trigger_at_utc).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_current_alert(state: tauri::State<alerts::AlertState>) -> Option<reminders::ReminderDto> {
    alerts::current_alert(&state)
}

#[tauri::command]
fn acknowledge_reminder(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    id: String,
) -> Result<(), String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        reminders::acknowledge(&conn, &id).map_err(|e| e.to_string())?;
    }
    alerts::resolve_current_alert(&app);
    Ok(())
}

#[tauri::command]
fn snooze_reminder(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    id: String,
    new_trigger_at_utc: String,
) -> Result<(), String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        reminders::reschedule(&conn, &id, &new_trigger_at_utc).map_err(|e| e.to_string())?;
    }
    alerts::resolve_current_alert(&app);
    Ok(())
}

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

// Пробуем основной хоткей, при конфликте — запасной (раздел 10 ТЗ, риск конфликта с paste-without-formatting).
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
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let Some(icon) = app.default_window_icon().cloned() else {
        eprintln!("Нет иконки приложения для трея — трей не создан");
        return Ok(());
    };

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

    let mut builder = tauri::Builder::default();

    // Без иконки в таскбаре (skipTaskbar: true) пользователь легко забывает,
    // что приложение уже запущено и висит в трее, и может случайно поднять
    // второй процесс — а два процесса, одновременно пишущие в один vault.db
    // и в один и тот же ключ в OS keychain, это прямой путь к "file is not
    // a database". Второй запуск теперь просто поднимает существующее окно.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .manage(AppState {
            layer_front: AtomicBool::new(true),
            shortcut_status: Mutex::new(None),
        })
        .manage(alerts::AlertState::default())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_layer(app);
                    }
                })
                .build(),
        )
        .on_window_event(move |window, event| match event {
            tauri::WindowEvent::Moved(_) => {
                last_moved_for_event.store(now_millis(), Ordering::SeqCst);
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
            let conn = db::open(&data_dir.join("vault.db"))?;
            app.manage(db::Db(Mutex::new(conn)));

            spawn_bounds_watcher(app.handle().clone(), last_moved.clone());
            alerts::spawn_scheduler(app.handle().clone());
            setup_tray(app.handle())?;

            let handle = app.handle().clone();
            match register_layer_shortcut(&handle) {
                Ok(active) => store_shortcut_status(&handle, active),
                Err(err) => eprintln!("Не удалось зарегистрировать глобальный хоткей: {err}"),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            toggle_overlay_layer,
            get_shortcut_status,
            list_plan_items,
            create_plan_item,
            toggle_plan_item_done,
            list_notes,
            create_note,
            list_reminders,
            create_reminder,
            get_current_alert,
            acknowledge_reminder,
            snooze_reminder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
