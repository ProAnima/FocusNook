use rusqlite::Connection;

pub(crate) const MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS plan_items (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    progress_percent INTEGER,
    plan_date TEXT NOT NULL,
    is_long_running INTEGER NOT NULL DEFAULT 0,
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
    audio_path TEXT,
    group_id TEXT,
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
    // Загрузка/bootstrap самих значений — в lib.rs::run (setup), не здесь: db/
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
    "CREATE TABLE IF NOT EXISTS sync_snapshot_state (
    remote_profile_id TEXT PRIMARY KEY,
    seeded_at TEXT NOT NULL
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

pub(super) fn ensure_sync_blob_deliveries_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sync_blob_deliveries (
           profile_id TEXT NOT NULL,
           blob_id TEXT NOT NULL,
           remote_profile_id TEXT NOT NULL,
           delivered_at TEXT NOT NULL,
           PRIMARY KEY (profile_id, blob_id, remote_profile_id)
         );",
    )
    .map_err(|e| e.to_string())
}

pub(super) fn ensure_sync_operation_deliveries_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sync_operation_deliveries (
           operation_id TEXT NOT NULL,
           remote_profile_id TEXT NOT NULL,
           delivered_at TEXT NOT NULL,
           PRIMARY KEY (operation_id, remote_profile_id)
         );
         CREATE INDEX IF NOT EXISTS idx_sync_operation_deliveries_remote
           ON sync_operation_deliveries(remote_profile_id, delivered_at);",
    )
    .map_err(|e| e.to_string())
}

// Раздел 8 ТЗ, аудио-заметки: notes уже существовала до этой колонки, а
// MIGRATIONS выше — только "CREATE TABLE IF NOT EXISTS" (не версионированные
// шаги), поэтому добавление колонки идёт отдельно и проверяет PRAGMA
// table_info, чтобы ALTER TABLE не падал на "duplicate column" при повторном
// запуске уже мигрировавшей базы.
pub(super) fn ensure_plan_items_plan_date_column(conn: &Connection) -> Result<(), String> {
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

pub(super) fn ensure_plan_items_long_running_column(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(plan_items)")
        .map_err(|e| e.to_string())?;
    let has_column = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .any(|name| name == "is_long_running");
    if !has_column {
        conn.execute(
            "ALTER TABLE plan_items
             ADD COLUMN is_long_running INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) fn ensure_notes_audio_column(conn: &Connection) -> Result<(), String> {
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

pub(super) fn ensure_notes_group_column(conn: &Connection) -> Result<(), String> {
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

pub(super) fn ensure_reminders_audio_column(conn: &Connection) -> Result<(), String> {
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

pub(super) fn ensure_sync_operations_synced_at_column(conn: &Connection) -> Result<(), String> {
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

pub(super) fn ensure_sync_blobs_columns(conn: &Connection) -> Result<(), String> {
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
