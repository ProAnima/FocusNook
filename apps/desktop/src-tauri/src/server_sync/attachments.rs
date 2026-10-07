use super::credentials::{load_credentials, server_profile_id, ServerSyncCredentials};
use super::protocol::UploadBlobResponse;
use super::transport::http_client;
use super::BLOB_TRANSFER_TIMEOUT;
use crate::sync_blobs;

async fn upload_prepared_blob(
    credentials: &ServerSyncCredentials,
    request: sync_blobs::UploadBlobRequest,
) -> Result<(String, String, i64), String> {
    let client = http_client(BLOB_TRANSFER_TIMEOUT)?;
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
    Ok((body.blob_id, sha256, body.size_bytes))
}

async fn download_blob(
    credentials: &ServerSyncCredentials,
    profile_id: &str,
    blob_id: &str,
) -> Result<Option<sync_blobs::DownloadBlobResponse>, String> {
    let response = http_client(BLOB_TRANSFER_TIMEOUT)?
        .get(format!(
            "{}/v1/blobs/{}/{}",
            credentials.endpoint, profile_id, blob_id
        ))
        .bearer_auth(&credentials.token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
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

pub async fn ensure_audio_blob_downloaded(
    db: &crate::db::Db,
    profile_id: &str,
    audio_dir: &std::path::Path,
    audio_key: Option<&str>,
    blob_id: &str,
) -> Result<(), String> {
    if sync_blobs::audio_file_path(audio_dir, blob_id)?.exists() {
        return Ok(());
    }
    let credentials = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        load_credentials(Some(&conn), profile_id)?
    };
    let Some(credentials) = credentials else {
        return Ok(());
    };
    let remote_profile_id = server_profile_id(&credentials, profile_id);
    let media_key = credentials.media_key.as_deref().ok_or_else(|| {
        "server sync media key is missing; sign in again to enable encrypted attachments"
            .to_string()
    })?;
    let Some(downloaded) = download_blob(&credentials, &remote_profile_id, blob_id).await? else {
        return Err(format!(
            "audio blob {blob_id} is missing on the sync server"
        ));
    };
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    sync_blobs::materialize_download(
        &conn,
        profile_id,
        audio_dir,
        audio_key,
        media_key,
        blob_id,
        &downloaded,
    )
}

pub(super) async fn upload_pending_blobs(
    db: &crate::db::Db,
    credentials: &ServerSyncCredentials,
    local_profile_id: &str,
    remote_profile_id: &str,
    audio_dir: &std::path::Path,
    audio_key: Option<&str>,
) -> Result<usize, String> {
    let pending = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        sync_blobs::pending_uploads_for_remote(&conn, local_profile_id, remote_profile_id)?
    };
    let mut blocked = 0;
    let mut uploaded = 0;
    for record in pending {
        let path = sync_blobs::audio_file_path(audio_dir, &record.local_path)?;
        let Some(media_key) = credentials.media_key.as_deref() else {
            log::debug!("attachment {} is waiting for a media key", record.blob_id);
            blocked += 1;
            continue;
        };
        if !path.exists() {
            log::debug!(
                "attachment {} is waiting for its local file",
                record.blob_id
            );
            blocked += 1;
            continue;
        }
        let prepared = {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            sync_blobs::upload_request(
                &conn,
                local_profile_id,
                remote_profile_id,
                audio_dir,
                audio_key,
                media_key,
                &record,
            )
        };
        let request = match prepared {
            Ok(request) => request,
            Err(err) => {
                log::warn!("attachment {} could not be prepared: {err}", record.blob_id);
                blocked += 1;
                continue;
            }
        };
        let (blob_id, sha256, size_bytes) = match upload_prepared_blob(credentials, request).await {
            Ok(uploaded) => uploaded,
            Err(err) => {
                log::warn!("attachment {} upload is pending: {err}", record.blob_id);
                blocked += 1;
                continue;
            }
        };
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        sync_blobs::mark_uploaded(&conn, local_profile_id, &blob_id, &sha256, size_bytes)?;
        sync_blobs::mark_uploaded_to(&conn, local_profile_id, &blob_id, remote_profile_id)?;
        uploaded += 1;
    }
    if blocked > 0 || uploaded > 0 {
        log::info!("blob upload: загружено {uploaded}, отложено {blocked}");
    }
    Ok(blocked)
}

pub(super) async fn download_pending_blobs(
    db: &crate::db::Db,
    credentials: &ServerSyncCredentials,
    local_profile_id: &str,
    remote_profile_id: &str,
    audio_dir: &std::path::Path,
    audio_key: Option<&str>,
) -> Result<usize, String> {
    let pending = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        sync_blobs::pending_downloads(&conn, local_profile_id)?
    };
    let mut unavailable = 0;
    let mut downloaded_count = 0;
    for blob_id in pending {
        if sync_blobs::audio_file_path(audio_dir, &blob_id)?.exists() {
            continue;
        }
        let Some(media_key) = credentials.media_key.as_deref() else {
            log::debug!("attachment {blob_id} is waiting for a media key");
            unavailable += 1;
            continue;
        };
        let downloaded = match download_blob(credentials, remote_profile_id, &blob_id).await {
            Ok(Some(downloaded)) => downloaded,
            Ok(None) => {
                log::debug!("attachment {blob_id} is not available on the server");
                unavailable += 1;
                continue;
            }
            Err(err) => {
                log::warn!("attachment {blob_id} download is pending: {err}");
                unavailable += 1;
                continue;
            }
        };
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        if let Err(err) = sync_blobs::materialize_download(
            &conn,
            local_profile_id,
            audio_dir,
            audio_key,
            media_key,
            &blob_id,
            &downloaded,
        ) {
            log::warn!("attachment {blob_id} could not be materialized: {err}");
            unavailable += 1;
        } else {
            downloaded_count += 1;
        }
    }
    if unavailable > 0 || downloaded_count > 0 {
        log::info!("blob download: скачано {downloaded_count}, недоступно {unavailable}");
    }
    Ok(unavailable)
}

pub(super) fn transfer_status_message(
    blocked_uploads: usize,
    unavailable_downloads: usize,
) -> Option<String> {
    let mut notices = Vec::new();
    if blocked_uploads > 0 {
        notices.push(format!("{blocked_uploads} attachment upload(s) pending"));
    }
    if unavailable_downloads > 0 {
        notices.push(format!(
            "{unavailable_downloads} attachment download(s) unavailable"
        ));
    }
    (!notices.is_empty()).then(|| format!("sync completed; {}", notices.join("; ")))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn transfer_warning_reports_degraded_media_without_failing_sync() {
        assert_eq!(transfer_status_message(0, 0), None);
        assert_eq!(
            transfer_status_message(1, 2),
            Some(
                "sync completed; 1 attachment upload(s) pending; 2 attachment download(s) unavailable"
                    .to_string()
            )
        );
    }
}
