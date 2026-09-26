pub mod analytics;
pub mod api_keys;
pub mod audit_logs;
pub mod auth;
pub mod files;
pub mod folders;
pub mod health;
pub mod operator;
pub mod projects;
pub mod setup;
pub mod storage_connections;
pub mod team;
pub mod transform;
pub mod upload_chunks;
pub mod upload_logs;
pub mod upload_presets;
pub mod uploads;
pub mod webhooks;

use axum::{
    routing::{get, patch, post},
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
        .route("/projects", get(projects::list).post(projects::create))
        .route(
            "/projects/:id",
            get(projects::get)
                .patch(projects::update)
                .delete(projects::delete),
        )
        .route("/projects/:id/members", get(team::list_members))
        .route(
            "/projects/:id/members/:user_id",
            patch(team::update_member).delete(team::remove_member),
        )
        .route(
            "/projects/:id/invitations",
            get(team::list_invitations).post(team::create_invitation),
        )
        .route(
            "/projects/:id/invitations/:invitation_id",
            axum::routing::delete(team::revoke_invitation),
        )
        .route("/team-invitations/accept", post(team::accept_invitation))
        .route("/team-invitations/preview", post(team::preview_invitation))
        .route(
            "/upload-presets",
            get(upload_presets::list).post(upload_presets::create),
        )
        .route(
            "/upload-presets/:id",
            get(upload_presets::get)
                .patch(upload_presets::update)
                .delete(upload_presets::delete),
        )
        .route("/api-keys", get(api_keys::list).post(api_keys::create))
        .route("/api-keys/:id/revoke", patch(api_keys::revoke))
        .route("/analytics/summary", get(analytics::summary))
        .route("/uploads/sign", post(uploads::sign))
        .route("/uploads", post(uploads::direct_upload))
        .route("/uploads/:session_id", post(uploads::session_upload))
        .route(
            "/uploads/:session_id/chunks",
            post(upload_chunks::upload_chunk)
                .get(upload_chunks::list_chunks)
                .delete(upload_chunks::abort_chunks),
        )
        .route(
            "/uploads/:session_id/complete",
            post(upload_chunks::complete_chunked_upload),
        )
        .route("/audit-logs", get(audit_logs::list))
        .route("/transform/:preset/:file_id", get(transform::fetch))
        .route("/admin/diagnostics", get(operator::diagnostics))
        .route("/admin/upgrade-check", get(operator::upgrade_check))
        .route("/admin/jobs/failed", get(operator::failed_jobs))
        .route("/admin/jobs/:job_id/retry", post(operator::retry_job))
        .route(
            "/admin/jobs/:job_id",
            axum::routing::delete(operator::delete_job),
        )
        .route("/admin/maintenance/cleanup", post(operator::cleanup))
        .route("/files", get(files::list))
        .route("/folders", get(folders::list))
        .route("/files/:id", get(files::get).delete(files::delete))
        .route("/files/:id/logs", get(files::logs))
        .route("/upload-logs", get(upload_logs::list))
        .route("/webhooks", get(webhooks::list).post(webhooks::create))
        .route(
            "/webhooks/:id",
            get(webhooks::get)
                .patch(webhooks::update)
                .delete(webhooks::delete),
        )
        .route("/webhooks/:id/deliveries", get(webhooks::deliveries))
}

async fn root() -> &'static str {
    "FileBase API"
}
