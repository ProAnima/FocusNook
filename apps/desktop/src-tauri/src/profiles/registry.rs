use super::{
    ProfileDto, ProfileRecord, ProfilesFile, ProfilesResponse, ProfilesState, AVATAR_COLORS,
    LEGACY_KEYRING_USER, LEGACY_VAULT_FILENAME, PROFILES_FILENAME,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub(super) fn vault_filename(profile_id: &str) -> String {
    format!("vault-{profile_id}.db")
}

pub(super) fn pick_color(existing_count: usize) -> String {
    AVATAR_COLORS[existing_count % AVATAR_COLORS.len()].to_string()
}

pub(super) fn index_path(data_dir: &Path) -> PathBuf {
    data_dir.join(PROFILES_FILENAME)
}

pub(super) fn save(data_dir: &Path, file: &ProfilesFile) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    fs::write(index_path(data_dir), raw).map_err(|e| e.to_string())
}

// Раздел 15 ТЗ: до этой версии был один общий vault.db без профилей. Если
// он существует — переносим его в первый профиль вместо того, чтобы молча
// завести пустой новый (иначе пользователь решит, что данные пропали).
// Унаследованный профиль нарочно продолжает использовать старое фиксированное
// имя в keychain (LEGACY_KEYRING_USER) — так его уже зашифрованный vault
// остаётся читаемым без пересохранения; только НОВЫЕ профили получают
// keyring-имя на основе своего id.
fn load_or_migrate(data_dir: &Path) -> Result<ProfilesFile, String> {
    let path = index_path(data_dir);
    if path.exists() {
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        return serde_json::from_str(&raw).map_err(|e| e.to_string());
    }

    let legacy_vault = data_dir.join(LEGACY_VAULT_FILENAME);
    let id = uuid::Uuid::now_v7().to_string();
    let keyring_user = if legacy_vault.exists() {
        let new_path = data_dir.join(vault_filename(&id));
        fs::rename(&legacy_vault, &new_path).map_err(|e| e.to_string())?;
        LEGACY_KEYRING_USER.to_string()
    } else {
        format!("vault-key-{id}")
    };

    let file = ProfilesFile {
        profiles: vec![ProfileRecord {
            id: id.clone(),
            display_name: "Профиль".to_string(),
            avatar_color: pick_color(0),
            keyring_user,
            email: None,
            password_hash: None,
            sync_enabled: false,
        }],
        active_profile_id: Some(id),
        session_locked: false,
    };
    save(data_dir, &file)?;
    Ok(file)
}

pub fn init(data_dir: &Path) -> Result<ProfilesState, String> {
    let file = load_or_migrate(data_dir)?;
    Ok(ProfilesState {
        data_dir: data_dir.to_path_buf(),
        file: Mutex::new(file),
    })
}

pub(super) fn to_response(file: &ProfilesFile) -> Result<ProfilesResponse, String> {
    let active_profile_id = file
        .active_profile_id
        .clone()
        .ok_or_else(|| "нет активного профиля".to_string())?;
    Ok(ProfilesResponse {
        profiles: file.profiles.iter().map(ProfileDto::from).collect(),
        active_profile_id,
        session_locked: file.session_locked,
    })
}

pub fn list(state: &ProfilesState) -> Result<ProfilesResponse, String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    to_response(&file)
}

// Раздел 9 ТЗ, Iteration 2: мутирующие команды в commands/ дергают это на каждую
// операцию (нужен profile_id для sync_operations) — отдельная лёгкая функция,
// а не profiles::list(...)?.active_profile_id, чтобы не собирать Vec<ProfileDto>
// всех профилей только ради одного поля.
pub fn active_profile_id(state: &ProfilesState) -> Result<String, String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    file.active_profile_id
        .clone()
        .ok_or_else(|| "нет активного профиля".to_string())
}

pub fn audio_dir(state: &ProfilesState) -> Result<PathBuf, String> {
    let profile_id = active_profile_id(state)?;
    let account_dir = state.data_dir.join("accounts").join(profile_id);
    let target = account_dir.join("audio");
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let legacy = state.data_dir.join("audio");
    let migration_marker = account_dir.join(".legacy-audio-migrated");
    if legacy.exists() && !migration_marker.exists() {
        for entry in fs::read_dir(&legacy).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                continue;
            }
            let destination = target.join(entry.file_name());
            if !destination.exists() {
                fs::copy(entry.path(), destination).map_err(|e| e.to_string())?;
            }
        }
        fs::write(migration_marker, b"1").map_err(|e| e.to_string())?;
    }
    Ok(target)
}

// Общий data_dir приложения (НЕ per-profile) — используется для ресурсов, не
// требующих изоляции по профилю сейчас (пока только audio/, см. notes.rs).
pub fn data_dir(state: &ProfilesState) -> &Path {
    &state.data_dir
}

// Путь до vault-файла и keyring-имя для профиля — используются вызывающей
// стороной (commands/profiles.rs), чтобы открыть/переоткрыть db::Connection.
pub fn vault_location(
    state: &ProfilesState,
    profile_id: &str,
) -> Result<(PathBuf, String), String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    let record = file
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| "профиль не найден".to_string())?;
    Ok((
        state.data_dir.join(vault_filename(&record.id)),
        record.keyring_user.clone(),
    ))
}

pub fn set_active(state: &ProfilesState, profile_id: &str) -> Result<(), String> {
    let mut file = state.file.lock().map_err(|e| e.to_string())?;
    if !file.profiles.iter().any(|p| p.id == profile_id) {
        return Err("профиль не найден".to_string());
    }
    file.active_profile_id = Some(profile_id.to_string());
    save(&state.data_dir, &file)
}
