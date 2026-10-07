use super::patch::attachment_refs_from_patch;
use super::protocol::LocalOperation;
use super::{FULL_RECONCILE_INTERVAL_SECONDS, MAX_OPS_PER_EXCHANGE, MAX_PENDING_OPERATION_SCAN};
use crate::sync_blobs;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

pub(crate) fn last_pulled_hlc(
    conn: &Connection,
    profile_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT last_pulled_hlc FROM sync_pull_state WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub(crate) fn store_last_pulled_hlc(
    conn: &Connection,
    profile_id: &str,
    last_pulled: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO sync_pull_state (profile_id, last_pulled_hlc, updated_at)
         VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(profile_id) DO UPDATE SET
           last_pulled_hlc = excluded.last_pulled_hlc,
           updated_at = datetime('now')",
        params![profile_id, last_pulled],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn should_run_full_reconcile(
    conn: &Connection,
    profile_id: &str,
) -> Result<bool, String> {
    let elapsed_seconds: Option<i64> = conn
        .query_row(
            "SELECT CAST(strftime('%s', 'now') AS INTEGER) - CAST(strftime('%s', last_reconciled_at) AS INTEGER)
             FROM sync_reconcile_state
             WHERE profile_id = ?1",
            params![profile_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(elapsed_seconds
        .map(|seconds| seconds >= FULL_RECONCILE_INTERVAL_SECONDS)
        .unwrap_or(true))
}

pub(super) fn mark_full_reconciled(conn: &Connection, profile_id: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO sync_reconcile_state (profile_id, last_reconciled_at)
         VALUES (?1, datetime('now'))
         ON CONFLICT(profile_id) DO UPDATE SET
           last_reconciled_at = excluded.last_reconciled_at",
        params![profile_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(feature = "cloud-providers")]
pub(crate) fn unsynced_operations(
    conn: &Connection,
    profile_id: &str,
) -> Result<Vec<LocalOperation>, String> {
    unsynced_operations_with_limit(conn, profile_id, MAX_OPS_PER_EXCHANGE)
}

#[cfg(feature = "cloud-providers")]
fn unsynced_operations_with_limit(
    conn: &Connection,
    profile_id: &str,
    limit: usize,
) -> Result<Vec<LocalOperation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT operation_id, device_id, entity_type, entity_id, op, patch, hlc, schema_version
             FROM sync_operations
             WHERE profile_id = ?1 AND synced_at IS NULL
             ORDER BY hlc ASC, operation_id ASC
             LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![profile_id, limit as i64], |row| {
            Ok(LocalOperation {
                operation_id: row.get(0)?,
                device_id: row.get(1)?,
                entity_type: row.get(2)?,
                entity_id: row.get(3)?,
                op: row.get(4)?,
                patch: row.get(5)?,
                hlc: row.get(6)?,
                schema_version: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn undelivered_operations_with_limit(
    conn: &Connection,
    profile_id: &str,
    remote_profile_id: &str,
    limit: usize,
) -> Result<Vec<LocalOperation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT operation_id, device_id, entity_type, entity_id, op, patch, hlc, schema_version
         FROM sync_operations AS operation
         WHERE operation.profile_id = ?1
           AND NOT EXISTS (
             SELECT 1 FROM sync_operation_deliveries AS delivery
             WHERE delivery.operation_id = operation.operation_id
               AND delivery.remote_profile_id = ?2
           )
         ORDER BY hlc ASC, operation_id ASC
         LIMIT ?3",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            params![profile_id, remote_profile_id, limit as i64],
            |row| {
                Ok(LocalOperation {
                    operation_id: row.get(0)?,
                    device_id: row.get(1)?,
                    entity_type: row.get(2)?,
                    entity_id: row.get(3)?,
                    op: row.get(4)?,
                    patch: row.get(5)?,
                    hlc: row.get(6)?,
                    schema_version: row.get(7)?,
                })
            },
        )
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn local_attachment_ids(operation: &LocalOperation) -> Result<Vec<String>, String> {
    if operation.op == "delete" {
        return Ok(Vec::new());
    }
    let patch: Value = serde_json::from_str(&operation.patch)
        .map_err(|e| format!("local operation patch is invalid json: {e}"))?;
    Ok(attachment_refs_from_patch(&patch)
        .into_iter()
        .map(|(id, _)| id)
        .collect())
}

pub(super) fn ready_unsynced_operations(
    conn: &Connection,
    profile_id: &str,
    remote_profile_id: &str,
) -> Result<Vec<LocalOperation>, String> {
    let candidates = undelivered_operations_with_limit(
        conn,
        profile_id,
        remote_profile_id,
        MAX_PENDING_OPERATION_SCAN,
    )?;
    let mut ready = Vec::with_capacity(MAX_OPS_PER_EXCHANGE);
    for operation in candidates {
        let attachment_ids = local_attachment_ids(&operation)?;
        let mut dependencies_ready = true;
        for blob_id in attachment_ids {
            if !sync_blobs::is_uploaded_to(conn, profile_id, &blob_id, remote_profile_id)? {
                dependencies_ready = false;
                break;
            }
        }
        if dependencies_ready {
            ready.push(operation);
            if ready.len() == MAX_OPS_PER_EXCHANGE {
                break;
            }
        }
    }
    Ok(ready)
}

pub(super) fn mark_delivered(
    conn: &Connection,
    remote_profile_id: &str,
    operation_ids: &[String],
) -> Result<(), String> {
    for id in operation_ids {
        conn.execute(
            "INSERT OR IGNORE INTO sync_operation_deliveries
             (operation_id, remote_profile_id, delivered_at) VALUES (?1, ?2, datetime('now'))",
            params![id, remote_profile_id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn mark_synced(conn: &Connection, operation_ids: &[String]) -> Result<(), String> {
    for id in operation_ids {
        conn.execute(
            "UPDATE sync_operations SET synced_at = datetime('now') WHERE operation_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) fn prepare_account_scope_if_needed(
    conn: &Connection,
    local_profile_id: &str,
    remote_profile_id: &str,
    local_device_id: &str,
) -> Result<(), String> {
    if local_profile_id == remote_profile_id || last_pulled_hlc(conn, remote_profile_id)?.is_some()
    {
        return Ok(());
    }

    conn.execute(
        "UPDATE sync_operations
         SET synced_at = NULL
         WHERE profile_id = ?1 AND device_id = ?2",
        params![local_profile_id, local_device_id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sync_blobs
         SET uploaded_at = NULL
         WHERE profile_id = ?1 AND deleted_at IS NULL",
        params![local_profile_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::server_sync::test_support::sync_test_conn;

    #[test]
    fn first_account_scope_requeues_local_device_operations_and_blobs() -> Result<(), String> {
        let conn = sync_test_conn();
        conn.execute(
            "INSERT INTO sync_operations
                (operation_id, profile_id, device_id, entity_type, entity_id, op, patch, hlc,
                 schema_version, created_at, synced_at)
             VALUES
                ('local-op', 'local-profile', 'desktop-device', 'note', 'note-1', 'create', '{}',
                 '2026-07-07T10:00:00.000Z-0000-desktop-device', 1, datetime('now'), datetime('now')),
                ('remote-op', 'local-profile', 'phone-device', 'note', 'note-2', 'create', '{}',
                 '2026-07-07T10:01:00.000Z-0000-phone-device', 1, datetime('now'), datetime('now'))",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO sync_blobs
                (profile_id, blob_id, local_path, content_type, uploaded_at, created_at)
             VALUES
                ('local-profile', 'voice.webm', 'voice.webm', 'audio/webm', datetime('now'), datetime('now'))",
            [],
        )
        .map_err(|e| e.to_string())?;

        prepare_account_scope_if_needed(
            &conn,
            "local-profile",
            "account-user-id",
            "desktop-device",
        )?;

        let local_synced: Option<String> = conn
            .query_row(
                "SELECT synced_at FROM sync_operations WHERE operation_id = 'local-op'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let remote_synced: Option<String> = conn
            .query_row(
                "SELECT synced_at FROM sync_operations WHERE operation_id = 'remote-op'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let blob_uploaded: Option<String> = conn
            .query_row(
                "SELECT uploaded_at FROM sync_blobs WHERE blob_id = 'voice.webm'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        assert!(local_synced.is_none());
        assert!(remote_synced.is_some());
        assert!(blob_uploaded.is_none());
        Ok(())
    }

    #[test]
    fn blocked_attachment_does_not_starve_independent_text_operations() {
        let conn = sync_test_conn();
        conn.execute_batch(
            "INSERT INTO sync_operations
                (operation_id, profile_id, device_id, entity_type, entity_id, op, patch, hlc,
                 schema_version, created_at)
             VALUES
                ('audio-op', 'profile', 'device', 'note', 'audio', 'create',
                 '{\"audioPath\":\"missing.webm\"}', '2026-07-01T00:00:00.000Z-0000-device', 1,
                 datetime('now')),
                ('text-op', 'profile', 'device', 'note', 'text', 'create',
                 '{\"body\":\"ready\"}', '2026-07-01T00:00:01.000Z-0000-device', 1,
                 datetime('now')),
                ('image-op', 'profile', 'device', 'note', 'image', 'create',
                 '{\"attachments\":[{\"id\":\"picture.png\",\"contentType\":\"image/png\"}]}',
                 '2026-07-01T00:00:02.000Z-0000-device', 1, datetime('now'));
             INSERT INTO sync_blobs
                (profile_id, blob_id, local_path, content_type, created_at)
             VALUES ('profile', 'picture.png', 'picture.png', 'image/png', datetime('now'));",
        )
        .unwrap();

        let ready = ready_unsynced_operations(&conn, "profile", "remote").unwrap();
        assert_eq!(
            ready
                .iter()
                .map(|operation| operation.operation_id.as_str())
                .collect::<Vec<_>>(),
            vec!["text-op"]
        );

        sync_blobs::mark_uploaded(&conn, "profile", "picture.png", "sha", 10).unwrap();
        sync_blobs::mark_uploaded_to(&conn, "profile", "picture.png", "remote").unwrap();
        let ready = ready_unsynced_operations(&conn, "profile", "remote").unwrap();
        assert_eq!(
            ready
                .iter()
                .map(|operation| operation.operation_id.as_str())
                .collect::<Vec<_>>(),
            vec!["text-op", "image-op"]
        );
    }

    #[test]
    fn delivery_state_is_scoped_to_each_remote_account() {
        let conn = sync_test_conn();
        conn.execute(
            "INSERT INTO sync_operations
             (operation_id, profile_id, device_id, entity_type, entity_id, op, patch, hlc,
              schema_version, created_at)
             VALUES ('local-op', 'profile', 'device', 'note', 'note', 'create', '{}',
              '2026-07-01T00:00:00.000Z-0000-device', 1, datetime('now'))",
            [],
        )
        .unwrap();

        assert_eq!(
            undelivered_operations_with_limit(&conn, "profile", "account-a", 10)
                .unwrap()
                .len(),
            1
        );
        mark_delivered(&conn, "account-a", &["local-op".to_string()]).unwrap();
        assert!(
            undelivered_operations_with_limit(&conn, "profile", "account-a", 10)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            undelivered_operations_with_limit(&conn, "profile", "account-b", 10)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn full_reconcile_is_due_until_marked_recently() -> Result<(), String> {
        let conn = sync_test_conn();

        assert!(should_run_full_reconcile(&conn, "account-user-id")?);
        mark_full_reconciled(&conn, "account-user-id")?;
        assert!(!should_run_full_reconcile(&conn, "account-user-id")?);

        conn.execute(
            "UPDATE sync_reconcile_state
             SET last_reconciled_at = datetime('now', '-20 minutes')
             WHERE profile_id = 'account-user-id'",
            [],
        )
        .map_err(|e| e.to_string())?;
        assert!(should_run_full_reconcile(&conn, "account-user-id")?);
        Ok(())
    }
}
