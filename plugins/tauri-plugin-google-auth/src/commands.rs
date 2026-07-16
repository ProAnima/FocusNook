use tauri::{command, AppHandle, Runtime};

use crate::{models::*, GoogleAuthExt};

#[command]
pub(crate) fn connect<R: Runtime>(
    app: AppHandle<R>,
    payload: TokenRequest,
) -> crate::Result<TokenResponse> {
    app.google_auth().connect(payload)
}

#[command]
pub(crate) fn access_token<R: Runtime>(
    app: AppHandle<R>,
    payload: TokenRequest,
) -> crate::Result<TokenResponse> {
    app.google_auth().access_token(payload)
}

#[command]
pub(crate) fn is_connected<R: Runtime>(app: AppHandle<R>) -> crate::Result<ConnectionResponse> {
    app.google_auth().is_connected()
}

#[command]
pub(crate) fn disconnect<R: Runtime>(app: AppHandle<R>) -> crate::Result<()> {
    app.google_auth().disconnect()
}
