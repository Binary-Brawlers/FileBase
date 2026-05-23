use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::entities::user;
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::services::{
    jwt::{issue_token, TOKEN_TTL_HOURS},
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
    Json(payload): Json<LoginRequest>,
) -> ApiResult<Response> {
    if payload.email.is_empty() || payload.password.is_empty() {
        return Err(ApiError::Validation("email and password are required".into()));
    }

    let found = user::Entity::find()
        .filter(user::Column::Email.eq(payload.email.to_lowercase()))
        .one(&state.db)
        .await?;

    let user = match found {
        Some(u) if password::verify(&payload.password, &u.password_hash) => u,
        _ => return Err(ApiError::Unauthorized),
    };

    let token = issue_token(&state.config.jwt_secret, &user.id, &user.email)?;
    let body = Json(json!({
        "data": LoginResponse {
            token: token.clone(),
            user: user.into(),
        }
    }));
    let cookie = format!(
        "{TOKEN_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        TOKEN_TTL_HOURS * 3600
    );
    Ok((StatusCode::OK, [(header::SET_COOKIE, cookie)], body).into_response())
}

pub async fn logout() -> Response {
    let cookie = format!("{TOKEN_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response()
}

pub async fn me(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Response> {
    let user = user::Entity::find_by_id(auth.claims.sub.clone())
        .one(&state.db)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(json!({ "data": PublicUser::from(user) })).into_response())
}
