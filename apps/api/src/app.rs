use std::sync::Arc;

use axum::Router;
use filebase_migration::{Migrator, MigratorTrait};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{PropagateRequestIdLayer, SetRequestIdLayer},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::config::Config;
use crate::db;
use crate::middleware::{MakeUuidRequestId, REQUEST_ID_HEADER};
use crate::routes;
use crate::state::AppState;

pub struct BuiltApp {
    pub router: Router,
    pub bind_address: std::net::SocketAddr,
}

pub async fn build(config: Config) -> anyhow::Result<BuiltApp> {
    let db = db::connect(&config).await?;
    Migrator::up(&db, None).await?;

    let redis = db::redis::connect(&config)?;
    let bind_address = config.bind_address;

    let state = AppState {
        db,
        redis,
        config: Arc::new(config),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(REQUEST_ID_HEADER, MakeUuidRequestId))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER))
        .layer(cors);

    let router = routes::router().layer(middleware).with_state(state);

    Ok(BuiltApp {
        router,
        bind_address,
    })
}
