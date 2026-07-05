use crate::{db, reminders};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub const ALERT_WINDOW_LABEL: &str = "reminder-alert";
const POLL_INTERVAL_SECS: u64 = 20;

// Раздел 10 ТЗ: отдельное topmost-окно на напоминание, не главное окно.
// Очередь — на случай, если несколько напоминаний срабатывают почти
// одновременно (раздел 10, риск, который раньше был открытым вопросом):
// показываем по одному, следующее берём из очереди при закрытии текущего.
#[derive(Default)]
pub struct AlertState {
    queue: Mutex<VecDeque<reminders::ReminderDto>>,
    current: Mutex<Option<reminders::ReminderDto>>,
}

pub fn current_alert(state: &AlertState) -> Option<reminders::ReminderDto> {
    state.current.lock().ok()?.clone()
}

pub fn spawn_scheduler(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
        check_due_reminders(&app);
    });
}

fn check_due_reminders(app: &tauri::AppHandle) {
    let db = app.state::<db::Db>();
    let due = {
        let Ok(conn) = db.0.lock() else { return };
        reminders::due(&conn).unwrap_or_default()
    };
    if due.is_empty() {
        return;
    }

    if let Ok(conn) = db.0.lock() {
        for reminder in &due {
            let _ = reminders::mark_firing(&conn, &reminder.id);
        }
    }

    let state = app.state::<AlertState>();
    if let Ok(mut queue) = state.queue.lock() {
        queue.extend(due);
    }

    let app_for_main = app.clone();
    let _ = app.run_on_main_thread(move || show_next_alert_if_idle(&app_for_main));
}

pub fn show_next_alert_if_idle(app: &tauri::AppHandle) {
    if app.get_webview_window(ALERT_WINDOW_LABEL).is_some() {
        return;
    }
    let state = app.state::<AlertState>();
    let next = state.queue.lock().ok().and_then(|mut q| q.pop_front());
    let Some(reminder) = next else {
        return;
    };
    if let Ok(mut current) = state.current.lock() {
        *current = Some(reminder);
    }
    let _ = open_alert_window(app);
}

fn open_alert_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, ALERT_WINDOW_LABEL, WebviewUrl::App("index.html".into()))
        .title("Напоминание")
        .inner_size(300.0, 190.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .center()
        .build()?;
    let _ = window.set_focus();
    Ok(())
}

pub fn resolve_current_alert(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(ALERT_WINDOW_LABEL) {
        let _ = window.close();
    }
    if let Ok(mut current) = app.state::<AlertState>().current.lock() {
        *current = None;
    }
    show_next_alert_if_idle(app);
}
