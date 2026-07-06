use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use sha2::{Digest, Sha256};

// Метка формата на диске: без неё нельзя отличить новый зашифрованный файл
// от .webm, записанного до этого фикса (раздел P1 ревью — аудио раньше
// писалось в открытую). Отсутствие метки — не ошибка, а сигнал "это старый
// plaintext-файл", см. decrypt_if_needed.
const MAGIC: &[u8; 4] = b"FNAE";
const NONCE_LEN: usize = 12;

// Аудиофайлы лежат в общей (не per-profile) папке на диске (см. notes.rs),
// поэтому им нужен свой ключ, а не голый vault-ключ профиля напрямую —
// доменное разделение через SHA-256(vault_key || ":audio"), чтобы утечка
// производного аудио-ключа не давала прямого доступа к ключу самой БД.
fn derive_key(vault_key_hex: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(vault_key_hex.as_bytes());
    hasher.update(b":audio");
    hasher.finalize().into()
}

// Формат на диске: [MAGIC (4)][nonce (12)][ciphertext+tag]. Nonce — новый
// случайный на каждый файл (AES-GCM требует уникальности nonce на ключ,
// иначе конфиденциальность всей схемы ломается) — берём из Uuid::new_v4()
// (уже CSPRNG-источник в проекте), не тащим отдельный rand ради 12 байт.
pub fn encrypt(vault_key_hex: &str, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let key = derive_key(vault_key_hex);
    let cipher = Aes256Gcm::new((&key).into());
    let nonce_source = uuid::Uuid::new_v4();
    let nonce_bytes = &nonce_source.as_bytes()[..NONCE_LEN];
    let nonce = Nonce::from_slice(nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("не удалось зашифровать аудиозапись: {e}"))?;

    let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

// Расшифровывает, если файл несёт метку нового формата; иначе возвращает
// байты как есть — так уже существующие (записанные до этого фикса)
// аудиозаметки остаются читаемыми, а не превращаются в "не удалось
// расшифровать" при первом же открытии после обновления.
pub fn decrypt_if_needed(vault_key_hex: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    if !data.starts_with(MAGIC) {
        return Ok(data.to_vec());
    }
    let rest = &data[MAGIC.len()..];
    if rest.len() < NONCE_LEN {
        return Err("повреждённый файл аудиозаписи (нет nonce)".to_string());
    }
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);
    let key = derive_key(vault_key_hex);
    let cipher = Aes256Gcm::new((&key).into());
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "не удалось расшифровать аудиозапись".to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn round_trips_through_encrypt_and_decrypt() {
        let encrypted = encrypt("test-key-hex", b"hello world").unwrap();
        assert_ne!(encrypted, b"hello world");
        assert!(encrypted.starts_with(MAGIC));
        assert_eq!(
            decrypt_if_needed("test-key-hex", &encrypted).unwrap(),
            b"hello world"
        );
    }

    // Раздел P1 ревью: файлы, записанные до этого фикса, — обычный webm без
    // заголовка. Регрессионный тест на то, что они остаются читаемыми.
    #[test]
    fn legacy_plaintext_without_magic_passes_through_unchanged() {
        let legacy_bytes = b"fake webm bytes, no magic header";
        assert_eq!(
            decrypt_if_needed("any-key", legacy_bytes).unwrap(),
            legacy_bytes
        );
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let encrypted = encrypt("key-a", b"secret").unwrap();
        assert!(decrypt_if_needed("key-b", &encrypted).is_err());
    }

    #[test]
    fn tampered_ciphertext_fails_the_auth_tag() {
        let mut encrypted = encrypt("test-key-hex", b"hello world").unwrap();
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xff;
        assert!(decrypt_if_needed("test-key-hex", &encrypted).is_err());
    }

    #[test]
    fn each_encryption_uses_a_fresh_nonce() {
        let a = encrypt("test-key-hex", b"same plaintext").unwrap();
        let b = encrypt("test-key-hex", b"same plaintext").unwrap();
        assert_ne!(
            a, b,
            "одинаковый plaintext не должен давать одинаковый шифртекст"
        );
    }
}
