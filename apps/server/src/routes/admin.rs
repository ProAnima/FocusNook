use super::helpers::{client_ip, validate_label};
use crate::auth::{issue_token, require_admin};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub(super) async fn admin_web_login(
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

pub(super) async fn admin_monitor(
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

pub(super) async fn admin_stats(
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

pub(super) async fn create_user(
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

fn require_admin_web_session(headers: &HeaderMap, state: &AppState) -> AppResult<()> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let token = raw.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
    state.web_admin.authorize(token)
}

#[derive(Deserialize)]
pub(super) struct AdminLoginRequest {
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminLoginResponse {
    session_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminMonitorResponse {
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
pub(super) struct AdminStatsResponse {
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
pub(super) struct CreateUserRequest {
    display_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateUserResponse {
    user_id: Uuid,
    user_token: String,
}
