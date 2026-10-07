use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

mod accounts;
mod create;
mod registry;
#[cfg(test)]
mod tests;

// Пути `crate::profiles::X` остаются прежними — реэкспорт из подмодулей.
pub use accounts::{
    active_account_identity, active_sync_enabled, configure_active_account, lock_session,
    set_sync_enabled, unlock_account, verify_account_password, verify_active_password,
};
pub use create::{commit_create, prepare_create};
pub use registry::{
    active_profile_id, audio_dir, data_dir, init, list, set_active, vault_location,
};

// Раздел 8 ТЗ: UserProfile, урезано под Iteration 1 — locale/timezone/
// activeSyncProviderId/encryptionKeyRef придут вместе с i18n и sync, когда
// появится реальная надобность в них.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileRecord {
    id: String,
    display_name: String,
    avatar_color: String,
    // Раздел 16 ТЗ: "ключ базы не хранится рядом с базой" — здесь хранится
    // не сам ключ, а то, под каким именем его искать в OS keychain (нужно
    // разное имя на профиль, иначе все профили делили бы один ключ).
    keyring_user: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    password_hash: Option<String>,
    #[serde(default)]
    sync_enabled: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDto {
    pub id: String,
    pub display_name: String,
    pub avatar_color: String,
    pub email: Option<String>,
    pub account_configured: bool,
    pub sync_enabled: bool,
}

impl From<&ProfileRecord> for ProfileDto {
    fn from(record: &ProfileRecord) -> Self {
        ProfileDto {
            id: record.id.clone(),
            display_name: record.display_name.clone(),
            avatar_color: record.avatar_color.clone(),
            email: record.email.clone(),
            account_configured: record.email.is_some() && record.password_hash.is_some(),
            sync_enabled: record.sync_enabled,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesResponse {
    pub profiles: Vec<ProfileDto>,
    pub active_profile_id: String,
    pub session_locked: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct ProfilesFile {
    profiles: Vec<ProfileRecord>,
    active_profile_id: Option<String>,
    #[serde(default)]
    session_locked: bool,
}

pub struct ProfilesState {
    data_dir: PathBuf,
    file: Mutex<ProfilesFile>,
}

const PROFILES_FILENAME: &str = "profiles.json";
const LEGACY_VAULT_FILENAME: &str = "vault.db";
const LEGACY_KEYRING_USER: &str = "vault-key";
const AVATAR_COLORS: &[&str] = &["#f2b463", "#7cb9e8", "#a3d9a5", "#e8a3c9", "#c9a3e8"];
