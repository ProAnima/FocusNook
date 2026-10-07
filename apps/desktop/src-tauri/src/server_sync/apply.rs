use super::apply_entities::{apply_note, apply_note_group, apply_plan_item, apply_reminder};
use super::credentials::ServerSyncCredentials;
use super::journal::{mark_delivered, mark_synced};
use super::patch::attachment_refs_from_operation;
use super::payload::decode_operation_payload;
use super::protocol::{RemoteOperation, SyncExchangeResponse};
use crate::{sync_blobs, sync_log};
use rusqlite::{params, Connection, OptionalExtension};

pub(super) fn operation_exists(
    conn: &Connection,
    profile_id: &str,
    operation_id: &str,
) -> Result<bool, String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT operation_id FROM sync_operations WHERE profile_id = ?1 AND operation_id = ?2",
            params![profile_id, operation_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(existing.is_some())
}

fn insert_remote_operation(
    conn: &Connection,
    profile_id: &str,
    operation: &RemoteOperation,
) -> Result<(), String> {
    if operation_exists(conn, profile_id, &operation.operation_id)? {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO sync_operations
            (operation_id, profile_id, device_id, entity_type, entity_id, op, patch, hlc,
             schema_version, created_at, synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'), datetime('now'))",
        params![
            operation.operation_id,
            profile_id,
            operation.device_id,
            operation.entity_type,
            operation.entity_id,
            operation.op,
            operation.payload_ciphertext,
            operation.hlc,
            operation.schema_version,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn latest_entity_hlc(
    conn: &Connection,
    profile_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT MAX(hlc)
         FROM sync_operations
         WHERE profile_id = ?1 AND entity_type = ?2 AND entity_id = ?3",
        params![profile_id, entity_type, entity_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

fn operation_is_stale_for_entity(
    conn: &Connection,
    profile_id: &str,
    operation: &RemoteOperation,
) -> Result<bool, String> {
    let latest = latest_entity_hlc(
        conn,
        profile_id,
        &operation.entity_type,
        &operation.entity_id,
    )?;
    Ok(latest
        .as_deref()
        .map(|hlc| hlc >= operation.hlc.as_str())
        .unwrap_or(false))
}

pub(crate) fn apply_remote_operation(
    conn: &Connection,
    profile_id: &str,
    local_device_id: &str,
    operation: &RemoteOperation,
    media_key: Option<&str>,
) -> Result<(), String> {
    if operation.device_id == local_device_id {
        insert_remote_operation(conn, profile_id, operation)?;
        return Ok(());
    }
    if operation_exists(conn, profile_id, &operation.operation_id)? {
        return Ok(());
    }
    if operation_is_stale_for_entity(conn, profile_id, operation)? {
        insert_remote_operation(conn, profile_id, operation)?;
        return Ok(());
    }

    let patch = decode_operation_payload(media_key, &operation.payload_ciphertext)?;
    match operation.entity_type.as_str() {
        "plan_item" => apply_plan_item(conn, operation, &patch)?,
        "note_group" => apply_note_group(conn, operation, &patch)?,
        "note" => apply_note(conn, profile_id, operation, &patch)?,
        "reminder" => apply_reminder(conn, profile_id, operation, &patch)?,
        _ => {}
    }
    insert_remote_operation(conn, profile_id, operation)
}

/// Everything an exchange response is applied against, besides the connection.
pub(super) struct ExchangeScope<'a> {
    pub(super) app: &'a tauri::AppHandle,
    pub(super) hlc_state: &'a sync_log::HlcClockState,
    pub(super) profile_id: &'a str,
    pub(super) remote_profile_id: &'a str,
    pub(super) credentials: &'a ServerSyncCredentials,
}

pub(super) fn apply_exchange_response(
    scope: &ExchangeScope<'_>,
    conn: &Connection,
    sent_operation_ids: &[String],
    response: SyncExchangeResponse,
) -> Result<(Option<String>, Vec<String>), String> {
    let ExchangeScope {
        app,
        hlc_state,
        profile_id,
        remote_profile_id,
        credentials,
    } = *scope;
    validate_confirmed_count(
        response.accepted_count,
        response.duplicate_count,
        sent_operation_ids.len(),
    )?;
    mark_synced(conn, sent_operation_ids)?;
    mark_delivered(conn, remote_profile_id, sent_operation_ids)?;
    let mut missing_blobs = Vec::new();
    let mut confirmed_pull_hlc = None;
    for operation in &response.operations {
        apply_remote_operation(
            conn,
            profile_id,
            &credentials.device_id,
            operation,
            credentials.media_key.as_deref(),
        )?;
        mark_delivered(
            conn,
            remote_profile_id,
            std::slice::from_ref(&operation.operation_id),
        )?;
        let parsed_hlc = sync_log::Hlc::parse(&operation.hlc)
            .ok_or_else(|| format!("remote operation has invalid hlc: {}", operation.hlc))?;
        hlc_state
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .observe(conn, &parsed_hlc)
            .map_err(|e| e.to_string())?;
        confirmed_pull_hlc = Some(operation.hlc.clone());
        if operation.device_id != credentials.device_id {
            let attachment_refs =
                attachment_refs_from_operation(operation, credentials.media_key.as_deref());
            for (blob_id, content_type) in &attachment_refs {
                if let Err(err) =
                    sync_blobs::ensure_downloadable_blob(conn, profile_id, blob_id, content_type)
                {
                    log::warn!("ignored invalid attachment reference {blob_id}: {err}");
                    continue;
                }
                missing_blobs.push(blob_id.clone());
            }
        }
        reconcile_remote_reminder_alarm(app, conn, &credentials.device_id, operation)?;
    }
    Ok((confirmed_pull_hlc.or(response.next_pull_hlc), missing_blobs))
}

pub(super) fn validate_confirmed_count(
    accepted_count: i64,
    duplicate_count: i64,
    sent_count: usize,
) -> Result<(), String> {
    let confirmed_count = accepted_count
        .checked_add(duplicate_count)
        .ok_or_else(|| "sync server returned invalid operation counters".to_string())?;
    if accepted_count < 0 || duplicate_count < 0 || confirmed_count != sent_count as i64 {
        return Err(format!(
            "sync server confirmed {confirmed_count} of {sent_count} sent operations"
        ));
    }
    Ok(())
}

pub(crate) fn reconcile_remote_reminder_alarm(
    app: &tauri::AppHandle,
    conn: &Connection,
    local_device_id: &str,
    operation: &RemoteOperation,
) -> Result<(), String> {
    if operation.entity_type != "reminder" || operation.device_id == local_device_id {
        return Ok(());
    }
    if operation.op == "delete" {
        crate::alarms::cancel_android_alarm(app, &operation.entity_id);
        return Ok(());
    }

    let reminder = conn
        .query_row(
            "SELECT id, title, audio_path, trigger_at_utc, status FROM reminders WHERE id = ?1",
            params![operation.entity_id],
            |row| {
                Ok(crate::reminders::ReminderDto {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    audio_path: row.get(2)?,
                    trigger_at_utc: row.get(3)?,
                    status: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    match reminder {
        Some(reminder) if reminder.status == "scheduled" => {
            crate::alarms::schedule_android_alarm(app, &reminder);
        }
        _ => crate::alarms::cancel_android_alarm(app, &operation.entity_id),
    }
    Ok(())
}

#[cfg(test)]
mod tests;
