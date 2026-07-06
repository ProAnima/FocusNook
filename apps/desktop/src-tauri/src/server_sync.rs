use crate::{config, profiles, sync_log};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const SERVER_SYNC_KEYRING_SERVICE: &str = "com.proanima.focusnook.server-sync";
const SERVER_SYNC_KEY_PREFIX: &str = "vds_server";

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
struct ServerSyncCredentials {
    #[serde(default)]
    device_id: String,
    endpoint: String,
    token: String,
}

#[derive(Clone, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerSyncStatus {
    available: bool,
    connected: bool,
    endpoint: Option<String>,
    message: Option<String>,
}

fn keyring_user(profile_id: &str) -> String {
    format!("{SERVER_SYNC_KEY_PREFIX}-{profile_id}")
}

fn host_after_scheme<'a>(endpoint: &'a str, scheme: &str) -> Option<&'a str> {
    let rest = endpoint.get(scheme.len()..)?;
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .map(|value| value.trim_matches('[').trim_matches(']'))?;
    if host.is_empty() || host.starts_with(':') {
        return None;
    }
    Some(host.split(':').next().unwrap_or(host))
}

fn is_local_http_endpoint(endpoint: &str) -> bool {
    let Some(host) = host_after_scheme(endpoint, "http://") else {
        return false;
    };
    matches!(host, "localhost" | "127.0.0.1")
}

fn normalize_endpoint(raw: &str) -> Result<String, String> {
    let endpoint = raw.trim().trim_end_matches('/').to_string();
    if endpoint.is_empty() {
        return Err("sync server endpoint is required".to_string());
    }
    if endpoint.len() > 2048 || endpoint.chars().any(char::is_whitespace) {
        return Err("sync server endpoint is invalid".to_string());
    }

    let lower = endpoint.to_ascii_lowercase();
    let is_https = lower.starts_with("https://") && host_after_scheme(&lower, "https://").is_some();
    let is_local_dev = is_local_http_endpoint(&lower);
    if !is_https && !is_local_dev {
        return Err("sync server endpoint must use https".to_string());
    }

    Ok(endpoint)
}

fn normalize_token(raw: &str) -> Result<String, String> {
    let token = raw.trim().to_string();
    if token.is_empty() {
        return Err("sync server token is required".to_string());
    }
    if token.len() > 8192 {
        return Err("sync server token is too long".to_string());
    }
    Ok(token)
}

fn entry(profile_id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVER_SYNC_KEYRING_SERVICE, &keyring_user(profile_id))
        .map_err(|e| e.to_string())
}

fn store_credentials(
    profile_id: &str,
    endpoint: &str,
    token: &str,
    device_id: &str,
) -> Result<(), String> {
    let credentials = ServerSyncCredentials {
        device_id: normalize_token(device_id)?,
        endpoint: normalize_endpoint(endpoint)?,
        token: normalize_token(token)?,
    };
    let raw = serde_json::to_string(&credentials).map_err(|e| e.to_string())?;
    entry(profile_id)?
        .set_password(&raw)
        .map_err(|e| e.to_string())
}

fn load_credentials(profile_id: &str) -> Result<Option<ServerSyncCredentials>, String> {
    match entry(profile_id)?.get_password() {
        Ok(raw) => serde_json::from_str(&raw)
            .map(Some)
            .map_err(|e| e.to_string()),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

fn delete_credentials(profile_id: &str) -> Result<(), String> {
    match entry(profile_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn status_for_profile(
    profile_id: &str,
    configured: bool,
    message: Option<String>,
) -> Result<ServerSyncStatus, String> {
    let credentials = load_credentials(profile_id)?;
    Ok(ServerSyncStatus {
        available: configured,
        connected: credentials.is_some(),
        endpoint: credentials.map(|value| value.endpoint),
        message,
    })
}

#[tauri::command]
pub fn server_sync_status(
    config_state: tauri::State<config::SyncProvidersConfig>,
    profiles_state: tauri::State<profiles::ProfilesState>,
) -> Result<ServerSyncStatus, String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let configured = config_state.server.is_some();
    let message = (!configured).then(|| "server sync bootstrap is not configured".to_string());
    status_for_profile(&profile_id, configured, message)
}

#[tauri::command]
pub fn connect_server_sync(
    profiles_state: tauri::State<profiles::ProfilesState>,
    endpoint: String,
    token: String,
) -> Result<ServerSyncStatus, String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    store_credentials(&profile_id, &endpoint, &token, "manual-device")?;
    status_for_profile(&profile_id, true, None)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterDeviceRequest<'a> {
    device_id: &'a str,
    display_name: &'a str,
    platform: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterDeviceResponse {
    device_token: String,
}

async fn register_device(
    endpoint: &str,
    user_token: &str,
    device_id: &str,
) -> Result<String, String> {
    let response = reqwest::Client::new()
        .post(format!("{endpoint}/v1/devices"))
        .bearer_auth(user_token)
        .json(&RegisterDeviceRequest {
            device_id,
            display_name: "FocusNook desktop",
            platform: std::env::consts::OS,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("sync server returned {}", response.status()));
    }
    let body = response
        .json::<RegisterDeviceResponse>()
        .await
        .map_err(|e| e.to_string())?;
    normalize_token(&body.device_token)
}

fn ensure_local_device_id(conn: &Connection) -> Result<String, String> {
    sync_log::ensure_device_identity(conn)
}

#[tauri::command]
pub async fn connect_default_server_sync(
    db: tauri::State<'_, crate::db::Db>,
    config_state: tauri::State<'_, config::SyncProvidersConfig>,
    profiles_state: tauri::State<'_, profiles::ProfilesState>,
) -> Result<ServerSyncStatus, String> {
    let bootstrap = config_state
        .server
        .clone()
        .ok_or_else(|| "server sync bootstrap is not configured".to_string())?;
    let endpoint = normalize_endpoint(&bootstrap.endpoint)?;
    let device_id = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        ensure_local_device_id(&conn)?
    };
    let token = register_device(&endpoint, &bootstrap.user_token, &device_id).await?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    store_credentials(&profile_id, &endpoint, &token, &device_id)?;
    status_for_profile(&profile_id, true, None)
}

#[tauri::command]
pub fn disconnect_server_sync(
    profiles_state: tauri::State<profiles::ProfilesState>,
) -> Result<(), String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    delete_credentials(&profile_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_profile_id() -> String {
        format!("server-sync-test-{}", uuid::Uuid::now_v7())
    }

    #[test]
    fn stores_server_credentials_per_profile() -> Result<(), String> {
        let profile_id = unique_profile_id();

        store_credentials(
            &profile_id,
            "https://sync.example.com/",
            "secret-token",
            "device-a",
        )?;

        let status = status_for_profile(&profile_id, true, None)?;
        assert_eq!(
            status,
            ServerSyncStatus {
                available: true,
                connected: true,
                endpoint: Some("https://sync.example.com".to_string()),
                message: None
            }
        );

        delete_credentials(&profile_id)?;
        Ok(())
    }

    #[test]
    fn rejects_non_https_remote_endpoint() {
        let err = normalize_endpoint("http://sync.example.com").err();
        assert_eq!(err, Some("sync server endpoint must use https".to_string()));
    }

    #[test]
    fn allows_local_http_for_development() -> Result<(), String> {
        assert_eq!(
            normalize_endpoint("http://localhost:8080/api/")?,
            "http://localhost:8080/api"
        );
        Ok(())
    }

    #[test]
    fn rejects_localhost_prefix_spoofing() {
        let err = normalize_endpoint("http://localhost.example.com").err();
        assert_eq!(err, Some("sync server endpoint must use https".to_string()));
    }

    #[test]
    fn deleting_missing_credentials_is_ok() -> Result<(), String> {
        delete_credentials(&unique_profile_id())
    }
}
