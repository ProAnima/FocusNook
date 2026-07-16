use crate::account_auth::{hash_password, normalize_email, validate_password, verify_password};
use crate::admin_web::ADMIN_HTML;
use crate::auth::{issue_token, require_admin, require_device, require_user};
use crate::error::{AppError, AppResult};
use crate::legal::PRIVACY_POLICY_VERSION;
use crate::state::AppState;
use axum::extract::DefaultBodyLimit;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Html;
use axum::routing::{delete, get, post};
use axum::Json;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

pub fn router(state: AppState) -> axum::Router {
    let max_body = state.config.max_blob_bytes + 1024 * 1024;
    axum::Router::new()
        .route("/", get(admin_console))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/privacy", get(privacy_policy))
        .route("/terms", get(terms))
        .route("/v1/admin/web/login", post(admin_web_login))
        .route("/v1/admin/monitor", get(admin_monitor))
        .route("/v1/admin/stats", get(admin_stats))
        .route("/v1/admin/users", post(create_user))
        .route("/v1/accounts/register", post(register_account))
        .route("/v1/accounts/login", post(login_account))
        .route("/v1/accounts", delete(delete_account))
        .route("/v1/devices", post(register_device))
        .route("/v1/sync/exchange", post(exchange))
        .route("/v1/sync/events", get(wait_for_sync_event))
        .route("/v1/blobs", post(upload_blob))
        .route("/v1/blobs/:profile_id/:blob_id", get(download_blob))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(max_body))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn admin_console() -> Html<&'static str> {
    Html(ADMIN_HTML)
}

async fn privacy_policy(State(state): State<AppState>) -> Html<String> {
    Html(state.config.legal_identity.privacy_html())
}

async fn terms(State(state): State<AppState>) -> Html<String> {
    Html(state.config.legal_identity.terms_html())
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

async fn readyz(State(state): State<AppState>) -> AppResult<Json<HealthResponse>> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(Json(HealthResponse { ok: true }))
}

async fn admin_web_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AdminLoginRequest>,
) -> AppResult<Json<AdminLoginResponse>> {
    let session_token = state.web_admin.login(
        &client_ip(&headers),
        &request.password,
        &state.config.admin_web_password,
    )?;
    Ok(Json(AdminLoginResponse { session_token }))
}

async fn admin_monitor(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AdminMonitorResponse>> {
    require_admin_web_session(&headers, &state)?;
    let rows = sqlx::query_as::<_, AdminUserMetricRow>(
        "SELECT
            u.id AS user_id,
            u.display_name,
            u.disabled_at IS NOT NULL AS disabled,
            COALESCE((SELECT count(*) FROM devices d WHERE d.user_id = u.id AND d.revoked_at IS NULL), 0) AS devices,
            COALESCE((SELECT count(*) FROM sync_operations o WHERE o.user_id = u.id), 0) AS operations,
            COALESCE((SELECT count(*) FROM sync_blobs b WHERE b.user_id = u.id), 0) AS blobs,
            COALESCE((SELECT sum(octet_length(o.payload_ciphertext_enc)) FROM sync_operations o WHERE o.user_id = u.id), 0)::BIGINT
              + COALESCE((SELECT sum(b.size_bytes) FROM sync_blobs b WHERE b.user_id = u.id), 0)::BIGINT AS storage_bytes,
            COALESCE(t.inbound_bytes, 0) AS inbound_bytes,
            COALESCE(t.outbound_bytes, 0) AS outbound_bytes,
            GREATEST(
              COALESCE((SELECT max(d.last_seen_at) FROM devices d WHERE d.user_id = u.id), u.created_at),
              COALESCE((SELECT max(o.created_at) FROM sync_operations o WHERE o.user_id = u.id), u.created_at),
              COALESCE((SELECT max(b.created_at) FROM sync_blobs b WHERE b.user_id = u.id), u.created_at)
            ) AS last_seen_at
         FROM users u
         LEFT JOIN sync_traffic_counters t ON t.user_id = u.id
         ORDER BY last_seen_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;
    let users = rows
        .into_iter()
        .map(|row| AdminUserMetric {
            blobs: row.blobs,
            devices: row.devices,
            disabled: row.disabled,
            display_name: row.display_name,
            inbound_bytes: row.inbound_bytes,
            last_seen_at: row.last_seen_at,
            operations: row.operations,
            outbound_bytes: row.outbound_bytes,
            storage_bytes: row.storage_bytes,
            user_id: row.user_id,
        })
        .collect::<Vec<_>>();
    let summary = AdminMonitorSummary {
        blobs: users.iter().map(|user| user.blobs).sum(),
        devices: users.iter().map(|user| user.devices).sum(),
        inbound_bytes: users.iter().map(|user| user.inbound_bytes).sum(),
        operations: users.iter().map(|user| user.operations).sum(),
        outbound_bytes: users.iter().map(|user| user.outbound_bytes).sum(),
        storage_bytes: users.iter().map(|user| user.storage_bytes).sum(),
        users: users.len() as i64,
    };
    Ok(Json(AdminMonitorResponse {
        generated_at: Utc::now(),
        summary,
        users,
    }))
}

async fn admin_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AdminStatsResponse>> {
    require_admin(&headers, &state)?;
    let row = sqlx::query_as::<_, AdminStatsRow>(
        "SELECT
            (SELECT count(*) FROM users WHERE disabled_at IS NULL) AS users,
            (SELECT count(*) FROM devices WHERE revoked_at IS NULL) AS devices,
            (SELECT count(*) FROM sync_operations) AS operations,
            (SELECT count(*) FROM sync_blobs) AS blobs",
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(AdminStatsResponse {
        blobs: row.blobs,
        devices: row.devices,
        operations: row.operations,
        users: row.users,
    }))
}

async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateUserRequest>,
) -> AppResult<Json<CreateUserResponse>> {
    require_admin(&headers, &state)?;
    validate_label(&request.display_name, "displayName", 120)?;

    let user_id =
        sqlx::query_scalar::<_, Uuid>("INSERT INTO users (display_name) VALUES ($1) RETURNING id")
            .bind(request.display_name.trim())
            .fetch_one(&state.pool)
            .await?;

    let issued = issue_token("fnk_user", &state.config.token_pepper)?;
    sqlx::query("INSERT INTO user_tokens (user_id, token_hash, label) VALUES ($1, $2, 'primary')")
        .bind(user_id)
        .bind(&issued.token_hash)
        .execute(&state.pool)
        .await?;

    Ok(Json(CreateUserResponse {
        user_id,
        user_token: issued.token,
    }))
}

async fn register_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AccountAuthRequest>,
) -> AppResult<Json<AccountAuthResponse>> {
    if !request.privacy_accepted
        || request.privacy_policy_version.as_deref() != Some(PRIVACY_POLICY_VERSION)
    {
        return Err(AppError::BadRequest(
            "privacy policy consent is required".to_string(),
        ));
    }
    let ip = client_ip(&headers);
    let email = normalize_email(&request.email)?;
    validate_password(&request.password)?;
    validate_id(&request.device_id, "deviceId", 160)?;
    validate_label(&request.device_name, "deviceName", 120)?;
    validate_label(&request.platform, "platform", 40)?;
    state.account_auth.record_registration(&ip)?;
    let display_name = account_display_name(request.display_name.as_deref(), &email)?;
    // P0 аудит: Argon2 — секунды CPU под нагрузкой, а не миллисекунды; синхронный
    // вызов внутри async-хендлера блокирует сам Tokio worker-поток, а не только
    // этот запрос. spawn_blocking уводит его в отдельный блокирующий пул, у
    // которого уже есть свой верхний предел потоков (в отличие от worker-пула).
    let password = request.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|_| AppError::Internal("password hashing task panicked".to_string()))??;
    let mut tx = state.pool.begin().await?;
    let user_id =
        sqlx::query_scalar::<_, Uuid>("INSERT INTO users (display_name) VALUES ($1) RETURNING id")
            .bind(&display_name)
            .fetch_one(&mut *tx)
            .await?;
    let account_result = sqlx::query(
        "INSERT INTO user_accounts (user_id, email, password_hash)
         VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(&email)
    .bind(&password_hash)
    .execute(&mut *tx)
    .await;
    if is_unique_violation(&account_result) {
        return Err(AppError::Conflict(
            "email is already registered".to_string(),
        ));
    }
    account_result?;
    sqlx::query("INSERT INTO privacy_consents (user_id, policy_version) VALUES ($1, $2)")
        .bind(user_id)
        .bind(PRIVACY_POLICY_VERSION)
        .execute(&mut *tx)
        .await?;
    let device_token = upsert_device_token(
        &mut tx,
        &state,
        user_id,
        &request.device_id,
        &request.device_name,
        &request.platform,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(AccountAuthResponse {
        device_token,
        email,
        display_name,
        user_id,
    }))
}

async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<DeleteAccountRequest>,
) -> AppResult<Json<HealthResponse>> {
    let auth = require_device(&headers, &state).await?;
    let password_hash = sqlx::query_scalar::<_, String>(
        "SELECT password_hash FROM user_accounts WHERE user_id = $1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    let password = request.password;
    let valid = tokio::task::spawn_blocking(move || verify_password(&password, &password_hash))
        .await
        .map_err(|_| AppError::Internal("password verification task panicked".to_string()))??;
    if !valid {
        return Err(AppError::Unauthorized);
    }
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(auth.user_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(HealthResponse { ok: true }))
}

async fn login_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AccountAuthRequest>,
) -> AppResult<Json<AccountAuthResponse>> {
    let ip = client_ip(&headers);
    let email = normalize_email(&request.email)?;
    validate_id(&request.device_id, "deviceId", 160)?;
    validate_label(&request.device_name, "deviceName", 120)?;
    validate_label(&request.platform, "platform", 40)?;
    state.account_auth.ensure_not_locked(&ip, &email)?;
    let row = sqlx::query_as::<_, AccountLoginRow>(
        "SELECT u.id AS user_id, u.display_name, a.password_hash
         FROM user_accounts a
         JOIN users u ON u.id = a.user_id
         WHERE a.email = $1 AND u.disabled_at IS NULL",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await?;
    let Some(row) = row else {
        state.account_auth.record_failure(&ip, &email)?;
        return Err(AppError::Unauthorized);
    };
    // См. комментарий у hash_password в register_account — то же самое для
    // проверки пароля на входе.
    let password = request.password.clone();
    let stored_hash = row.password_hash.clone();
    let password_is_valid =
        tokio::task::spawn_blocking(move || verify_password(&password, &stored_hash))
            .await
            .map_err(|_| AppError::Internal("password verification task panicked".to_string()))??;
    if !password_is_valid {
        state.account_auth.record_failure(&ip, &email)?;
        return Err(AppError::Unauthorized);
    }
    state.account_auth.clear_failures(&ip, &email)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE user_accounts SET last_login_at = now() WHERE user_id = $1")
        .bind(row.user_id)
        .execute(&mut *tx)
        .await?;
    let device_token = upsert_device_token(
        &mut tx,
        &state,
        row.user_id,
        &request.device_id,
        &request.device_name,
        &request.platform,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(AccountAuthResponse {
        device_token,
        email,
        display_name: row.display_name,
        user_id: row.user_id,
    }))
}

async fn register_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterDeviceRequest>,
) -> AppResult<Json<RegisterDeviceResponse>> {
    let auth = require_user(&headers, &state).await?;
    validate_id(&request.device_id, "deviceId", 160)?;
    validate_label(&request.display_name, "displayName", 120)?;
    validate_label(&request.platform, "platform", 40)?;

    let mut tx = state.pool.begin().await?;
    let (device_token, device_row_id, created_at) = upsert_device_token_with_row(
        &mut tx,
        &state,
        auth.user_id,
        &request.device_id,
        &request.display_name,
        &request.platform,
    )
    .await?;
    tx.commit().await?;

    Ok(Json(RegisterDeviceResponse {
        device_row_id,
        created_at,
        device_token,
    }))
}

async fn wait_for_sync_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SyncEventQuery>,
) -> AppResult<Json<SyncEventResponse>> {
    let auth = require_device(&headers, &state).await?;
    let timeout_ms = query.timeout_ms.unwrap_or(25_000).clamp(1_000, 30_000);
    let event = state
        .sync_events
        .wait(auth.user_id, std::time::Duration::from_millis(timeout_ms))
        .await;
    Ok(Json(match event {
        Some(event) => SyncEventResponse {
            changed: true,
            reason: Some(event.reason),
            sequence: event.sequence,
        },
        None => SyncEventResponse {
            changed: false,
            reason: None,
            sequence: 0,
        },
    }))
}

async fn upsert_device_token(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &AppState,
    user_id: Uuid,
    device_id: &str,
    display_name: &str,
    platform: &str,
) -> AppResult<String> {
    upsert_device_token_with_row(tx, state, user_id, device_id, display_name, platform)
        .await
        .map(|row| row.0)
}

async fn upsert_device_token_with_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &AppState,
    user_id: Uuid,
    device_id: &str,
    display_name: &str,
    platform: &str,
) -> AppResult<(String, Uuid, DateTime<Utc>)> {
    let issued = issue_token("fnk_device", &state.config.token_pepper)?;
    let row = sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
        "INSERT INTO devices (user_id, device_id, display_name, platform, token_hash, last_seen_at)
         VALUES ($1, $2, $3, $4, $5, now())
         ON CONFLICT (user_id, device_id)
         DO UPDATE SET
           display_name = excluded.display_name,
           platform = excluded.platform,
           token_hash = excluded.token_hash,
           last_seen_at = now(),
           revoked_at = NULL
         RETURNING id, created_at",
    )
    .bind(user_id)
    .bind(device_id.trim())
    .bind(display_name.trim())
    .bind(platform.trim())
    .bind(&issued.token_hash)
    .fetch_one(&mut **tx)
    .await?;
    Ok((issued.token, row.0, row.1))
}

async fn exchange(
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

async fn upload_blob(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UploadBlobRequest>,
) -> AppResult<Json<UploadBlobResponse>> {
    let auth = require_device(&headers, &state).await?;
    validate_id(&request.profile_id, "profileId", 160)?;
    validate_id(&request.blob_id, "blobId", 240)?;
    validate_label(&request.content_type, "contentType", 120)?;
    validate_id(&request.sha256, "sha256", 128)?;
    let content_type = request.content_type.trim();
    let sha256 = request.sha256.trim();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(request.bytes_base64.trim())
        .map_err(|_| AppError::BadRequest("bytesBase64 must be base64".to_string()))?;
    if bytes.len() > state.config.max_blob_bytes {
        return Err(AppError::BadRequest("blob is too large".to_string()));
    }
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
    .bind(sha256)
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
            sha256,
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

async fn download_blob(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((profile_id, blob_id)): Path<(String, String)>,
) -> AppResult<Json<DownloadBlobResponse>> {
    let auth = require_device(&headers, &state).await?;
    validate_id(&profile_id, "profileId", 160)?;
    validate_id(&blob_id, "blobId", 240)?;
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

async fn record_traffic(
    state: &AppState,
    user_id: Uuid,
    inbound_bytes: i64,
    outbound_bytes: i64,
) -> AppResult<()> {
    if inbound_bytes == 0 && outbound_bytes == 0 {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO sync_traffic_counters (user_id, inbound_bytes, outbound_bytes, updated_at)
         VALUES ($1, $2, $3, now())
         ON CONFLICT (user_id)
         DO UPDATE SET
           inbound_bytes = sync_traffic_counters.inbound_bytes + excluded.inbound_bytes,
           outbound_bytes = sync_traffic_counters.outbound_bytes + excluded.outbound_bytes,
           updated_at = now()",
    )
    .bind(user_id)
    .bind(inbound_bytes)
    .bind(outbound_bytes)
    .execute(&state.pool)
    .await?;
    Ok(())
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

fn canonical_profile_id(user_id: Uuid) -> String {
    user_id.to_string()
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

fn validate_operation(
    operation: &ClientOperation,
    expected_device_id: &str,
    state: &AppState,
) -> AppResult<()> {
    validate_id(&operation.operation_id, "operationId", 160)?;
    validate_id(&operation.device_id, "deviceId", 160)?;
    validate_id(&operation.entity_type, "entityType", 80)?;
    validate_id(&operation.entity_id, "entityId", 160)?;
    validate_id(&operation.op, "op", 80)?;
    validate_hlc(&operation.hlc)?;
    if operation.device_id != expected_device_id {
        return Err(AppError::Forbidden);
    }
    if operation.schema_version < 1 {
        return Err(AppError::BadRequest(
            "schemaVersion must be positive".to_string(),
        ));
    }
    if operation.payload_ciphertext.is_empty()
        || operation.payload_ciphertext.len() > state.config.max_operation_payload_bytes
    {
        return Err(AppError::BadRequest(
            "payloadCiphertext has invalid size".to_string(),
        ));
    }
    Ok(())
}

fn validate_hlc(value: &str) -> AppResult<()> {
    validate_id(value, "hlc", 220)?;
    let bytes = value.as_bytes();
    let has_min_shape = value.len() >= 31
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
        && bytes.get(19) == Some(&b'.')
        && bytes.get(23) == Some(&b'Z')
        && bytes.get(24) == Some(&b'-')
        && bytes.get(29) == Some(&b'-');
    if has_min_shape {
        Ok(())
    } else {
        Err(AppError::BadRequest("hlc is invalid".to_string()))
    }
}

fn operation_digest(operation: &ClientOperation) -> String {
    let mut hasher = Sha256::new();
    hash_part(&mut hasher, &operation.operation_id);
    hash_part(&mut hasher, &operation.device_id);
    hash_part(&mut hasher, &operation.entity_type);
    hash_part(&mut hasher, &operation.entity_id);
    hash_part(&mut hasher, &operation.op);
    hash_part(&mut hasher, &operation.hlc);
    hasher.update(operation.schema_version.to_be_bytes());
    hash_part(&mut hasher, &operation.payload_ciphertext);
    hash_optional_part(&mut hasher, operation.payload_key_id.as_deref());
    hash_optional_part(&mut hasher, operation.payload_nonce.as_deref());
    hex_encode(&hasher.finalize())
}

fn hash_part(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

fn hash_optional_part(hasher: &mut Sha256, value: Option<&str>) {
    match value {
        Some(value) => {
            hasher.update([1]);
            hash_part(hasher, value);
        }
        None => hasher.update([0]),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_id(value: &str, field: &str, max_len: usize) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max_len || trimmed.chars().any(char::is_whitespace) {
        return Err(AppError::BadRequest(format!("{field} is invalid")));
    }
    Ok(())
}

fn validate_label(value: &str, field: &str, max_len: usize) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max_len {
        return Err(AppError::BadRequest(format!("{field} is invalid")));
    }
    Ok(())
}

fn account_display_name(raw: Option<&str>, email: &str) -> AppResult<String> {
    let fallback = email.split('@').next().unwrap_or("FocusNook").to_string();
    let display_name = raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&fallback);
    validate_label(display_name, "displayName", 120)?;
    Ok(display_name.to_string())
}

fn is_unique_violation(result: &Result<sqlx::postgres::PgQueryResult, sqlx::Error>) -> bool {
    result
        .as_ref()
        .err()
        .and_then(sqlx::Error::as_database_error)
        .and_then(|err| err.code())
        .is_some_and(|code| code == "23505")
}

fn require_admin_web_session(headers: &HeaderMap, state: &AppState) -> AppResult<()> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let token = raw.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
    state.web_admin.authorize(token)
}

// P0 аудит: раньше X-Forwarded-For проверялся первым и брался первым
// значением из списка — а nginx (см. focusnook.conf, $proxy_add_x_forwarded_for)
// дописывает настоящий $remote_addr В КОНЕЦ существующего заголовка, а не
// заменяет его, так что первое значение остаётся полностью под контролем
// клиента. Любой желающий обойти rate-limit на логин/регистрацию просто
// присылал свой X-Forwarded-For на каждый запрос.
//
// X-Real-IP — не то же самое: nginx выставляет его через `proxy_set_header
// X-Real-IP $remote_addr`, что заменяет любое клиентское значение целиком, а
// не дописывает к нему. Он всегда достоверен для этого конкретного nginx-фронта,
// поэтому теперь основной источник — он; X-Forwarded-For остаётся только
// запасным вариантом для деплоя без X-Real-IP (например, прямого доступа в
// дев-окружении без nginx), где он настолько же спуфится, что и раньше.
fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("unknown")
        .to_string()
}

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
}

#[derive(Deserialize)]
struct AdminLoginRequest {
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminLoginResponse {
    session_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminMonitorResponse {
    generated_at: DateTime<Utc>,
    summary: AdminMonitorSummary,
    users: Vec<AdminUserMetric>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminMonitorSummary {
    blobs: i64,
    devices: i64,
    inbound_bytes: i64,
    operations: i64,
    outbound_bytes: i64,
    storage_bytes: i64,
    users: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminUserMetric {
    blobs: i64,
    devices: i64,
    disabled: bool,
    display_name: String,
    inbound_bytes: i64,
    last_seen_at: DateTime<Utc>,
    operations: i64,
    outbound_bytes: i64,
    storage_bytes: i64,
    user_id: Uuid,
}

#[derive(FromRow)]
struct AdminUserMetricRow {
    blobs: i64,
    devices: i64,
    disabled: bool,
    display_name: String,
    inbound_bytes: i64,
    last_seen_at: DateTime<Utc>,
    operations: i64,
    outbound_bytes: i64,
    storage_bytes: i64,
    user_id: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminStatsResponse {
    blobs: i64,
    devices: i64,
    operations: i64,
    users: i64,
}

#[derive(FromRow)]
struct AdminStatsRow {
    blobs: i64,
    devices: i64,
    operations: i64,
    users: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserRequest {
    display_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserResponse {
    user_id: Uuid,
    user_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterDeviceRequest {
    device_id: String,
    display_name: String,
    platform: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterDeviceResponse {
    created_at: DateTime<Utc>,
    device_row_id: Uuid,
    device_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountAuthRequest {
    email: String,
    password: String,
    display_name: Option<String>,
    device_id: String,
    device_name: String,
    platform: String,
    #[serde(default)]
    privacy_accepted: bool,
    #[serde(default)]
    privacy_policy_version: Option<String>,
}

#[derive(Deserialize)]
struct DeleteAccountRequest {
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountAuthResponse {
    device_token: String,
    email: String,
    display_name: String,
    user_id: Uuid,
}

#[derive(FromRow)]
struct AccountLoginRow {
    display_name: String,
    password_hash: String,
    user_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncExchangeRequest {
    device_id: String,
    last_pulled_hlc: Option<String>,
    operations: Vec<ClientOperation>,
    profile_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncEventQuery {
    timeout_ms: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncEventResponse {
    changed: bool,
    reason: Option<String>,
    sequence: u64,
}

#[derive(Clone, Deserialize)]
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncExchangeResponse {
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadBlobRequest {
    blob_id: String,
    bytes_base64: String,
    content_type: String,
    profile_id: String,
    sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadBlobResponse {
    blob_id: String,
    size_bytes: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadBlobResponse {
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
    fn rejects_whitespace_in_identifiers() {
        assert!(validate_id("abc def", "field", 20).is_err());
        assert!(validate_id("abcdef", "field", 20).is_ok());
    }

    #[test]
    fn validates_expected_hlc_shape() {
        assert!(validate_hlc("2026-07-06T19:22:10.100Z-0001-device-a").is_ok());
        assert!(validate_hlc("2026-07-06 19:22:10").is_err());
    }

    fn headers_with(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        headers
    }

    // P0 аудит: nginx дописывает $remote_addr в конец X-Forwarded-For, а не
    // заменяет его — так что клиент, приславший свой X-Forwarded-For, не
    // может подменить X-Real-IP тем же трюком (nginx выставляет его через
    // proxy_set_header, который заменяет значение целиком).
    #[test]
    fn client_ip_prefers_x_real_ip_over_a_spoofed_x_forwarded_for() {
        let headers = headers_with(&[("x-forwarded-for", "1.2.3.4"), ("x-real-ip", "203.0.113.9")]);
        assert_eq!(client_ip(&headers), "203.0.113.9");
    }

    #[test]
    fn client_ip_falls_back_to_x_forwarded_for_without_x_real_ip() {
        let headers = headers_with(&[("x-forwarded-for", "203.0.113.9, 10.0.0.1")]);
        assert_eq!(client_ip(&headers), "203.0.113.9");
    }

    #[test]
    fn client_ip_is_unknown_without_either_header() {
        assert_eq!(client_ip(&HeaderMap::new()), "unknown");
    }

    #[test]
    fn operation_digest_changes_when_payload_changes() {
        let first = ClientOperation {
            device_id: "device".to_string(),
            entity_id: "note-1".to_string(),
            entity_type: "note".to_string(),
            hlc: "2026-07-06T19:22:10.100Z-0001-device".to_string(),
            op: "upsert".to_string(),
            operation_id: "op-1".to_string(),
            payload_ciphertext: "payload-a".to_string(),
            payload_key_id: Some("key-1".to_string()),
            payload_nonce: Some("nonce-1".to_string()),
            schema_version: 1,
        };
        let mut second = first.clone();
        second.payload_ciphertext = "payload-b".to_string();

        assert_ne!(operation_digest(&first), operation_digest(&second));
        assert_eq!(operation_digest(&first), operation_digest(&first));
    }
}
