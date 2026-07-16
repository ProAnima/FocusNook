use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// android_vault_key.rs — а не tauri-plugin-secure-storage — владеет смыслом
// "vault-key": сам плагин ничего не знает про vault-key/SQLCipher/hex-формат,
// он только шифрует/дешифрует произвольный blob под alias'ом (см. комментарий
// в plugins/tauri-plugin-secure-storage/src/lib.rs). Та же граница, что у
// db.rs::vault_key() на десктопе, только вместо keyring::Entry — этот файл.

// Совпадает буквально с KeystoreHelper.KEY_UNAVAILABLE_PREFIX в
// plugins/tauri-plugin-secure-storage/android/.../KeystoreHelper.kt — Tauri
// mobile bridge отдаёт reject(message) только как строку, без структурного
// кода ошибки, так что различать "ключа больше нет" (можно сгенерировать
// новый) от любой другой ошибки дешифровки приходится по префиксу сообщения.
//
// #[allow(dead_code)] на этом и следующих нескольких элементах до
// decode_key_hex — тот же случай, что у oauth.rs::ensure_valid_token и
// sync_log.rs::Hlc::parse: код настоящий и покрыт тестами ниже, просто
// единственный не-тестовый вызывающий код (resolve_for_platform на Android)
// компилируется только под target_os = "android", которого у desktop-сборки
// (и, значит, у cargo clippy/test на этой машине) нет.
#[allow(dead_code)]
const KEY_UNAVAILABLE_PREFIX: &str = "secure-storage:key-unavailable:";

#[allow(dead_code)]
#[derive(Clone, Serialize, Deserialize)]
struct EncryptedBlob {
    ciphertext_base64: String,
    iv_base64: String,
}

#[allow(dead_code)]
trait SecureStorageBackend {
    fn encrypt(&self, alias: &str, plaintext_base64: String) -> Result<EncryptedBlob, String>;
    fn decrypt(&self, alias: &str, blob: &EncryptedBlob) -> Result<String, String>;
}

#[cfg(target_os = "android")]
struct PluginBackend<'a> {
    app: &'a tauri::AppHandle,
}

#[cfg(target_os = "android")]
impl SecureStorageBackend for PluginBackend<'_> {
    fn encrypt(&self, alias: &str, plaintext_base64: String) -> Result<EncryptedBlob, String> {
        use tauri_plugin_secure_storage::{EncryptRequest, SecureStorageExt};
        let response = self
            .app
            .secure_storage()
            .encrypt(EncryptRequest {
                alias: alias.to_string(),
                plaintext_base64,
            })
            .map_err(|e| e.to_string())?;
        Ok(EncryptedBlob {
            ciphertext_base64: response.ciphertext_base64,
            iv_base64: response.iv_base64,
        })
    }

    fn decrypt(&self, alias: &str, blob: &EncryptedBlob) -> Result<String, String> {
        use tauri_plugin_secure_storage::{DecryptRequest, SecureStorageExt};
        let response = self
            .app
            .secure_storage()
            .decrypt(DecryptRequest {
                alias: alias.to_string(),
                ciphertext_base64: blob.ciphertext_base64.clone(),
                iv_base64: blob.iv_base64.clone(),
            })
            .map_err(|e| e.to_string())?;
        Ok(response.plaintext_base64)
    }
}

#[allow(dead_code)]
fn blob_path(data_dir: &Path, keyring_user: &str) -> PathBuf {
    data_dir.join(format!("vault-key-{keyring_user}.enc.json"))
}

// "Ключ отсутствует" (файла нет) обрабатывается сразу здесь, а не как ошибка
// decrypt — регенерация в ответ на реальную ошибку decrypt() разрешена только
// по специфичному KEY_UNAVAILABLE_PREFIX ниже. Android Auto Backup включён по
// умолчанию для этого приложения (см. docs/v1-release-plan.md) — восстановление
// на новом устройстве закономерно попадает в "blob есть, ключа Keystore нет",
// это рутинный путь, не редкий угловой случай, и он не должен молча проглатывать
// баг проводки, который выглядел бы точно так же.
#[allow(dead_code)]
fn resolve_with_backend<B: SecureStorageBackend>(
    backend: &B,
    data_dir: &Path,
    keyring_user: &str,
) -> Result<String, String> {
    let path = blob_path(data_dir, keyring_user);
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            let blob: EncryptedBlob = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
            match backend.decrypt(keyring_user, &blob) {
                Ok(plaintext_base64) => decode_key_hex(&plaintext_base64),
                Err(err) if err.contains(KEY_UNAVAILABLE_PREFIX) => {
                    generate_and_store(backend, &path, keyring_user)
                }
                Err(err) => Err(err),
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            generate_and_store(backend, &path, keyring_user)
        }
        Err(err) => Err(err.to_string()),
    }
}

#[allow(dead_code)]
fn generate_and_store<B: SecureStorageBackend>(
    backend: &B,
    path: &Path,
    keyring_user: &str,
) -> Result<String, String> {
    let key_hex = crate::db::generate_key_hex();
    let blob = backend.encrypt(keyring_user, STANDARD.encode(key_hex.as_bytes()))?;
    std::fs::write(
        path,
        serde_json::to_string(&blob).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(key_hex)
}

#[allow(dead_code)]
fn decode_key_hex(plaintext_base64: &str) -> Result<String, String> {
    let bytes = STANDARD
        .decode(plaintext_base64.trim())
        .map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// Единая точка входа для lib.rs — сигнатура одинаковая на обеих платформах,
/// поэтому вызывающему коду не нужен свой `#[cfg(target_os = "android")]`.
#[cfg(target_os = "android")]
pub fn resolve_for_platform(
    app: &tauri::AppHandle,
    data_dir: &Path,
    keyring_user: &str,
) -> Result<Option<String>, String> {
    let backend = PluginBackend { app };
    resolve_with_backend(&backend, data_dir, keyring_user).map(Some)
}

#[cfg(not(target_os = "android"))]
pub fn resolve_for_platform(
    _app: &tauri::AppHandle,
    _data_dir: &Path,
    _keyring_user: &str,
) -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};

    // AndroidKeyStore не имеет JCA-провайдера вне настоящего Android-рантайма
    // и не может быть вызван из `cargo test` на Windows-хосте вообще — не
    // подмена ради простоты, а единственный возможный вариант здесь (в отличие
    // от sync_tokens.rs, который тестируется против настоящего Windows
    // Credential Manager). Фейк проверяет только логику этого файла:
    // ветвление файл-есть/файл-нет/ключ-недоступен, а не сам Keystore.
    #[derive(Default)]
    struct FakeBackend {
        store: RefCell<HashMap<String, EncryptedBlob>>,
        unavailable: RefCell<HashSet<String>>,
        next_decrypt_error: RefCell<Option<String>>,
    }

    impl FakeBackend {
        fn forget_key(&self, alias: &str) {
            self.unavailable.borrow_mut().insert(alias.to_string());
        }

        fn fail_next_decrypt_with(&self, message: &str) {
            *self.next_decrypt_error.borrow_mut() = Some(message.to_string());
        }
    }

    impl SecureStorageBackend for FakeBackend {
        fn encrypt(&self, alias: &str, plaintext_base64: String) -> Result<EncryptedBlob, String> {
            self.unavailable.borrow_mut().remove(alias);
            let blob = EncryptedBlob {
                ciphertext_base64: plaintext_base64,
                iv_base64: "fake-iv".to_string(),
            };
            self.store
                .borrow_mut()
                .insert(alias.to_string(), blob.clone());
            Ok(blob)
        }

        fn decrypt(&self, alias: &str, _blob: &EncryptedBlob) -> Result<String, String> {
            if let Some(message) = self.next_decrypt_error.borrow_mut().take() {
                return Err(message);
            }
            if self.unavailable.borrow().contains(alias) {
                return Err(format!("{KEY_UNAVAILABLE_PREFIX}no key for alias {alias}"));
            }
            self.store
                .borrow()
                .get(alias)
                .map(|stored| stored.ciphertext_base64.clone())
                .ok_or_else(|| "fake backend: no such alias".to_string())
        }
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "focusnook-android-vault-key-test-{}",
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn file_absent_generates_and_persists_a_fresh_key() {
        let dir = temp_dir();
        let backend = FakeBackend::default();

        let key = resolve_with_backend(&backend, &dir, "vault-key-1").unwrap();

        assert_eq!(key.len(), 64);
        assert!(blob_path(&dir, "vault-key-1").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_present_decrypts_the_same_key_again() {
        let dir = temp_dir();
        let backend = FakeBackend::default();

        let first = resolve_with_backend(&backend, &dir, "vault-key-1").unwrap();
        let second = resolve_with_backend(&backend, &dir, "vault-key-1").unwrap();

        assert_eq!(first, second);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn key_unavailable_signal_regenerates_a_new_key() {
        let dir = temp_dir();
        let backend = FakeBackend::default();
        let first = resolve_with_backend(&backend, &dir, "vault-key-1").unwrap();

        backend.forget_key("vault-key-1");
        let second = resolve_with_backend(&backend, &dir, "vault-key-1").unwrap();

        assert_ne!(first, second);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn other_decrypt_error_hard_fails_without_regenerating() {
        let dir = temp_dir();
        let backend = FakeBackend::default();
        let first = resolve_with_backend(&backend, &dir, "vault-key-1").unwrap();

        backend.fail_next_decrypt_with("plugin wiring bug, not a missing key");
        let result = resolve_with_backend(&backend, &dir, "vault-key-1");
        assert!(result.is_err());

        // The stored blob must be untouched — a bug elsewhere must never look
        // identical to "key genuinely gone" and silently discard a good key.
        let second = resolve_with_backend(&backend, &dir, "vault-key-1").unwrap();
        assert_eq!(first, second);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
