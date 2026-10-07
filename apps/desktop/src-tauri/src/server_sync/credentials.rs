use super::protocol::ServerAccountSession;
use crate::config;
use rusqlite::Connection;
#[cfg(target_os = "android")]
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

#[cfg(not(target_os = "android"))]
const SERVER_SYNC_KEYRING_SERVICE: &str = "com.proanima.focusnook.server-sync";
#[cfg(not(target_os = "android"))]
const SERVER_SYNC_KEY_PREFIX: &str = "vds_server";

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct ServerSyncCredentials {
    #[serde(default)]
    pub(super) account_email: Option<String>,
    #[serde(default)]
    pub(super) account_user_id: Option<String>,
    #[serde(default)]
    pub(super) device_id: String,
    pub(super) endpoint: String,
    #[serde(default)]
    pub(super) display_name: Option<String>,
    #[serde(default)]
    pub(super) media_key: Option<String>,
    pub(super) token: String,
}

#[derive(Clone, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerSyncStatus {
    pub(super) available: bool,
    pub(super) account_email: Option<String>,
    pub(super) account_user_id: Option<String>,
    pub(super) connected: bool,
    pub(super) display_name: Option<String>,
    pub(super) endpoint: Option<String>,
    pub(super) media_ready: bool,
    pub(super) message: Option<String>,
}

#[cfg(not(target_os = "android"))]
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

pub(super) fn normalize_endpoint(raw: &str) -> Result<String, String> {
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

pub(super) fn normalize_token(raw: &str) -> Result<String, String> {
    let token = raw.trim().to_string();
    if token.is_empty() {
        return Err("sync server token is required".to_string());
    }
    if token.len() > 8192 {
        return Err("sync server token is too long".to_string());
    }
    Ok(token)
}

#[cfg(not(target_os = "android"))]
fn entry(profile_id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVER_SYNC_KEYRING_SERVICE, &keyring_user(profile_id))
        .map_err(|e| e.to_string())
}

pub(super) fn store_credentials(
    conn: Option<&Connection>,
    profile_id: &str,
    endpoint: &str,
    token: &str,
    device_id: &str,
    account: Option<&ServerAccountSession>,
    media_key: Option<String>,
) -> Result<(), String> {
    let credentials = ServerSyncCredentials {
        account_email: account.map(|value| value.email.clone()),
        account_user_id: account.map(|value| value.user_id.clone()),
        device_id: normalize_token(device_id)?,
        display_name: account.map(|value| value.display_name.clone()),
        endpoint: normalize_endpoint(endpoint)?,
        media_key,
        token: normalize_token(token)?,
    };
    let raw = serde_json::to_string(&credentials).map_err(|e| e.to_string())?;
    store_credentials_raw(conn, profile_id, &raw)
}

#[cfg(not(target_os = "android"))]
fn store_credentials_raw(
    _conn: Option<&Connection>,
    profile_id: &str,
    raw: &str,
) -> Result<(), String> {
    entry(profile_id)?
        .set_password(raw)
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "android")]
fn store_credentials_raw(
    conn: Option<&Connection>,
    profile_id: &str,
    raw: &str,
) -> Result<(), String> {
    let conn =
        conn.ok_or_else(|| "local database is required for Android server sync".to_string())?;
    conn.execute(
        "INSERT INTO server_sync_credentials (profile_id, raw_json, updated_at)
         VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(profile_id) DO UPDATE SET raw_json = excluded.raw_json, updated_at = datetime('now')",
        params![profile_id, raw],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn load_credentials(
    conn: Option<&Connection>,
    profile_id: &str,
) -> Result<Option<ServerSyncCredentials>, String> {
    let raw = load_credentials_raw(conn, profile_id)?;
    raw.map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
        .transpose()
}

#[cfg(not(target_os = "android"))]
fn load_credentials_raw(
    _conn: Option<&Connection>,
    profile_id: &str,
) -> Result<Option<String>, String> {
    match entry(profile_id)?.get_password() {
        Ok(raw) => Ok(Some(raw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(target_os = "android")]
fn load_credentials_raw(
    conn: Option<&Connection>,
    profile_id: &str,
) -> Result<Option<String>, String> {
    let conn =
        conn.ok_or_else(|| "local database is required for Android server sync".to_string())?;
    conn.query_row(
        "SELECT raw_json FROM server_sync_credentials WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub(super) fn delete_credentials(
    conn: Option<&Connection>,
    profile_id: &str,
) -> Result<(), String> {
    delete_credentials_raw(conn, profile_id)
}

#[cfg(not(target_os = "android"))]
fn delete_credentials_raw(_conn: Option<&Connection>, profile_id: &str) -> Result<(), String> {
    match entry(profile_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(target_os = "android")]
fn delete_credentials_raw(conn: Option<&Connection>, profile_id: &str) -> Result<(), String> {
    let conn =
        conn.ok_or_else(|| "local database is required for Android server sync".to_string())?;
    conn.execute(
        "DELETE FROM server_sync_credentials WHERE profile_id = ?1",
        params![profile_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn status_for_profile(
    conn: Option<&Connection>,
    profile_id: &str,
    configured: bool,
    message: Option<String>,
) -> Result<ServerSyncStatus, String> {
    let credentials = load_credentials(conn, profile_id)?;
    Ok(ServerSyncStatus {
        available: configured,
        account_email: credentials
            .as_ref()
            .and_then(|value| value.account_email.clone()),
        account_user_id: credentials
            .as_ref()
            .and_then(|value| value.account_user_id.clone()),
        connected: credentials.is_some(),
        display_name: credentials
            .as_ref()
            .and_then(|value| value.display_name.clone()),
        media_ready: credentials
            .as_ref()
            .and_then(|value| value.media_key.as_ref())
            .is_some(),
        endpoint: credentials.map(|value| value.endpoint),
        message,
    })
}

pub(super) fn endpoint_from_config(
    config_state: &config::SyncProvidersConfig,
) -> Result<String, String> {
    let endpoint = config_state
        .server
        .as_ref()
        .map(|value| value.endpoint.as_str())
        .unwrap_or(config::DEFAULT_SERVER_ENDPOINT);
    normalize_endpoint(endpoint)
}

pub(super) fn server_profile_id(
    credentials: &ServerSyncCredentials,
    local_profile_id: &str,
) -> String {
    credentials
        .account_user_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(local_profile_id)
        .to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn unique_profile_id() -> String {
        format!("server-sync-test-{}", uuid::Uuid::now_v7())
    }

    #[test]
    fn stores_server_credentials_per_profile() -> Result<(), String> {
        let profile_id = unique_profile_id();

        store_credentials(
            None,
            &profile_id,
            "https://sync.example.com/",
            "secret-token",
            "device-a",
            None,
            None,
        )?;

        let status = status_for_profile(None, &profile_id, true, None)?;
        assert_eq!(
            status,
            ServerSyncStatus {
                available: true,
                account_email: None,
                account_user_id: None,
                connected: true,
                display_name: None,
                endpoint: Some("https://sync.example.com".to_string()),
                media_ready: false,
                message: None
            }
        );

        delete_credentials(None, &profile_id)?;
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
        delete_credentials(None, &unique_profile_id())
    }

    #[test]
    fn account_credentials_use_user_id_as_shared_server_profile() {
        let credentials = ServerSyncCredentials {
            account_email: Some("user@example.com".to_string()),
            account_user_id: Some("account-user-id".to_string()),
            device_id: "desktop-device".to_string(),
            display_name: Some("User".to_string()),
            endpoint: "https://sync.example.com".to_string(),
            media_key: Some("media-key".to_string()),
            token: "token".to_string(),
        };

        assert_eq!(
            server_profile_id(&credentials, "local-profile-id"),
            "account-user-id"
        );
    }
}
