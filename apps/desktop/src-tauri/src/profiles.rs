use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDto {
    pub id: String,
    pub display_name: String,
    pub avatar_color: String,
}

impl From<&ProfileRecord> for ProfileDto {
    fn from(record: &ProfileRecord) -> Self {
        ProfileDto {
            id: record.id.clone(),
            display_name: record.display_name.clone(),
            avatar_color: record.avatar_color.clone(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesResponse {
    pub profiles: Vec<ProfileDto>,
    pub active_profile_id: String,
}

#[derive(Serialize, Deserialize, Default)]
struct ProfilesFile {
    profiles: Vec<ProfileRecord>,
    active_profile_id: Option<String>,
}

pub struct ProfilesState {
    data_dir: PathBuf,
    file: Mutex<ProfilesFile>,
}

const PROFILES_FILENAME: &str = "profiles.json";
const LEGACY_VAULT_FILENAME: &str = "vault.db";
const LEGACY_KEYRING_USER: &str = "vault-key";
const AVATAR_COLORS: &[&str] = &["#f2b463", "#7cb9e8", "#a3d9a5", "#e8a3c9", "#c9a3e8"];

fn vault_filename(profile_id: &str) -> String {
    format!("vault-{profile_id}.db")
}

fn pick_color(existing_count: usize) -> String {
    AVATAR_COLORS[existing_count % AVATAR_COLORS.len()].to_string()
}

fn index_path(data_dir: &Path) -> PathBuf {
    data_dir.join(PROFILES_FILENAME)
}

fn save(data_dir: &Path, file: &ProfilesFile) -> Result<(), String> {
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
        }],
        active_profile_id: Some(id),
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

fn to_response(file: &ProfilesFile) -> Result<ProfilesResponse, String> {
    let active_profile_id = file
        .active_profile_id
        .clone()
        .ok_or_else(|| "нет активного профиля".to_string())?;
    Ok(ProfilesResponse {
        profiles: file.profiles.iter().map(ProfileDto::from).collect(),
        active_profile_id,
    })
}

pub fn list(state: &ProfilesState) -> Result<ProfilesResponse, String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    to_response(&file)
}

// Раздел 9 ТЗ, Iteration 2: мутирующие команды в lib.rs дергают это на каждую
// операцию (нужен profile_id для sync_operations) — отдельная лёгкая функция,
// а не profiles::list(...)?.active_profile_id, чтобы не собирать Vec<ProfileDto>
// всех профилей только ради одного поля.
pub fn active_profile_id(state: &ProfilesState) -> Result<String, String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    file.active_profile_id
        .clone()
        .ok_or_else(|| "нет активного профиля".to_string())
}

// Общий data_dir приложения (НЕ per-profile) — используется для ресурсов, не
// требующих изоляции по профилю сейчас (пока только audio/, см. notes.rs).
pub fn data_dir(state: &ProfilesState) -> &Path {
    &state.data_dir
}

// Путь до vault-файла и keyring-имя для профиля — используются вызывающей
// стороной (lib.rs), чтобы открыть/переоткрыть db::Connection.
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

// Запись профиля, ещё не сохранённая в profiles.json — держит вызывающую
// сторону (lib.rs) от того, чтобы записать профиль до того, как для него
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
) -> Result<(PendingProfile, PathBuf), String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::now_v7().to_string();
    let record = ProfileRecord {
        id: id.clone(),
        display_name: display_name.to_string(),
        avatar_color: pick_color(file.profiles.len()),
        keyring_user: format!("vault-key-{id}"),
    };
    let vault_path = state.data_dir.join(vault_filename(&id));
    Ok((PendingProfile { record }, vault_path))
}

// Записывает подготовленный профиль в profiles.json — вызывать только
// после того, как его vault успешно открылся (prepare_create выше).
pub fn commit_create(state: &ProfilesState, pending: PendingProfile) -> Result<ProfileDto, String> {
    let mut file = state.file.lock().map_err(|e| e.to_string())?;
    file.profiles.push(pending.record.clone());
    save(&state.data_dir, &file)?;
    Ok(ProfileDto::from(&pending.record))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("focusnook-profiles-test-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // Тестовый эквивалент старой create(): здесь нет реального vault, который
    // мог бы не открыться, поэтому commit сразу вслед за prepare безопасен.
    fn create_for_test(state: &ProfilesState, display_name: &str) -> ProfileDto {
        let (pending, _vault_path) = prepare_create(state, display_name).unwrap();
        commit_create(state, pending).unwrap()
    }

    #[test]
    fn init_on_a_fresh_dir_creates_one_default_profile() {
        let dir = temp_dir();
        let state = init(&dir).unwrap();
        let response = list(&state).unwrap();

        assert_eq!(response.profiles.len(), 1);
        assert_eq!(response.profiles[0].id, response.active_profile_id);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn create_adds_a_profile_and_cycles_avatar_colors() {
        let dir = temp_dir();
        let state = init(&dir).unwrap();
        let second = create_for_test(&state, "Рабочий");
        let third = create_for_test(&state, "Учёба");

        let response = list(&state).unwrap();
        assert_eq!(response.profiles.len(), 3);
        // Раздел 15 ТЗ: цвета аватаров циклически из фиксированной палитры.
        assert_eq!(second.avatar_color, AVATAR_COLORS[1]);
        assert_eq!(third.avatar_color, AVATAR_COLORS[2]);
        fs::remove_dir_all(&dir).unwrap();
    }

    // Раздел 15 ТЗ + разбор ревью: раньше create() писал профиль в
    // profiles.json ДО того, как вызывающая сторона (lib.rs) успевала
    // открыть его vault — неудачное открытие оставляло в списке
    // "осиротевший" профиль, на который нельзя переключиться и который
    // нечем удалить. prepare_create теперь ничего не пишет на диск;
    // commit_create — единственное, что пишет, и его нужно вызывать явно.
    #[test]
    fn prepare_create_does_not_persist_until_commit_create_is_called() {
        let dir = temp_dir();
        let state = init(&dir).unwrap();
        let before = list(&state).unwrap().profiles.len();

        let (_pending, _vault_path) = prepare_create(&state, "Рабочий").unwrap();
        // Ни commit, ни запись на диск не происходили — как если бы
        // открытие vault упало до вызова commit_create.
        assert_eq!(list(&state).unwrap().profiles.len(), before);
        let raw = fs::read_to_string(index_path(&dir)).unwrap();
        assert!(
            !raw.contains("Рабочий"),
            "профиль не должен попасть на диск без commit_create"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn set_active_switches_the_active_profile() {
        let dir = temp_dir();
        let state = init(&dir).unwrap();
        let second = create_for_test(&state, "Рабочий");

        set_active(&state, &second.id).unwrap();
        assert_eq!(list(&state).unwrap().active_profile_id, second.id);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn set_active_rejects_an_unknown_profile_id() {
        let dir = temp_dir();
        let state = init(&dir).unwrap();
        assert!(set_active(&state, "не-существует").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn vault_location_is_keyed_by_profile_id_and_rejects_unknown_ids() {
        let dir = temp_dir();
        let state = init(&dir).unwrap();
        let active_id = list(&state).unwrap().active_profile_id;

        let (path, keyring_user) = vault_location(&state, &active_id).unwrap();
        assert_eq!(path, dir.join(format!("vault-{active_id}.db")));
        assert_eq!(keyring_user, format!("vault-key-{active_id}"));
        assert!(vault_location(&state, "не-существует").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    // Раздел 15 ТЗ: старый однопрофильный vault.db должен стать первым
    // профилем, а не молча потеряться — именно это раньше уже один раз
    // вызывало ложную тревогу при ручном тестировании (см. архитектурный
    // документ). Дальше — регрессионный тест на тот самый сценарий.
    #[test]
    fn legacy_vault_is_migrated_into_the_first_profile() {
        let dir = temp_dir();
        let legacy_path = dir.join(LEGACY_VAULT_FILENAME);
        fs::write(&legacy_path, b"fake sqlcipher bytes").unwrap();

        let state = init(&dir).unwrap();
        let response = list(&state).unwrap();

        assert_eq!(response.profiles.len(), 1);
        assert!(
            !legacy_path.exists(),
            "старый vault.db должен быть переименован, не скопирован"
        );

        let (new_path, keyring_user) = vault_location(&state, &response.active_profile_id).unwrap();
        assert!(new_path.exists());
        assert_eq!(fs::read(&new_path).unwrap(), b"fake sqlcipher bytes");
        assert_eq!(keyring_user, LEGACY_KEYRING_USER);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn new_profiles_get_a_per_profile_keyring_user_not_the_legacy_one() {
        let dir = temp_dir();
        fs::write(dir.join(LEGACY_VAULT_FILENAME), b"fake").unwrap();
        let state = init(&dir).unwrap();

        let second = create_for_test(&state, "Рабочий");
        let (_, keyring_user) = vault_location(&state, &second.id).unwrap();
        assert_ne!(keyring_user, LEGACY_KEYRING_USER);
        assert_eq!(keyring_user, format!("vault-key-{}", second.id));
        fs::remove_dir_all(&dir).unwrap();
    }

    // Повторный запуск (перезапуск приложения) должен читать уже
    // сохранённый profiles.json, а не мигрировать/создавать профиль заново.
    #[test]
    fn second_init_reads_existing_state_without_duplicating_profiles() {
        let dir = temp_dir();
        let first_run = init(&dir).unwrap();
        let original_id = list(&first_run).unwrap().active_profile_id;

        let second_run = init(&dir).unwrap();
        let response = list(&second_run).unwrap();
        assert_eq!(response.profiles.len(), 1);
        assert_eq!(response.active_profile_id, original_id);
        fs::remove_dir_all(&dir).unwrap();
    }
}
