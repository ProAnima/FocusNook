use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::GoogleAuth;
#[cfg(mobile)]
use mobile::GoogleAuth;

pub trait GoogleAuthExt<R: Runtime> {
    fn google_auth(&self) -> &GoogleAuth<R>;
}

impl<R: Runtime, T: Manager<R>> crate::GoogleAuthExt<R> for T {
    fn google_auth(&self) -> &GoogleAuth<R> {
        self.state::<GoogleAuth<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("google-auth")
        .invoke_handler(tauri::generate_handler![
            commands::connect,
            commands::access_token,
            commands::is_connected,
            commands::disconnect,
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let google_auth = mobile::init(app, api)?;
            #[cfg(desktop)]
            let google_auth = desktop::init(app, api)?;
            app.manage(google_auth);
            Ok(())
        })
        .build()
}
