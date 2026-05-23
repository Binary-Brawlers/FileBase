mod app;
mod config;
mod db;
mod entities;
mod error;
mod middleware;
mod routes;
mod services;
mod state;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .init();

    let config = Config::from_env()?;
    let built = app::build(config).await?;

    let listener = TcpListener::bind(built.bind_address).await?;
    tracing::info!(addr = %built.bind_address, "FileBase API listening");

    axum::serve(listener, built.router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
