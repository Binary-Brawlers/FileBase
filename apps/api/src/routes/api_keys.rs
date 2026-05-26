use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use rand::{distributions::Alphanumeric, Rng};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::entities::{api_key, project};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    pub project_id: String,
    pub name: String,
    pub mode: KeyMode,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyMode {
    Live,
    Test,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyView {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub prefix: String,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreatedApiKeyView {
    #[serde(flatten)]
    pub key: ApiKeyView,
    pub secret: String,
}

impl From<api_key::Model> for ApiKeyView {
    fn from(m: api_key::Model) -> Self {
        Self {
            id: m.id,
            project_id: m.project_id,
            name: m.name,
            prefix: m.prefix,
            last_used_at: m.last_used_at.map(|v| v.to_rfc3339()),
            created_at: m.created_at.to_rfc3339(),
            revoked_at: m.revoked_at.map(|v| v.to_rfc3339()),
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateRequest>,
) -> ApiResult<Response> {
    ensure_project_owner(&state, &auth.claims.sub, &payload.project_id).await?;
    let name = validate_name(&payload.name)?;
    let secret = generate_key(&payload.mode);
    let prefix = secret.chars().take(16).collect::<String>();
    let hash = hash_key(&secret);
    let now = Utc::now().into();

    let inserted = api_key::ActiveModel {
        id: Set(new_id("key")),
        project_id: Set(payload.project_id),
        name: Set(name),
        key_hash: Set(hash),
        prefix: Set(prefix),
        last_used_at: Set(None),
        created_at: Set(now),
        revoked_at: Set(None),
    }
    .insert(&state.db)
    .await?;

    tracing::info!(
        user_id = %auth.claims.sub,
        project_id = %inserted.project_id,
        api_key_id = %inserted.id,
        api_key_prefix = %inserted.prefix,
        "audit.api_key.created"
    );

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "data": CreatedApiKeyView {
                key: ApiKeyView::from(inserted),
                secret,
            }
        })),
    )
        .into_response())
}

pub async fn list(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Response> {
    let projects = project::Entity::find()
        .filter(project::Column::UserId.eq(auth.claims.sub.clone()))
        .all(&state.db)
        .await?;
    let ids: Vec<String> = projects.into_iter().map(|p| p.id).collect();
    if ids.is_empty() {
        return Ok(Json(json!({ "data": Vec::<ApiKeyView>::new() })).into_response());
    }
    let rows = api_key::Entity::find()
        .filter(api_key::Column::ProjectId.is_in(ids))
        .order_by_desc(api_key::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let view: Vec<ApiKeyView> = rows.into_iter().map(ApiKeyView::from).collect();
    Ok(Json(json!({ "data": view })).into_response())
}

pub async fn revoke(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_owned(&state, &auth.claims.sub, &id).await?;
    let mut active = model.into_active_model();
    active.revoked_at = Set(Some(Utc::now().into()));
    let saved = active.update(&state.db).await?;
    tracing::info!(
        user_id = %auth.claims.sub,
        project_id = %saved.project_id,
        api_key_id = %saved.id,
        api_key_prefix = %saved.prefix,
        "audit.api_key.revoked"
    );
    Ok(Json(json!({ "data": ApiKeyView::from(saved) })).into_response())
}

async fn load_owned(state: &AppState, user_id: &str, id: &str) -> Result<api_key::Model, ApiError> {
    let model = api_key::Entity::find_by_id(id.to_string())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    ensure_project_owner(state, user_id, &model.project_id).await?;
    Ok(model)
}

async fn ensure_project_owner(
    state: &AppState,
    user_id: &str,
    project_id: &str,
) -> Result<(), ApiError> {
    let project = project::Entity::find_by_id(project_id.to_string())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if project.user_id != user_id {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::Validation("API key name is required".into()));
    }
    Ok(name.to_string())
}

fn generate_key(mode: &KeyMode) -> String {
    let prefix = match mode {
        KeyMode::Live => "fb_live_",
        KeyMode::Test => "fb_test_",
    };
    let suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect();
    format!("{prefix}{suffix}")
}

fn hash_key(key: &str) -> String {
    let digest = Sha256::digest(key.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}
