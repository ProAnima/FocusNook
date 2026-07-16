use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::{models::*, Error};

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<GoogleAuth<R>> {
    Ok(GoogleAuth(app.clone()))
}

pub struct GoogleAuth<R: Runtime>(AppHandle<R>);

impl<R: Runtime> GoogleAuth<R> {
    pub fn connect(&self, _payload: TokenRequest) -> crate::Result<TokenResponse> {
        Err(Error::Unsupported)
    }

    pub fn access_token(&self, _payload: TokenRequest) -> crate::Result<TokenResponse> {
        Err(Error::Unsupported)
    }

    pub fn is_connected(&self) -> crate::Result<ConnectionResponse> {
        Ok(ConnectionResponse {
            connected: false,
            email: None,
        })
    }

    pub fn disconnect(&self) -> crate::Result<()> {
        Ok(())
    }
}
