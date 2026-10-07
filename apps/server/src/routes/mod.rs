mod accounts;
mod admin;
mod blobs;
mod events;
mod helpers;
mod operations;
mod sync;

use crate::admin_web::ADMIN_HTML;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::DefaultBodyLimit;
use axum::extract::State;
use axum::response::Html;
use axum::routing::{delete, get, post};
use axum::Json;
use serde::Serialize;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> axum::Router {
    let max_body = state.config.max_blob_bytes + 1024 * 1024;
    axum::Router::new()
        .route("/", get(admin_console))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/privacy", get(privacy_policy))
        .route("/terms", get(terms))
        .route("/v1/admin/web/login", post(admin::admin_web_login))
        .route("/v1/admin/monitor", get(admin::admin_monitor))
        .route("/v1/admin/stats", get(admin::admin_stats))
        .route("/v1/admin/users", post(admin::create_user))
        .route("/v1/accounts/register", post(accounts::register_account))
        .route("/v1/accounts/login", post(accounts::login_account))
        .route("/v1/accounts", delete(accounts::delete_account))
        .route("/v1/devices", post(accounts::register_device))
        .route("/v1/sync/exchange", post(sync::exchange))
        .route("/v1/sync/events", get(events::wait_for_sync_event))
        .route("/v1/blobs", post(blobs::upload_blob))
        .route("/v1/blobs/:profile_id/:blob_id", get(blobs::download_blob))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(max_body))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn admin_console() -> Html<&'static str> {
    Html(ADMIN_HTML)
}

async fn privacy_policy(State(state): State<AppState>) -> AppResult<Html<String>> {
    let identity = state
        .config
        .legal_identity
        .as_ref()
        .ok_or(AppError::NotFound)?;
    Ok(Html(identity.privacy_html()))
}

async fn terms(State(state): State<AppState>) -> AppResult<Html<String>> {
    let identity = state
        .config
        .legal_identity
        .as_ref()
        .ok_or(AppError::NotFound)?;
    Ok(Html(identity.terms_html()))
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

async fn readyz(State(state): State<AppState>) -> AppResult<Json<HealthResponse>> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(Json(HealthResponse { ok: true }))
}

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
}
