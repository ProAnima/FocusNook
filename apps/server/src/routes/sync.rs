use super::helpers::{canonical_profile_id, record_traffic, validate_id};
use super::operations::{operation_digest, validate_hlc, validate_operation, ClientOperation};
use crate::auth::require_device;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub(super) async fn exchange(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SyncExchangeRequest>,
) -> AppResult<Json<SyncExchangeResponse>> {
    let auth = require_device(&headers, &state).await?;
    validate_id(&request.profile_id, "profileId", 160)?;
    validate_id(&request.device_id, "deviceId", 160)?;
    if let Some(last_pulled_hlc) = &request.last_pulled_hlc {
        validate_hlc(last_pulled_hlc)?;
    }
    if request.device_id != auth.device_id {
        return Err(AppError::Forbidden);
    }
    if request.operations.len() as i64 > state.config.max_ops_per_exchange {
        return Err(AppError::BadRequest("too many operations".to_string()));
    }

    let canonical_profile_id = canonical_profile_id(auth.user_id);
    let mut accepted_count = 0_i64;
    let mut duplicate_count = 0_i64;
    let mut tx = state.pool.begin().await?;
    let migrated_legacy_scope =
        normalize_legacy_profile_scopes(&mut tx, auth.user_id, &canonical_profile_id).await?;
    for operation in &request.operations {
        validate_operation(operation, &request.device_id, &state)?;
        let digest = operation_digest(operation);
        let encrypted = state
            .crypto
            .encrypt(operation.payload_ciphertext.as_bytes())?;
        let result = sqlx::query(
            "INSERT INTO sync_operations
                (user_id, profile_id, operation_id, device_id, entity_type, entity_id, op, hlc,
                 schema_version, operation_digest, payload_ciphertext_enc, payload_nonce, payload_key_id,
                 server_nonce)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (user_id, profile_id, operation_id) DO NOTHING",
        )
        .bind(auth.user_id)
        .bind(&canonical_profile_id)
        .bind(&operation.operation_id)
        .bind(&operation.device_id)
        .bind(&operation.entity_type)
        .bind(&operation.entity_id)
        .bind(&operation.op)
        .bind(&operation.hlc)
        .bind(operation.schema_version)
        .bind(&digest)
        .bind(&encrypted.ciphertext)
        .bind(&operation.payload_nonce)
        .bind(&operation.payload_key_id)
        .bind(&encrypted.nonce)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 1 {
            accepted_count += 1;
        } else {
            ensure_existing_operation_matches(
                &mut tx,
                auth.user_id,
                &canonical_profile_id,
                &operation.operation_id,
                &digest,
            )
            .await?;
            duplicate_count += 1;
        }
    }

    let last_pulled_hlc = if migrated_legacy_scope || request.profile_id != canonical_profile_id {
        None
    } else {
        request.last_pulled_hlc.as_deref()
    };
    let rows = pull_operations(&mut tx, &state, &auth, last_pulled_hlc).await?;
    tx.commit().await?;
    let operations = decrypt_operations(&state, rows)?;
    let outbound_bytes = operations
        .iter()
        .map(|operation| operation.payload_ciphertext.len() as i64)
        .sum::<i64>();
    let inbound_bytes = request
        .operations
        .iter()
        .map(|operation| operation.payload_ciphertext.len() as i64)
        .sum::<i64>();
    record_traffic(&state, auth.user_id, inbound_bytes, outbound_bytes).await?;
    let next_pull_hlc = operations
        .last()
        .map(|operation| operation.hlc.clone())
        .or(request.last_pulled_hlc);
    if accepted_count > 0 {
        state.sync_events.notify(auth.user_id, "operation");
    }

    Ok(Json(SyncExchangeResponse {
        accepted_count,
        duplicate_count,
        next_pull_hlc,
        operations,
    }))
}

async fn pull_operations(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &AppState,
    auth: &crate::auth::DeviceAuth,
    last_pulled_hlc: Option<&str>,
) -> AppResult<Vec<DbOperation>> {
    sqlx::query_as::<_, DbOperation>(
        "SELECT operation_id, device_id, entity_type, entity_id, op, hlc, schema_version,
                payload_ciphertext_enc, payload_nonce, payload_key_id, server_nonce, created_at
         FROM sync_operations
         WHERE user_id = $1
           AND ($2::TEXT IS NULL OR hlc > $2)
         ORDER BY hlc ASC, operation_id ASC
         LIMIT $3",
    )
    .bind(auth.user_id)
    .bind(last_pulled_hlc)
    .bind(state.config.max_ops_per_exchange)
    .fetch_all(&mut **tx)
    .await
    .map_err(AppError::from)
}

async fn normalize_legacy_profile_scopes(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    canonical_profile_id: &str,
) -> AppResult<bool> {
    let deleted_ops = sqlx::query(
        "DELETE FROM sync_operations AS legacy
         USING sync_operations AS canonical
         WHERE legacy.user_id = $1
           AND legacy.profile_id <> $2
           AND canonical.user_id = legacy.user_id
           AND canonical.profile_id = $2
           AND canonical.operation_id = legacy.operation_id",
    )
    .bind(user_id)
    .bind(canonical_profile_id)
    .execute(&mut **tx)
    .await?
    .rows_affected();

    let moved_ops = sqlx::query(
        "UPDATE sync_operations
         SET profile_id = $2
         WHERE user_id = $1 AND profile_id <> $2",
    )
    .bind(user_id)
    .bind(canonical_profile_id)
    .execute(&mut **tx)
    .await?
    .rows_affected();

    let deleted_blobs = sqlx::query(
        "DELETE FROM sync_blobs AS legacy
         USING sync_blobs AS canonical
         WHERE legacy.user_id = $1
           AND legacy.profile_id <> $2
           AND canonical.user_id = legacy.user_id
           AND canonical.profile_id = $2
           AND canonical.blob_id = legacy.blob_id",
    )
    .bind(user_id)
    .bind(canonical_profile_id)
    .execute(&mut **tx)
    .await?
    .rows_affected();

    let moved_blobs = sqlx::query(
        "UPDATE sync_blobs
         SET profile_id = $2
         WHERE user_id = $1 AND profile_id <> $2",
    )
    .bind(user_id)
    .bind(canonical_profile_id)
    .execute(&mut **tx)
    .await?
    .rows_affected();

    Ok(deleted_ops + moved_ops + deleted_blobs + moved_blobs > 0)
}

async fn ensure_existing_operation_matches(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    profile_id: &str,
    operation_id: &str,
    digest: &str,
) -> AppResult<()> {
    let existing = sqlx::query_scalar::<_, String>(
        "SELECT operation_digest
         FROM sync_operations
         WHERE user_id = $1 AND profile_id = $2 AND operation_id = $3",
    )
    .bind(user_id)
    .bind(profile_id)
    .bind(operation_id)
    .fetch_optional(&mut **tx)
    .await?;

    match existing {
        Some(existing_digest) if existing_digest == digest => Ok(()),
        Some(_) => Err(AppError::Conflict(
            "operationId already exists with a different payload".to_string(),
        )),
        None => Err(AppError::Conflict(
            "operation insert conflicted but existing operation was not found".to_string(),
        )),
    }
}

fn decrypt_operations(state: &AppState, rows: Vec<DbOperation>) -> AppResult<Vec<RemoteOperation>> {
    rows.into_iter()
        .map(|row| {
            let bytes = state
                .crypto
                .decrypt(&row.payload_ciphertext_enc, &row.server_nonce)?;
            let payload_ciphertext = String::from_utf8(bytes)
                .map_err(|_| AppError::Internal("stored payload is not utf-8".to_string()))?;
            Ok(RemoteOperation {
                created_at: row.created_at,
                device_id: row.device_id,
                entity_id: row.entity_id,
                entity_type: row.entity_type,
                hlc: row.hlc,
                op: row.op,
                operation_id: row.operation_id,
                payload_ciphertext,
                payload_key_id: row.payload_key_id,
                payload_nonce: row.payload_nonce,
                schema_version: row.schema_version,
            })
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncExchangeRequest {
    device_id: String,
    last_pulled_hlc: Option<String>,
    operations: Vec<ClientOperation>,
    profile_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncExchangeResponse {
    accepted_count: i64,
    duplicate_count: i64,
    next_pull_hlc: Option<String>,
    operations: Vec<RemoteOperation>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteOperation {
    created_at: DateTime<Utc>,
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

#[derive(FromRow)]
struct DbOperation {
    created_at: DateTime<Utc>,
    device_id: String,
    entity_id: String,
    entity_type: String,
    hlc: String,
    op: String,
    operation_id: String,
    payload_ciphertext_enc: Vec<u8>,
    payload_key_id: Option<String>,
    payload_nonce: Option<String>,
    schema_version: i32,
    server_nonce: Vec<u8>,
}
