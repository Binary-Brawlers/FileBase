use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use filebase_image_processing::validate_transformations_json;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use uuid::Uuid;

use crate::entities::{project, storage_connection, upload_preset};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    pub project_id: String,
    pub storage_connection_id: Option<String>,
    pub name: String,
    pub folder: String,
    pub allowed_mime_types: Option<Vec<String>>,
    pub allowed_extensions: Option<Vec<String>>,
    pub max_file_size: i64,
    pub duplicate_strategy: String,
    pub filename_strategy: String,
    pub transformations: Option<JsonValue>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub storage_connection_id: Option<String>,
    pub name: Option<String>,
    pub folder: Option<String>,
    pub allowed_mime_types: Option<Vec<String>>,
    pub allowed_extensions: Option<Vec<String>>,
    pub max_file_size: Option<i64>,
    pub duplicate_strategy: Option<String>,
    pub filename_strategy: Option<String>,
    pub transformations: Option<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct PresetView {
    pub id: String,
    pub project_id: String,
    pub storage_connection_id: Option<String>,
    pub name: String,
    pub folder: String,
    pub allowed_mime_types: JsonValue,
    pub allowed_extensions: JsonValue,
    pub max_file_size: i64,
    pub duplicate_strategy: String,
    pub filename_strategy: String,
    pub transformations: JsonValue,
    pub created_at: String,
    pub updated_at: String,
}

impl From<upload_preset::Model> for PresetView {
    fn from(m: upload_preset::Model) -> Self {
        Self {
            id: m.id,
            project_id: m.project_id,
            storage_connection_id: m.storage_connection_id,
            name: m.name,
            folder: m.folder,
            allowed_mime_types: m.allowed_mime_types,
            allowed_extensions: m.allowed_extensions,
            max_file_size: m.max_file_size,
            duplicate_strategy: m.duplicate_strategy,
            filename_strategy: m.filename_strategy,
            transformations: m.transformations_json,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateRequest>,
) -> ApiResult<Response> {
    ensure_project_owner(&state, &auth.claims.sub, &payload.project_id).await?;
    validate_storage_connection(
        &state,
        &payload.project_id,
        payload.storage_connection_id.as_deref(),
    )
    .await?;
    let values = validate_preset_values(PresetValues {
        name: payload.name,
        folder: payload.folder,
        allowed_mime_types: payload.allowed_mime_types.unwrap_or_default(),
        allowed_extensions: payload.allowed_extensions.unwrap_or_default(),
        max_file_size: payload.max_file_size,
        duplicate_strategy: payload.duplicate_strategy,
        filename_strategy: payload.filename_strategy,
        transformations: payload.transformations.unwrap_or_else(|| json!({})),
    })?;

    let now = Utc::now().into();
    let inserted = upload_preset::ActiveModel {
        id: Set(new_id("ups")),
        project_id: Set(payload.project_id),
        storage_connection_id: Set(payload.storage_connection_id),
        name: Set(values.name),
        folder: Set(values.folder),
        allowed_mime_types: Set(json!(values.allowed_mime_types)),
        allowed_extensions: Set(json!(values.allowed_extensions)),
        max_file_size: Set(values.max_file_size),
        duplicate_strategy: Set(values.duplicate_strategy),
        filename_strategy: Set(values.filename_strategy),
        transformations_json: Set(values.transformations),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": PresetView::from(inserted) })),
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
        return Ok(Json(json!({ "data": Vec::<PresetView>::new() })).into_response());
    }
    let rows = upload_preset::Entity::find()
        .filter(upload_preset::Column::ProjectId.is_in(ids))
        .order_by_asc(upload_preset::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let view: Vec<PresetView> = rows.into_iter().map(PresetView::from).collect();
    Ok(Json(json!({ "data": view })).into_response())
}

pub async fn get(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_owned(&state, &auth.claims.sub, &id).await?;
    Ok(Json(json!({ "data": PresetView::from(model) })).into_response())
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateRequest>,
) -> ApiResult<Response> {
    let model = load_owned(&state, &auth.claims.sub, &id).await?;
    let project_id = model.project_id.clone();

    let mut active = model.into_active_model();

    if let Some(connection_id) = payload.storage_connection_id {
        validate_storage_connection(&state, &project_id, Some(&connection_id)).await?;
        active.storage_connection_id = Set(Some(connection_id));
    }
    if let Some(name) = payload.name {
        active.name = Set(validate_name(&name)?);
    }
    if let Some(folder) = payload.folder {
        active.folder = Set(validate_folder(&folder)?);
    }
    if let Some(mime_types) = payload.allowed_mime_types {
        active.allowed_mime_types = Set(json!(validate_mime_types(mime_types)?));
    }
    if let Some(extensions) = payload.allowed_extensions {
        active.allowed_extensions = Set(json!(validate_extensions(extensions)?));
    }
    if let Some(max_file_size) = payload.max_file_size {
        active.max_file_size = Set(validate_max_file_size(max_file_size)?);
    }
    if let Some(strategy) = payload.duplicate_strategy {
        active.duplicate_strategy = Set(validate_duplicate_strategy(&strategy)?);
    }
    if let Some(strategy) = payload.filename_strategy {
        active.filename_strategy = Set(validate_filename_strategy(&strategy)?);
    }
    if let Some(transformations) = payload.transformations {
        active.transformations_json = Set(validate_transformations(transformations)?);
    }

    active.updated_at = Set(Utc::now().into());
    let saved = active.update(&state.db).await?;
    Ok(Json(json!({ "data": PresetView::from(saved) })).into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_owned(&state, &auth.claims.sub, &id).await?;
    upload_preset::Entity::delete_by_id(model.id)
        .exec(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
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

async fn load_owned(
    state: &AppState,
    user_id: &str,
    id: &str,
) -> Result<upload_preset::Model, ApiError> {
    let model = upload_preset::Entity::find_by_id(id.to_string())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    ensure_project_owner(state, user_id, &model.project_id).await?;
    Ok(model)
}

async fn validate_storage_connection(
    state: &AppState,
    project_id: &str,
    connection_id: Option<&str>,
) -> Result<(), ApiError> {
    let Some(connection_id) = connection_id else {
        return Ok(());
    };
    let target = storage_connection::Entity::find_by_id(connection_id.to_string())
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::Validation("storage connection not found".into()))?;
    if target.project_id != project_id {
        return Err(ApiError::Validation(
            "storage connection belongs to a different project".into(),
        ));
    }
    Ok(())
}

struct PresetValues {
    name: String,
    folder: String,
    allowed_mime_types: Vec<String>,
    allowed_extensions: Vec<String>,
    max_file_size: i64,
    duplicate_strategy: String,
    filename_strategy: String,
    transformations: JsonValue,
}

fn validate_preset_values(values: PresetValues) -> Result<PresetValues, ApiError> {
    Ok(PresetValues {
        name: validate_name(&values.name)?,
        folder: validate_folder(&values.folder)?,
        allowed_mime_types: validate_mime_types(values.allowed_mime_types)?,
        allowed_extensions: validate_extensions(values.allowed_extensions)?,
        max_file_size: validate_max_file_size(values.max_file_size)?,
        duplicate_strategy: validate_duplicate_strategy(&values.duplicate_strategy)?,
        filename_strategy: validate_filename_strategy(&values.filename_strategy)?,
        transformations: validate_transformations(values.transformations)?,
    })
}

fn validate_name(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::Validation("preset name is required".into()));
    }
    Ok(name.to_string())
}

fn validate_folder(folder: &str) -> Result<String, ApiError> {
    let folder = folder.trim().trim_matches('/');
    if folder.is_empty() || folder.contains("..") || folder.starts_with('.') {
        return Err(ApiError::Validation(
            "folder must be a safe relative path".into(),
        ));
    }
    Ok(folder.to_string())
}

fn validate_string_list(values: Vec<String>, field: &str) -> Result<Vec<String>, ApiError> {
    let mut out = Vec::new();
    for value in values {
        let value = value.trim().to_lowercase();
        if value.is_empty() {
            return Err(ApiError::Validation(format!(
                "{field} cannot contain empty values"
            )));
        }
        out.push(value);
    }
    Ok(out)
}

fn validate_mime_types(values: Vec<String>) -> Result<Vec<String>, ApiError> {
    let values = validate_string_list(values, "allowed_mime_types")?;
    for value in &values {
        let parts: Vec<&str> = value.split('/').collect();
        if parts.len() != 2
            || parts
                .iter()
                .any(|part| part.is_empty() || part.contains(' '))
        {
            return Err(ApiError::Validation(
                "allowed_mime_types must contain valid MIME types".into(),
            ));
        }
    }
    Ok(values)
}

fn validate_extensions(values: Vec<String>) -> Result<Vec<String>, ApiError> {
    let values = validate_string_list(values, "allowed_extensions")?;
    for value in &values {
        let ext = value.strip_prefix('.').unwrap_or(value);
        if ext.is_empty() || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(ApiError::Validation(
                "allowed_extensions must contain simple extensions".into(),
            ));
        }
    }
    Ok(values)
}

fn validate_max_file_size(size: i64) -> Result<i64, ApiError> {
    if size <= 0 {
        return Err(ApiError::Validation(
            "max_file_size must be greater than zero".into(),
        ));
    }
    Ok(size)
}

fn validate_duplicate_strategy(strategy: &str) -> Result<String, ApiError> {
    match strategy {
        "return_existing" | "upload_new_copy" | "reject_duplicate" => Ok(strategy.to_string()),
        other => Err(ApiError::Validation(format!(
            "invalid duplicate_strategy: {other}"
        ))),
    }
}

fn validate_filename_strategy(strategy: &str) -> Result<String, ApiError> {
    match strategy {
        "slug_random" | "uuid" | "hash" | "timestamp_random" | "random" | "original_suffix" => {
            Ok(strategy.to_string())
        }
        other => Err(ApiError::Validation(format!(
            "invalid filename_strategy: {other}"
        ))),
    }
}

fn validate_transformations(value: JsonValue) -> Result<JsonValue, ApiError> {
    if !value.is_object() {
        return Err(ApiError::Validation(
            "transformations must be an object".into(),
        ));
    }
    validate_transformations_json(&value).map_err(|e| ApiError::Validation(e.to_string()))?;
    Ok(value)
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}
