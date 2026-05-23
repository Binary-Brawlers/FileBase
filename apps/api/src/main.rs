mod config;
mod db;
mod health;
mod redis_client;
mod state;

use axum::{routing::get, Router};
use filebase_migration::{Migrator, MigratorTrait};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;

    let db = db::connect(&config).await?;
    Migrator::up(&db, None).await?;

    let redis = redis_client::connect(&config)?;
    let state = AppState { db, redis };

    let app = Router::new()
        .route("/", get(root))
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .with_state(state);

    let listener = TcpListener::bind(config.bind_address).await?;
    tracing::info!(addr = %config.bind_address, "FileBase API listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn root() -> &'static str {
    "FileBase API"
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
