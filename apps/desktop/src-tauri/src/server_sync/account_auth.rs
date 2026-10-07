use super::account::{ensure_local_device_id, server_sync_status};
use super::credentials::{
    delete_credentials, endpoint_from_config, normalize_token, status_for_profile,
    store_credentials, ServerSyncStatus,
};
use super::engine::spawn_best_effort;
use super::protocol::{
    AccountAuthRequest, AccountAuthResponse, ServerAccountRegistration, ServerAccountSession,
};
use super::transport::http_client;
use super::{HTTP_REQUEST_TIMEOUT, PRIVACY_POLICY_VERSION};
use crate::{blob_crypto, config, profiles};

pub(super) async fn authenticate_account(
    endpoint: &str,
    path: &str,
    email: &str,
    password: &str,
    display_name: Option<&str>,
    device_id: &str,
    privacy_accepted: bool,
) -> Result<(String, ServerAccountSession), String> {
    let response = http_client(HTTP_REQUEST_TIMEOUT)?
        .post(format!("{endpoint}{path}"))
        .json(&AccountAuthRequest {
            email,
            password,
            display_name,
            device_id,
            device_name: device_display_name(),
            platform: std::env::consts::OS,
            privacy_accepted,
            privacy_policy_version: privacy_accepted.then_some(PRIVACY_POLICY_VERSION),
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("sync server returned {}", response.status()));
    }
    let body = response
        .json::<AccountAuthResponse>()
        .await
        .map_err(|e| e.to_string())?;
    Ok((
        normalize_token(&body.device_token)?,
        ServerAccountSession {
            email: body.email,
            display_name: body.display_name,
            user_id: body.user_id,
        },
    ))
}

fn device_display_name() -> &'static str {
    if cfg!(target_os = "android") {
        "FocusNook Android"
    } else {
        "FocusNook desktop"
    }
}

#[tauri::command]
pub async fn set_account_sync_enabled(
    app: tauri::AppHandle,
    db: tauri::State<'_, crate::db::Db>,
    config_state: tauri::State<'_, config::SyncProvidersConfig>,
    profiles_state: tauri::State<'_, profiles::ProfilesState>,
    enabled: bool,
    password: String,
    privacy_accepted: bool,
) -> Result<ServerSyncStatus, String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    if !enabled {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        delete_credentials(Some(&conn), &profile_id)?;
        drop(conn);
        profiles::set_sync_enabled(&profiles_state, false)?;
        return server_sync_status(db, config_state, profiles_state);
    }

    profiles::verify_active_password(&profiles_state, &password)?;
    let (display_name, email) = profiles::active_account_identity(&profiles_state)?;
    let endpoint = endpoint_from_config(&config_state)?;
    let device_id = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        ensure_local_device_id(&conn)?
    };
    let login = authenticate_account(
        &endpoint,
        "/v1/accounts/login",
        &email,
        &password,
        None,
        &device_id,
        false,
    )
    .await;
    let (token, account) = match login {
        Ok(session) => session,
        Err(error) if error.contains("401") && privacy_accepted => {
            authenticate_account(
                &endpoint,
                "/v1/accounts/register",
                &email,
                &password,
                Some(&display_name),
                &device_id,
                true,
            )
            .await?
        }
        Err(error) => return Err(error),
    };
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store_credentials(
        Some(&conn),
        &profile_id,
        &endpoint,
        &token,
        &device_id,
        Some(&account),
        Some(blob_crypto::derive_media_key(&account.email, &password)),
    )?;
    drop(conn);
    profiles::set_sync_enabled(&profiles_state, true)?;
    spawn_best_effort(app);
    server_sync_status(db, config_state, profiles_state)
}

#[tauri::command]
pub async fn register_server_account(
    app: tauri::AppHandle,
    db: tauri::State<'_, crate::db::Db>,
    config_state: tauri::State<'_, config::SyncProvidersConfig>,
    profiles_state: tauri::State<'_, profiles::ProfilesState>,
    request: ServerAccountRegistration,
) -> Result<ServerSyncStatus, String> {
    if !request.privacy_accepted {
        return Err("privacy policy consent is required".to_string());
    }
    let endpoint = endpoint_from_config(&config_state)?;
    let device_id = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        ensure_local_device_id(&conn)?
    };
    let display_name = request.display_name.trim().to_string();
    let display_name_ref = (!display_name.is_empty()).then_some(display_name.as_str());
    let (token, account) = authenticate_account(
        &endpoint,
        "/v1/accounts/register",
        &request.email,
        &request.password,
        display_name_ref,
        &device_id,
        true,
    )
    .await?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store_credentials(
        Some(&conn),
        &profile_id,
        &endpoint,
        &token,
        &device_id,
        Some(&account),
        Some(blob_crypto::derive_media_key(
            &account.email,
            &request.password,
        )),
    )?;
    let status = status_for_profile(Some(&conn), &profile_id, true, None)?;
    drop(conn);
    profiles::set_sync_enabled(&profiles_state, true)?;
    spawn_best_effort(app);
    Ok(status)
}

#[tauri::command]
pub async fn login_server_account(
    app: tauri::AppHandle,
    db: tauri::State<'_, crate::db::Db>,
    config_state: tauri::State<'_, config::SyncProvidersConfig>,
    profiles_state: tauri::State<'_, profiles::ProfilesState>,
    email: String,
    password: String,
) -> Result<ServerSyncStatus, String> {
    let endpoint = endpoint_from_config(&config_state)?;
    let device_id = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        ensure_local_device_id(&conn)?
    };
    let (token, account) = authenticate_account(
        &endpoint,
        "/v1/accounts/login",
        &email,
        &password,
        None,
        &device_id,
        false,
    )
    .await?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    store_credentials(
        Some(&conn),
        &profile_id,
        &endpoint,
        &token,
        &device_id,
        Some(&account),
        Some(blob_crypto::derive_media_key(&account.email, &password)),
    )?;
    let status = status_for_profile(Some(&conn), &profile_id, true, None)?;
    drop(conn);
    profiles::set_sync_enabled(&profiles_state, true)?;
    spawn_best_effort(app);
    Ok(status)
}
