const COMMANDS: &[&str] = &["connect", "access_token", "is_connected", "disconnect"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
