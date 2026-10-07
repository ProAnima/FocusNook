use super::patch::{bool_field, nullable_string_field, string_field};
use super::protocol::RemoteOperation;
use crate::sync_blobs;
use rusqlite::{params, Connection};
use serde_json::Value;

pub(super) fn apply_plan_item(
    conn: &Connection,
    operation: &RemoteOperation,
    patch: &Value,
) -> Result<(), String> {
    match operation.op.as_str() {
        "create" => {
            let title = string_field(patch, "title").unwrap_or("");
            let status = string_field(patch, "status").unwrap_or("open");
            let plan_date = string_field(patch, "planDate").unwrap_or("1970-01-01");
            let progress = patch.get("progressPercent").and_then(Value::as_i64);
            let is_long_running = bool_field(patch, "isLongRunning").unwrap_or(false);
            conn.execute(
                "INSERT INTO plan_items
                   (id, title, status, progress_percent, plan_date, is_long_running, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
                 ON CONFLICT(id) DO UPDATE SET
                   title = excluded.title,
                   status = excluded.status,
                   progress_percent = excluded.progress_percent,
                   plan_date = excluded.plan_date,
                   is_long_running = excluded.is_long_running",
                params![
                    operation.entity_id,
                    title,
                    status,
                    progress,
                    plan_date,
                    is_long_running
                ],
            )
        }
        "update" => {
            if let Some(status) = string_field(patch, "status") {
                let progress = patch.get("progressPercent").and_then(Value::as_i64);
                conn.execute(
                    "UPDATE plan_items SET status = ?1, progress_percent = ?2 WHERE id = ?3",
                    params![status, progress, operation.entity_id],
                )
                .map_err(|e| e.to_string())?;
            }
            if let Some(plan_date) = string_field(patch, "planDate") {
                conn.execute(
                    "UPDATE plan_items SET plan_date = ?1 WHERE id = ?2",
                    params![plan_date, operation.entity_id],
                )
                .map_err(|e| e.to_string())?;
            }
            if let Some(is_long_running) = bool_field(patch, "isLongRunning") {
                conn.execute(
                    "UPDATE plan_items SET is_long_running = ?1 WHERE id = ?2",
                    params![is_long_running, operation.entity_id],
                )
                .map_err(|e| e.to_string())?;
            }
            Ok(0)
        }
        "delete" => conn.execute(
            "DELETE FROM plan_items WHERE id = ?1",
            params![operation.entity_id],
        ),
        _ => Ok(0),
    }
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub(super) fn apply_note_group(
    conn: &Connection,
    operation: &RemoteOperation,
    patch: &Value,
) -> Result<(), String> {
    match operation.op.as_str() {
        "create" => conn.execute(
            "INSERT INTO note_groups (id, name, created_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET name = excluded.name",
            params![
                operation.entity_id,
                string_field(patch, "name").unwrap_or("")
            ],
        ),
        "delete" => conn.execute(
            "DELETE FROM note_groups WHERE id = ?1",
            params![operation.entity_id],
        ),
        _ => Ok(0),
    }
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn register_legacy_audio_reference(conn: &Connection, profile_id: &str, filename: &str) {
    if let Err(err) = sync_blobs::ensure_downloadable_audio_blob(conn, profile_id, filename) {
        log::warn!("ignored invalid audio attachment reference {filename}: {err}");
    }
}

pub(super) fn apply_note(
    conn: &Connection,
    profile_id: &str,
    operation: &RemoteOperation,
    patch: &Value,
) -> Result<(), String> {
    match operation.op.as_str() {
        "create" => {
            let kind = string_field(patch, "kind").unwrap_or("text");
            let body = string_field(patch, "body").unwrap_or("");
            let audio_path = string_field(patch, "audioPath");
            let group_id = nullable_string_field(patch, "groupId").flatten();
            conn.execute(
                "INSERT INTO notes (id, title, body, kind, audio_path, group_id, created_at)
                 VALUES (?1, NULL, ?2, ?3, ?4, ?5, datetime('now'))
                 ON CONFLICT(id) DO UPDATE SET
                   body = excluded.body,
                   kind = excluded.kind,
                   audio_path = excluded.audio_path,
                   group_id = excluded.group_id",
                params![operation.entity_id, body, kind, audio_path, group_id],
            )
            .map_err(|e| e.to_string())?;
            if let Some(filename) = audio_path {
                register_legacy_audio_reference(conn, profile_id, filename);
            }
            Ok(())
        }
        "update" => {
            if let Some(body) = string_field(patch, "body") {
                conn.execute(
                    "UPDATE notes SET body = ?1 WHERE id = ?2 AND kind != 'audio'",
                    params![body, operation.entity_id],
                )
                .map_err(|e| e.to_string())?;
            }
            if let Some(group_id) = nullable_string_field(patch, "groupId") {
                conn.execute(
                    "UPDATE notes SET group_id = ?1 WHERE id = ?2",
                    params![group_id, operation.entity_id],
                )
                .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        "delete" => conn
            .execute(
                "DELETE FROM notes WHERE id = ?1",
                params![operation.entity_id],
            )
            .map(|_| ())
            .map_err(|e| e.to_string()),
        _ => Ok(()),
    }
}

pub(super) fn apply_reminder(
    conn: &Connection,
    profile_id: &str,
    operation: &RemoteOperation,
    patch: &Value,
) -> Result<(), String> {
    match operation.op.as_str() {
        "create" => {
            let audio_path = string_field(patch, "audioPath");
            conn.execute(
                "INSERT INTO reminders (id, title, audio_path, trigger_at_utc, status, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
                 ON CONFLICT(id) DO UPDATE SET
                   title = excluded.title,
                   audio_path = excluded.audio_path,
                   trigger_at_utc = excluded.trigger_at_utc,
                   status = excluded.status",
                params![
                    operation.entity_id,
                    string_field(patch, "title").unwrap_or(""),
                    audio_path,
                    string_field(patch, "triggerAtUtc").unwrap_or("1970-01-01T00:00:00.000Z"),
                    string_field(patch, "status").unwrap_or("scheduled"),
                ],
            )
            .map_err(|e| e.to_string())?;
            if let Some(filename) = audio_path {
                register_legacy_audio_reference(conn, profile_id, filename);
            }
            Ok(())
        }
        "update" => {
            if let Some(status) = string_field(patch, "status") {
                conn.execute(
                    "UPDATE reminders SET status = ?1 WHERE id = ?2",
                    params![status, operation.entity_id],
                )
                .map_err(|e| e.to_string())?;
            }
            if let Some(trigger) = string_field(patch, "triggerAtUtc") {
                conn.execute(
                    "UPDATE reminders SET trigger_at_utc = ?1 WHERE id = ?2",
                    params![trigger, operation.entity_id],
                )
                .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        "delete" => conn
            .execute(
                "DELETE FROM reminders WHERE id = ?1",
                params![operation.entity_id],
            )
            .map(|_| ())
            .map_err(|e| e.to_string()),
        _ => Ok(()),
    }
}
