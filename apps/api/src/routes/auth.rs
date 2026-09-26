use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::entities::user;
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::{extract_token, AuthUser};
use crate::services::{
    audit::{self, AuditEvent},
    jwt::{decode_token, issue_token, TOKEN_TTL_HOURS},
    password,
};
use crate::state::AppState;

const TOKEN_COOKIE: &str = "filebase_session";

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: PublicUser,
}

#[derive(Debug, Serialize)]
pub struct PublicUser {
    pub id: String,
    pub name: String,
    pub email: String,
}

impl From<user::Model> for PublicUser {
    fn from(u: user::Model) -> Self {
        Self {
            id: u.id,
            name: u.name,
            email: u.email,
        }
    }
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<Response> {
    let client = audit::client_context(&headers);
    if payload.email.is_empty() || payload.password.is_empty() {
        return Err(ApiError::Validation(
            "email and password are required".into(),
        ));
    }

    let email = payload.email.to_lowercase();
    let found = user::Entity::find()
        .filter(user::Column::Email.eq(email.clone()))
        .one(&state.db)
        .await?;

    let user = match found {
        Some(u) if password::verify(&payload.password, &u.password_hash) => u,
        _ => {
            audit::record_best_effort(
                &state,
                AuditEvent::system("auth.login_failed")
                    .with_status("failure")
                    .with_actor_email(&email)
                    .with_client(&client)
                    .with_metadata(json!({ "email": email })),
            )
            .await;
            return Err(ApiError::Unauthorized);
        }
    };

    let token = issue_token(&state.config.jwt_secret, &user.id, &user.email)?;
    audit::record(
        &state,
        AuditEvent::user_identity(&user.id, &user.email, "auth.login_succeeded")
            .with_client(&client),
    )
    .await?;
    let body = Json(json!({
        "data": LoginResponse {
            token: token.clone(),
            user: user.into(),
        }
    }));
    let cookie = session_cookie(&state, Some(&token), TOKEN_TTL_HOURS * 3600);
    Ok((StatusCode::OK, [(header::SET_COOKIE, cookie)], body).into_response())
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let client = audit::client_context(&headers);
    if let Some(claims) = extract_token(&headers)
        .and_then(|token| decode_token(&state.config.jwt_secret, &token).ok())
    {
        audit::record_best_effort(
            &state,
            AuditEvent::user(&claims, "auth.logout").with_client(&client),
        )
        .await;
    }
    let cookie = session_cookie(&state, None, 0);
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response()
}

pub async fn me(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Response> {
    let user = user::Entity::find_by_id(auth.claims.sub.clone())
        .one(&state.db)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(json!({ "data": PublicUser::from(user) })).into_response())
}

pub(crate) fn session_cookie(state: &AppState, token: Option<&str>, max_age: i64) -> String {
    let value = token.unwrap_or_default();
    let secure = if state.config.app_url.starts_with("https://") {
        "; Secure"
    } else {
        ""
    };
    format!("{TOKEN_COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}")
}
