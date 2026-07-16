use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::SecureStorage;
#[cfg(mobile)]
use mobile::SecureStorage;

pub trait SecureStorageExt<R: Runtime> {
    fn secure_storage(&self) -> &SecureStorage<R>;
}

impl<R: Runtime, T: Manager<R>> crate::SecureStorageExt<R> for T {
    fn secure_storage(&self) -> &SecureStorage<R> {
        self.state::<SecureStorage<R>>().inner()
    }
}

// Нет invoke_handler/commands — это осознанно, не пропуск шаблона. Единственный
// вызывающий код — android_vault_key.rs (доверенный Rust), не webview. Раскрытие
// encrypt/decrypt через invoke() сделало бы hardware-backed ключ decrypt-ораклом
// для любого JS в этом контексте. См. build.rs (COMMANDS = &[]) и то, что в
// capabilities/*.json для этого плагина сознательно нет записи.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("secure-storage")
        .setup(|app, api| {
            #[cfg(mobile)]
            let secure_storage = mobile::init(app, api)?;
            #[cfg(desktop)]
            let secure_storage = desktop::init(app, api)?;
            app.manage(secure_storage);
            Ok(())
        })
        .build()
}
