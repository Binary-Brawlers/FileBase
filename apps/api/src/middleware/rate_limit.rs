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
        "/auth/login"
        | "/setup/initialize"
        | "/team-invitations/accept"
        | "/team-invitations/preview" => Some(("auth", state.config.auth_rate_limit_per_minute)),
        "/uploads" | "/uploads/sign" => Some(("upload", state.config.upload_rate_limit_per_minute)),
        _ if path.starts_with("/uploads/") => {
            Some(("upload", state.config.upload_rate_limit_per_minute))
        }
        _ if path.starts_with("/transform/") => {
            Some(("transform", state.config.transform_rate_limit_per_minute))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_state() -> AppState {
        let mut config = Config::for_tests("postgres://unused", "/tmp/unused");
        config.auth_rate_limit_per_minute = 5;
        config.upload_rate_limit_per_minute = 50;
        config.transform_rate_limit_per_minute = 500;
        AppState::disconnected_for_tests(config)
    }

    #[test]
    fn sensitive_paths_map_to_their_rate_limit_category() {
        let state = test_state();
        assert_eq!(
            rate_limit_for_path(&state, "/auth/login"),
            Some(("auth", 5))
        );
        assert_eq!(
            rate_limit_for_path(&state, "/setup/initialize"),
            Some(("auth", 5))
        );
        assert_eq!(
            rate_limit_for_path(&state, "/team-invitations/accept"),
            Some(("auth", 5))
        );
        assert_eq!(
            rate_limit_for_path(&state, "/uploads"),
            Some(("upload", 50))
        );
        assert_eq!(
            rate_limit_for_path(&state, "/uploads/session_1/chunks"),
            Some(("upload", 50))
        );
        assert_eq!(
            rate_limit_for_path(&state, "/transform/thumb/file_1"),
            Some(("transform", 500))
        );
    }

    #[test]
    fn other_paths_are_not_rate_limited() {
        let state = test_state();
        assert_eq!(rate_limit_for_path(&state, "/files"), None);
        assert_eq!(rate_limit_for_path(&state, "/auth/me"), None);
        assert_eq!(rate_limit_for_path(&state, "/health/live"), None);
    }

    fn request_with_headers(headers: &[(&str, &str)]) -> Request<Body> {
        let mut builder = Request::builder().uri("/auth/login");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        match builder.body(Body::empty()) {
            Ok(request) => request,
            Err(error) => panic!("request build failed: {error}"),
        }
    }

    #[test]
    fn client_key_prefers_first_forwarded_address() {
        let request = request_with_headers(&[
            ("x-forwarded-for", " 203.0.113.9 , 10.0.0.1"),
            ("x-real-ip", "198.51.100.2"),
        ]);
        assert_eq!(client_key(&request), "203.0.113.9");
    }

    #[test]
    fn client_key_falls_back_to_real_ip_then_unknown() {
        let request = request_with_headers(&[("x-real-ip", "198.51.100.2")]);
        assert_eq!(client_key(&request), "198.51.100.2");
        assert_eq!(client_key(&request_with_headers(&[])), "unknown");
    }
}
