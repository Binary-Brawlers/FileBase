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
use uuid::Uuid;

use crate::entities::{project, webhook, webhook_delivery_log};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

const ALLOWED_EVENTS: &[&str] = &[
    "file.uploaded",
    "file.deleted",
    "file.duplicate_detected",
    "file.optimized",
    "file.failed",
];

#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    pub project_id: String,
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub secret: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct WebhookView {
    pub id: String,
    pub project_id: String,
    pub url: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreatedWebhookView {
    #[serde(flatten)]
    pub webhook: WebhookView,
    pub signing_secret: String,
}

#[derive(Debug, Serialize)]
pub struct WebhookDeliveryLogView {
    pub id: String,
    pub webhook_id: String,
    pub project_id: String,
    pub file_id: Option<String>,
    pub event: String,
    pub status: String,
    pub attempt: i32,
    pub status_code: Option<i32>,
    pub error: Option<String>,
    pub request: serde_json::Value,
    pub response: serde_json::Value,
    pub created_at: String,
}

impl From<webhook_delivery_log::Model> for WebhookDeliveryLogView {
    fn from(log: webhook_delivery_log::Model) -> Self {
        Self {
            id: log.id,
            webhook_id: log.webhook_id,
            project_id: log.project_id,
            file_id: log.file_id,
            event: log.event,
            status: log.status,
            attempt: log.attempt,
            status_code: log.status_code,
            error: log.error,
            request: log.request_json,
            response: log.response_json,
            created_at: log.created_at.to_rfc3339(),
        }
    }
}

impl From<webhook::Model> for WebhookView {
    fn from(hook: webhook::Model) -> Self {
        let events = hook
            .events
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            id: hook.id,
            project_id: hook.project_id,
            url: hook.url,
            events,
            is_active: hook.is_active,
            created_at: hook.created_at.to_rfc3339(),
            updated_at: hook.updated_at.to_rfc3339(),
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateRequest>,
) -> ApiResult<Response> {
    ensure_project_owner(&state, &auth.claims.sub, &payload.project_id).await?;
    let url = validate_url(&payload.url)?;
    let events = validate_events(payload.events)?;
    let now = Utc::now().into();
    let inserted = webhook::ActiveModel {
        id: Set(new_id("wh")),
        project_id: Set(payload.project_id),
        url: Set(url),
        secret: Set(payload.secret.unwrap_or_else(random_secret)),
        events: Set(json!(events)),
        is_active: Set(payload.is_active.unwrap_or(true)),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&state.db)
    .await?;

    let signing_secret = inserted.secret.clone();
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "data": CreatedWebhookView {
                webhook: WebhookView::from(inserted),
                signing_secret,
            }
        })),
    )
        .into_response())
}

pub async fn list(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Response> {
    let project_ids = owned_project_ids(&state, &auth.claims.sub).await?;
    if project_ids.is_empty() {
        return Ok(Json(json!({ "data": Vec::<WebhookView>::new() })).into_response());
    }
    let rows = webhook::Entity::find()
        .filter(webhook::Column::ProjectId.is_in(project_ids))
        .order_by_desc(webhook::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let view: Vec<WebhookView> = rows.into_iter().map(WebhookView::from).collect();
    Ok(Json(json!({ "data": view })).into_response())
}

pub async fn get(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let row = load_owned(&state, &auth.claims.sub, &id).await?;
    Ok(Json(json!({ "data": WebhookView::from(row) })).into_response())
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateRequest>,
) -> ApiResult<Response> {
    let row = load_owned(&state, &auth.claims.sub, &id).await?;
    let mut active = row.into_active_model();
    if let Some(url) = payload.url {
        active.url = Set(validate_url(&url)?);
    }
    if let Some(events) = payload.events {
        active.events = Set(json!(validate_events(events)?));
    }
    if let Some(secret) = payload.secret {
        active.secret = Set(validate_secret(&secret)?);
    }
    if let Some(is_active) = payload.is_active {
        active.is_active = Set(is_active);
    }
    active.updated_at = Set(Utc::now().into());
    let saved = active.update(&state.db).await?;
    Ok(Json(json!({ "data": WebhookView::from(saved) })).into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let row = load_owned(&state, &auth.claims.sub, &id).await?;
    webhook::Entity::delete_by_id(row.id)
        .exec(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn deliveries(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let row = load_owned(&state, &auth.claims.sub, &id).await?;
    let rows = webhook_delivery_log::Entity::find()
        .filter(webhook_delivery_log::Column::WebhookId.eq(row.id))
        .order_by_desc(webhook_delivery_log::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let view: Vec<WebhookDeliveryLogView> =
        rows.into_iter().map(WebhookDeliveryLogView::from).collect();
    Ok(Json(json!({ "data": view })).into_response())
}

async fn load_owned(state: &AppState, user_id: &str, id: &str) -> Result<webhook::Model, ApiError> {
    let row = webhook::Entity::find_by_id(id.to_string())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    ensure_project_owner(state, user_id, &row.project_id).await?;
    Ok(row)
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

async fn owned_project_ids(state: &AppState, user_id: &str) -> Result<Vec<String>, ApiError> {
    let projects = project::Entity::find()
        .filter(project::Column::UserId.eq(user_id.to_string()))
        .all(&state.db)
        .await?;
    Ok(projects.into_iter().map(|p| p.id).collect())
}

fn validate_url(url: &str) -> Result<String, ApiError> {
    let url = url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(ApiError::Validation(
            "webhook url must be http or https".into(),
        ));
    }
    Ok(url.to_string())
}

fn validate_events(events: Vec<String>) -> Result<Vec<String>, ApiError> {
    if events.is_empty() {
        return Err(ApiError::Validation(
            "at least one webhook event is required".into(),
        ));
    }
    let mut normalized = Vec::new();
    for event in events {
        let event = event.trim().to_string();
        if !ALLOWED_EVENTS.contains(&event.as_str()) {
            return Err(ApiError::Validation(format!(
                "unsupported webhook event: {event}"
            )));
        }
        if !normalized.contains(&event) {
            normalized.push(event);
        }
    }
    Ok(normalized)
}

fn validate_secret(secret: &str) -> Result<String, ApiError> {
    let secret = secret.trim();
    if secret.len() < 16 {
        return Err(ApiError::Validation(
            "webhook secret must be at least 16 characters".into(),
        ));
    }
    Ok(secret.to_string())
}

fn random_secret() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}
