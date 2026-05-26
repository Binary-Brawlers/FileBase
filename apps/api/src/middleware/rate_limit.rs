use std::time::Duration;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::state::AppState;

const WINDOW: Duration = Duration::from_secs(60);

pub async fn enforce(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    let Some((category, limit)) = rate_limit_for_path(&state, path) else {
        return next.run(request).await;
    };

    let client = client_key(&request);
    let key = format!("{category}:{client}");
    if state.rate_limiter.allow(&key, limit, WINDOW) {
        return next.run(request).await;
    }

    tracing::warn!(category, client = %client, path, "rate limit exceeded");
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "error": {
                "code": "rate_limited",
                "message": "too many requests"
            }
        })),
    )
        .into_response()
}

fn rate_limit_for_path(state: &AppState, path: &str) -> Option<(&'static str, usize)> {
    match path {
        "/auth/login" | "/setup/initialize" => {
            Some(("auth", state.config.auth_rate_limit_per_minute))
        }
        "/uploads" | "/uploads/sign" => Some(("upload", state.config.upload_rate_limit_per_minute)),
        _ if path.starts_with("/uploads/") => {
            Some(("upload", state.config.upload_rate_limit_per_minute))
        }
        _ => None,
    }
}

fn client_key(request: &Request<Body>) -> String {
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
        })
        .unwrap_or("unknown")
        .to_string()
}
