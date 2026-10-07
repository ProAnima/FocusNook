use super::helpers::{client_ip, validate_id, validate_label};
use super::HealthResponse;
use crate::account_auth::{hash_password, normalize_email, validate_password, verify_password};
use crate::auth::{issue_token, require_device, require_user};
use crate::error::{AppError, AppResult};
use crate::legal::PRIVACY_POLICY_VERSION;
use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub(super) async fn register_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AccountAuthRequest>,
) -> AppResult<Json<AccountAuthResponse>> {
    if state.config.legal_identity.is_none() {
        return Err(AppError::NotFound);
    }
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

pub(super) async fn delete_account(
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

pub(super) async fn login_account(
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

pub(super) async fn register_device(
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RegisterDeviceRequest {
    device_id: String,
    display_name: String,
    platform: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RegisterDeviceResponse {
    created_at: DateTime<Utc>,
    device_row_id: Uuid,
    device_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AccountAuthRequest {
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
pub(super) struct DeleteAccountRequest {
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AccountAuthResponse {
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
