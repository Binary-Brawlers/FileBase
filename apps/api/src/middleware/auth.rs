use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};

use crate::error::ApiError;
use crate::services::jwt::{decode_token, Claims};
use crate::state::AppState;

const TOKEN_COOKIE: &str = "filebase_session";

fn extract_token(parts: &Parts) -> Option<String> {
    if let Some(value) = parts.headers.get(header::AUTHORIZATION) {
        if let Ok(s) = value.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    if let Some(value) = parts.headers.get(header::COOKIE) {
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

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = extract_token(parts).ok_or(ApiError::Unauthorized)?;
        let claims = decode_token(&state.config.jwt_secret, &token)?;
        Ok(AuthUser { claims })
    }
}
