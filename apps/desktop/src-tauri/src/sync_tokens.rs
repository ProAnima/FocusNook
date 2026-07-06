use crate::oauth::ProviderId;

// Раздел 16 ТЗ: тот же паттерн keyring::Entry, что уже в db.rs для ключа
// vault (тот же windows-native feature, уже включённый в Cargo.toml), но
// отдельный KEYRING_SERVICE — не тот же, что у ключа шифрования БД, чтобы
// будущее "отключить sync для профиля" не могло случайно задеть пространство
// имён ключа vault через общий неймспейс в OS keychain.
const SYNC_KEYRING_SERVICE: &str = "com.proanima.focusnook.sync";

fn keyring_user(profile_id: &str, provider: ProviderId) -> String {
    format!("{}-{profile_id}", provider.keyring_prefix())
}

// Хранится только refresh-токен, не access-токен: тот короткоживущий и
// пересоздаётся по требованию (см. oauth.rs::ensure_valid_token) — вторая
// копия чувствительных данных без выгоды хранить не стоит.
pub fn store_refresh_token(
    profile_id: &str,
    provider: ProviderId,
    refresh_token: &str,
) -> Result<(), String> {
    let entry = keyring::Entry::new(SYNC_KEYRING_SERVICE, &keyring_user(profile_id, provider))
        .map_err(|e| e.to_string())?;
    entry.set_password(refresh_token).map_err(|e| e.to_string())
}

// Ok(None) — "профиль ещё не подключал этот провайдер", нормальное состояние,
// не ошибка. Err только для настоящих сбоев доступа к keychain.
pub fn load_refresh_token(
    profile_id: &str,
    provider: ProviderId,
) -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(SYNC_KEYRING_SERVICE, &keyring_user(profile_id, provider))
        .map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn delete_refresh_token(profile_id: &str, provider: ProviderId) -> Result<(), String> {
    let entry = keyring::Entry::new(SYNC_KEYRING_SERVICE, &keyring_user(profile_id, provider))
        .map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    // db.rs::vault_key (единственный до этого keyring-код в проекте) не имел
    // ни одного прямого юнит-теста — только ручная проверка через cmdkey (см.
    // архитектурный документ). Этот модуль закрывает тот пробел здесь, а не
    // повторяет его молча. Реальный Windows Credential Manager, не мок —
    // медленнее и с другими режимами сбоя, чем in-memory тесты остального
    // проекта, но это и есть код, который реально исполняется в проде.
    fn unique_profile_id() -> String {
        format!("sync-tokens-test-{}", uuid::Uuid::now_v7())
    }

    #[test]
    fn round_trips_a_stored_token() {
        let profile_id = unique_profile_id();
        store_refresh_token(&profile_id, ProviderId::GoogleDrive, "refresh-abc").unwrap();

        assert_eq!(
            load_refresh_token(&profile_id, ProviderId::GoogleDrive).unwrap(),
            Some("refresh-abc".to_string())
        );

        delete_refresh_token(&profile_id, ProviderId::GoogleDrive).unwrap();
    }

    #[test]
    fn missing_token_is_ok_none_not_an_error() {
        let profile_id = unique_profile_id();
        assert_eq!(
            load_refresh_token(&profile_id, ProviderId::YandexDisk).unwrap(),
            None
        );
    }

    #[test]
    fn delete_then_load_returns_none_again() {
        let profile_id = unique_profile_id();
        store_refresh_token(&profile_id, ProviderId::YandexDisk, "refresh-xyz").unwrap();
        delete_refresh_token(&profile_id, ProviderId::YandexDisk).unwrap();
        assert_eq!(
            load_refresh_token(&profile_id, ProviderId::YandexDisk).unwrap(),
            None
        );
    }

    #[test]
    fn deleting_an_absent_entry_does_not_error() {
        let profile_id = unique_profile_id();
        assert!(delete_refresh_token(&profile_id, ProviderId::GoogleDrive).is_ok());
    }

    #[test]
    fn google_and_yandex_entries_for_the_same_profile_are_independent() {
        let profile_id = unique_profile_id();
        store_refresh_token(&profile_id, ProviderId::GoogleDrive, "google-token").unwrap();
        store_refresh_token(&profile_id, ProviderId::YandexDisk, "yandex-token").unwrap();

        assert_eq!(
            load_refresh_token(&profile_id, ProviderId::GoogleDrive).unwrap(),
            Some("google-token".to_string())
        );
        assert_eq!(
            load_refresh_token(&profile_id, ProviderId::YandexDisk).unwrap(),
            Some("yandex-token".to_string())
        );

        delete_refresh_token(&profile_id, ProviderId::GoogleDrive).unwrap();
        delete_refresh_token(&profile_id, ProviderId::YandexDisk).unwrap();
    }
}
