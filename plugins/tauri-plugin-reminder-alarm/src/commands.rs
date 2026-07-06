use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::ReminderAlarmExt;
use crate::Result;

#[command]
pub(crate) async fn schedule_exact_alarm<R: Runtime>(
  app: AppHandle<R>,
  payload: ScheduleRequest,
) -> Result<ScheduleResponse> {
  app.reminder_alarm().schedule_exact_alarm(payload)
}

#[command]
pub(crate) async fn cancel_alarm<R: Runtime>(app: AppHandle<R>, payload: CancelRequest) -> Result<()> {
  app.reminder_alarm().cancel_alarm(payload)
}

#[command]
pub(crate) async fn can_schedule_exact_alarms<R: Runtime>(
  app: AppHandle<R>,
) -> Result<CanScheduleExactResponse> {
  app.reminder_alarm().can_schedule_exact_alarms()
}

#[command]
pub(crate) async fn request_exact_alarm_permission<R: Runtime>(app: AppHandle<R>) -> Result<()> {
  app.reminder_alarm().request_exact_alarm_permission()
}

#[command]
pub(crate) async fn ensure_notification_permission<R: Runtime>(app: AppHandle<R>) -> Result<()> {
  app.reminder_alarm().ensure_notification_permission()
}
