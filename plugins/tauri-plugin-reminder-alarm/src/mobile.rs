use serde::de::DeserializeOwned;
use tauri::{
  plugin::{PluginApi, PluginHandle},
  AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_reminder_alarm);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
  _app: &AppHandle<R>,
  api: PluginApi<R, C>,
) -> crate::Result<ReminderAlarm<R>> {
  #[cfg(target_os = "android")]
  let handle = api.register_android_plugin("com.proanima.reminderalarm", "ReminderAlarmPlugin")?;
  #[cfg(target_os = "ios")]
  let handle = api.register_ios_plugin(init_plugin_reminder_alarm)?;
  Ok(ReminderAlarm(handle))
}

/// Access to the reminder-alarm APIs.
pub struct ReminderAlarm<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> ReminderAlarm<R> {
  pub fn schedule_exact_alarm(&self, payload: ScheduleRequest) -> crate::Result<ScheduleResponse> {
    self
      .0
      .run_mobile_plugin("scheduleExactAlarm", payload)
      .map_err(Into::into)
  }

  pub fn cancel_alarm(&self, payload: CancelRequest) -> crate::Result<()> {
    self
      .0
      .run_mobile_plugin("cancelAlarm", payload)
      .map_err(Into::into)
  }

  pub fn can_schedule_exact_alarms(&self) -> crate::Result<CanScheduleExactResponse> {
    self
      .0
      .run_mobile_plugin("canScheduleExactAlarms", ())
      .map_err(Into::into)
  }

  pub fn request_exact_alarm_permission(&self) -> crate::Result<()> {
    self
      .0
      .run_mobile_plugin("requestExactAlarmPermission", ())
      .map_err(Into::into)
  }

  pub fn ensure_notification_permission(&self) -> crate::Result<()> {
    self
      .0
      .run_mobile_plugin("ensureNotificationPermission", ())
      .map_err(Into::into)
  }
}
