use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::ReminderAlarm;
#[cfg(mobile)]
use mobile::ReminderAlarm;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the reminder-alarm APIs.
pub trait ReminderAlarmExt<R: Runtime> {
  fn reminder_alarm(&self) -> &ReminderAlarm<R>;
}

impl<R: Runtime, T: Manager<R>> crate::ReminderAlarmExt<R> for T {
  fn reminder_alarm(&self) -> &ReminderAlarm<R> {
    self.state::<ReminderAlarm<R>>().inner()
  }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("reminder-alarm")
    .invoke_handler(tauri::generate_handler![
      commands::schedule_exact_alarm,
      commands::cancel_alarm,
      commands::can_schedule_exact_alarms,
      commands::request_exact_alarm_permission,
      commands::ensure_notification_permission,
    ])
    .setup(|app, api| {
      #[cfg(mobile)]
      let reminder_alarm = mobile::init(app, api)?;
      #[cfg(desktop)]
      let reminder_alarm = desktop::init(app, api)?;
      app.manage(reminder_alarm);
      Ok(())
    })
    .build()
}
