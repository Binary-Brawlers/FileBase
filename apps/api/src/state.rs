use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub redis: redis::Client,
    pub config: Arc<Config>,
}
