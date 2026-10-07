//! Системные будильники напоминаний (плагин reminder-alarm). Используются
//! командами напоминаний, применением удалённых операций и стартом приложения.

use crate::reminders;
use tauri_plugin_reminder_alarm::{CancelRequest, ReminderAlarmExt, ScheduleRequest};

// Настоящее срабатывание на Android идёт через AlarmManager (плагин
// reminder-alarm), а не через опрос alerts::spawn_scheduler — тот выключен на
// Android (#[cfg(desktop)] в setup()), потому что процесс приложения может
// быть убит системой в фоне, а системный alarm это переживает (раздел 11 ТЗ).
// На десктопе плагин — no-op (см. plugins/tauri-plugin-reminder-alarm/src/desktop.rs),
// поэтому вызов ниже безопасен без cfg(target_os) на каждом месте.
pub(crate) fn schedule_android_alarm(app: &tauri::AppHandle, reminder: &reminders::ReminderDto) {
    let Some(trigger_at_millis) = reminders::parse_trigger_millis(&reminder.trigger_at_utc) else {
        log::warn!(
            "не удалось разобрать trigger_at_utc напоминания {}",
            reminder.id
        );
        return;
    };
    if let Err(err) = app.reminder_alarm().schedule_exact_alarm(ScheduleRequest {
        id: reminder.id.clone(),
        title: reminder.title.clone(),
        trigger_at_millis,
    }) {
        log::warn!("не удалось запланировать alarm: {err}");
    }
    if let Err(err) = app.reminder_alarm().ensure_notification_permission() {
        log::warn!("не удалось запросить разрешение на уведомления: {err}");
    }
}

pub(crate) fn cancel_android_alarm(app: &tauri::AppHandle, id: &str) {
    if let Err(err) = app
        .reminder_alarm()
        .cancel_alarm(CancelRequest { id: id.to_string() })
    {
        log::warn!("не удалось отменить alarm: {err}");
    }
}

#[cfg(target_os = "android")]
pub(crate) fn restore_android_alarms(app: &tauri::AppHandle, conn: &rusqlite::Connection) {
    match reminders::list(conn) {
        Ok(items) => {
            for reminder in items.iter().filter(|item| item.status == "scheduled") {
                schedule_android_alarm(app, reminder);
            }
        }
        Err(err) => log::error!("не удалось восстановить alarms: {err}"),
    }
}
