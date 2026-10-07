use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

mod migrations;
mod plaintext_migration;
#[cfg(all(test, not(target_os = "android")))]
mod tests;
mod vault_key;

use migrations::{
    ensure_notes_audio_column, ensure_notes_group_column, ensure_plan_items_long_running_column,
    ensure_plan_items_plan_date_column, ensure_reminders_audio_column,
    ensure_sync_blob_deliveries_table, ensure_sync_blobs_columns,
    ensure_sync_operation_deliveries_table, ensure_sync_operations_synced_at_column,
};
use plaintext_migration::migrate_plaintext_if_needed;

// Пути `crate::db::X` остаются прежними — реэкспорт из подмодулей.
pub(crate) use migrations::MIGRATIONS;
pub(crate) use vault_key::generate_key_hex;
#[cfg(not(target_os = "android"))]
use vault_key::vault_key;
#[cfg(not(target_os = "android"))]
pub use vault_key::vault_key_for_audio;

// Раздел 9 ТЗ: локальная база на профиль — один файл на профиль, путь и
// per-profile ключ приходят из profiles::vault_location (раздел 15).
pub struct Db(pub Mutex<Connection>);

// android_key_hex — ключ, уже полученный вызывающим кодом через
// android_vault_key::resolve_for_platform ДО этого вызова (нужен AppHandle,
// которого у db/ нет и не должно быть — см. android_vault_key.rs). На
// desktop параметр игнорируется, там ключ по-прежнему берётся из OS keyring
// напрямую внутри этой функции, без изменений.
pub fn open(
    path: &Path,
    keyring_user: &str,
    android_key_hex: Option<&str>,
) -> Result<Connection, String> {
    #[cfg(not(target_os = "android"))]
    let _ = android_key_hex;
    #[cfg(not(target_os = "android"))]
    let key = vault_key(keyring_user)?;
    #[cfg(target_os = "android")]
    let _ = keyring_user;
    #[cfg(target_os = "android")]
    let key = android_key_hex
        .ok_or_else(|| "android vault key was not resolved before db::open".to_string())?
        .to_string();

    // Ранние Android-сборки использовали обычный bundled SQLite. После
    // включения SQLCipher такой plaintext-vault нельзя открывать сразу с
    // PRAGMA key: SQLCipher возвращает "file is not a database" и Tauri падает
    // в setup. Теперь обе платформы линкуются с SQLCipher, поэтому выполняем
    // одну и ту же безопасную конвертацию с сохранением исходного backup.
    migrate_plaintext_if_needed(path, &key)?;

    let conn = Connection::open(path).map_err(|e| e.to_string())?;

    // Raw-key синтаксис SQLCipher: PRAGMA key = "x'<64 hex>'" — обязательно
    // через execute_batch как есть, иначе rusqlite экранирует кавычки внутри
    // значения и это перестаёт быть распознаваемым BLOB-литералом.
    conn.execute_batch(&format!("PRAGMA key = \"x'{key}'\";"))
        .map_err(|e| e.to_string())?;

    for migration in MIGRATIONS {
        conn.execute(migration, []).map_err(|e| e.to_string())?;
    }
    ensure_plan_items_plan_date_column(&conn)?;
    ensure_plan_items_long_running_column(&conn)?;
    ensure_notes_audio_column(&conn)?;
    ensure_notes_group_column(&conn)?;
    ensure_reminders_audio_column(&conn)?;
    ensure_sync_operations_synced_at_column(&conn)?;
    ensure_sync_operation_deliveries_table(&conn)?;
    ensure_sync_blobs_columns(&conn)?;
    ensure_sync_blob_deliveries_table(&conn)?;
    Ok(conn)
}
