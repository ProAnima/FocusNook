use crate::{audio_crypto, blob_crypto};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const AUDIO_WEBM: &str = "audio/webm";

#[derive(Clone)]
pub struct BlobRecord {
    pub blob_id: String,
    pub local_path: String,
    pub content_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadBlobRequest {
    pub blob_id: String,
    pub bytes_base64: String,
    pub content_type: String,
    pub profile_id: String,
    pub sha256: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadBlobResponse {
    pub bytes_base64: String,
    pub content_type: String,
    pub sha256: String,
    pub size_bytes: i64,
}

pub fn ensure_audio_blob(
    conn: &Connection,
    profile_id: &str,
    filename: &str,
) -> Result<(), String> {
    upsert_local_blob(conn, profile_id, filename, filename, AUDIO_WEBM)
}

pub fn ensure_downloadable_audio_blob(
    conn: &Connection,
    profile_id: &str,
    filename: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO sync_blobs
            (profile_id, blob_id, local_path, content_type, uploaded_at, created_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))
         ON CONFLICT(profile_id, blob_id) DO UPDATE SET
           local_path = excluded.local_path,
           content_type = excluded.content_type,
           uploaded_at = COALESCE(sync_blobs.uploaded_at, excluded.uploaded_at)",
        params![profile_id, filename, filename, AUDIO_WEBM],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn upsert_local_blob(
    conn: &Connection,
    profile_id: &str,
    blob_id: &str,
    local_path: &str,
    content_type: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO sync_blobs
            (profile_id, blob_id, local_path, content_type, created_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(profile_id, blob_id) DO UPDATE SET
           local_path = excluded.local_path,
           content_type = excluded.content_type",
        params![profile_id, blob_id, local_path, content_type],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn blob_for_audio(
    conn: &Connection,
    profile_id: &str,
    filename: &str,
) -> Result<Option<BlobRecord>, String> {
    let record = conn
        .query_row(
            "SELECT blob_id, local_path, content_type
             FROM sync_blobs
             WHERE profile_id = ?1 AND blob_id = ?2",
            params![profile_id, filename],
            |row| {
                Ok(BlobRecord {
                    blob_id: row.get(0)?,
                    local_path: row.get(1)?,
                    content_type: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(record)
}

pub fn pending_uploads(conn: &Connection, profile_id: &str) -> Result<Vec<BlobRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT blob_id, local_path, content_type
             FROM sync_blobs
             WHERE profile_id = ?1 AND uploaded_at IS NULL AND deleted_at IS NULL
             ORDER BY created_at ASC, blob_id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok(BlobRecord {
            blob_id: row.get(0)?,
            local_path: row.get(1)?,
            content_type: row.get(2)?,
        })
    });
    let collected = rows
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(collected)
}

pub fn upload_request(
    conn: &Connection,
    local_profile_id: &str,
    remote_profile_id: &str,
    audio_dir: &Path,
    audio_key: Option<&str>,
    media_key: &str,
    record: &BlobRecord,
) -> Result<UploadBlobRequest, String> {
    let bytes = sync_ciphertext_for_upload(
        conn,
        local_profile_id,
        audio_dir,
        audio_key,
        media_key,
        record,
    )?;
    let sha256 = blob_crypto::sha256_hex(&bytes);
    Ok(UploadBlobRequest {
        blob_id: record.blob_id.clone(),
        bytes_base64: STANDARD.encode(bytes),
        content_type: record.content_type.clone(),
        profile_id: remote_profile_id.to_string(),
        sha256,
    })
}

fn sync_ciphertext_for_upload(
    conn: &Connection,
    profile_id: &str,
    audio_dir: &Path,
    audio_key: Option<&str>,
    media_key: &str,
    record: &BlobRecord,
) -> Result<Vec<u8>, String> {
    if let Some(existing) = read_cached_upload(conn, profile_id, &record.blob_id, record)? {
        return Ok(existing);
    }
    let raw = fs::read(audio_dir.join(&record.local_path)).map_err(|e| e.to_string())?;
    let plaintext = match audio_key {
        Some(key) => audio_crypto::decrypt_if_needed(key, &raw)?,
        None => raw,
    };
    let encrypted = blob_crypto::encrypt(media_key, &plaintext)?;
    cache_upload(conn, profile_id, &record.blob_id, &encrypted)?;
    Ok(encrypted)
}

pub fn mark_uploaded(
    conn: &Connection,
    profile_id: &str,
    blob_id: &str,
    sha256: &str,
    size_bytes: i64,
) -> Result<(), String> {
    conn.execute(
        "UPDATE sync_blobs
         SET sha256 = ?1, size_bytes = ?2, uploaded_at = datetime('now')
         WHERE profile_id = ?3 AND blob_id = ?4",
        params![sha256, size_bytes, profile_id, blob_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_missing_upload_deferred(
    conn: &Connection,
    profile_id: &str,
    blob_id: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE sync_blobs
         SET uploaded_at = COALESCE(uploaded_at, datetime('now'))
         WHERE profile_id = ?1 AND blob_id = ?2",
        params![profile_id, blob_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn materialize_download(
    conn: &Connection,
    profile_id: &str,
    audio_dir: &Path,
    audio_key: Option<&str>,
    media_key: &str,
    blob_id: &str,
    response: &DownloadBlobResponse,
) -> Result<(), String> {
    if blob_for_audio(conn, profile_id, blob_id)?.is_some() && audio_dir.join(blob_id).exists() {
        return Ok(());
    }
    let encrypted = STANDARD
        .decode(response.bytes_base64.trim())
        .map_err(|e| e.to_string())?;
    if blob_crypto::sha256_hex(&encrypted) != response.sha256 {
        return Err("downloaded sync blob checksum mismatch".to_string());
    }
    let plaintext = blob_crypto::decrypt(media_key, &encrypted)?;
    let local_bytes = match audio_key {
        Some(key) => audio_crypto::encrypt(key, &plaintext)?,
        None => plaintext,
    };
    fs::create_dir_all(audio_dir).map_err(|e| e.to_string())?;
    fs::write(audio_dir.join(blob_id), local_bytes).map_err(|e| e.to_string())?;
    upsert_local_blob(conn, profile_id, blob_id, blob_id, &response.content_type)?;
    conn.execute(
        "UPDATE sync_blobs
         SET sha256 = ?1, size_bytes = ?2, downloaded_at = datetime('now'), uploaded_at = COALESCE(uploaded_at, datetime('now'))
         WHERE profile_id = ?3 AND blob_id = ?4",
        params![response.sha256, response.size_bytes, profile_id, blob_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_deleted(conn: &Connection, profile_id: &str, blob_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE sync_blobs SET deleted_at = datetime('now') WHERE profile_id = ?1 AND blob_id = ?2",
        params![profile_id, blob_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn read_cached_upload(
    conn: &Connection,
    profile_id: &str,
    blob_id: &str,
    record: &BlobRecord,
) -> Result<Option<Vec<u8>>, String> {
    let encoded: Option<String> = conn
        .query_row(
            "SELECT sync_payload_base64 FROM sync_blobs
             WHERE profile_id = ?1 AND blob_id = ?2 AND local_path = ?3 AND sync_payload_base64 IS NOT NULL",
            params![profile_id, blob_id, record.local_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    encoded
        .map(|value| STANDARD.decode(value).map_err(|e| e.to_string()))
        .transpose()
}

fn cache_upload(
    conn: &Connection,
    profile_id: &str,
    blob_id: &str,
    bytes: &[u8],
) -> Result<(), String> {
    conn.execute(
        "UPDATE sync_blobs SET sync_payload_base64 = ?1 WHERE profile_id = ?2 AND blob_id = ?3",
        params![STANDARD.encode(bytes), profile_id, blob_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
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
        conn
    }

    #[test]
    fn upload_payload_is_client_encrypted_and_stable_after_cache() {
        let conn = conn();
        let dir =
            std::env::temp_dir().join(format!("focusnook-blob-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("voice.webm"), b"voice bytes").unwrap();
        upsert_local_blob(&conn, "profile", "voice.webm", "voice.webm", AUDIO_WEBM).unwrap();
        let record = blob_for_audio(&conn, "profile", "voice.webm")
            .unwrap()
            .unwrap();
        let media_key = blob_crypto::derive_media_key("a@example.com", "password");

        let first = upload_request(
            &conn,
            "profile",
            "remote-profile",
            &dir,
            None,
            &media_key,
            &record,
        )
        .unwrap();
        let second = upload_request(
            &conn,
            "profile",
            "remote-profile",
            &dir,
            None,
            &media_key,
            &record,
        )
        .unwrap();

        assert_eq!(first.bytes_base64, second.bytes_base64);
        assert_eq!(first.profile_id, "remote-profile");
        assert!(!first.bytes_base64.contains("voice"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn downloadable_remote_audio_is_not_treated_as_a_pending_upload() {
        let conn = conn();

        ensure_downloadable_audio_blob(&conn, "profile", "remote-voice.webm").unwrap();

        assert!(pending_uploads(&conn, "profile").unwrap().is_empty());
        let uploaded_at: Option<String> = conn
            .query_row(
                "SELECT uploaded_at FROM sync_blobs WHERE profile_id = 'profile' AND blob_id = 'remote-voice.webm'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let downloaded_at: Option<String> = conn
            .query_row(
                "SELECT downloaded_at FROM sync_blobs WHERE profile_id = 'profile' AND blob_id = 'remote-voice.webm'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(uploaded_at.is_some());
        assert!(downloaded_at.is_none());
    }

    #[test]
    fn missing_local_upload_can_be_deferred_without_marking_downloaded() {
        let conn = conn();
        ensure_audio_blob(&conn, "profile", "missing-voice.webm").unwrap();
        assert_eq!(pending_uploads(&conn, "profile").unwrap().len(), 1);

        mark_missing_upload_deferred(&conn, "profile", "missing-voice.webm").unwrap();

        assert!(pending_uploads(&conn, "profile").unwrap().is_empty());
        let downloaded_at: Option<String> = conn
            .query_row(
                "SELECT downloaded_at FROM sync_blobs WHERE profile_id = 'profile' AND blob_id = 'missing-voice.webm'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(downloaded_at.is_none());
    }
}
