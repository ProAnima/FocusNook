use super::credentials::ServerSyncCredentials;
use super::payload::client_operation;
use super::protocol::{
    LocalOperation, SyncEventResponse, SyncExchangeRequest, SyncExchangeResponse,
};
use super::{HTTP_CONNECT_TIMEOUT, HTTP_REQUEST_TIMEOUT, SERVER_EVENT_WAIT_TIMEOUT};

pub(super) fn http_client(timeout: std::time::Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .timeout(timeout)
        .build()
        .map_err(|e| e.to_string())
}

pub(super) async fn exchange_with_server(
    credentials: &ServerSyncCredentials,
    profile_id: &str,
    last_pulled: Option<String>,
    operations: &[LocalOperation],
) -> Result<SyncExchangeResponse, String> {
    let encrypted_operations = operations
        .iter()
        .map(|operation| client_operation(operation, credentials.media_key.as_deref()))
        .collect::<Result<Vec<_>, _>>()?;
    let request = SyncExchangeRequest {
        device_id: credentials.device_id.clone(),
        last_pulled_hlc: last_pulled,
        operations: encrypted_operations,
        profile_id: profile_id.to_string(),
    };
    let response = http_client(HTTP_REQUEST_TIMEOUT)?
        .post(format!("{}/v1/sync/exchange", credentials.endpoint))
        .bearer_auth(&credentials.token)
        .json(&request)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("sync server returned {}", response.status()));
    }
    response
        .json::<SyncExchangeResponse>()
        .await
        .map_err(|e| e.to_string())
}

pub(super) async fn wait_for_server_event(
    credentials: &ServerSyncCredentials,
    after_sequence: u64,
) -> Result<SyncEventResponse, String> {
    let response = http_client(SERVER_EVENT_WAIT_TIMEOUT + std::time::Duration::from_secs(5))?
        .get(format!(
            "{}/v1/sync/events?timeoutMs={}&afterSequence={after_sequence}",
            credentials.endpoint,
            SERVER_EVENT_WAIT_TIMEOUT.as_millis()
        ))
        .bearer_auth(&credentials.token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("sync event wait returned {}", response.status()));
    }
    response
        .json::<SyncEventResponse>()
        .await
        .map_err(|e| e.to_string())
}
