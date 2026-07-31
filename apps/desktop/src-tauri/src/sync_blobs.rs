use crate::{audio_crypto, blob_crypto};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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

#[derive(Deserialize, Serialize)]
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
    validate_audio_filename(filename)?;
    upsert_local_blob(conn, profile_id, filename, filename, AUDIO_WEBM)
}

pub fn ensure_downloadable_audio_blob(
    conn: &Connection,
    profile_id: &str,
    filename: &str,
) -> Result<(), String> {
    ensure_downloadable_blob(conn, profile_id, filename, AUDIO_WEBM)
}

pub fn ensure_downloadable_blob(
    conn: &Connection,
    profile_id: &str,
    blob_id: &str,
    content_type: &str,
) -> Result<(), String> {
    validate_audio_filename(blob_id)?;
    conn.execute(
        "INSERT INTO sync_blobs
            (profile_id, blob_id, local_path, content_type, uploaded_at, created_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))
         ON CONFLICT(profile_id, blob_id) DO UPDATE SET
           local_path = excluded.local_path,
           content_type = excluded.content_type,
           uploaded_at = COALESCE(sync_blobs.uploaded_at, excluded.uploaded_at)",
        params![profile_id, blob_id, blob_id, content_type],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn audio_file_path(audio_dir: &Path, filename: &str) -> Result<PathBuf, String> {
    validate_audio_filename(filename)?;
    Ok(audio_dir.join(filename))
}

fn validate_audio_filename(filename: &str) -> Result<(), String> {
    if filename.is_empty()
        || filename == "."
        || filename == ".."
        || filename.contains('/')
        || filename.contains('\\')
    {
        return Err("sync audio blob id is not a safe filename".to_string());
    }
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

pub fn pending_downloads(conn: &Connection, profile_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT blob_id
             FROM sync_blobs
             WHERE profile_id = ?1
               AND uploaded_at IS NOT NULL
               AND downloaded_at IS NULL
               AND deleted_at IS NULL
             ORDER BY created_at ASC, blob_id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![profile_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

pub fn is_uploaded(conn: &Connection, profile_id: &str, blob_id: &str) -> Result<bool, String> {
    let uploaded_at = conn
        .query_row(
            "SELECT uploaded_at FROM sync_blobs WHERE profile_id = ?1 AND blob_id = ?2",
            params![profile_id, blob_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    Ok(uploaded_at.is_some())
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
    let path = audio_file_path(audio_dir, &record.local_path)?;
    let plaintext = match audio_key {
        Some(key) => audio_crypto::read_and_migrate(&path, key)?,
        None => fs::read(path).map_err(|e| e.to_string())?,
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

pub fn materialize_download(
    conn: &Connection,
    profile_id: &str,
    audio_dir: &Path,
    audio_key: Option<&str>,
    media_key: &str,
    blob_id: &str,
    response: &DownloadBlobResponse,
) -> Result<(), String> {
    let encrypted = STANDARD
        .decode(response.bytes_base64.trim())
        .map_err(|e| e.to_string())?;
    if response.size_bytes < 0 || encrypted.len() as i64 != response.size_bytes {
        return Err("downloaded sync blob size mismatch".to_string());
    }
    if blob_crypto::sha256_hex(&encrypted) != response.sha256 {
        return Err("downloaded sync blob checksum mismatch".to_string());
    }
    let plaintext = blob_crypto::decrypt(media_key, &encrypted)?;
    let path = audio_file_path(audio_dir, blob_id)?;
    if path.exists() {
        if let Ok(existing) = local_plaintext(&path, audio_key) {
            if existing == plaintext {
                return store_download_metadata(conn, profile_id, blob_id, response);
            }
        }
    }
    let locally_encrypted = audio_key
        .map(|key| audio_crypto::encrypt(key, &plaintext))
        .transpose()?;
    let local_bytes = locally_encrypted.as_deref().unwrap_or(&plaintext);
    fs::create_dir_all(audio_dir).map_err(|e| e.to_string())?;
    write_download_atomically(&path, local_bytes, audio_key, &plaintext)?;
    store_download_metadata(conn, profile_id, blob_id, response)
}

fn local_plaintext(path: &Path, audio_key: Option<&str>) -> Result<Vec<u8>, String> {
    match audio_key {
        Some(key) => audio_crypto::read_and_migrate(path, key),
        None => fs::read(path).map_err(|e| e.to_string()),
    }
}

fn write_download_atomically(
    path: &Path,
    local_bytes: &[u8],
    audio_key: Option<&str>,
    expected_plaintext: &[u8],
) -> Result<(), String> {
    let pending = std::path::PathBuf::from(format!("{}.focusnook-download.tmp", path.display()));
    let mut file = fs::File::create(&pending).map_err(|e| e.to_string())?;
    file.write_all(local_bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);

    if local_plaintext(&pending, audio_key)? != expected_plaintext {
        return Err("downloaded audio verification failed".to_string());
    }
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    fs::rename(&pending, path).map_err(|e| e.to_string())
}

fn store_download_metadata(
    conn: &Connection,
    profile_id: &str,
    blob_id: &str,
    response: &DownloadBlobResponse,
) -> Result<(), String> {
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
        assert_eq!(
            pending_downloads(&conn, "profile").unwrap(),
            vec!["remote-voice.webm"]
        );
    }

    #[test]
    fn missing_local_upload_remains_pending_for_retry() {
        let conn = conn();
        ensure_audio_blob(&conn, "profile", "missing-voice.webm").unwrap();

        assert_eq!(pending_uploads(&conn, "profile").unwrap().len(), 1);
        assert!(!is_uploaded(&conn, "profile", "missing-voice.webm").unwrap());
    }

    #[test]
    fn image_attachment_uses_the_same_encrypted_transfer_contract() {
        let conn = conn();
        let dir =
            std::env::temp_dir().join(format!("focusnook-image-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("picture.png"), b"png bytes").unwrap();
        upsert_local_blob(&conn, "profile", "picture.png", "picture.png", "image/png").unwrap();
        let record = blob_for_audio(&conn, "profile", "picture.png")
            .unwrap()
            .unwrap();
        let media_key = blob_crypto::derive_media_key("a@example.com", "password");

        let request =
            upload_request(&conn, "profile", "remote", &dir, None, &media_key, &record).unwrap();

        assert_eq!(request.content_type, "image/png");
        assert!(!request.bytes_base64.contains("png bytes"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unsafe_audio_blob_ids_are_rejected() {
        let conn = conn();

        assert!(ensure_audio_blob(&conn, "profile", "../outside.webm").is_err());
        assert!(audio_file_path(Path::new("audio"), r"..\outside.webm").is_err());
    }

    #[test]
    fn audio_round_trips_between_devices_with_distinct_local_keys() {
        let sender = conn();
        let receiver = conn();
        let sender_dir =
            std::env::temp_dir().join(format!("focusnook-sender-{}", uuid::Uuid::new_v4()));
        let receiver_dir =
            std::env::temp_dir().join(format!("focusnook-receiver-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&sender_dir).unwrap();
        fs::create_dir_all(&receiver_dir).unwrap();
        let blob_id = "cross-device.webm";
        let plaintext = b"audio recorded on device A";
        let sender_key = "device-a-vault-key";
        let receiver_key = "device-b-vault-key";
        let media_key = blob_crypto::derive_media_key("a@example.com", "password");
        fs::write(
            sender_dir.join(blob_id),
            audio_crypto::encrypt(sender_key, plaintext).unwrap(),
        )
        .unwrap();
        ensure_audio_blob(&sender, "profile-a", blob_id).unwrap();
        let record = blob_for_audio(&sender, "profile-a", blob_id)
            .unwrap()
            .unwrap();

        let upload = upload_request(
            &sender,
            "profile-a",
            "account-profile",
            &sender_dir,
            Some(sender_key),
            &media_key,
            &record,
        )
        .unwrap();
        let response = DownloadBlobResponse {
            size_bytes: STANDARD.decode(&upload.bytes_base64).unwrap().len() as i64,
            bytes_base64: upload.bytes_base64,
            content_type: upload.content_type,
            sha256: upload.sha256,
        };

        materialize_download(
            &receiver,
            "profile-b",
            &receiver_dir,
            Some(receiver_key),
            &media_key,
            blob_id,
            &response,
        )
        .unwrap();

        let stored = fs::read(receiver_dir.join(blob_id)).unwrap();
        assert_ne!(stored, plaintext);
        assert_eq!(
            audio_crypto::decrypt_if_needed(receiver_key, &stored).unwrap(),
            plaintext
        );
        assert!(pending_uploads(&receiver, "profile-b").unwrap().is_empty());
        fs::remove_dir_all(sender_dir).unwrap();
        fs::remove_dir_all(receiver_dir).unwrap();
    }

    #[test]
    fn verified_download_replaces_a_corrupt_local_audio_file() {
        let conn = conn();
        let dir = std::env::temp_dir().join(format!("focusnook-repair-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let blob_id = "repair.webm";
        let media_key = "shared-media-key";
        let receiver_key = "receiver-vault-key";
        let plaintext = b"server copy wins after local corruption";
        let encrypted = blob_crypto::encrypt(media_key, plaintext).unwrap();
        fs::write(dir.join(blob_id), b"corrupt local bytes").unwrap();
        let response = DownloadBlobResponse {
            bytes_base64: STANDARD.encode(&encrypted),
            content_type: AUDIO_WEBM.to_string(),
            sha256: blob_crypto::sha256_hex(&encrypted),
            size_bytes: encrypted.len() as i64,
        };

        materialize_download(
            &conn,
            "profile",
            &dir,
            Some(receiver_key),
            media_key,
            blob_id,
            &response,
        )
        .unwrap();

        let stored = fs::read(dir.join(blob_id)).unwrap();
        assert_eq!(
            audio_crypto::decrypt_if_needed(receiver_key, &stored).unwrap(),
            plaintext
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn corrupt_download_is_rejected_before_writing_audio() {
        let conn = conn();
        let dir = std::env::temp_dir().join(format!("focusnook-reject-{}", uuid::Uuid::new_v4()));
        let response = DownloadBlobResponse {
            bytes_base64: STANDARD.encode(b"tampered"),
            content_type: AUDIO_WEBM.to_string(),
            sha256: "0".repeat(64),
            size_bytes: 8,
        };

        assert!(materialize_download(
            &conn,
            "profile",
            &dir,
            Some("receiver-key"),
            "media-key",
            "bad.webm",
            &response,
        )
        .is_err());
        assert!(!dir.join("bad.webm").exists());
    }
}
