use crate::sync_log::{self, HlcClock};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};

struct SnapshotEntity {
    entity_type: &'static str,
    id: String,
    patch: Value,
}

pub fn ensure_queued(
    conn: &mut Connection,
    clock: &mut HlcClock,
    local_profile_id: &str,
    remote_profile_id: &str,
) -> Result<(), String> {
    if is_seeded(conn, remote_profile_id)? {
        return Ok(());
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for entity in collect_entities(&tx)? {
        let hlc = clock.next(&tx).map_err(|e| e.to_string())?;
        sync_log::record_operation(
            &tx,
            &hlc,
            local_profile_id,
            entity.entity_type,
            &entity.id,
            "create",
            &entity.patch,
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute(
        "UPDATE sync_blobs SET uploaded_at = NULL
         WHERE profile_id = ?1 AND deleted_at IS NULL",
        params![local_profile_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO sync_snapshot_state (remote_profile_id, seeded_at)
         VALUES (?1, datetime('now'))",
        params![remote_profile_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn is_seeded(conn: &Connection, remote_profile_id: &str) -> Result<bool, String> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM sync_snapshot_state WHERE remote_profile_id = ?1",
            params![remote_profile_id],
            |_| Ok(()),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .is_some())
}

fn collect_entities(tx: &Transaction) -> Result<Vec<SnapshotEntity>, String> {
    let mut entities = plan_items(tx)?;
    entities.extend(note_groups(tx)?);
    entities.extend(notes(tx)?);
    entities.extend(reminders(tx)?);
    Ok(entities)
}

fn plan_items(tx: &Transaction) -> Result<Vec<SnapshotEntity>, String> {
    query_entities(
        tx,
        "SELECT id, title, status, progress_percent, plan_date, is_long_running
         FROM plan_items",
        "plan_item",
        |row| {
            Ok(json!({
                "title": row.get::<_, String>(1)?,
                "status": row.get::<_, String>(2)?,
                "progressPercent": row.get::<_, Option<i64>>(3)?,
                "planDate": row.get::<_, String>(4)?,
                "isLongRunning": row.get::<_, bool>(5)?,
            }))
        },
    )
}

fn note_groups(tx: &Transaction) -> Result<Vec<SnapshotEntity>, String> {
    query_entities(
        tx,
        "SELECT id, name FROM note_groups",
        "note_group",
        |row| Ok(json!({ "name": row.get::<_, String>(1)? })),
    )
}

fn notes(tx: &Transaction) -> Result<Vec<SnapshotEntity>, String> {
    query_entities(
        tx,
        "SELECT id, body, kind, audio_path, group_id FROM notes",
        "note",
        |row| {
            Ok(json!({
                "body": row.get::<_, String>(1)?,
                "kind": row.get::<_, String>(2)?,
                "audioPath": row.get::<_, Option<String>>(3)?,
                "groupId": row.get::<_, Option<String>>(4)?,
            }))
        },
    )
}

fn reminders(tx: &Transaction) -> Result<Vec<SnapshotEntity>, String> {
    query_entities(
        tx,
        "SELECT id, title, audio_path, trigger_at_utc, status FROM reminders",
        "reminder",
        |row| {
            Ok(json!({
                "title": row.get::<_, String>(1)?,
                "audioPath": row.get::<_, Option<String>>(2)?,
                "triggerAtUtc": row.get::<_, String>(3)?,
                "status": row.get::<_, String>(4)?,
            }))
        },
    )
}

fn query_entities<F>(
    tx: &Transaction,
    sql: &str,
    entity_type: &'static str,
    patch: F,
) -> Result<Vec<SnapshotEntity>, String>
where
    F: Fn(&rusqlite::Row<'_>) -> rusqlite::Result<Value>,
{
    let mut statement = tx.prepare(sql).map_err(|e| e.to_string())?;
    let entities = statement
        .query_map([], |row| {
            Ok(SnapshotEntity {
                entity_type,
                id: row.get(0)?,
                patch: patch(row)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(entities)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn setup() -> (Connection, HlcClock) {
        let conn = Connection::open_in_memory().unwrap();
        for migration in crate::db::MIGRATIONS {
            conn.execute(migration, []).unwrap();
        }
        conn.execute(
            "INSERT INTO plan_items
             (id, title, status, progress_percent, plan_date, created_at)
             VALUES ('task-1', 'Keep me', 'open', NULL, '2026-07-24', datetime('now'))",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE plan_items SET is_long_running = 1 WHERE id = 'task-1'",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO reminders
             (id, title, audio_path, trigger_at_utc, status, created_at)
             VALUES ('reminder-1', 'Later', NULL, '2026-07-25T10:00:00.000Z', 'acknowledged',
                     datetime('now'))",
            [],
        )
        .unwrap();
        let device_id = sync_log::ensure_device_identity(&conn).unwrap();
        let clock = HlcClock::load(&conn, device_id).unwrap();
        (conn, clock)
    }

    #[test]
    fn queues_a_snapshot_once_for_each_remote_account() {
        let (mut conn, mut clock) = setup();
        ensure_queued(&mut conn, &mut clock, "local", "account-a").unwrap();
        ensure_queued(&mut conn, &mut clock, "local", "account-a").unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sync_operations WHERE entity_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        let task_patch: String = conn
            .query_row(
                "SELECT patch FROM sync_operations WHERE entity_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(task_patch.contains("\"isLongRunning\":true"));
        let reminder_patch: String = conn
            .query_row(
                "SELECT patch FROM sync_operations WHERE entity_id = 'reminder-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(reminder_patch.contains("\"status\":\"acknowledged\""));
    }
}
