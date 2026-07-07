mod account_auth;
mod admin_web;
mod auth;
mod config;
mod crypto;
mod error;
mod routes;
mod state;
mod sync_events;

use crate::config::Config;
use crate::error::AppResult;
use crate::routes::router;
use crate::state::AppState;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> AppResult<()> {
    if std::env::args().any(|arg| arg == "healthcheck") {
        return healthcheck();
    }

    init_tracing();
    let config = Config::from_env()?;
    let bind_addr = config.bind_addr;
    let state = AppState::connect(config).await?;
    let public_base_url = state.config.public_base_url.clone();
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    tracing::info!(%bind_addr, %public_base_url, "FocusNook sync server listening");
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("focusnook_sync_server=info,tower_http=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .init();
}

fn healthcheck() -> AppResult<()> {
    let config = Config::from_env()?;
    let target = SocketAddr::new("127.0.0.1".parse()?, config.bind_addr.port());
    TcpStream::connect_timeout(&target, Duration::from_secs(2))?;
    Ok(())
}

async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::warn!(%err, "failed to install Ctrl+C shutdown handler");
    }
}
