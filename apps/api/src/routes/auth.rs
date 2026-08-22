use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::config::DeploymentMode;
use crate::entities::{project, project_member, user};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::services::authorization::ProjectRole;
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

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub project_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub token: String,
    pub user: PublicUser,
    pub project_id: String,
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

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> ApiResult<Response> {
    if state.config.deployment_mode != DeploymentMode::Hosted
        || !state.config.public_registration_enabled
    {
        return Err(ApiError::Forbidden);
    }

    let name = payload.name.trim();
    let email = payload.email.trim().to_lowercase();
    if name.is_empty() {
        return Err(ApiError::Validation("name is required".into()));
    }
    if !email.contains('@') {
        return Err(ApiError::Validation("email is invalid".into()));
    }
    if payload.password.len() < 8 {
        return Err(ApiError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }

    if user::Entity::find()
        .filter(user::Column::Email.eq(email.clone()))
        .one(&state.db)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(
            "an account with this email already exists".into(),
        ));
    }

    let project_name = payload
        .project_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("My Project")
        .to_string();
    let password_hash = password::hash(&payload.password)?;
    let user_id = new_id("usr");
    let project_id = new_id("prj");
    let slug_base = slugify(&project_name);
    let slug = format!(
        "{}-{}",
        if slug_base.is_empty() {
            "project"
        } else {
            &slug_base
        },
        &Uuid::new_v4().simple().to_string()[..8]
    );
    let now = Utc::now().into();
    let txn = state.db.begin().await?;

    let user_insert = user::ActiveModel {
        id: Set(user_id.clone()),
        name: Set(name.to_string()),
        email: Set(email.clone()),
        password_hash: Set(password_hash),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&txn)
    .await;

    if let Err(error) = user_insert {
        txn.rollback().await?;
        if error.to_string().to_ascii_lowercase().contains("unique") {
            return Err(ApiError::Conflict(
                "an account with this email already exists".into(),
            ));
        }
        return Err(ApiError::Database(error));
    }

    project::ActiveModel {
        id: Set(project_id.clone()),
        user_id: Set(user_id.clone()),
        name: Set(project_name),
        slug: Set(slug),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&txn)
    .await?;

    project_member::ActiveModel {
        project_id: Set(project_id.clone()),
        user_id: Set(user_id.clone()),
        role: Set(ProjectRole::Owner.as_str().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&txn)
    .await?;

    txn.commit().await?;

    let token = issue_token(&state.config.jwt_secret, &user_id, &email)?;
    tracing::info!(user_id = %user_id, project_id = %project_id, "audit.auth.registered");
    let body = Json(json!({
        "data": RegisterResponse {
            token: token.clone(),
            user: PublicUser {
                id: user_id,
                name: name.to_string(),
                email,
            },
            project_id,
        }
    }));
    let cookie = session_cookie(&state, Some(&token), TOKEN_TTL_HOURS * 3600);
    Ok((StatusCode::CREATED, [(header::SET_COOKIE, cookie)], body).into_response())
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<Response> {
    if payload.email.is_empty() || payload.password.is_empty() {
        return Err(ApiError::Validation(
            "email and password are required".into(),
        ));
    }

    let found = user::Entity::find()
        .filter(user::Column::Email.eq(payload.email.to_lowercase()))
        .one(&state.db)
        .await?;

    let user = match found {
        Some(u) if password::verify(&payload.password, &u.password_hash) => u,
        _ => {
            tracing::warn!("audit.auth.login_failed");
            return Err(ApiError::Unauthorized);
        }
    };

    let token = issue_token(&state.config.jwt_secret, &user.id, &user.email)?;
    tracing::info!(user_id = %user.id, "audit.auth.login_succeeded");
    let body = Json(json!({
        "data": LoginResponse {
            token: token.clone(),
            user: user.into(),
        }
    }));
    let cookie = session_cookie(&state, Some(&token), TOKEN_TTL_HOURS * 3600);
    Ok((StatusCode::OK, [(header::SET_COOKIE, cookie)], body).into_response())
}

pub async fn logout(State(state): State<AppState>) -> Response {
    tracing::info!("audit.auth.logout");
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

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}

fn slugify(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
