use super::credentials::{
    delete_credentials, endpoint_from_config, load_credentials, normalize_endpoint,
    normalize_token, status_for_profile, store_credentials, ServerSyncStatus,
};
use super::protocol::{DeleteAccountRequest, RegisterDeviceRequest, RegisterDeviceResponse};
use super::transport::http_client;
use super::HTTP_REQUEST_TIMEOUT;
use crate::{config, profiles, sync_log};
use rusqlite::Connection;

pub(super) async fn register_device(
    endpoint: &str,
    user_token: &str,
    device_id: &str,
) -> Result<String, String> {
    let response = http_client(HTTP_REQUEST_TIMEOUT)?
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

pub(crate) fn ensure_local_device_id(conn: &Connection) -> Result<String, String> {
    sync_log::ensure_device_identity(conn)
}

#[tauri::command]
pub fn server_sync_status(
    db: tauri::State<crate::db::Db>,
    config_state: tauri::State<config::SyncProvidersConfig>,
    profiles_state: tauri::State<profiles::ProfilesState>,
) -> Result<ServerSyncStatus, String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let configured = endpoint_from_config(&config_state).is_ok();
    let message = (!configured).then(|| "server sync endpoint is not configured".to_string());
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    status_for_profile(Some(&conn), &profile_id, configured, message)
}

#[tauri::command]
pub fn connect_server_sync(
    db: tauri::State<crate::db::Db>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    endpoint: String,
    token: String,
) -> Result<ServerSyncStatus, String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store_credentials(
        Some(&conn),
        &profile_id,
        &endpoint,
        &token,
        "manual-device",
        None,
        None,
    )?;
    profiles::set_sync_enabled(&profiles_state, true)?;
    status_for_profile(Some(&conn), &profile_id, true, None)
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
    let user_token = bootstrap
        .user_token
        .as_deref()
        .ok_or_else(|| "legacy server sync bootstrap token is not configured".to_string())?;
    let device_id = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        ensure_local_device_id(&conn)?
    };
    let token = register_device(&endpoint, user_token, &device_id).await?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store_credentials(
        Some(&conn),
        &profile_id,
        &endpoint,
        &token,
        &device_id,
        None,
        None,
    )?;
    profiles::set_sync_enabled(&profiles_state, true)?;
    status_for_profile(Some(&conn), &profile_id, true, None)
}

#[tauri::command]
pub fn disconnect_server_sync(
    db: tauri::State<crate::db::Db>,
    profiles_state: tauri::State<profiles::ProfilesState>,
) -> Result<(), String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    delete_credentials(Some(&conn), &profile_id)?;
    drop(conn);
    profiles::set_sync_enabled(&profiles_state, false)?;
    Ok(())
}

#[tauri::command]
pub async fn delete_server_account(
    db: tauri::State<'_, crate::db::Db>,
    profiles_state: tauri::State<'_, profiles::ProfilesState>,
    password: String,
) -> Result<(), String> {
    if password.is_empty() {
        return Err("account password is required".to_string());
    }
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let credentials = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        load_credentials(Some(&conn), &profile_id)?
            .ok_or_else(|| "server account is not connected".to_string())?
    };
    if credentials.account_user_id.is_none() {
        return Err("connected credentials do not belong to an account".to_string());
    }
    let response = http_client(HTTP_REQUEST_TIMEOUT)?
        .delete(format!("{}/v1/accounts", credentials.endpoint))
        .bearer_auth(&credentials.token)
        .json(&DeleteAccountRequest {
            password: &password,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("sync server returned {}", response.status()));
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    delete_credentials(Some(&conn), &profile_id)?;
    drop(conn);
    profiles::set_sync_enabled(&profiles_state, false)?;
    Ok(())
}
