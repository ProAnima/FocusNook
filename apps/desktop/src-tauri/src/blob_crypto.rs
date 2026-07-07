use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use argon2::Argon2;
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 4] = b"FNSB";
const NONCE_LEN: usize = 12;

pub fn derive_media_key(email: &str, password: &str) -> String {
    let normalized_email = email.trim().to_ascii_lowercase();
    let mut salt_hasher = Sha256::new();
    salt_hasher.update(b"focusnook-sync-media-key-salt-v1");
    salt_hasher.update(normalized_email.as_bytes());
    let salt = salt_hasher.finalize();
    let mut key = [0_u8; 32];
    if Argon2::default()
        .hash_password_into(password.as_bytes(), &salt, &mut key)
        .is_ok()
    {
        return hex_encode(&key);
    }

    let mut fallback = Sha256::new();
    fallback.update(b"focusnook-sync-media-key-fallback-v1");
    fallback.update(normalized_email.as_bytes());
    fallback.update([0]);
    fallback.update(password.as_bytes());
    hex_encode(&fallback.finalize())
}

pub fn encrypt(media_key_hex: &str, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let key = key_bytes(media_key_hex);
    let cipher = Aes256Gcm::new((&key).into());
    let nonce_source = uuid::Uuid::new_v4();
    let nonce_bytes = &nonce_source.as_bytes()[..NONCE_LEN];
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(nonce_bytes), plaintext)
        .map_err(|e| format!("failed to encrypt sync blob: {e}"))?;
    let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt(media_key_hex: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    if !data.starts_with(MAGIC) {
        return Err("sync blob has an unknown encryption format".to_string());
    }
    let rest = &data[MAGIC.len()..];
    if rest.len() < NONCE_LEN {
        return Err("sync blob is corrupted: missing nonce".to_string());
    }
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);
    let key = key_bytes(media_key_hex);
    let cipher = Aes256Gcm::new((&key).into());
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| "failed to decrypt sync blob".to_string())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_encode(&Sha256::digest(bytes))
}

fn key_bytes(media_key_hex: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"focusnook-sync-blob-aes-gcm-v1");
    hasher.update(media_key_hex.as_bytes());
    hasher.finalize().into()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn media_key_is_stable_for_email_case_and_password() {
        assert_eq!(
            derive_media_key("USER@example.com", "secret-pass"),
            derive_media_key("user@example.com", "secret-pass")
        );
        assert_ne!(
            derive_media_key("user@example.com", "secret-pass"),
            derive_media_key("user@example.com", "other-pass")
        );
    }

    #[test]
    fn sync_blob_roundtrips_and_is_not_plaintext() {
        let key = derive_media_key("user@example.com", "secret-pass");
        let encrypted = encrypt(&key, b"voice bytes").unwrap();
        assert_ne!(encrypted, b"voice bytes");
        assert_eq!(decrypt(&key, &encrypted).unwrap(), b"voice bytes");
    }

    #[test]
    fn wrong_key_cannot_decrypt_sync_blob() {
        let encrypted =
            encrypt(&derive_media_key("a@example.com", "pass"), b"voice bytes").unwrap();
        assert!(decrypt(&derive_media_key("b@example.com", "pass"), &encrypted).is_err());
    }
}
