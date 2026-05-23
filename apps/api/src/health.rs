use axum::{extract::State, http::StatusCode, Json};
use sea_orm::{ConnectionTrait, Statement};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
pub struct ReadinessResponse {
    status: &'static str,
    database: &'static str,
    redis: &'static str,
}

pub async fn live() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<ReadinessResponse>) {
    let database_ok = state
        .db
        .query_one(Statement::from_string(
            state.db.get_database_backend(),
            "SELECT 1".to_string(),
        ))
        .await
        .is_ok();

    let redis_ok = match state.redis.get_multiplexed_async_connection().await {
        Ok(mut connection) => {
            let result: redis::RedisResult<String> =
                redis::cmd("PING").query_async(&mut connection).await;
            result.is_ok()
        }
        Err(_) => false,
    };

    let status = if database_ok && redis_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(ReadinessResponse {
            status: if status == StatusCode::OK {
                "ok"
            } else {
                "degraded"
            },
            database: if database_ok { "ok" } else { "unavailable" },
            redis: if redis_ok { "ok" } else { "unavailable" },
        }),
    )
}
