use super::registry::{save, to_response};
use super::{ProfileRecord, ProfilesResponse, ProfilesState};
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

pub(super) fn normalize_email(value: &str) -> Result<String, String> {
    let email = value.trim().to_lowercase();
    if email.len() < 3
        || email.len() > 254
        || !email.contains('@')
        || email.chars().any(char::is_whitespace)
    {
        return Err("invalid account email".to_string());
    }
    Ok(email)
}

pub(super) fn hash_password(password: &str) -> Result<String, String> {
    if password.chars().count() < 10 {
        return Err("account password must contain at least 10 characters".to_string());
    }
    let salt =
        SaltString::encode_b64(uuid::Uuid::now_v7().as_bytes()).map_err(|e| e.to_string())?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| e.to_string())
}

fn password_matches(record: &ProfileRecord, password: &str) -> Result<bool, String> {
    let encoded = record
        .password_hash
        .as_deref()
        .ok_or_else(|| "account is not configured".to_string())?;
    let parsed = PasswordHash::new(encoded).map_err(|e| e.to_string())?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn verify_active_password(state: &ProfilesState, password: &str) -> Result<(), String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    let active_id = file
        .active_profile_id
        .as_deref()
        .ok_or_else(|| "no active account".to_string())?;
    let record = file
        .profiles
        .iter()
        .find(|record| record.id == active_id)
        .ok_or_else(|| "active account not found".to_string())?;
    if password_matches(record, password)? {
        Ok(())
    } else {
        Err("invalid account password".to_string())
    }
}

pub fn verify_account_password(
    state: &ProfilesState,
    profile_id: &str,
    password: &str,
) -> Result<(), String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    let record = file
        .profiles
        .iter()
        .find(|record| record.id == profile_id)
        .ok_or_else(|| "account not found".to_string())?;
    if password_matches(record, password)? {
        Ok(())
    } else {
        Err("invalid account password".to_string())
    }
}

pub fn configure_active_account(
    state: &ProfilesState,
    display_name: &str,
    email: &str,
    password: &str,
) -> Result<ProfilesResponse, String> {
    let normalized_email = normalize_email(email)?;
    let password_hash = hash_password(password)?;
    let name = display_name.trim();
    if name.is_empty() {
        return Err("account name is required".to_string());
    }
    let mut file = state.file.lock().map_err(|e| e.to_string())?;
    let active_id = file
        .active_profile_id
        .clone()
        .ok_or_else(|| "no active account".to_string())?;
    if file.profiles.iter().any(|record| {
        record.id != active_id && record.email.as_deref() == Some(normalized_email.as_str())
    }) {
        return Err("an account with this email already exists on this device".to_string());
    }
    let record = file
        .profiles
        .iter_mut()
        .find(|record| record.id == active_id)
        .ok_or_else(|| "active account not found".to_string())?;
    if record.password_hash.is_some() {
        return Err("account is already configured".to_string());
    }
    record.display_name = name.to_string();
    record.email = Some(normalized_email);
    record.password_hash = Some(password_hash);
    file.session_locked = false;
    save(&state.data_dir, &file)?;
    to_response(&file)
}

pub fn lock_session(state: &ProfilesState) -> Result<ProfilesResponse, String> {
    let mut file = state.file.lock().map_err(|e| e.to_string())?;
    file.session_locked = true;
    save(&state.data_dir, &file)?;
    to_response(&file)
}

pub fn unlock_account(
    state: &ProfilesState,
    profile_id: &str,
    password: &str,
) -> Result<(), String> {
    let mut file = state.file.lock().map_err(|e| e.to_string())?;
    let record = file
        .profiles
        .iter()
        .find(|record| record.id == profile_id)
        .ok_or_else(|| "account not found".to_string())?;
    if !password_matches(record, password)? {
        return Err("invalid account password".to_string());
    }
    file.active_profile_id = Some(profile_id.to_string());
    file.session_locked = false;
    save(&state.data_dir, &file)
}

pub fn set_sync_enabled(state: &ProfilesState, enabled: bool) -> Result<ProfilesResponse, String> {
    let mut file = state.file.lock().map_err(|e| e.to_string())?;
    let active_id = file
        .active_profile_id
        .clone()
        .ok_or_else(|| "no active account".to_string())?;
    let record = file
        .profiles
        .iter_mut()
        .find(|record| record.id == active_id)
        .ok_or_else(|| "active account not found".to_string())?;
    record.sync_enabled = enabled;
    save(&state.data_dir, &file)?;
    to_response(&file)
}

pub fn active_sync_enabled(state: &ProfilesState) -> Result<bool, String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    let active_id = file
        .active_profile_id
        .as_deref()
        .ok_or_else(|| "no active account".to_string())?;
    file.profiles
        .iter()
        .find(|record| record.id == active_id)
        .map(|record| record.sync_enabled)
        .ok_or_else(|| "active account not found".to_string())
}

pub fn active_account_identity(state: &ProfilesState) -> Result<(String, String), String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    let active_id = file
        .active_profile_id
        .as_deref()
        .ok_or_else(|| "no active account".to_string())?;
    let record = file
        .profiles
        .iter()
        .find(|record| record.id == active_id)
        .ok_or_else(|| "active account not found".to_string())?;
    let email = record
        .email
        .clone()
        .ok_or_else(|| "account is not configured".to_string())?;
    Ok((record.display_name.clone(), email))
}
