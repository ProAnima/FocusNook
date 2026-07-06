use crate::error::{AppError, AppResult};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;

#[derive(Clone)]
pub struct CryptoBox {
    cipher: Aes256Gcm,
}

pub struct EncryptedBytes {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

impl CryptoBox {
    pub fn new(key: &[u8]) -> AppResult<Self> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|_| AppError::Config("invalid encryption key".to_string()))?;
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> AppResult<EncryptedBytes> {
        let mut nonce = vec![0_u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        let ciphertext = self
            .cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|_| AppError::Internal("encryption failed".to_string()))?;
        Ok(EncryptedBytes { ciphertext, nonce })
    }

    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8]) -> AppResult<Vec<u8>> {
        if nonce.len() != 12 {
            return Err(AppError::Internal("invalid stored nonce".to_string()));
        }
        self.cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| AppError::Internal("decryption failed".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypts_with_a_fresh_nonce_and_round_trips() -> AppResult<()> {
        let key = [7_u8; 32];
        let crypto = CryptoBox::new(&key)?;
        let first = crypto.encrypt(b"payload")?;
        let second = crypto.encrypt(b"payload")?;

        assert_ne!(first.nonce, second.nonce);
        assert_ne!(first.ciphertext, second.ciphertext);
        assert_eq!(crypto.decrypt(&first.ciphertext, &first.nonce)?, b"payload");
        Ok(())
    }
}
