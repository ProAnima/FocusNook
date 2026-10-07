use crate::auth::require_device;
use crate::error::AppResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};

pub(super) async fn wait_for_sync_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SyncEventQuery>,
) -> AppResult<Json<SyncEventResponse>> {
    let auth = require_device(&headers, &state).await?;
    let timeout_ms = query.timeout_ms.unwrap_or(25_000).clamp(1_000, 30_000);
    let event = state
        .sync_events
        .wait_after(
            auth.user_id,
            query.after_sequence.unwrap_or(0),
            std::time::Duration::from_millis(timeout_ms),
        )
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncEventQuery {
    after_sequence: Option<u64>,
    timeout_ms: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncEventResponse {
    changed: bool,
    reason: Option<String>,
    sequence: u64,
}
