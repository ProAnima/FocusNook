use rusqlite::Connection;
use std::path::Path;
#[cfg(any(not(target_os = "android"), test))]
use std::path::PathBuf;
use std::sync::Mutex;

// Раздел 9 ТЗ: локальная база на профиль. Пока нет multi-account — один файл
// на дефолтный профиль; per-profile пути придут вместе с Iteration 1.
pub struct Db(pub Mutex<Connection>);

const MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS plan_items (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    progress_percent INTEGER,
    plan_date TEXT NOT NULL,
    created_at TEXT NOT NULL
)",
    "CREATE TABLE IF NOT EXISTS note_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL
)",
    "CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    title TEXT,
    body TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'text',
    created_at TEXT NOT NULL
)",
    "CREATE TABLE IF NOT EXISTS reminders (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    audio_path TEXT,
    trigger_at_utc TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'scheduled',
    created_at TEXT NOT NULL
)",
    // Раздел 9 ТЗ, Iteration 2 (первый локальный шаг, без провайдеров) —
    // journal операций, персистентное состояние HLC и минимальный device_id.
    // Загрузка/bootstrap самих значений — в lib.rs::setup, не здесь: db.rs
    // отвечает только за схему, оркестрация (sync_log::ensure_device_identity
    // + HlcClock::load после open()) — там же, где уже собираются profiles+db.
    "CREATE TABLE IF NOT EXISTS sync_operations (
    operation_id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    op TEXT NOT NULL,
    patch TEXT NOT NULL,
    hlc TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    created_at TEXT NOT NULL
)",
    "CREATE INDEX IF NOT EXISTS idx_sync_operations_hlc ON sync_operations(hlc)",
    "CREATE TABLE IF NOT EXISTS sync_clock_state (
    id INTEGER PRIMARY KEY CHECK (id = 0),
    last_millis INTEGER NOT NULL,
    last_counter INTEGER NOT NULL
)",
    "CREATE TABLE IF NOT EXISTS device_identity (
    id INTEGER PRIMARY KEY CHECK (id = 0),
    device_id TEXT NOT NULL
)",
    "CREATE TABLE IF NOT EXISTS server_sync_credentials (
    profile_id TEXT PRIMARY KEY,
    raw_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
)",
    "CREATE TABLE IF NOT EXISTS sync_pull_state (
    profile_id TEXT PRIMARY KEY,
    last_pulled_hlc TEXT,
    updated_at TEXT NOT NULL
)",
    "CREATE TABLE IF NOT EXISTS sync_reconcile_state (
    profile_id TEXT PRIMARY KEY,
    last_reconciled_at TEXT NOT NULL
)",
    "CREATE TABLE IF NOT EXISTS sync_blobs (
    profile_id TEXT NOT NULL,
    blob_id TEXT NOT NULL,
    local_path TEXT NOT NULL,
    content_type TEXT NOT NULL,
    sha256 TEXT,
    size_bytes INTEGER,
    sync_payload_base64 TEXT,
    uploaded_at TEXT,
    downloaded_at TEXT,
    deleted_at TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY(profile_id, blob_id)
)",
    "CREATE INDEX IF NOT EXISTS idx_sync_blobs_pending_upload ON sync_blobs(profile_id, uploaded_at, deleted_at)",
];

#[cfg(not(target_os = "android"))]
const KEYRING_SERVICE: &str = "com.proanima.focusnook";

#[cfg(not(target_os = "android"))]
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(not(target_os = "android"))]
fn generate_key_hex() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    hex_encode(&bytes)
}

// Раздел 16 ТЗ: "ключ базы не хранится рядом с базой" — OS keychain (Windows
// Credential Manager / macOS Keychain / Secret Service на Linux). Раздел 15:
// у каждого профиля свой vault и свой ключ — keyring_user разный на профиль,
// keyring_service общий (это просто "неймспейс" приложения в keychain).
//
// ВАЖНО: `keyring` v3 не включает backend платформы по умолчанию — без
// features = ["windows-native"] крейт молча работал с no-op хранилищем
// (set_password не падал, но ничего не сохранял, get_password всегда
// возвращал NoEntry). Ключ пропадал на каждом перезапуске процесса, база
// переставала открываться. Если понадобится macOS/Linux — не забыть
// добавить apple-native / linux-native соответствующим фичам в Cargo.toml.
#[cfg(not(target_os = "android"))]
fn vault_key(keyring_user: &str) -> Result<String, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, keyring_user).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(existing) => {
            eprintln!("vault: используем существующий ключ из OS keychain");
            Ok(existing)
        }
        Err(keyring::Error::NoEntry) => {
            eprintln!("vault: ключ не найден в OS keychain, генерирую новый");
            let key = generate_key_hex();
            entry.set_password(&key).map_err(|e| e.to_string())?;
            Ok(key)
        }
        Err(e) => {
            eprintln!("vault: ошибка чтения OS keychain: {e}");
            Err(e.to_string())
        }
    }
}

// notes.rs шифрует аудиофайлы производным от этого же ключа (см.
// audio_crypto.rs) — им нужен per-profile секрет, но не сам PRAGMA key
// напрямую (доменное разделение). На Android своего Keystore-эквивалента
// пока нет (раздел 26), поэтому там аудио остаётся как есть, без шифрования
// — тот же уровень защиты, что уже честно объявлен для самой БД.
#[cfg(not(target_os = "android"))]
pub fn vault_key_for_audio(keyring_user: &str) -> Result<Option<String>, String> {
    vault_key(keyring_user).map(Some)
}

#[cfg(target_os = "android")]
pub fn vault_key_for_audio(_keyring_user: &str) -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(not(target_os = "android"))]
const SQLITE_PLAINTEXT_MAGIC: &[u8; 16] = b"SQLite format 3\0";

// Настоящий (незашифрованный) SQLite-файл всегда начинается с этой сигнатуры;
// у SQLCipher-зашифрованного файла первые байты неотличимы от случайных.
// Разбор ошибок чтения (файла нет / он короче 16 байт) как "не похоже на
// plaintext" — не ошибка сама по себе, migrate_plaintext_if_needed ниже и так
// отдельно проверяет существование файла до вызова этой функции.
#[cfg(not(target_os = "android"))]
fn looks_like_plaintext_sqlite(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut header = [0u8; 16];
    if file.read_exact(&mut header).is_err() {
        return false;
    }
    &header == SQLITE_PLAINTEXT_MAGIC
}

// P1 ревью + раздел 26 ТЗ: Iteration 0 (без шифрования) вышла раньше
// Iteration 1 (шифрование) — установки с того периода имеют настоящий
// plaintext vault.db. Открыть такой файл через голый PRAGMA key бессмысленно:
// SQLCipher примет любой ключ молча, но не сможет прочитать уже
// существующие данные как валидный формат. Конвертируем через официальный
// sqlcipher_export (не свою бинарную миграцию) — задокументированный,
// проверенный путь plaintext -> encrypted у самого SQLCipher.
//
// Оригинал не трогаем "на месте": переименовываем в .plaintext-backup ДО
// конвертации, так что при сбое sqlcipher_export исходные данные остаются
// целы и читаемы вручную, а не теряются между переименованием и записью.
// Если предыдущая попытка миграции упала между этими двумя шагами (path
// уже нет, а .plaintext-backup ещё есть) — доводим её до конца с backup,
// а не заводим на его месте пустой новый vault.
#[cfg(not(target_os = "android"))]
fn migrate_plaintext_if_needed(path: &Path, key_hex: &str) -> Result<(), String> {
    let backup_path = PathBuf::from(format!("{}.plaintext-backup", path.display()));

    let source = if path.exists() {
        if !looks_like_plaintext_sqlite(path) {
            return Ok(());
        }
        std::fs::rename(path, &backup_path).map_err(|e| e.to_string())?;
        backup_path
    } else if backup_path.exists() {
        backup_path
    } else {
        return Ok(());
    };

    eprintln!(
        "vault: обнаружен незашифрованный vault, конвертирую в SQLCipher (источник: {})",
        source.display()
    );
    let plain_conn = Connection::open(&source).map_err(|e| e.to_string())?;
    let escaped_target = path.display().to_string().replace('\'', "''");
    plain_conn
        .execute_batch(&format!(
            "ATTACH DATABASE '{escaped_target}' AS encrypted KEY \"x'{key_hex}'\";
             SELECT sqlcipher_export('encrypted');
             DETACH DATABASE encrypted;"
        ))
        .map_err(|e| {
            format!(
                "не удалось сконвертировать старую базу в зашифрованный формат: {e}. \
                 Исходные данные сохранены в {}",
                source.display()
            )
        })?;
    Ok(())
}

// На Android этот же вызов открывает обычный незашифрованный SQLite (см.
// комментарий у target-specific rusqlite-зависимостей в Cargo.toml): сборка
// bundled-sqlcipher-vendored-openssl с Windows-хоста под Android не собирается
// из-за требований Perl в OpenSSL Configure — открытый риск раздела 26.
pub fn open(path: &Path, keyring_user: &str) -> Result<Connection, String> {
    #[cfg(target_os = "android")]
    let _ = keyring_user;
    #[cfg(not(target_os = "android"))]
    let key = vault_key(keyring_user)?;
    #[cfg(not(target_os = "android"))]
    migrate_plaintext_if_needed(path, &key)?;

    let conn = Connection::open(path).map_err(|e| e.to_string())?;

    #[cfg(not(target_os = "android"))]
    {
        // Raw-key синтаксис SQLCipher: PRAGMA key = "x'<64 hex>'" — обязательно
        // через execute_batch как есть, иначе rusqlite экранирует кавычки внутри
        // значения и это перестаёт быть распознаваемым BLOB-литералом.
        conn.execute_batch(&format!("PRAGMA key = \"x'{key}'\";"))
            .map_err(|e| e.to_string())?;
    }

    for migration in MIGRATIONS {
        conn.execute(migration, []).map_err(|e| e.to_string())?;
    }
    ensure_plan_items_plan_date_column(&conn)?;
    ensure_notes_audio_column(&conn)?;
    ensure_notes_group_column(&conn)?;
    ensure_reminders_audio_column(&conn)?;
    ensure_sync_operations_synced_at_column(&conn)?;
    ensure_sync_blobs_columns(&conn)?;
    Ok(conn)
}

// Раздел 8 ТЗ, аудио-заметки: notes уже существовала до этой колонки, а
// MIGRATIONS выше — только "CREATE TABLE IF NOT EXISTS" (не версионированные
// шаги), поэтому добавление колонки идёт отдельно и проверяет PRAGMA
// table_info, чтобы ALTER TABLE не падал на "duplicate column" при повторном
// запуске уже мигрировавшей базы.
fn ensure_plan_items_plan_date_column(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(plan_items)")
        .map_err(|e| e.to_string())?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    let has_column = columns.iter().any(|name| name == "plan_date");
    if !has_column {
        conn.execute("ALTER TABLE plan_items ADD COLUMN plan_date TEXT", [])
            .map_err(|e| e.to_string())?;
        let source = if columns.iter().any(|name| name == "created_at") {
            "COALESCE(NULLIF(date(created_at), ''), date('now'))"
        } else {
            "date('now')"
        };
        conn.execute(
            &format!(
                "UPDATE plan_items
                 SET plan_date = {source}
                 WHERE plan_date IS NULL OR plan_date = ''"
            ),
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ensure_notes_audio_column(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(notes)")
        .map_err(|e| e.to_string())?;
    let has_column = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .any(|name| name == "audio_path");
    if !has_column {
        conn.execute("ALTER TABLE notes ADD COLUMN audio_path TEXT", [])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ensure_notes_group_column(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(notes)")
        .map_err(|e| e.to_string())?;
    let has_column = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .any(|name| name == "group_id");
    if !has_column {
        conn.execute("ALTER TABLE notes ADD COLUMN group_id TEXT", [])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ensure_reminders_audio_column(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(reminders)")
        .map_err(|e| e.to_string())?;
    let has_column = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .any(|name| name == "audio_path");
    if !has_column {
        conn.execute("ALTER TABLE reminders ADD COLUMN audio_path TEXT", [])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ensure_sync_operations_synced_at_column(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(sync_operations)")
        .map_err(|e| e.to_string())?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    if !columns.iter().any(|name| name == "synced_at") {
        conn.execute("ALTER TABLE sync_operations ADD COLUMN synced_at TEXT", [])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ensure_sync_blobs_columns(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(sync_blobs)")
        .map_err(|e| e.to_string())?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    for (name, definition) in [
        ("sha256", "TEXT"),
        ("size_bytes", "INTEGER"),
        ("sync_payload_base64", "TEXT"),
        ("uploaded_at", "TEXT"),
        ("downloaded_at", "TEXT"),
        ("deleted_at", "TEXT"),
    ] {
        if !columns.iter().any(|column| column == name) {
            conn.execute(
                &format!("ALTER TABLE sync_blobs ADD COLUMN {name} {definition}"),
                [],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// Плейнтекст-детект/миграция и per-profile ключ — понятия, которых на
// Android нет (см. #[cfg] на самих функциях), поэтому и тесты на них имеют
// смысл только здесь.
#[cfg(all(test, not(target_os = "android")))]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::fs;

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

        // Backup сохранён и содержит оригинальные, нетронутые данные.
        let backup_path = PathBuf::from(format!("{}.plaintext-backup", path.display()));
        assert!(backup_path.exists());
        assert_eq!(
            read_legacy_title(&Connection::open(&backup_path).unwrap()),
            "legacy task"
        );

        // Windows не даёт удалить файл, пока для него открыт хендл соединения
        // (в отличие от Unix) — drop() обязателен перед remove_file ниже.
        drop(conn);
        fs::remove_file(&path).unwrap();
        fs::remove_file(&backup_path).unwrap();
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
    fn migrate_plaintext_if_needed_is_a_no_op_when_nothing_exists() {
        let path = temp_path("nothing-here");
        assert!(migrate_plaintext_if_needed(&path, &"cc".repeat(32)).is_ok());
        assert!(!path.exists());
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

        drop(conn);
        fs::remove_file(&path).unwrap();
        fs::remove_file(&backup_path).unwrap();
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

        let conn = open(&path, &keyring_user).unwrap();
        assert_eq!(read_legacy_title(&conn), "legacy task");
        drop(conn);

        // Повторный open() с тем же keyring_user продолжает работать нормально
        // (второй прогон не должен снова решить, что это plaintext).
        let conn2 = open(&path, &keyring_user).unwrap();
        assert_eq!(read_legacy_title(&conn2), "legacy task");
        drop(conn2);

        fs::remove_file(&path).unwrap();
        fs::remove_file(format!("{}.plaintext-backup", path.display())).unwrap();
        let _ =
            keyring::Entry::new(KEYRING_SERVICE, &keyring_user).and_then(|e| e.delete_credential());
    }
}
