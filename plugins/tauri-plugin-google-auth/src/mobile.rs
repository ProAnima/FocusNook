use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<GoogleAuth<R>> {
    let handle = api.register_android_plugin("com.proanima.googleauth", "GoogleAuthPlugin")?;
    Ok(GoogleAuth(handle))
}

pub struct GoogleAuth<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> GoogleAuth<R> {
    pub fn connect(&self, payload: TokenRequest) -> crate::Result<TokenResponse> {
        self.0
            .run_mobile_plugin("connect", payload)
            .map_err(Into::into)
    }

    pub fn access_token(&self, payload: TokenRequest) -> crate::Result<TokenResponse> {
        self.0
            .run_mobile_plugin("accessToken", payload)
            .map_err(Into::into)
    }

    pub fn is_connected(&self) -> crate::Result<ConnectionResponse> {
        self.0
            .run_mobile_plugin("isConnected", ())
            .map_err(Into::into)
    }

    pub fn disconnect(&self) -> crate::Result<()> {
        self.0
            .run_mobile_plugin("disconnect", ())
            .map_err(Into::into)
    }
}
