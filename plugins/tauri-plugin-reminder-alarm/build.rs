const COMMANDS: &[&str] = &[
  "schedule_exact_alarm",
  "cancel_alarm",
  "can_schedule_exact_alarms",
  "request_exact_alarm_permission",
  "ensure_notification_permission",
];

fn main() {
  tauri_plugin::Builder::new(COMMANDS)
    .android_path("android")
    .ios_path("ios")
    .build();
}
