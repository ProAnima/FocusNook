#![allow(clippy::unwrap_used)]
use super::registry::index_path;
use super::*;
use std::fs;
use std::path::PathBuf;

fn temp_dir() -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("focusnook-profiles-test-{}", uuid::Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

// Тестовый эквивалент старой create(): здесь нет реального vault, который
// мог бы не открыться, поэтому commit сразу вслед за prepare безопасен.
fn create_for_test(state: &ProfilesState, display_name: &str) -> ProfileDto {
    let email = format!("{}@example.test", uuid::Uuid::now_v7());
    let (pending, _vault_path) =
        prepare_create(state, display_name, &email, "test-password").unwrap();
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
// profiles.json ДО того, как вызывающая сторона (commands/profiles.rs) успевала
// открыть его vault — неудачное открытие оставляло в списке
// "осиротевший" профиль, на который нельзя переключиться и который
// нечем удалить. prepare_create теперь ничего не пишет на диск;
// commit_create — единственное, что пишет, и его нужно вызывать явно.
#[test]
fn prepare_create_does_not_persist_until_commit_create_is_called() {
    let dir = temp_dir();
    let state = init(&dir).unwrap();
    let before = list(&state).unwrap().profiles.len();

    let (_pending, _vault_path) =
        prepare_create(&state, "Рабочий", "worker@example.test", "test-password").unwrap();
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

#[test]
fn legacy_profile_can_be_upgraded_to_a_password_protected_account() {
    let dir = temp_dir();
    let state = init(&dir).unwrap();
    let response =
        configure_active_account(&state, "Ян", " I@Example.Test ", "StrongPass123").unwrap();

    assert!(response.profiles[0].account_configured);
    assert_eq!(
        response.profiles[0].email.as_deref(),
        Some("i@example.test")
    );
    assert!(verify_active_password(&state, "StrongPass123").is_ok());
    assert!(verify_active_password(&state, "wrong-password").is_err());
    let raw = fs::read_to_string(index_path(&dir)).unwrap();
    assert!(!raw.contains("StrongPass123"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn logout_locks_the_session_without_removing_accounts() {
    let dir = temp_dir();
    let state = init(&dir).unwrap();
    configure_active_account(&state, "Ян", "i@example.test", "StrongPass123").unwrap();
    let locked = lock_session(&state).unwrap();
    assert!(locked.session_locked);
    assert_eq!(locked.profiles.len(), 1);
    unlock_account(&state, &locked.active_profile_id, "StrongPass123").unwrap();
    assert!(!list(&state).unwrap().session_locked);
    fs::remove_dir_all(&dir).unwrap();
}
