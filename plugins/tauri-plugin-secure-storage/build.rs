// Пустой список — этот плагин не вызывается из JS/webview никогда (см.
// комментарий в src/lib.rs): раскрытие decrypt() как invoke()-команды
// превратило бы hardware-backed ключ в decrypt-оракл для кода в webview.
const COMMANDS: &[&str] = &[];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
