pub mod auth;
pub mod health;
pub mod projects;
pub mod setup;
pub mod storage_connections;
pub mod upload_presets;

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
        .route(
            "/storage-connections",
            get(storage_connections::list).post(storage_connections::create),
        )
        .route(
            "/storage-connections/:id",
            get(storage_connections::get)
                .patch(storage_connections::update)
                .delete(storage_connections::delete),
        )
        .route(
            "/storage-connections/:id/test",
            post(storage_connections::test),
        )
        .route("/projects", get(projects::list))
        .route("/upload-presets/:id", axum::routing::patch(upload_presets::update))
}

async fn root() -> &'static str {
    "FileBase API"
}
