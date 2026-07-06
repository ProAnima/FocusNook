use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<ReminderAlarm<R>> {
  Ok(ReminderAlarm(app.clone()))
}

/// Плагин осмыслен только на Android — на десктопе напоминания уже показывает
/// отдельное alert-окно (см. apps/desktop/src-tauri/src/alerts.rs). Все методы
/// здесь no-op, чтобы вызывающему коду не нужен был cfg(target_os) на каждом
/// вызове.
pub struct ReminderAlarm<R: Runtime>(AppHandle<R>);

impl<R: Runtime> ReminderAlarm<R> {
  pub fn schedule_exact_alarm(&self, _payload: ScheduleRequest) -> crate::Result<ScheduleResponse> {
    Ok(ScheduleResponse { exact: false })
  }

  pub fn cancel_alarm(&self, _payload: CancelRequest) -> crate::Result<()> {
    Ok(())
  }

  pub fn can_schedule_exact_alarms(&self) -> crate::Result<CanScheduleExactResponse> {
    Ok(CanScheduleExactResponse { value: false })
  }

  pub fn request_exact_alarm_permission(&self) -> crate::Result<()> {
    Ok(())
  }

  pub fn ensure_notification_permission(&self) -> crate::Result<()> {
    Ok(())
  }
}
