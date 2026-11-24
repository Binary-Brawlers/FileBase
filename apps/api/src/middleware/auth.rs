use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};

use crate::error::ApiError;
use crate::services::jwt::{decode_token, Claims};
use crate::state::AppState;

const TOKEN_COOKIE: &str = "filebase_session";

pub(crate) fn extract_token(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(value) = headers.get(header::AUTHORIZATION) {
        if let Ok(s) = value.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    if let Some(value) = headers.get(header::COOKIE) {
        if let Ok(s) = value.to_str() {
            for part in s.split(';') {
                let part = part.trim();
                if let Some(token) = part.strip_prefix(&format!("{TOKEN_COOKIE}=")) {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

pub struct AuthUser {
    pub claims: Claims,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_token(&parts.headers).ok_or(ApiError::Unauthorized)?;
        let claims = decode_token(&state.config.jwt_secret, &token)?;
        Ok(AuthUser { claims })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    fn headers(name: header::HeaderName, value: &'static str) -> HeaderMap {
        let mut map = HeaderMap::new();
        map.insert(name, HeaderValue::from_static(value));
        map
    }

    #[test]
    fn reads_bearer_token_from_authorization_header() {
        let map = headers(header::AUTHORIZATION, "Bearer abc.def.ghi");
        assert_eq!(extract_token(&map).as_deref(), Some("abc.def.ghi"));
    }

    #[test]
    fn reads_session_cookie_among_other_cookies() {
        let map = headers(
            header::COOKIE,
            "theme=dark; filebase_session=tok123; other=1",
        );
        assert_eq!(extract_token(&map).as_deref(), Some("tok123"));
    }

    #[test]
    fn ignores_non_bearer_schemes_and_missing_credentials() {
        assert_eq!(
            extract_token(&headers(header::AUTHORIZATION, "Basic dXNlcjpwYXNz")),
            None
        );
        assert_eq!(extract_token(&headers(header::COOKIE, "theme=dark")), None);
        assert_eq!(extract_token(&HeaderMap::new()), None);
    }
}
