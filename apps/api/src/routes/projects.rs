use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::entities::{project, project_member};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::services::authorization::{
    accessible_project_ids, project_role, require_project_role, ProjectRole,
};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    pub name: String,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectView {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
    pub role: ProjectRole,
}

impl ProjectView {
    fn from_model(p: project::Model, role: ProjectRole) -> Self {
        Self {
            id: p.id,
            name: p.name,
            slug: p.slug,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
            role,
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateRequest>,
) -> ApiResult<Response> {
    let name = validate_name(&payload.name)?;
    let slug = validate_slug(payload.slug.as_deref().unwrap_or(&name))?;
    ensure_slug_available(&state, &slug, None).await?;

    let now = Utc::now().into();
    let inserted = project::ActiveModel {
        id: Set(new_id("prj")),
        user_id: Set(auth.claims.sub),
        name: Set(name),
        slug: Set(slug),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&state.db)
    .await?;

    project_member::ActiveModel {
        project_id: Set(inserted.id.clone()),
        user_id: Set(inserted.user_id.clone()),
        role: Set(ProjectRole::Owner.as_str().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": ProjectView::from_model(inserted, ProjectRole::Owner) })),
    )
        .into_response())
}

pub async fn list(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Response> {
    let ids = accessible_project_ids(&state, &auth.claims.sub).await?;
    if ids.is_empty() {
        return Ok(Json(json!({ "data": Vec::<ProjectView>::new() })).into_response());
    }
    let rows = project::Entity::find()
        .filter(project::Column::Id.is_in(ids))
        .order_by_asc(project::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let mut view = Vec::with_capacity(rows.len());
    for row in rows {
        let role = project_role(&state, &auth.claims.sub, &row.id).await?;
        view.push(ProjectView::from_model(row, role));
    }
    Ok(Json(json!({ "data": view })).into_response())
}

pub async fn get(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let role = require_project_role(&state, &auth.claims.sub, &id, ProjectRole::Viewer).await?;
    let row = load_project(&state, &id).await?;
    Ok(Json(json!({ "data": ProjectView::from_model(row, role) })).into_response())
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateRequest>,
) -> ApiResult<Response> {
    let role = require_project_role(&state, &auth.claims.sub, &id, ProjectRole::Admin).await?;
    let row = load_project(&state, &id).await?;
    let mut active = row.into_active_model();

    if let Some(name) = payload.name {
        active.name = Set(validate_name(&name)?);
    }
    if let Some(slug) = payload.slug {
        let slug = validate_slug(&slug)?;
        ensure_slug_available(&state, &slug, Some(&id)).await?;
        active.slug = Set(slug);
    }
    active.updated_at = Set(Utc::now().into());

    let saved = active.update(&state.db).await?;
    Ok(Json(json!({ "data": ProjectView::from_model(saved, role) })).into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    require_project_role(&state, &auth.claims.sub, &id, ProjectRole::Owner).await?;
    let row = load_project(&state, &id).await?;
    project::Entity::delete_by_id(row.id)
        .exec(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn load_project(state: &AppState, id: &str) -> Result<project::Model, ApiError> {
    project::Entity::find_by_id(id.to_string())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)
}

async fn ensure_slug_available(
    state: &AppState,
    slug: &str,
    except_id: Option<&str>,
) -> Result<(), ApiError> {
    let existing = project::Entity::find()
        .filter(project::Column::Slug.eq(slug.to_string()))
        .one(&state.db)
        .await?;
    if existing.is_some_and(|p| Some(p.id.as_str()) != except_id) {
        return Err(ApiError::Conflict("project slug already exists".into()));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::Validation("project name is required".into()));
    }
    Ok(name.to_string())
}

fn validate_slug(slug: &str) -> Result<String, ApiError> {
    let slug = slugify(slug);
    if slug.is_empty() {
        return Err(ApiError::Validation("project slug cannot be empty".into()));
    }
    Ok(slug)
}

fn slugify(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}
