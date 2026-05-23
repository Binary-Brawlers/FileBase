pub mod redis;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;

use crate::config::Config;

pub async fn connect(config: &Config) -> anyhow::Result<DatabaseConnection> {
    let mut options = ConnectOptions::new(config.database_url.clone());
    options
        .max_connections(config.database_max_connections)
        .min_connections(config.database_min_connections)
        .connect_timeout(Duration::from_secs(config.database_connect_timeout_seconds))
        .sqlx_logging(false);

    Ok(Database::connect(options).await?)
}
