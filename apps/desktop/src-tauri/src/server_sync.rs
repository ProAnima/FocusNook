use crate::{blob_crypto, config, profiles, sync_blobs, sync_log};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};

#[cfg(not(target_os = "android"))]
const SERVER_SYNC_KEYRING_SERVICE: &str = "com.proanima.focusnook.server-sync";
#[cfg(not(target_os = "android"))]
const SERVER_SYNC_KEY_PREFIX: &str = "vds_server";
const MAX_OPS_PER_EXCHANGE: usize = 100;
const MAX_EXCHANGE_ROUNDS: usize = 20;
static SYNC_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static SYNC_RERUN_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
struct ServerSyncCredentials {
    #[serde(default)]
    account_email: Option<String>,
    #[serde(default)]
    account_user_id: Option<String>,
    #[serde(default)]
    device_id: String,
    endpoint: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    media_key: Option<String>,
    token: String,
}

#[derive(Clone, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerSyncStatus {
    available: bool,
    account_email: Option<String>,
    account_user_id: Option<String>,
    connected: bool,
    display_name: Option<String>,
    endpoint: Option<String>,
    message: Option<String>,
}

#[cfg(not(target_os = "android"))]
fn keyring_user(profile_id: &str) -> String {
    format!("{SERVER_SYNC_KEY_PREFIX}-{profile_id}")
}

fn host_after_scheme<'a>(endpoint: &'a str, scheme: &str) -> Option<&'a str> {
    let rest = endpoint.get(scheme.len()..)?;
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .map(|value| value.trim_matches('[').trim_matches(']'))?;
    if host.is_empty() || host.starts_with(':') {
        return None;
    }
    Some(host.split(':').next().unwrap_or(host))
}

fn is_local_http_endpoint(endpoint: &str) -> bool {
    let Some(host) = host_after_scheme(endpoint, "http://") else {
        return false;
    };
    matches!(host, "localhost" | "127.0.0.1")
}

fn normalize_endpoint(raw: &str) -> Result<String, String> {
    let endpoint = raw.trim().trim_end_matches('/').to_string();
    if endpoint.is_empty() {
        return Err("sync server endpoint is required".to_string());
    }
    if endpoint.len() > 2048 || endpoint.chars().any(char::is_whitespace) {
        return Err("sync server endpoint is invalid".to_string());
    }

    let lower = endpoint.to_ascii_lowercase();
    let is_https = lower.starts_with("https://") && host_after_scheme(&lower, "https://").is_some();
    let is_local_dev = is_local_http_endpoint(&lower);
    if !is_https && !is_local_dev {
        return Err("sync server endpoint must use https".to_string());
    }

    Ok(endpoint)
}

fn normalize_token(raw: &str) -> Result<String, String> {
    let token = raw.trim().to_string();
    if token.is_empty() {
        return Err("sync server token is required".to_string());
    }
    if token.len() > 8192 {
        return Err("sync server token is too long".to_string());
    }
    Ok(token)
}

#[cfg(not(target_os = "android"))]
fn entry(profile_id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVER_SYNC_KEYRING_SERVICE, &keyring_user(profile_id))
        .map_err(|e| e.to_string())
}

fn store_credentials(
    conn: Option<&Connection>,
    profile_id: &str,
    endpoint: &str,
    token: &str,
    device_id: &str,
    account: Option<&ServerAccountSession>,
    media_key: Option<String>,
) -> Result<(), String> {
    let credentials = ServerSyncCredentials {
        account_email: account.map(|value| value.email.clone()),
        account_user_id: account.map(|value| value.user_id.clone()),
        device_id: normalize_token(device_id)?,
        display_name: account.map(|value| value.display_name.clone()),
        endpoint: normalize_endpoint(endpoint)?,
        media_key,
        token: normalize_token(token)?,
    };
    let raw = serde_json::to_string(&credentials).map_err(|e| e.to_string())?;
    store_credentials_raw(conn, profile_id, &raw)
}

#[cfg(not(target_os = "android"))]
fn store_credentials_raw(
    _conn: Option<&Connection>,
    profile_id: &str,
    raw: &str,
) -> Result<(), String> {
    entry(profile_id)?
        .set_password(raw)
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "android")]
fn store_credentials_raw(
    conn: Option<&Connection>,
    profile_id: &str,
    raw: &str,
) -> Result<(), String> {
    let conn =
        conn.ok_or_else(|| "local database is required for Android server sync".to_string())?;
    conn.execute(
        "INSERT INTO server_sync_credentials (profile_id, raw_json, updated_at)
         VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(profile_id) DO UPDATE SET raw_json = excluded.raw_json, updated_at = datetime('now')",
        params![profile_id, raw],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn load_credentials(
    conn: Option<&Connection>,
    profile_id: &str,
) -> Result<Option<ServerSyncCredentials>, String> {
    let raw = load_credentials_raw(conn, profile_id)?;
    raw.map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
        .transpose()
}

#[cfg(not(target_os = "android"))]
fn load_credentials_raw(
    _conn: Option<&Connection>,
    profile_id: &str,
) -> Result<Option<String>, String> {
    match entry(profile_id)?.get_password() {
        Ok(raw) => Ok(Some(raw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(target_os = "android")]
fn load_credentials_raw(
    conn: Option<&Connection>,
    profile_id: &str,
) -> Result<Option<String>, String> {
    let conn =
        conn.ok_or_else(|| "local database is required for Android server sync".to_string())?;
    conn.query_row(
        "SELECT raw_json FROM server_sync_credentials WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn delete_credentials(conn: Option<&Connection>, profile_id: &str) -> Result<(), String> {
    delete_credentials_raw(conn, profile_id)
}

#[cfg(not(target_os = "android"))]
fn delete_credentials_raw(_conn: Option<&Connection>, profile_id: &str) -> Result<(), String> {
    match entry(profile_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(target_os = "android")]
fn delete_credentials_raw(conn: Option<&Connection>, profile_id: &str) -> Result<(), String> {
    let conn =
        conn.ok_or_else(|| "local database is required for Android server sync".to_string())?;
    conn.execute(
        "DELETE FROM server_sync_credentials WHERE profile_id = ?1",
        params![profile_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn status_for_profile(
    conn: Option<&Connection>,
    profile_id: &str,
    configured: bool,
    message: Option<String>,
) -> Result<ServerSyncStatus, String> {
    let credentials = load_credentials(conn, profile_id)?;
    Ok(ServerSyncStatus {
        available: configured,
        account_email: credentials
            .as_ref()
            .and_then(|value| value.account_email.clone()),
        account_user_id: credentials
            .as_ref()
            .and_then(|value| value.account_user_id.clone()),
        connected: credentials.is_some(),
        display_name: credentials
            .as_ref()
            .and_then(|value| value.display_name.clone()),
        endpoint: credentials.map(|value| value.endpoint),
        message,
    })
}

fn endpoint_from_config(config_state: &config::SyncProvidersConfig) -> Result<String, String> {
    let endpoint = config_state
        .server
        .as_ref()
        .map(|value| value.endpoint.as_str())
        .unwrap_or(config::DEFAULT_SERVER_ENDPOINT);
    normalize_endpoint(endpoint)
}

#[tauri::command]
pub fn server_sync_status(
    db: tauri::State<crate::db::Db>,
    config_state: tauri::State<config::SyncProvidersConfig>,
    profiles_state: tauri::State<profiles::ProfilesState>,
) -> Result<ServerSyncStatus, String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let configured = endpoint_from_config(&config_state).is_ok();
    let message = (!configured).then(|| "server sync endpoint is not configured".to_string());
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    status_for_profile(Some(&conn), &profile_id, configured, message)
}

#[tauri::command]
pub fn connect_server_sync(
    db: tauri::State<crate::db::Db>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    endpoint: String,
    token: String,
) -> Result<ServerSyncStatus, String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store_credentials(
        Some(&conn),
        &profile_id,
        &endpoint,
        &token,
        "manual-device",
        None,
        None,
    )?;
    status_for_profile(Some(&conn), &profile_id, true, None)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterDeviceRequest<'a> {
    device_id: &'a str,
    display_name: &'a str,
    platform: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterDeviceResponse {
    device_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountAuthRequest<'a> {
    email: &'a str,
    password: &'a str,
    display_name: Option<&'a str>,
    device_id: &'a str,
    device_name: &'a str,
    platform: &'a str,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountAuthResponse {
    device_token: String,
    email: String,
    display_name: String,
    user_id: String,
}

struct ServerAccountSession {
    email: String,
    display_name: String,
    user_id: String,
}

#[derive(Clone)]
struct LocalOperation {
    operation_id: String,
    device_id: String,
    entity_type: String,
    entity_id: String,
    op: String,
    patch: String,
    hlc: String,
    schema_version: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncExchangeRequest {
    device_id: String,
    last_pulled_hlc: Option<String>,
    operations: Vec<ClientOperation>,
    profile_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientOperation {
    device_id: String,
    entity_id: String,
    entity_type: String,
    hlc: String,
    op: String,
    operation_id: String,
    payload_ciphertext: String,
    payload_key_id: Option<String>,
    payload_nonce: Option<String>,
    schema_version: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncExchangeResponse {
    accepted_count: i64,
    duplicate_count: i64,
    next_pull_hlc: Option<String>,
    operations: Vec<RemoteOperation>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteOperation {
    device_id: String,
    entity_id: String,
    entity_type: String,
    hlc: String,
    op: String,
    operation_id: String,
    payload_ciphertext: String,
    schema_version: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadBlobResponse {
    blob_id: String,
    size_bytes: i64,
}

async fn register_device(
    endpoint: &str,
    user_token: &str,
    device_id: &str,
) -> Result<String, String> {
    let response = reqwest::Client::new()
        .post(format!("{endpoint}/v1/devices"))
        .bearer_auth(user_token)
        .json(&RegisterDeviceRequest {
            device_id,
            display_name: "FocusNook desktop",
            platform: std::env::consts::OS,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("sync server returned {}", response.status()));
    }
    let body = response
        .json::<RegisterDeviceResponse>()
        .await
        .map_err(|e| e.to_string())?;
    normalize_token(&body.device_token)
}

fn ensure_local_device_id(conn: &Connection) -> Result<String, String> {
    sync_log::ensure_device_identity(conn)
}

async fn authenticate_account(
    endpoint: &str,
    path: &str,
    email: &str,
    password: &str,
    display_name: Option<&str>,
    device_id: &str,
) -> Result<(String, ServerAccountSession), String> {
    let response = reqwest::Client::new()
        .post(format!("{endpoint}{path}"))
        .json(&AccountAuthRequest {
            email,
            password,
            display_name,
            device_id,
            device_name: device_display_name(),
            platform: std::env::consts::OS,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("sync server returned {}", response.status()));
    }
    let body = response
        .json::<AccountAuthResponse>()
        .await
        .map_err(|e| e.to_string())?;
    Ok((
        normalize_token(&body.device_token)?,
        ServerAccountSession {
            email: body.email,
            display_name: body.display_name,
            user_id: body.user_id,
        },
    ))
}

fn device_display_name() -> &'static str {
    if cfg!(target_os = "android") {
        "FocusNook Android"
    } else {
        "FocusNook desktop"
    }
}

fn last_pulled_hlc(conn: &Connection, profile_id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT last_pulled_hlc FROM sync_pull_state WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn store_last_pulled_hlc(
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

fn unsynced_operations(conn: &Connection, profile_id: &str) -> Result<Vec<LocalOperation>, String> {
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
        .query_map(params![profile_id, MAX_OPS_PER_EXCHANGE as i64], |row| {
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

fn mark_synced(conn: &Connection, operation_ids: &[String]) -> Result<(), String> {
    for id in operation_ids {
        conn.execute(
            "UPDATE sync_operations SET synced_at = datetime('now') WHERE operation_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn client_operation(operation: &LocalOperation) -> ClientOperation {
    ClientOperation {
        device_id: operation.device_id.clone(),
        entity_id: operation.entity_id.clone(),
        entity_type: operation.entity_type.clone(),
        hlc: operation.hlc.clone(),
        op: operation.op.clone(),
        operation_id: operation.operation_id.clone(),
        payload_ciphertext: operation.patch.clone(),
        payload_key_id: None,
        payload_nonce: None,
        schema_version: operation.schema_version,
    }
}

async fn exchange_with_server(
    credentials: &ServerSyncCredentials,
    profile_id: &str,
    last_pulled: Option<String>,
    operations: &[LocalOperation],
) -> Result<SyncExchangeResponse, String> {
    let request = SyncExchangeRequest {
        device_id: credentials.device_id.clone(),
        last_pulled_hlc: last_pulled,
        operations: operations.iter().map(client_operation).collect(),
        profile_id: profile_id.to_string(),
    };
    let response = reqwest::Client::new()
        .post(format!("{}/v1/sync/exchange", credentials.endpoint))
        .bearer_auth(&credentials.token)
        .json(&request)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("sync server returned {}", response.status()));
    }
    response
        .json::<SyncExchangeResponse>()
        .await
        .map_err(|e| e.to_string())
}

async fn upload_prepared_blobs(
    credentials: &ServerSyncCredentials,
    uploads: Vec<sync_blobs::UploadBlobRequest>,
) -> Result<Vec<(String, String, i64)>, String> {
    let client = reqwest::Client::new();
    let mut uploaded = Vec::with_capacity(uploads.len());
    for request in uploads {
        let sha256 = request.sha256.clone();
        let response = client
            .post(format!("{}/v1/blobs", credentials.endpoint))
            .bearer_auth(&credentials.token)
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("sync blob upload returned {}", response.status()));
        }
        let body = response
            .json::<UploadBlobResponse>()
            .await
            .map_err(|e| e.to_string())?;
        uploaded.push((body.blob_id, sha256, body.size_bytes));
    }
    Ok(uploaded)
}

async fn download_blob(
    credentials: &ServerSyncCredentials,
    profile_id: &str,
    blob_id: &str,
) -> Result<Option<sync_blobs::DownloadBlobResponse>, String> {
    let response = reqwest::Client::new()
        .get(format!(
            "{}/v1/blobs/{}/{}",
            credentials.endpoint, profile_id, blob_id
        ))
        .bearer_auth(&credentials.token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        eprintln!("server-sync: blob {blob_id} is missing on server, keeping metadata only");
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(format!("sync blob download returned {}", response.status()));
    }
    response
        .json::<sync_blobs::DownloadBlobResponse>()
        .await
        .map(Some)
        .map_err(|e| e.to_string())
}

async fn upload_pending_blobs(
    db: &crate::db::Db,
    credentials: &ServerSyncCredentials,
    profile_id: &str,
    audio_dir: &std::path::Path,
    audio_key: Option<&str>,
) -> Result<(), String> {
    let prepared_uploads = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let pending = sync_blobs::pending_uploads(&conn, profile_id)?;
        if pending.is_empty() {
            Vec::new()
        } else {
            let media_key = credentials.media_key.as_deref().ok_or_else(|| {
                "server sync media key is missing; sign in again to enable encrypted attachments"
                    .to_string()
            })?;
            pending
                .iter()
                .map(|record| {
                    sync_blobs::upload_request(
                        &conn, profile_id, audio_dir, audio_key, media_key, record,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    let uploaded = upload_prepared_blobs(credentials, prepared_uploads).await?;
    if !uploaded.is_empty() {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        for (blob_id, sha256, size_bytes) in uploaded {
            sync_blobs::mark_uploaded(&conn, profile_id, &blob_id, &sha256, size_bytes)?;
        }
    }
    Ok(())
}

fn operation_exists(conn: &Connection, operation_id: &str) -> Result<bool, String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT operation_id FROM sync_operations WHERE operation_id = ?1",
            params![operation_id],
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
    conn.execute(
        "INSERT OR IGNORE INTO sync_operations
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

fn string_field<'a>(patch: &'a Value, key: &str) -> Option<&'a str> {
    patch.get(key).and_then(Value::as_str)
}

fn nullable_string_field(patch: &Value, key: &str) -> Option<Option<String>> {
    patch.get(key).map(|value| {
        if value.is_null() {
            None
        } else {
            value.as_str().map(str::to_string)
        }
    })
}

fn audio_blob_id_from_operation(operation: &RemoteOperation) -> Option<String> {
    if !matches!(operation.entity_type.as_str(), "note" | "reminder") || operation.op == "delete" {
        return None;
    }
    let patch = serde_json::from_str::<Value>(&operation.payload_ciphertext).ok()?;
    string_field(&patch, "audioPath").map(str::to_string)
}

fn apply_plan_item(
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
            conn.execute(
                "INSERT INTO plan_items (id, title, status, progress_percent, plan_date, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
                 ON CONFLICT(id) DO UPDATE SET
                   title = excluded.title,
                   status = excluded.status,
                   progress_percent = excluded.progress_percent,
                   plan_date = excluded.plan_date",
                params![operation.entity_id, title, status, progress, plan_date],
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

fn apply_note_group(
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

fn apply_note(
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
                sync_blobs::ensure_audio_blob(conn, profile_id, filename)?;
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

fn apply_reminder(
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
                 VALUES (?1, ?2, ?3, ?4, 'scheduled', datetime('now'))
                 ON CONFLICT(id) DO UPDATE SET
                   title = excluded.title,
                   audio_path = excluded.audio_path,
                   trigger_at_utc = excluded.trigger_at_utc",
                params![
                    operation.entity_id,
                    string_field(patch, "title").unwrap_or(""),
                    audio_path,
                    string_field(patch, "triggerAtUtc").unwrap_or("1970-01-01T00:00:00.000Z"),
                ],
            )
            .map_err(|e| e.to_string())?;
            if let Some(filename) = audio_path {
                sync_blobs::ensure_audio_blob(conn, profile_id, filename)?;
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

fn apply_remote_operation(
    conn: &Connection,
    profile_id: &str,
    local_device_id: &str,
    operation: &RemoteOperation,
) -> Result<(), String> {
    if operation.device_id == local_device_id {
        insert_remote_operation(conn, profile_id, operation)?;
        return Ok(());
    }
    if operation_exists(conn, &operation.operation_id)? {
        return Ok(());
    }

    let patch = serde_json::from_str::<Value>(&operation.payload_ciphertext)
        .map_err(|e| format!("remote operation payload is invalid json: {e}"))?;
    match operation.entity_type.as_str() {
        "plan_item" => apply_plan_item(conn, operation, &patch)?,
        "note_group" => apply_note_group(conn, operation, &patch)?,
        "note" => apply_note(conn, profile_id, operation, &patch)?,
        "reminder" => apply_reminder(conn, profile_id, operation, &patch)?,
        _ => {}
    }
    insert_remote_operation(conn, profile_id, operation)
}

fn apply_exchange_response(
    app: &tauri::AppHandle,
    conn: &Connection,
    profile_id: &str,
    credentials: &ServerSyncCredentials,
    sent_operation_ids: &[String],
    response: SyncExchangeResponse,
) -> Result<(Option<String>, Vec<String>), String> {
    let _accepted = response.accepted_count;
    let _duplicates = response.duplicate_count;
    mark_synced(conn, sent_operation_ids)?;
    let mut missing_blobs = Vec::new();
    for operation in &response.operations {
        apply_remote_operation(conn, profile_id, &credentials.device_id, operation)?;
        if operation.device_id != credentials.device_id {
            if let Some(blob_id) = audio_blob_id_from_operation(operation) {
                missing_blobs.push(blob_id);
            }
        }
        reconcile_remote_reminder_alarm(app, conn, &credentials.device_id, operation)?;
    }
    Ok((response.next_pull_hlc, missing_blobs))
}

fn reconcile_remote_reminder_alarm(
    app: &tauri::AppHandle,
    conn: &Connection,
    local_device_id: &str,
    operation: &RemoteOperation,
) -> Result<(), String> {
    if operation.entity_type != "reminder" || operation.device_id == local_device_id {
        return Ok(());
    }
    if operation.op == "delete" {
        crate::cancel_android_alarm(app, &operation.entity_id);
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
            crate::schedule_android_alarm(app, &reminder);
        }
        _ => crate::cancel_android_alarm(app, &operation.entity_id),
    }
    Ok(())
}

async fn perform_sync(app: tauri::AppHandle) -> Result<ServerSyncStatus, String> {
    let db = app.state::<crate::db::Db>();
    let config = app.state::<config::SyncProvidersConfig>();
    let profiles_state = app.state::<profiles::ProfilesState>();
    let audio_key_state = app.state::<crate::AudioKeyState>();
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let configured = endpoint_from_config(&config).is_ok();
    let audio_dir = profiles::data_dir(&profiles_state).join("audio");
    let audio_key = audio_key_state.0.lock().map_err(|e| e.to_string())?.clone();

    let credentials = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        load_credentials(Some(&conn), &profile_id)?
            .ok_or_else(|| "server sync account is not connected".to_string())?
    };

    for _ in 0..MAX_EXCHANGE_ROUNDS {
        upload_pending_blobs(
            &db,
            &credentials,
            &profile_id,
            &audio_dir,
            audio_key.as_deref(),
        )
        .await?;

        let (last_pulled, operations) = {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            let last_pulled = last_pulled_hlc(&conn, &profile_id)?;
            let operations = unsynced_operations(&conn, &profile_id)?;
            (last_pulled, operations)
        };
        upload_pending_blobs(
            &db,
            &credentials,
            &profile_id,
            &audio_dir,
            audio_key.as_deref(),
        )
        .await?;
        let sent_full_page = operations.len() == MAX_OPS_PER_EXCHANGE;
        let sent_operation_ids = operations
            .iter()
            .map(|operation| operation.operation_id.clone())
            .collect::<Vec<_>>();
        let response =
            exchange_with_server(&credentials, &profile_id, last_pulled, &operations).await?;
        let pulled_full_page = response.operations.len() == MAX_OPS_PER_EXCHANGE;

        let (next_pull_hlc, mut missing_blobs) = {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            apply_exchange_response(
                &app,
                &conn,
                &profile_id,
                &credentials,
                &sent_operation_ids,
                response,
            )?
        };
        missing_blobs.sort();
        missing_blobs.dedup();
        if !missing_blobs.is_empty() {
            let media_key = credentials.media_key.as_deref().ok_or_else(|| {
                "server sync media key is missing; sign in again to enable encrypted attachments"
                    .to_string()
            })?;
            for blob_id in missing_blobs {
                let Some(downloaded) = download_blob(&credentials, &profile_id, &blob_id).await?
                else {
                    continue;
                };
                let conn = db.0.lock().map_err(|e| e.to_string())?;
                sync_blobs::materialize_download(
                    &conn,
                    &profile_id,
                    &audio_dir,
                    audio_key.as_deref(),
                    media_key,
                    &blob_id,
                    &downloaded,
                )?;
            }
        }

        let conn = db.0.lock().map_err(|e| e.to_string())?;
        store_last_pulled_hlc(&conn, &profile_id, next_pull_hlc.as_deref())?;

        if !sent_full_page && !pulled_full_page {
            return status_for_profile(Some(&conn), &profile_id, configured, None);
        }
    }

    Err("server sync backlog is too large, please run sync again".to_string())
}

pub fn spawn_best_effort(app: tauri::AppHandle) {
    if SYNC_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        SYNC_RERUN_REQUESTED.store(true, Ordering::SeqCst);
        return;
    }
    tauri::async_runtime::spawn(async move {
        let mut result;
        loop {
            SYNC_RERUN_REQUESTED.store(false, Ordering::SeqCst);
            result = perform_sync(app.clone()).await;
            if !SYNC_RERUN_REQUESTED.swap(false, Ordering::SeqCst) {
                break;
            }
        }
        SYNC_IN_FLIGHT.store(false, Ordering::SeqCst);
        if SYNC_RERUN_REQUESTED.swap(false, Ordering::SeqCst) {
            spawn_best_effort(app);
            return;
        }
        match result {
            Ok(_) => {
                let _ = app.emit("server-sync-completed", ());
            }
            Err(err) => {
                eprintln!("server-sync: best-effort sync failed: {err}");
                let _ = app.emit("server-sync-failed", err);
            }
        }
    });
}

#[tauri::command]
pub async fn sync_server_now(app: tauri::AppHandle) -> Result<ServerSyncStatus, String> {
    perform_sync(app).await
}

#[tauri::command]
pub async fn connect_default_server_sync(
    db: tauri::State<'_, crate::db::Db>,
    config_state: tauri::State<'_, config::SyncProvidersConfig>,
    profiles_state: tauri::State<'_, profiles::ProfilesState>,
) -> Result<ServerSyncStatus, String> {
    let bootstrap = config_state
        .server
        .clone()
        .ok_or_else(|| "server sync bootstrap is not configured".to_string())?;
    let endpoint = normalize_endpoint(&bootstrap.endpoint)?;
    let user_token = bootstrap
        .user_token
        .as_deref()
        .ok_or_else(|| "legacy server sync bootstrap token is not configured".to_string())?;
    let device_id = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        ensure_local_device_id(&conn)?
    };
    let token = register_device(&endpoint, user_token, &device_id).await?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store_credentials(
        Some(&conn),
        &profile_id,
        &endpoint,
        &token,
        &device_id,
        None,
        None,
    )?;
    status_for_profile(Some(&conn), &profile_id, true, None)
}

#[tauri::command]
pub async fn register_server_account(
    app: tauri::AppHandle,
    db: tauri::State<'_, crate::db::Db>,
    config_state: tauri::State<'_, config::SyncProvidersConfig>,
    profiles_state: tauri::State<'_, profiles::ProfilesState>,
    email: String,
    password: String,
    display_name: String,
) -> Result<ServerSyncStatus, String> {
    let endpoint = endpoint_from_config(&config_state)?;
    let device_id = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        ensure_local_device_id(&conn)?
    };
    let display_name = display_name.trim().to_string();
    let display_name_ref = (!display_name.is_empty()).then_some(display_name.as_str());
    let (token, account) = authenticate_account(
        &endpoint,
        "/v1/accounts/register",
        &email,
        &password,
        display_name_ref,
        &device_id,
    )
    .await?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store_credentials(
        Some(&conn),
        &profile_id,
        &endpoint,
        &token,
        &device_id,
        Some(&account),
        Some(blob_crypto::derive_media_key(&account.email, &password)),
    )?;
    let status = status_for_profile(Some(&conn), &profile_id, true, None)?;
    drop(conn);
    spawn_best_effort(app);
    Ok(status)
}

#[tauri::command]
pub async fn login_server_account(
    app: tauri::AppHandle,
    db: tauri::State<'_, crate::db::Db>,
    config_state: tauri::State<'_, config::SyncProvidersConfig>,
    profiles_state: tauri::State<'_, profiles::ProfilesState>,
    email: String,
    password: String,
) -> Result<ServerSyncStatus, String> {
    let endpoint = endpoint_from_config(&config_state)?;
    let device_id = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        ensure_local_device_id(&conn)?
    };
    let (token, account) = authenticate_account(
        &endpoint,
        "/v1/accounts/login",
        &email,
        &password,
        None,
        &device_id,
    )
    .await?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store_credentials(
        Some(&conn),
        &profile_id,
        &endpoint,
        &token,
        &device_id,
        Some(&account),
        Some(blob_crypto::derive_media_key(&account.email, &password)),
    )?;
    let status = status_for_profile(Some(&conn), &profile_id, true, None)?;
    drop(conn);
    spawn_best_effort(app);
    Ok(status)
}

#[tauri::command]
pub fn disconnect_server_sync(
    db: tauri::State<crate::db::Db>,
    profiles_state: tauri::State<profiles::ProfilesState>,
) -> Result<(), String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    delete_credentials(Some(&conn), &profile_id)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn unique_profile_id() -> String {
        format!("server-sync-test-{}", uuid::Uuid::now_v7())
    }

    fn sync_test_conn() -> Connection {
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
        conn
    }

    #[test]
    fn stores_server_credentials_per_profile() -> Result<(), String> {
        let profile_id = unique_profile_id();

        store_credentials(
            None,
            &profile_id,
            "https://sync.example.com/",
            "secret-token",
            "device-a",
            None,
            None,
        )?;

        let status = status_for_profile(None, &profile_id, true, None)?;
        assert_eq!(
            status,
            ServerSyncStatus {
                available: true,
                account_email: None,
                account_user_id: None,
                connected: true,
                display_name: None,
                endpoint: Some("https://sync.example.com".to_string()),
                message: None
            }
        );

        delete_credentials(None, &profile_id)?;
        Ok(())
    }

    #[test]
    fn rejects_non_https_remote_endpoint() {
        let err = normalize_endpoint("http://sync.example.com").err();
        assert_eq!(err, Some("sync server endpoint must use https".to_string()));
    }

    #[test]
    fn allows_local_http_for_development() -> Result<(), String> {
        assert_eq!(
            normalize_endpoint("http://localhost:8080/api/")?,
            "http://localhost:8080/api"
        );
        Ok(())
    }

    #[test]
    fn rejects_localhost_prefix_spoofing() {
        let err = normalize_endpoint("http://localhost.example.com").err();
        assert_eq!(err, Some("sync server endpoint must use https".to_string()));
    }

    #[test]
    fn deleting_missing_credentials_is_ok() -> Result<(), String> {
        delete_credentials(None, &unique_profile_id())
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
                "planDate": "2026-07-07"
            })
            .to_string(),
            schema_version: 1,
        };

        apply_remote_operation(&conn, "profile-1", "desktop-device", &operation)?;
        apply_remote_operation(&conn, "profile-1", "desktop-device", &operation)?;

        let title: String = conn
            .query_row(
                "SELECT title FROM plan_items WHERE id = 'task-1'",
                [],
                |row| row.get(0),
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
        assert_eq!(op_count, 1);
        assert_eq!(synced_count, 1);
        Ok(())
    }
}
