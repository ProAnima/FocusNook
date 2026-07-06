#[cfg(desktop)]
use crate::db;
use crate::reminders;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;
#[cfg(desktop)]
use tauri::{WebviewUrl, WebviewWindowBuilder};

pub const ALERT_WINDOW_LABEL: &str = "reminder-alert";
#[cfg(desktop)]
const POLL_INTERVAL_SECS: u64 = 20;

// Раздел 10 ТЗ: отдельное topmost-окно на напоминание, не главное окно.
// Очередь — на случай, если несколько напоминаний срабатывают почти
// одновременно (раздел 10, риск, который раньше был открытым вопросом):
// показываем по одному, следующее берём из очереди при закрытии текущего.
#[derive(Default)]
pub struct AlertState {
    queue: Mutex<VecDeque<reminders::ReminderDto>>,
    current: Mutex<Option<reminders::ReminderDto>>,
    // Раздел 19 ТЗ, "reminder scheduler health" — 0 значит "ещё ни разу не
    // опрашивал" (на Android так и остаётся: там опрос не запускается, см.
    // spawn_scheduler ниже), а не наносекунду unix-эпохи.
    last_poll_at_millis: AtomicI64,
}

pub fn current_alert(state: &AlertState) -> Option<reminders::ReminderDto> {
    state.current.lock().ok()?.clone()
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn seconds_since_last_poll(state: &AlertState) -> Option<i64> {
    let last = state.last_poll_at_millis.load(Ordering::Relaxed);
    if last == 0 {
        return None;
    }
    Some((now_millis() - last) / 1000)
}

// Только десктоп: на Android срабатывание идёт через системный AlarmManager
// (плагин reminder-alarm, планируется прямо при create/snooze_reminder в
// lib.rs) — он переживает смерть процесса, а этот опрос не переживает.
#[cfg(desktop)]
pub fn spawn_scheduler(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS));
        check_due_reminders(&app);
    });
}

#[cfg(desktop)]
fn check_due_reminders(app: &tauri::AppHandle) {
    let state = app.state::<AlertState>();
    state
        .last_poll_at_millis
        .store(now_millis(), Ordering::Relaxed);

    let db = app.state::<db::Db>();
    let due = {
        let Ok(conn) = db.0.lock() else { return };
        reminders::due(&conn).unwrap_or_default()
    };
    if due.is_empty() {
        return;
    }
    eprintln!("alerts: найдено due-напоминаний: {}", due.len());

    if let Ok(conn) = db.0.lock() {
        for reminder in &due {
            let _ = reminders::mark_firing(&conn, &reminder.id);
        }
    }

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
    if let Err(err) = open_alert_window(app) {
        eprintln!("alerts: не удалось открыть окно напоминания: {err}");
    }
}

#[cfg(desktop)]
fn open_alert_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(
        app,
        ALERT_WINDOW_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Напоминание")
    .inner_size(300.0, 190.0)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .resizable(false)
    .skip_taskbar(true)
    .center()
    .build()?;
    eprintln!("alerts: окно напоминания открыто");
    let _ = window.set_focus();
    Ok(())
}

// Раздел 11 ТЗ: на Android показ идёт через системное уведомление, не через
// отдельное окно (мобильная модель Tauri — одна Activity/WebView на
// приложение, второе WebviewWindow как на десктопе не открыть). Само
// уведомление и AlarmManager-планирование теперь реализованы в плагине
// reminder-alarm (см. lib.rs::schedule_android_alarm) — эта функция здесь
// по сути недостижима на Android (очередь AlertState никогда не заполняется
// без spawn_scheduler, см. выше), оставлена как безопасный no-op на случай,
// если resolve_current_alert всё же вызовется.
#[cfg(not(desktop))]
fn open_alert_window(_app: &tauri::AppHandle) -> tauri::Result<()> {
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
