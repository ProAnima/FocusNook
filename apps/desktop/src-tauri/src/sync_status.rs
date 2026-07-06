use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncReadinessStatus {
    pub profile_id_hash: String,
    pub device_id_hash: Option<String>,
    pub operation_count: i64,
    pub last_operation_at: Option<String>,
    pub last_operation_hlc: Option<String>,
}

fn short_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

pub fn build(conn: &Connection, active_profile_id: &str) -> Result<SyncReadinessStatus, String> {
    let device_id = conn
        .query_row(
            "SELECT device_id FROM device_identity WHERE id = 0",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let operation_count = conn
        .query_row(
            "SELECT COUNT(*) FROM sync_operations WHERE profile_id = ?1",
            params![active_profile_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| e.to_string())?;

    let last_operation = conn
        .query_row(
            "SELECT created_at, hlc FROM sync_operations WHERE profile_id = ?1 ORDER BY hlc DESC LIMIT 1",
            params![active_profile_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    Ok(SyncReadinessStatus {
        profile_id_hash: short_hash(active_profile_id),
        device_id_hash: device_id.as_deref().map(short_hash),
        operation_count,
        last_operation_at: last_operation.as_ref().map(|value| value.0.clone()),
        last_operation_hlc: last_operation.map(|value| value.1),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
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
                created_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE device_identity (
                id INTEGER PRIMARY KEY CHECK (id = 0),
                device_id TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn builds_status_without_exposing_raw_profile_or_device_id() {
        let conn = setup_conn();
        conn.execute("INSERT INTO device_identity VALUES (0, 'device-raw')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO sync_operations VALUES
            ('op-1', 'profile-a', 'device-raw', 'plan_item', 'item-1', 'create', '{}', '2026-07-04T12:10:22.003Z-0001-device-raw', 1, '2026-07-04 12:10:22'),
            ('op-2', 'profile-a', 'device-raw', 'note', 'note-1', 'update', '{}', '2026-07-04T12:11:22.003Z-0000-device-raw', 1, '2026-07-04 12:11:22'),
            ('op-3', 'profile-b', 'device-raw', 'note', 'note-2', 'create', '{}', '2026-07-04T12:12:22.003Z-0000-device-raw', 1, '2026-07-04 12:12:22')",
            [],
        )
        .unwrap();

        let status = build(&conn, "profile-a").unwrap();

        assert_eq!(status.operation_count, 2);
        assert_eq!(
            status.last_operation_at,
            Some("2026-07-04 12:11:22".to_string())
        );
        assert_eq!(
            status.last_operation_hlc,
            Some("2026-07-04T12:11:22.003Z-0000-device-raw".to_string())
        );
        assert_ne!(status.profile_id_hash, "profile-a");
        assert_ne!(status.device_id_hash, Some("device-raw".to_string()));
    }

    #[test]
    fn works_before_any_local_operation_exists() {
        let conn = setup_conn();
        let status = build(&conn, "empty-profile").unwrap();

        assert_eq!(status.operation_count, 0);
        assert_eq!(status.device_id_hash, None);
        assert_eq!(status.last_operation_at, None);
        assert_eq!(status.last_operation_hlc, None);
    }
}
