#![allow(clippy::unwrap_used)]

use super::*;
use crate::server_sync::test_support::sync_test_conn;

#[test]
fn does_not_accept_partial_server_confirmation() {
    let error = validate_confirmed_count(0, 0, 1);
    assert_eq!(
        error,
        Err("sync server confirmed 0 of 1 sent operations".to_string())
    );
    assert!(validate_confirmed_count(1, 1, 2).is_ok());
    assert!(validate_confirmed_count(-1, 2, 1).is_err());
}

#[test]
fn operation_existence_is_scoped_to_profile() -> Result<(), String> {
    let conn = sync_test_conn();
    conn.execute(
        "INSERT INTO sync_operations
            (operation_id, profile_id, device_id, entity_type, entity_id, op, patch, hlc,
             schema_version, created_at, synced_at)
         VALUES
            ('shared-op-id', 'other-profile', 'phone-device', 'note', 'note-1', 'create', '{}',
             '2026-07-07T10:00:00.000Z-0000-phone-device', 1, datetime('now'), datetime('now'))",
        [],
    )
    .map_err(|e| e.to_string())?;

    assert!(operation_exists(&conn, "other-profile", "shared-op-id")?);
    assert!(!operation_exists(&conn, "active-profile", "shared-op-id")?);
    Ok(())
}

#[test]
fn applies_remote_plan_item_once_and_marks_operation_synced() -> Result<(), String> {
    let conn = sync_test_conn();
    let operation = RemoteOperation {
        device_id: "phone-device".to_string(),
        entity_id: "task-1".to_string(),
        entity_type: "plan_item".to_string(),
        hlc: "2026-07-07T10:00:00.000Z-0000-phone-device".to_string(),
        op: "create".to_string(),
        operation_id: "op-1".to_string(),
        payload_ciphertext: serde_json::json!({
            "title": "Call",
            "status": "open",
            "progressPercent": null,
            "planDate": "2026-07-07",
            "isLongRunning": true
        })
        .to_string(),
        schema_version: 1,
    };

    apply_remote_operation(&conn, "profile-1", "desktop-device", &operation, None)?;
    apply_remote_operation(&conn, "profile-1", "desktop-device", &operation, None)?;

    let (title, is_long_running): (String, bool) = conn
        .query_row(
            "SELECT title, is_long_running FROM plan_items WHERE id = 'task-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let op_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sync_operations", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let synced_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sync_operations WHERE synced_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    assert_eq!(title, "Call");
    assert!(is_long_running);
    assert_eq!(op_count, 1);
    assert_eq!(synced_count, 1);
    Ok(())
}

#[test]
fn stale_remote_update_does_not_overwrite_newer_local_plan_item() -> Result<(), String> {
    let conn = sync_test_conn();
    conn.execute(
        "INSERT INTO plan_items
            (id, title, status, progress_percent, plan_date, created_at)
         VALUES ('task-1', 'Call', 'done', NULL, '2026-07-07', datetime('now'))",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO sync_operations
            (operation_id, profile_id, device_id, entity_type, entity_id, op, patch, hlc,
             schema_version, created_at, synced_at)
         VALUES
            ('local-newer-op', 'profile-1', 'phone-device', 'plan_item', 'task-1', 'update',
             '{\"status\":\"done\"}', '2026-07-07T10:01:00.000Z-0000-phone-device',
             1, datetime('now'), NULL)",
        [],
    )
    .map_err(|e| e.to_string())?;
    let stale_remote = RemoteOperation {
        device_id: "desktop-device".to_string(),
        entity_id: "task-1".to_string(),
        entity_type: "plan_item".to_string(),
        hlc: "2026-07-07T10:00:00.000Z-0000-desktop-device".to_string(),
        op: "update".to_string(),
        operation_id: "remote-older-op".to_string(),
        payload_ciphertext: serde_json::json!({ "status": "open" }).to_string(),
        schema_version: 1,
    };

    apply_remote_operation(&conn, "profile-1", "phone-device", &stale_remote, None)?;

    let status: String = conn
        .query_row(
            "SELECT status FROM plan_items WHERE id = 'task-1'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let stored_remote: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sync_operations WHERE operation_id = 'remote-older-op' AND synced_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    assert_eq!(status, "done");
    assert_eq!(stored_remote, 1);
    Ok(())
}

#[test]
fn newer_remote_delete_wins_over_older_local_plan_item() -> Result<(), String> {
    let conn = sync_test_conn();
    conn.execute(
        "INSERT INTO plan_items
            (id, title, status, progress_percent, plan_date, created_at)
         VALUES ('task-1', 'Call', 'open', NULL, '2026-07-07', datetime('now'))",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO sync_operations
            (operation_id, profile_id, device_id, entity_type, entity_id, op, patch, hlc,
             schema_version, created_at, synced_at)
         VALUES
            ('local-older-op', 'profile-1', 'phone-device', 'plan_item', 'task-1', 'update',
             '{\"status\":\"open\"}', '2026-07-07T10:00:00.000Z-0000-phone-device',
             1, datetime('now'), datetime('now'))",
        [],
    )
    .map_err(|e| e.to_string())?;
    let newer_delete = RemoteOperation {
        device_id: "desktop-device".to_string(),
        entity_id: "task-1".to_string(),
        entity_type: "plan_item".to_string(),
        hlc: "2026-07-07T10:02:00.000Z-0000-desktop-device".to_string(),
        op: "delete".to_string(),
        operation_id: "remote-newer-delete".to_string(),
        payload_ciphertext: serde_json::json!({}).to_string(),
        schema_version: 1,
    };

    apply_remote_operation(&conn, "profile-1", "phone-device", &newer_delete, None)?;

    let remaining: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plan_items WHERE id = 'task-1'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    assert_eq!(remaining, 0);
    Ok(())
}
