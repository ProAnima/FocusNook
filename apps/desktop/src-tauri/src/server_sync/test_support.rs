#![allow(clippy::unwrap_used)]

use rusqlite::Connection;

pub(super) fn sync_test_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE plan_items (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            progress_percent INTEGER,
            plan_date TEXT NOT NULL,
            is_long_running INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE note_groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE notes (
            id TEXT PRIMARY KEY,
            title TEXT,
            body TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'text',
            audio_path TEXT,
            group_id TEXT,
            created_at TEXT NOT NULL
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE reminders (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            audio_path TEXT,
            trigger_at_utc TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'scheduled',
            created_at TEXT NOT NULL
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE sync_operations (
            operation_id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            op TEXT NOT NULL,
            patch TEXT NOT NULL,
            hlc TEXT NOT NULL,
            schema_version INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            synced_at TEXT
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE sync_operation_deliveries (
            operation_id TEXT NOT NULL,
            remote_profile_id TEXT NOT NULL,
            delivered_at TEXT NOT NULL,
            PRIMARY KEY (operation_id, remote_profile_id)
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE sync_pull_state (
            profile_id TEXT PRIMARY KEY,
            last_pulled_hlc TEXT,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE sync_reconcile_state (
            profile_id TEXT PRIMARY KEY,
            last_reconciled_at TEXT NOT NULL
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE sync_blobs (
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
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE sync_blob_deliveries (
            profile_id TEXT NOT NULL,
            blob_id TEXT NOT NULL,
            remote_profile_id TEXT NOT NULL,
            delivered_at TEXT NOT NULL,
            PRIMARY KEY(profile_id, blob_id, remote_profile_id)
        )",
        [],
    )
    .unwrap();
    conn
}
