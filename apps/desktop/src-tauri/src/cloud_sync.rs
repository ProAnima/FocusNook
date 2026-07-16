use crate::oauth::{self, ProviderId};
use crate::{config, profiles, server_sync, sync_blobs, sync_log};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::header::{CONTENT_TYPE, ETAG, IF_MATCH};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};

const GOOGLE_DRIVE_LIST_URL: &str = "https://www.googleapis.com/drive/v3/files";
const GOOGLE_DRIVE_UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
const JOURNAL_NAME: &str = "focusnook-sync-journal-v1.json";
const MAX_CLOUD_SYNC_ROUNDS: usize = 6;
const PERIODIC_CLOUD_SYNC_INTERVAL: std::time::Duration = std::time::Duration::from_secs(75);
static CLOUD_SYNC_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static CLOUD_SYNC_RERUN_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncStatus {
    connected: bool,
    last_operation_hlc: Option<String>,
    message: Option<String>,
    provider: ProviderId,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CloudJournal {
    media_key: String,
    operations: Vec<server_sync::RemoteOperation>,
    version: u32,
}

impl CloudJournal {
    fn empty() -> Self {
        Self {
            media_key: generate_media_key(),
            operations: Vec::new(),
            version: 1,
        }
    }
}

#[derive(Clone)]
struct RemoteFile {
    etag: Option<String>,
    id: String,
}

#[derive(Deserialize)]
struct DriveListResponse {
    files: Vec<DriveListedFile>,
}

#[derive(Deserialize)]
struct DriveListedFile {
    id: String,
}

fn generate_media_key() -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"focusnook-google-drive-media-key-v1");
    hasher.update(uuid::Uuid::new_v4().as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn provider_scope_key(provider: ProviderId, profile_id: &str) -> String {
    format!("{}:{profile_id}", provider.keyring_prefix())
}

#[cfg(not(target_os = "android"))]
fn credentials_for(
    config: &config::SyncProvidersConfig,
    provider: ProviderId,
) -> Option<&config::ProviderCredentials> {
    match provider {
        ProviderId::GoogleDrive => config.google.as_ref(),
        ProviderId::YandexDisk => config.yandex.as_ref(),
    }
}

async fn google_access_token(
    app: &tauri::AppHandle,
    _config: &config::SyncProvidersConfig,
    profile_id: &str,
) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        let creds = config::ProviderCredentials {
            client_id: String::new(),
            client_secret: None,
        };
        return oauth::ensure_valid_token(app, ProviderId::GoogleDrive, profile_id, &creds).await;
    }

    #[cfg(not(target_os = "android"))]
    {
        let creds = credentials_for(_config, ProviderId::GoogleDrive)
            .ok_or_else(|| "Google Drive sync is not configured".to_string())?;
        oauth::ensure_valid_token(app, ProviderId::GoogleDrive, profile_id, creds).await
    }
}

fn escaped_drive_name(name: &str) -> String {
    name.replace('\\', "\\\\").replace('\'', "\\'")
}

fn blob_file_name(blob_id: &str) -> String {
    format!("focusnook-blob-{blob_id}.json")
}

async fn find_google_file(access_token: &str, name: &str) -> Result<Option<RemoteFile>, String> {
    let query = format!("name='{}' and trashed=false", escaped_drive_name(name));
    let response = reqwest::Client::new()
        .get(GOOGLE_DRIVE_LIST_URL)
        .bearer_auth(access_token)
        .query(&[
            ("spaces", "appDataFolder"),
            ("q", query.as_str()),
            ("fields", "files(id)"),
            ("pageSize", "1"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Google Drive list returned {}", response.status()));
    }
    let body = response
        .json::<DriveListResponse>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(body.files.into_iter().next().map(|file| RemoteFile {
        etag: None,
        id: file.id,
    }))
}

async fn download_google_file(
    access_token: &str,
    file: &RemoteFile,
) -> Result<Option<(Vec<u8>, RemoteFile)>, String> {
    let response = reqwest::Client::new()
        .get(format!("{GOOGLE_DRIVE_LIST_URL}/{}", file.id))
        .bearer_auth(access_token)
        .query(&[("alt", "media")])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(format!(
            "Google Drive download returned {}",
            response.status()
        ));
    }
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = response.bytes().await.map_err(|e| e.to_string())?.to_vec();
    Ok(Some((
        bytes,
        RemoteFile {
            etag,
            id: file.id.clone(),
        },
    )))
}

fn multipart_body(
    name: &str,
    content_type: &str,
    bytes: &[u8],
) -> Result<(String, Vec<u8>), String> {
    let boundary = format!("focusnook-{}", uuid::Uuid::new_v4());
    let metadata = serde_json::json!({
        "name": name,
        "parents": ["appDataFolder"],
    });
    let metadata = serde_json::to_vec(&metadata).map_err(|e| e.to_string())?;
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
    body.extend_from_slice(&metadata);
    body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Ok((format!("multipart/related; boundary={boundary}"), body))
}

async fn create_google_file(
    access_token: &str,
    name: &str,
    content_type: &str,
    bytes: &[u8],
) -> Result<RemoteFile, String> {
    let (multipart_type, body) = multipart_body(name, content_type, bytes)?;
    let response = reqwest::Client::new()
        .post(GOOGLE_DRIVE_UPLOAD_URL)
        .bearer_auth(access_token)
        .query(&[("uploadType", "multipart"), ("fields", "id")])
        .header(CONTENT_TYPE, multipart_type)
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Google Drive create returned {}",
            response.status()
        ));
    }
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = response
        .json::<DriveListedFile>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(RemoteFile { etag, id: body.id })
}

async fn update_google_file(
    access_token: &str,
    file: &RemoteFile,
    content_type: &str,
    bytes: &[u8],
) -> Result<bool, String> {
    let mut request = reqwest::Client::new()
        .patch(format!("{GOOGLE_DRIVE_UPLOAD_URL}/{}", file.id))
        .bearer_auth(access_token)
        .query(&[("uploadType", "media")])
        .header(CONTENT_TYPE, content_type)
        .body(bytes.to_vec());
    if let Some(etag) = &file.etag {
        request = request.header(IF_MATCH, etag);
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::PRECONDITION_FAILED {
        return Ok(false);
    }
    if !response.status().is_success() {
        return Err(format!(
            "Google Drive update returned {}",
            response.status()
        ));
    }
    Ok(true)
}

// P0 аудит: раньше ошибка парсинга (битый/урезанный/конфликтующий файл) была
// неотличима от "файла ещё нет" — оба случая тихо подставляли пустой журнал
// с НОВЫМ media_key. Следующее сохранение перезаписывало реальный журнал на
// Google Drive этой пустышкой, стирая историю операций со всех устройств и
// делая старые аудио-blob'ы недекодируемыми (другой media_key). Отсутствие
// файла — не ошибка (обрабатывается двумя ветками выше, до вызова этой
// функции), а вот "файл есть, но не парсится" обязано быть жёсткой Err, а не
// тихой заменой на пустоту.
fn parse_journal(bytes: &[u8]) -> Result<CloudJournal, String> {
    serde_json::from_slice::<CloudJournal>(bytes).map_err(|e| {
        format!("Google Drive sync journal is corrupted, refusing to overwrite it: {e}")
    })
}

async fn load_journal(access_token: &str) -> Result<(CloudJournal, Option<RemoteFile>), String> {
    let Some(file) = find_google_file(access_token, JOURNAL_NAME).await? else {
        return Ok((CloudJournal::empty(), None));
    };
    let Some((bytes, file)) = download_google_file(access_token, &file).await? else {
        return Ok((CloudJournal::empty(), None));
    };
    let journal = parse_journal(&bytes)?;
    Ok((journal, Some(file)))
}

async fn save_journal(
    access_token: &str,
    journal: &CloudJournal,
    file: Option<&RemoteFile>,
) -> Result<bool, String> {
    let bytes = serde_json::to_vec(journal).map_err(|e| e.to_string())?;
    match file {
        Some(file) => update_google_file(access_token, file, "application/json", &bytes).await,
        None => {
            create_google_file(access_token, JOURNAL_NAME, "application/json", &bytes).await?;
            Ok(true)
        }
    }
}

fn merge_operations(
    remote: &[server_sync::RemoteOperation],
    local: &[server_sync::LocalOperation],
    media_key: &str,
) -> Vec<server_sync::RemoteOperation> {
    let mut merged = remote.to_vec();
    for operation in local {
        if !merged
            .iter()
            .any(|existing| existing.operation_id == operation.operation_id)
        {
            merged.push(server_sync::remote_operation_from_local(
                operation,
                Some(media_key),
            ));
        }
    }
    merged.sort_by(|a, b| {
        a.hlc
            .cmp(&b.hlc)
            .then_with(|| a.operation_id.cmp(&b.operation_id))
    });
    merged
}

fn latest_hlc(operations: &[server_sync::RemoteOperation]) -> Option<String> {
    operations
        .iter()
        .map(|operation| operation.hlc.as_str())
        .max()
        .map(str::to_string)
}

fn apply_remote_operations(
    app: &tauri::AppHandle,
    conn: &Connection,
    hlc_state: &sync_log::HlcClockState,
    profile_id: &str,
    local_device_id: &str,
    operations: &[server_sync::RemoteOperation],
    media_key: &str,
) -> Result<Vec<String>, String> {
    let mut missing_blobs = Vec::new();
    for operation in operations {
        server_sync::apply_remote_operation(
            conn,
            profile_id,
            local_device_id,
            operation,
            Some(media_key),
        )?;
        if let Some(parsed_hlc) = sync_log::Hlc::parse(&operation.hlc) {
            hlc_state
                .0
                .lock()
                .map_err(|e| e.to_string())?
                .observe(conn, &parsed_hlc)
                .map_err(|e| e.to_string())?;
        }
        if operation.device_id != local_device_id {
            if let Some(blob_id) =
                server_sync::audio_blob_id_from_operation(operation, Some(media_key))
            {
                missing_blobs.push(blob_id);
            }
        }
        server_sync::reconcile_remote_reminder_alarm(app, conn, local_device_id, operation)?;
    }
    missing_blobs.sort();
    missing_blobs.dedup();
    Ok(missing_blobs)
}

async fn upload_blob_to_google(
    access_token: &str,
    request: &sync_blobs::UploadBlobRequest,
) -> Result<i64, String> {
    let size_bytes = STANDARD
        .decode(request.bytes_base64.trim())
        .map(|bytes| bytes.len() as i64)
        .map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&sync_blobs::DownloadBlobResponse {
        bytes_base64: request.bytes_base64.clone(),
        content_type: request.content_type.clone(),
        sha256: request.sha256.clone(),
        size_bytes,
    })
    .map_err(|e| e.to_string())?;
    let name = blob_file_name(&request.blob_id);
    if let Some(file) = find_google_file(access_token, &name).await? {
        if update_google_file(access_token, &file, "application/json", &bytes).await? {
            return Ok(size_bytes);
        }
    }
    create_google_file(access_token, &name, "application/json", &bytes).await?;
    Ok(size_bytes)
}

async fn download_blob_from_google(
    access_token: &str,
    blob_id: &str,
) -> Result<Option<sync_blobs::DownloadBlobResponse>, String> {
    let Some(file) = find_google_file(access_token, &blob_file_name(blob_id)).await? else {
        return Ok(None);
    };
    let Some((bytes, _file)) = download_google_file(access_token, &file).await? else {
        return Ok(None);
    };
    serde_json::from_slice::<sync_blobs::DownloadBlobResponse>(&bytes)
        .map(Some)
        .map_err(|e| e.to_string())
}

async fn upload_pending_blobs(
    db: &crate::db::Db,
    access_token: &str,
    local_profile_id: &str,
    remote_profile_id: &str,
    audio_dir: &std::path::Path,
    audio_key: Option<&str>,
    media_key: &str,
) -> Result<(), String> {
    let prepared_uploads = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let mut prepared = Vec::new();
        for record in sync_blobs::pending_uploads(&conn, local_profile_id)? {
            if !audio_dir.join(&record.local_path).exists() {
                sync_blobs::mark_missing_upload_deferred(&conn, local_profile_id, &record.blob_id)?;
                continue;
            }
            match sync_blobs::upload_request(
                &conn,
                local_profile_id,
                remote_profile_id,
                audio_dir,
                audio_key,
                media_key,
                &record,
            ) {
                Ok(request) => prepared.push(request),
                Err(err) => eprintln!(
                    "cloud-sync: deferring blob upload {} because it cannot be prepared: {err}",
                    record.blob_id
                ),
            }
        }
        prepared
    };
    for request in prepared_uploads {
        let blob_id = request.blob_id.clone();
        let sha256 = request.sha256.clone();
        let size_bytes = upload_blob_to_google(access_token, &request).await?;
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        sync_blobs::mark_uploaded(&conn, local_profile_id, &blob_id, &sha256, size_bytes)?;
    }
    Ok(())
}

async fn download_missing_blobs(
    db: &crate::db::Db,
    access_token: &str,
    profile_id: &str,
    audio_dir: &std::path::Path,
    audio_key: Option<&str>,
    media_key: &str,
    blob_ids: Vec<String>,
) -> Result<(), String> {
    for blob_id in blob_ids {
        let Some(downloaded) = download_blob_from_google(access_token, &blob_id).await? else {
            continue;
        };
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        if let Err(err) = sync_blobs::materialize_download(
            &conn,
            profile_id,
            audio_dir,
            audio_key,
            media_key,
            &blob_id,
            &downloaded,
        ) {
            eprintln!("cloud-sync: blob materialize {blob_id} failed: {err}");
        }
    }
    Ok(())
}

async fn perform_google_sync(app: tauri::AppHandle) -> Result<CloudSyncStatus, String> {
    let db = app.state::<crate::db::Db>();
    let config = app.state::<config::SyncProvidersConfig>();
    let profiles_state = app.state::<profiles::ProfilesState>();
    let hlc_state = app.state::<sync_log::HlcClockState>();
    let audio_key_state = app.state::<crate::AudioKeyState>();
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let remote_profile_id = provider_scope_key(ProviderId::GoogleDrive, &profile_id);
    let audio_dir = profiles::data_dir(&profiles_state).join("audio");
    let audio_key = audio_key_state.0.lock().map_err(|e| e.to_string())?.clone();
    let access_token = google_access_token(&app, &config, &profile_id).await?;
    let local_device_id = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        server_sync::ensure_local_device_id(&conn)?
    };

    for _ in 0..MAX_CLOUD_SYNC_ROUNDS {
        let (mut journal, file) = load_journal(&access_token).await?;
        let missing_blobs = {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            apply_remote_operations(
                &app,
                &conn,
                &hlc_state,
                &profile_id,
                &local_device_id,
                &journal.operations,
                &journal.media_key,
            )?
        };
        download_missing_blobs(
            &db,
            &access_token,
            &profile_id,
            &audio_dir,
            audio_key.as_deref(),
            &journal.media_key,
            missing_blobs,
        )
        .await?;

        upload_pending_blobs(
            &db,
            &access_token,
            &profile_id,
            &remote_profile_id,
            &audio_dir,
            audio_key.as_deref(),
            &journal.media_key,
        )
        .await?;

        let (local_ops, sent_ids) = {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            let local_ops = server_sync::unsynced_operations(&conn, &profile_id)?;
            let sent_ids = local_ops
                .iter()
                .map(|operation| operation.operation_id.clone())
                .collect::<Vec<_>>();
            (local_ops, sent_ids)
        };
        let merged = merge_operations(&journal.operations, &local_ops, &journal.media_key);
        if merged == journal.operations {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            server_sync::store_last_pulled_hlc(
                &conn,
                &remote_profile_id,
                latest_hlc(&journal.operations).as_deref(),
            )?;
            return Ok(CloudSyncStatus {
                connected: true,
                last_operation_hlc: latest_hlc(&journal.operations),
                message: None,
                provider: ProviderId::GoogleDrive,
            });
        }
        journal.operations = merged;
        if !save_journal(&access_token, &journal, file.as_ref()).await? {
            continue;
        }
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        server_sync::mark_synced(&conn, &sent_ids)?;
        server_sync::store_last_pulled_hlc(
            &conn,
            &remote_profile_id,
            latest_hlc(&journal.operations).as_deref(),
        )?;
        return Ok(CloudSyncStatus {
            connected: true,
            last_operation_hlc: latest_hlc(&journal.operations),
            message: None,
            provider: ProviderId::GoogleDrive,
        });
    }

    Err("Google Drive sync changed concurrently, please run sync again".to_string())
}

pub fn spawn_best_effort(app: tauri::AppHandle) {
    if CLOUD_SYNC_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        CLOUD_SYNC_RERUN_REQUESTED.store(true, Ordering::SeqCst);
        return;
    }
    tauri::async_runtime::spawn(async move {
        let mut result;
        loop {
            CLOUD_SYNC_RERUN_REQUESTED.store(false, Ordering::SeqCst);
            result = perform_google_sync(app.clone()).await;
            if !CLOUD_SYNC_RERUN_REQUESTED.swap(false, Ordering::SeqCst) {
                break;
            }
        }
        CLOUD_SYNC_IN_FLIGHT.store(false, Ordering::SeqCst);
        if CLOUD_SYNC_RERUN_REQUESTED.swap(false, Ordering::SeqCst) {
            spawn_best_effort(app);
            return;
        }
        match result {
            Ok(_) => {
                let _ = app.emit("cloud-sync-completed", ());
            }
            Err(err) => {
                eprintln!("cloud-sync: best-effort sync failed: {err}");
                let _ = app.emit("cloud-sync-failed", err);
            }
        }
    });
}

pub fn spawn_periodic_best_effort(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(PERIODIC_CLOUD_SYNC_INTERVAL).await;
            spawn_best_effort(app.clone());
        }
    });
}

#[tauri::command]
pub async fn sync_cloud_now(
    app: tauri::AppHandle,
    provider: ProviderId,
) -> Result<CloudSyncStatus, String> {
    match provider {
        ProviderId::GoogleDrive => perform_google_sync(app).await,
        ProviderId::YandexDisk => {
            Err("Yandex Disk sync adapter is not implemented yet".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn remote(id: &str, hlc: &str) -> server_sync::RemoteOperation {
        server_sync::RemoteOperation {
            device_id: "remote-device".to_string(),
            entity_id: id.to_string(),
            entity_type: "note".to_string(),
            hlc: hlc.to_string(),
            op: "create".to_string(),
            operation_id: format!("op-{id}-{hlc}"),
            payload_ciphertext: "{}".to_string(),
            schema_version: 1,
        }
    }

    #[test]
    fn merge_keeps_remote_and_adds_missing_local_once() {
        let local = server_sync::LocalOperation {
            device_id: "local-device".to_string(),
            entity_id: "note-2".to_string(),
            entity_type: "note".to_string(),
            hlc: "2026-07-08T10:00:01.000Z-0000-local".to_string(),
            op: "create".to_string(),
            operation_id: "local-op".to_string(),
            patch: "{}".to_string(),
            schema_version: 1,
        };
        let merged = merge_operations(
            &[remote("note-1", "2026-07-08T10:00:00.000Z-0000-a")],
            &[local],
            "test-media-key",
        );

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[1].operation_id, "local-op");
    }

    #[test]
    fn drive_names_escape_quotes() {
        assert_eq!(escaped_drive_name("a'b"), "a\\'b");
    }

    #[test]
    fn parse_journal_round_trips_valid_bytes() -> Result<(), String> {
        let original = CloudJournal {
            media_key: "test-media-key".to_string(),
            operations: vec![remote("note-1", "2026-07-08T10:00:00.000Z-0000-a")],
            version: 1,
        };
        let bytes = serde_json::to_vec(&original).map_err(|e| e.to_string())?;

        let parsed = parse_journal(&bytes)?;

        assert_eq!(parsed.media_key, original.media_key);
        assert_eq!(parsed.operations.len(), 1);
        Ok(())
    }

    #[test]
    fn parse_journal_rejects_corrupted_bytes_instead_of_returning_empty() {
        let result = parse_journal(b"not valid json at all");
        assert!(result.is_err());
    }
}
