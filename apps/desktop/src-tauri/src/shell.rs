#[cfg(desktop)]
use crate::ShortcutStatus;
use crate::{now_millis, window_state, AppState};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
#[cfg(desktop)]
use tauri::image::Image;
#[cfg(desktop)]
use tauri::menu::{Menu, MenuItem};
#[cfg(desktop)]
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, PhysicalPosition};
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
#[cfg(desktop)]
const DEFAULT_SHORTCUT: &str = "ctrl+shift+v";
#[cfg(desktop)]
const FALLBACK_SHORTCUT: &str = "ctrl+alt+space";
const BOUNDS_SETTLE_MS: i64 = 150;
const WINDOW_STATE_SETTLE_MS: i64 = 300;
#[cfg(desktop)]
const INITIAL_DPI_SETTLE_MS: u64 = 200;
#[cfg(desktop)]
pub(crate) const DESKTOP_WEBVIEW_ZOOM: f64 = 1.0;
// Единая точка правды: и клик по кнопке, и глобальный хоткей идут сюда,
// поэтому front/back в Rust и в UI никогда не расходятся (было замечание
// ревью: раньше хоткей полагался на round-trip через ещё не готовый webview).
pub(crate) fn toggle_layer(app: &tauri::AppHandle) -> Option<bool> {
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
#[cfg(desktop)]
pub(crate) fn toggle_window_visibility(app: &tauri::AppHandle) {
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
pub(crate) fn clamp_to_monitor(window: &tauri::WebviewWindow) {
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
pub(crate) fn spawn_bounds_watcher(app: tauri::AppHandle, last_moved: Arc<AtomicI64>) {
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
pub(crate) fn spawn_window_state_watcher(
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
#[cfg(desktop)]
pub(crate) fn spawn_initial_window_state_reapply(app: tauri::AppHandle, data_dir: PathBuf) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(INITIAL_DPI_SETTLE_MS));
        let app_for_main = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(window) = app_for_main.get_webview_window("main") {
                // WebView2 keeps page zoom in its user-data profile, which
                // survives an app reinstall. Normalize it after initialization
                // so one machine cannot retain an enlarged, unclickable UI.
                let _ = window.set_zoom(DESKTOP_WEBVIEW_ZOOM);
                let scale_factor = window.scale_factor().unwrap_or(1.0);
                window_state::reapply_size_after_scale_change(&window, &data_dir, scale_factor);
            }
        });
    });
}
// Пробуем основной хоткей, при конфликте — запасной (раздел 10 ТЗ, риск конфликта с paste-without-formatting).
#[cfg(desktop)]
pub(crate) fn register_layer_shortcut(app: &tauri::AppHandle) -> Result<&'static str, String> {
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
pub(crate) fn store_shortcut_status(app: &tauri::AppHandle, active: &str) {
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
pub(crate) fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
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
