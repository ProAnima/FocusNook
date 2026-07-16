use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<SecureStorage<R>> {
    let handle =
        api.register_android_plugin("com.proanima.securestorage", "SecureStoragePlugin")?;
    Ok(SecureStorage(handle))
}

pub struct SecureStorage<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> SecureStorage<R> {
    pub fn encrypt(&self, payload: EncryptRequest) -> crate::Result<EncryptResponse> {
        self.0
            .run_mobile_plugin("encrypt", payload)
            .map_err(Into::into)
    }

    pub fn decrypt(&self, payload: DecryptRequest) -> crate::Result<DecryptResponse> {
        self.0
            .run_mobile_plugin("decrypt", payload)
            .map_err(Into::into)
    }
}
