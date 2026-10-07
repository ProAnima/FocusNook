#[cfg(not(target_os = "android"))]
pub(super) const KEYRING_SERVICE: &str = "com.proanima.focusnook";

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// pub(crate), не private: android_vault_key.rs генерирует тем же способом
// ключ, который потом уходит не в OS keyring, а через Keystore-плагин.
pub(crate) fn generate_key_hex() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    hex_encode(&bytes)
}

// Раздел 16 ТЗ: "ключ базы не хранится рядом с базой" — OS keychain (Windows
// Credential Manager / macOS Keychain / Secret Service на Linux). Раздел 15:
// у каждого профиля свой vault и свой ключ — keyring_user разный на профиль,
// keyring_service общий (это просто "неймспейс" приложения в keychain).
//
// ВАЖНО: `keyring` v3 не включает backend платформы по умолчанию — без
// features = ["windows-native"] крейт молча работал с no-op хранилищем
// (set_password не падал, но ничего не сохранял, get_password всегда
// возвращал NoEntry). Ключ пропадал на каждом перезапуске процесса, база
// переставала открываться. Если понадобится macOS/Linux — не забыть
// добавить apple-native / linux-native соответствующим фичам в Cargo.toml.
#[cfg(not(target_os = "android"))]
pub(super) fn vault_key(keyring_user: &str) -> Result<String, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, keyring_user).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(existing) => {
            log::debug!("vault: используем существующий ключ из OS keychain");
            Ok(existing)
        }
        Err(keyring::Error::NoEntry) => {
            log::info!("vault: ключ не найден в OS keychain, генерирую новый");
            let key = generate_key_hex();
            entry.set_password(&key).map_err(|e| e.to_string())?;
            Ok(key)
        }
        Err(e) => {
            log::error!("vault: ошибка чтения OS keychain: {e}");
            Err(e.to_string())
        }
    }
}

// notes.rs шифрует аудиофайлы производным от этого же ключа (см.
// audio_crypto.rs) — им нужен per-profile секрет, но не сам PRAGMA key
// напрямую (доменное разделение). Только десктоп: на Android эквивалентный
// ключ приходит из android_vault_key::resolve_for_platform, не отсюда —
// lib.rs вызывает эту функцию только когда android_key_hex вернул None.
#[cfg(not(target_os = "android"))]
pub fn vault_key_for_audio(keyring_user: &str) -> Result<Option<String>, String> {
    vault_key(keyring_user).map(Some)
}
