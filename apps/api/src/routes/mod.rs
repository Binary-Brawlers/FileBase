pub mod auth;
pub mod health;
pub mod setup;

use axum::{
    routing::{get, post},
    Router,
};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .route("/setup/status", get(setup::status))
        .route("/setup/initialize", post(setup::initialize))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
}

async fn root() -> &'static str {
    "FileBase API"
}
