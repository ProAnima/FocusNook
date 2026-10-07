#![allow(clippy::unwrap_used)]
use super::plaintext_migration::{
    looks_like_plaintext_sqlite, migrate_plaintext_if_needed, verify_encrypted_sqlite,
};
use super::vault_key::KEYRING_SERVICE;
use super::*;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("focusnook-db-test-{name}-{}", uuid::Uuid::now_v7()))
}

fn write_plaintext_db_with_a_row(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE plan_items (id TEXT PRIMARY KEY, title TEXT NOT NULL);
         INSERT INTO plan_items (id, title) VALUES ('1', 'legacy task');",
    )
    .unwrap();
}

fn open_with_key(path: &Path, key_hex: &str) -> Connection {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(&format!("PRAGMA key = \"x'{key_hex}'\";"))
        .unwrap();
    conn
}

fn read_legacy_title(conn: &Connection) -> String {
    conn.query_row("SELECT title FROM plan_items WHERE id = '1'", [], |r| {
        r.get(0)
    })
    .unwrap()
}

#[test]
fn looks_like_plaintext_sqlite_detects_a_real_plaintext_file() {
    let path = temp_path("plain");
    write_plaintext_db_with_a_row(&path);
    assert!(looks_like_plaintext_sqlite(&path));
    fs::remove_file(&path).unwrap();
}

#[test]
fn looks_like_plaintext_sqlite_is_false_for_a_missing_file() {
    assert!(!looks_like_plaintext_sqlite(&temp_path("missing")));
}

#[test]
fn migrate_plaintext_if_needed_converts_a_real_legacy_vault() {
    let path = temp_path("legacy");
    write_plaintext_db_with_a_row(&path);
    let key_hex = "aa".repeat(32);

    migrate_plaintext_if_needed(&path, &key_hex).unwrap();

    // Тем же ключом файл на исходном пути теперь читается как SQLCipher...
    let conn = open_with_key(&path, &key_hex);
    assert_eq!(read_legacy_title(&conn), "legacy task");
    // ...и уже не выглядит как обычный (незашифрованный) SQLite.
    assert!(!looks_like_plaintext_sqlite(&path));

    // После успешной проверки зашифрованной базы plaintext-копии не остаётся.
    let backup_path = PathBuf::from(format!("{}.plaintext-backup", path.display()));
    assert!(!backup_path.exists());

    // Windows не даёт удалить файл, пока для него открыт хендл соединения
    // (в отличие от Unix) — drop() обязателен перед remove_file ниже.
    drop(conn);
    fs::remove_file(&path).unwrap();
}

#[test]
fn migrate_plaintext_if_needed_is_a_no_op_for_an_already_encrypted_vault() {
    let path = temp_path("already-encrypted");
    let key_hex = "bb".repeat(32);
    {
        let conn = open_with_key(&path, &key_hex);
        conn.execute("CREATE TABLE t (id INTEGER)", []).unwrap();
    }

    migrate_plaintext_if_needed(&path, &key_hex).unwrap();

    let backup_path = PathBuf::from(format!("{}.plaintext-backup", path.display()));
    assert!(
        !backup_path.exists(),
        "уже зашифрованный vault не должен трогаться"
    );
    fs::remove_file(&path).unwrap();
}

#[test]
fn encrypted_vault_cleanup_removes_a_lingering_plaintext_backup() {
    let path = temp_path("encrypted-with-backup");
    let backup_path = PathBuf::from(format!("{}.plaintext-backup", path.display()));
    let key_hex = "be".repeat(32);
    {
        let conn = open_with_key(&path, &key_hex);
        conn.execute("CREATE TABLE t (id INTEGER)", []).unwrap();
    }
    write_plaintext_db_with_a_row(&backup_path);

    migrate_plaintext_if_needed(&path, &key_hex).unwrap();

    assert!(!backup_path.exists());
    verify_encrypted_sqlite(&path, &key_hex).unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn migrate_plaintext_if_needed_is_a_no_op_when_nothing_exists() {
    let path = temp_path("nothing-here");
    assert!(migrate_plaintext_if_needed(&path, &"cc".repeat(32)).is_ok());
    assert!(!path.exists());
}

#[test]
fn existing_plan_items_gain_the_long_running_column_with_a_safe_default() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE plan_items (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            progress_percent INTEGER,
            plan_date TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO plan_items
         (id, title, status, progress_percent, plan_date, created_at)
         VALUES ('task-1', 'Legacy', 'open', NULL, '2026-07-26', datetime('now'))",
        [],
    )
    .unwrap();

    ensure_plan_items_long_running_column(&conn).unwrap();

    let marker: bool = conn
        .query_row(
            "SELECT is_long_running FROM plan_items WHERE id = 'task-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!marker);
}

// Симулирует сбой между переименованием и sqlcipher_export: path
// отсутствует, а .plaintext-backup — настоящая plaintext-база с данными.
// Повторный вызов должен доводить миграцию до конца, а не считать, что
// делать нечего (иначе следующий open() тихо завёл бы пустой новый vault
// поверх ещё не сконвертированных данных).
#[test]
fn migrate_plaintext_if_needed_resumes_an_interrupted_migration() {
    let path = temp_path("resume");
    let backup_path = PathBuf::from(format!("{}.plaintext-backup", path.display()));
    write_plaintext_db_with_a_row(&backup_path);
    let key_hex = "dd".repeat(32);

    migrate_plaintext_if_needed(&path, &key_hex).unwrap();

    let conn = open_with_key(&path, &key_hex);
    assert_eq!(read_legacy_title(&conn), "legacy task");
    assert!(!backup_path.exists());

    drop(conn);
    fs::remove_file(&path).unwrap();
}

// Настоящий keyring (Windows Credential Manager), не мок — тот же принцип,
// что и в sync_tokens.rs: это и есть код, который реально исполняется в
// проде, а не только его форма. Проверяет весь путь open() целиком, а не
// только изолированный migrate_plaintext_if_needed выше.
#[test]
fn open_migrates_a_legacy_plaintext_vault_transparently() {
    let path = temp_path("open-real");
    write_plaintext_db_with_a_row(&path);
    let keyring_user = format!("db-test-audio-key-{}", uuid::Uuid::now_v7());

    let conn = open(&path, &keyring_user, None).unwrap();
    assert_eq!(read_legacy_title(&conn), "legacy task");
    drop(conn);

    // Повторный open() с тем же keyring_user продолжает работать нормально
    // (второй прогон не должен снова решить, что это plaintext).
    let conn2 = open(&path, &keyring_user, None).unwrap();
    assert_eq!(read_legacy_title(&conn2), "legacy task");
    drop(conn2);

    fs::remove_file(&path).unwrap();
    let _ = keyring::Entry::new(KEYRING_SERVICE, &keyring_user).and_then(|e| e.delete_credential());
}
