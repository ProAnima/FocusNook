use super::accounts::{hash_password, normalize_email};
use super::registry::{pick_color, save, vault_filename};
use super::{ProfileDto, ProfileRecord, ProfilesState};
use std::path::PathBuf;

// Запись профиля, ещё не сохранённая в profiles.json — держит вызывающую
// сторону (commands/profiles.rs) от того, чтобы записать профиль до того, как для него
// реально открылся vault. См. prepare_create/commit_create.
pub struct PendingProfile {
    record: ProfileRecord,
}

impl PendingProfile {
    pub fn keyring_user(&self) -> &str {
        &self.record.keyring_user
    }
}

// Готовит запись нового профиля и путь к его vault-файлу, но НЕ пишет в
// profiles.json. Разделение на prepare/commit — то, что закрывает
// non-atomicity: если commit_create вызвать раньше, чем vault реально
// откроется (db::open может упасть на sqlcipher/keyring-ошибке), профиль
// остаётся в списке "осиротевшим" — виден в UI, но переключиться на него
// нельзя, и убрать его тоже нечем (нет команды удаления профиля).
pub fn prepare_create(
    state: &ProfilesState,
    display_name: &str,
    email: &str,
    password: &str,
) -> Result<(PendingProfile, PathBuf), String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    let normalized_email = normalize_email(email)?;
    if file
        .profiles
        .iter()
        .any(|record| record.email.as_deref() == Some(normalized_email.as_str()))
    {
        return Err("an account with this email already exists on this device".to_string());
    }
    let id = uuid::Uuid::now_v7().to_string();
    let record = ProfileRecord {
        id: id.clone(),
        display_name: display_name.to_string(),
        avatar_color: pick_color(file.profiles.len()),
        keyring_user: format!("vault-key-{id}"),
        email: Some(normalized_email),
        password_hash: Some(hash_password(password)?),
        sync_enabled: false,
    };
    let vault_path = state.data_dir.join(vault_filename(&id));
    Ok((PendingProfile { record }, vault_path))
}

// Записывает подготовленный профиль в profiles.json — вызывать только
// после того, как его vault успешно открылся (prepare_create выше).
pub fn commit_create(state: &ProfilesState, pending: PendingProfile) -> Result<ProfileDto, String> {
    let mut file = state.file.lock().map_err(|e| e.to_string())?;
    file.profiles.push(pending.record.clone());
    file.active_profile_id = Some(pending.record.id.clone());
    file.session_locked = false;
    save(&state.data_dir, &file)?;
    Ok(ProfileDto::from(&pending.record))
}
