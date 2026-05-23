use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::entities::{file, project, storage_connection, upload_log};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::services::storage_factory;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub project_id: Option<String>,
    pub search: Option<String>,
    pub mime_type: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileView {
    pub id: String,
    pub project_id: String,
    pub storage_connection_id: String,
    pub original_name: String,
    pub saved_name: String,
    pub mime_type: String,
    pub extension: String,
    pub size: i64,
    pub hash: String,
    pub folder: String,
    pub path: String,
    pub url: String,
    pub status: String,
    pub duplicate_of_file_id: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct UploadLogView {
    pub id: String,
    pub project_id: String,
    pub file_id: Option<String>,
    pub event: String,
    pub status: String,
    pub message: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

impl From<file::Model> for FileView {
    fn from(m: file::Model) -> Self {
        Self {
            id: m.id,
            project_id: m.project_id,
            storage_connection_id: m.storage_connection_id,
            original_name: m.original_name,
            saved_name: m.saved_name,
            mime_type: m.mime_type,
            extension: m.extension,
            size: m.size,
            hash: m.hash,
            folder: m.folder,
            path: m.path,
            url: m.url,
            status: m.status,
            duplicate_of_file_id: m.duplicate_of_file_id,
            metadata: m.metadata_json,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

impl From<upload_log::Model> for UploadLogView {
    fn from(m: upload_log::Model) -> Self {
        Self {
            id: m.id,
            project_id: m.project_id,
            file_id: m.file_id,
            event: m.event,
            status: m.status,
            message: m.message,
            metadata: m.metadata_json,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListQuery>,
) -> ApiResult<Response> {
    let project_ids = owned_project_ids(&state, &auth.claims.sub).await?;
    if project_ids.is_empty() {
        return Ok(Json(json!({ "data": Vec::<FileView>::new() })).into_response());
    }
    if let Some(project_id) = &query.project_id {
        if !project_ids.iter().any(|id| id == project_id) {
            return Err(ApiError::Forbidden);
        }
    }

    let mut db_query = file::Entity::find()
        .filter(file::Column::ProjectId.is_in(project_ids))
        .order_by_desc(file::Column::CreatedAt);
    if let Some(project_id) = query.project_id {
        db_query = db_query.filter(file::Column::ProjectId.eq(project_id));
    }
    if let Some(mime_type) = query.mime_type.as_deref().filter(|v| !v.is_empty()) {
        db_query = db_query.filter(file::Column::MimeType.eq(mime_type.to_string()));
    }
    if let Some(from) = parse_date_filter(query.from.as_deref())? {
        db_query = db_query.filter(file::Column::CreatedAt.gte(from));
    }
    if let Some(to) = parse_date_filter(query.to.as_deref())? {
        db_query = db_query.filter(file::Column::CreatedAt.lte(to));
    }

    let mut rows = db_query.all(&state.db).await?;
    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        let search = search.to_lowercase();
        rows.retain(|f| {
            f.original_name.to_lowercase().contains(&search)
                || f.saved_name.to_lowercase().contains(&search)
                || f.path.to_lowercase().contains(&search)
        });
    }
    let view: Vec<FileView> = rows.into_iter().map(FileView::from).collect();
    Ok(Json(json!({ "data": view })).into_response())
}

pub async fn get(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_owned_file(&state, &auth.claims.sub, &id).await?;
    Ok(Json(json!({ "data": FileView::from(model) })).into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_owned_file(&state, &auth.claims.sub, &id).await?;
    let connection = storage_connection::Entity::find_by_id(model.storage_connection_id.clone())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    let adapter = storage_factory::build_adapter(&connection, &state.config.encryption_key)?;
    adapter
        .delete(&model.path)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    file::Entity::delete_by_id(model.id).exec(&state.db).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_owned_file(&state, &auth.claims.sub, &id).await?;
    let rows = upload_log::Entity::find()
        .filter(upload_log::Column::FileId.eq(model.id))
        .order_by_desc(upload_log::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let view: Vec<UploadLogView> = rows.into_iter().map(UploadLogView::from).collect();
    Ok(Json(json!({ "data": view })).into_response())
}

async fn load_owned_file(
    state: &AppState,
    user_id: &str,
    id: &str,
) -> Result<file::Model, ApiError> {
    let model = file::Entity::find_by_id(id.to_string())
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

async fn owned_project_ids(state: &AppState, user_id: &str) -> Result<Vec<String>, ApiError> {
    let projects = project::Entity::find()
        .filter(project::Column::UserId.eq(user_id.to_string()))
        .all(&state.db)
        .await?;
    Ok(projects.into_iter().map(|p| p.id).collect())
}

fn parse_date_filter(value: Option<&str>) -> Result<Option<DateTime<Utc>>, ApiError> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let dt = DateTime::parse_from_rfc3339(value)
        .map_err(|_| ApiError::Validation("date filters must be RFC3339 timestamps".into()))?;
    Ok(Some(dt.with_timezone(&Utc)))
}
