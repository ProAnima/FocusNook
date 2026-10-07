use super::helpers::{
    canonical_profile_id, hex_encode, record_traffic, validate_id, validate_label,
};
use crate::auth::require_device;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

pub(super) async fn upload_blob(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UploadBlobRequest>,
) -> AppResult<Json<UploadBlobResponse>> {
    let auth = require_device(&headers, &state).await?;
    validate_id(&request.profile_id, "profileId", 160)?;
    validate_blob_id(&request.blob_id)?;
    validate_label(&request.content_type, "contentType", 120)?;
    let content_type = request.content_type.trim();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(request.bytes_base64.trim())
        .map_err(|_| AppError::BadRequest("bytesBase64 must be base64".to_string()))?;
    if bytes.len() > state.config.max_blob_bytes {
        return Err(AppError::BadRequest("blob is too large".to_string()));
    }
    let sha256 = validate_blob_sha256(&request.sha256, &bytes)?;
    let encrypted = state.crypto.encrypt(&bytes)?;
    let canonical_profile_id = canonical_profile_id(auth.user_id);

    let result = sqlx::query(
        "INSERT INTO sync_blobs
            (user_id, profile_id, blob_id, content_type, sha256, size_bytes, bytes_enc, server_nonce)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (user_id, profile_id, blob_id) DO NOTHING",
    )
    .bind(auth.user_id)
    .bind(&canonical_profile_id)
    .bind(&request.blob_id)
    .bind(content_type)
    .bind(&sha256)
    .bind(bytes.len() as i64)
    .bind(&encrypted.ciphertext)
    .bind(&encrypted.nonce)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        ensure_existing_blob_matches(
            &state,
            auth.user_id,
            &canonical_profile_id,
            &request.blob_id,
            content_type,
            &sha256,
            bytes.len() as i64,
        )
        .await?;
    }
    record_traffic(&state, auth.user_id, bytes.len() as i64, 0).await?;
    state.sync_events.notify(auth.user_id, "blob");

    Ok(Json(UploadBlobResponse {
        blob_id: request.blob_id,
        size_bytes: bytes.len() as i64,
    }))
}

pub(super) async fn download_blob(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((profile_id, blob_id)): Path<(String, String)>,
) -> AppResult<Json<DownloadBlobResponse>> {
    let auth = require_device(&headers, &state).await?;
    validate_id(&profile_id, "profileId", 160)?;
    validate_blob_id(&blob_id)?;
    let row = sqlx::query_as::<_, DbBlob>(
        "SELECT blob_id, content_type, sha256, size_bytes, bytes_enc, server_nonce
         FROM sync_blobs
         WHERE user_id = $1 AND blob_id = $2
         ORDER BY CASE WHEN profile_id = $3 THEN 0 ELSE 1 END, created_at DESC
         LIMIT 1",
    )
    .bind(auth.user_id)
    .bind(&blob_id)
    .bind(canonical_profile_id(auth.user_id))
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let bytes = state.crypto.decrypt(&row.bytes_enc, &row.server_nonce)?;
    record_traffic(&state, auth.user_id, 0, bytes.len() as i64).await?;

    Ok(Json(DownloadBlobResponse {
        blob_id: row.blob_id,
        bytes_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        content_type: row.content_type,
        sha256: row.sha256,
        size_bytes: row.size_bytes,
    }))
}

async fn ensure_existing_blob_matches(
    state: &AppState,
    user_id: Uuid,
    profile_id: &str,
    blob_id: &str,
    content_type: &str,
    sha256: &str,
    size_bytes: i64,
) -> AppResult<()> {
    let existing = sqlx::query_as::<_, DbBlobMetadata>(
        "SELECT content_type, sha256, size_bytes
         FROM sync_blobs
         WHERE user_id = $1 AND profile_id = $2 AND blob_id = $3",
    )
    .bind(user_id)
    .bind(profile_id)
    .bind(blob_id)
    .fetch_optional(&state.pool)
    .await?;

    match existing {
        Some(row)
            if row.content_type == content_type
                && row.sha256 == sha256
                && row.size_bytes == size_bytes =>
        {
            Ok(())
        }
        Some(_) => Err(AppError::Conflict(
            "blobId already exists with different content".to_string(),
        )),
        None => Err(AppError::Conflict(
            "blob insert conflicted but existing blob was not found".to_string(),
        )),
    }
}

fn validate_blob_sha256(claimed: &str, bytes: &[u8]) -> AppResult<String> {
    let claimed = claimed.trim();
    if claimed.len() != 64
        || !claimed
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(AppError::BadRequest("sha256 is invalid".to_string()));
    }
    let computed = hex_encode(&Sha256::digest(bytes));
    if claimed != computed {
        return Err(AppError::BadRequest(
            "sha256 does not match uploaded bytes".to_string(),
        ));
    }
    Ok(computed)
}

fn validate_blob_id(value: &str) -> AppResult<()> {
    validate_id(value, "blobId", 240)?;
    if value == "." || value == ".." || value.contains('/') || value.contains('\\') {
        return Err(AppError::BadRequest("blobId is invalid".to_string()));
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UploadBlobRequest {
    blob_id: String,
    bytes_base64: String,
    content_type: String,
    profile_id: String,
    sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UploadBlobResponse {
    blob_id: String,
    size_bytes: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DownloadBlobResponse {
    blob_id: String,
    bytes_base64: String,
    content_type: String,
    sha256: String,
    size_bytes: i64,
}

#[derive(FromRow)]
struct DbBlob {
    blob_id: String,
    bytes_enc: Vec<u8>,
    content_type: String,
    server_nonce: Vec<u8>,
    sha256: String,
    size_bytes: i64,
}

#[derive(FromRow)]
struct DbBlobMetadata {
    content_type: String,
    sha256: String,
    size_bytes: i64,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn validates_uploaded_blob_digest_against_actual_bytes() {
        let bytes = b"encrypted cross-device audio";
        let digest = hex_encode(&Sha256::digest(bytes));

        assert_eq!(validate_blob_sha256(&digest, bytes).unwrap(), digest);
        assert!(validate_blob_sha256(&"0".repeat(64), bytes).is_err());
        assert!(validate_blob_sha256("ABC", bytes).is_err());
    }

    #[test]
    fn rejects_blob_ids_that_can_escape_the_audio_directory() {
        assert!(validate_blob_id("voice.webm").is_ok());
        assert!(validate_blob_id("../voice.webm").is_err());
        assert!(validate_blob_id(r"..\voice.webm").is_err());
    }
}
