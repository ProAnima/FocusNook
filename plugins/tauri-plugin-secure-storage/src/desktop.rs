use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::{models::*, Error};

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<SecureStorage<R>> {
    Ok(SecureStorage(app.clone()))
}

// Десктоп продолжает использовать db.rs::vault_key() + OS keyring — этот
// плагин осмыслен только на Android. Ошибка, а не тихая заглушка: подсунуть
// фейковый результат шифрования на десктопе было бы порчей данных, а не
// мелочью (см. android_vault_key.rs).
pub struct SecureStorage<R: Runtime>(AppHandle<R>);

impl<R: Runtime> SecureStorage<R> {
    pub fn encrypt(&self, _payload: EncryptRequest) -> crate::Result<EncryptResponse> {
        Err(Error::Unsupported)
    }

    pub fn decrypt(&self, _payload: DecryptRequest) -> crate::Result<DecryptResponse> {
        Err(Error::Unsupported)
    }
}
