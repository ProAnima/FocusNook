use crate::account_auth::AccountAuthState;
use crate::admin_web::AdminWebState;
use crate::config::Config;
use crate::crypto::CryptoBox;
use crate::error::AppResult;
use crate::sync_events::SyncEventHub;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub account_auth: Arc<AccountAuthState>,
    pub crypto: Arc<CryptoBox>,
    pub pool: PgPool,
    pub sync_events: Arc<SyncEventHub>,
    pub web_admin: Arc<AdminWebState>,
}

impl AppState {
    pub async fn connect(config: Config) -> AppResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(16)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&config.database_url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        let crypto = CryptoBox::new(&config.encryption_key)?;
        Ok(Self {
            config: Arc::new(config),
            account_auth: Arc::new(AccountAuthState::default()),
            crypto: Arc::new(crypto),
            pool,
            sync_events: Arc::new(SyncEventHub::default()),
            web_admin: Arc::new(AdminWebState::default()),
        })
    }
}
