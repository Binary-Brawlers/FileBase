use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::entities::{file, storage_connection, upload_log};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::routes::upload_logs::{self, UploadLogView};
use crate::services::audit::{self, AuditEvent};
use crate::services::authorization::{accessible_project_ids, require_project_role, ProjectRole};
use crate::services::{storage_factory, webhooks};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub project_id: Option<String>,
    pub search: Option<String>,
    pub folder: Option<String>,
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
    if let Some(folder) = normalize_folder_filter(query.folder.as_deref())? {
        db_query = db_query.filter(
            Condition::any()
                .add(file::Column::Folder.eq(folder.clone()))
                .add(file::Column::Folder.starts_with(format!("{folder}/"))),
        );
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
    let model = load_accessible_file(&state, &auth.claims.sub, &id, ProjectRole::Viewer).await?;
    Ok(Json(json!({ "data": FileView::from(model) })).into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_accessible_file(&state, &auth.claims.sub, &id, ProjectRole::Editor).await?;
    let connection = storage_connection::Entity::find_by_id(model.storage_connection_id.clone())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    let adapter = storage_factory::build_adapter(
        &connection,
        &state.config.encryption_key,
        state.config.cdn_base_url.as_deref(),
    )?;
    adapter
        .delete(&model.path)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    file::Entity::delete_by_id(model.id.clone())
        .exec(&state.db)
        .await?;
    audit::record(
        &state,
        AuditEvent::user(&auth.claims, "file.deleted")
            .with_project(&model.project_id)
            .with_resource("file", &model.id)
            .with_metadata(json!({
                "originalName": model.original_name,
                "path": model.path,
                "size": model.size,
                "mimeType": model.mime_type,
            })),
    )
    .await?;
    upload_logs::record(
        &state,
        &model.project_id,
        None,
        "file.deleted",
        "success",
        None,
        json!({
            "fileId": model.id,
            "originalName": model.original_name,
            "path": model.path,
            "url": model.url,
        }),
    )
    .await?;
    webhooks::emit_file_event(
        &state,
        &model.project_id,
        None,
        "file.deleted",
        json!({
            "fileId": model.id,
            "path": model.path,
            "url": model.url,
            "mimeType": model.mime_type,
            "size": model.size,
        }),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_accessible_file(&state, &auth.claims.sub, &id, ProjectRole::Viewer).await?;
    let rows = upload_log::Entity::find()
        .filter(upload_log::Column::FileId.eq(model.id))
        .order_by_desc(upload_log::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let file_name = model.original_name;
    let view: Vec<UploadLogView> = rows
        .into_iter()
        .map(|row| UploadLogView::from_model(row, Some(file_name.clone())))
        .collect();
    Ok(Json(json!({ "data": view })).into_response())
}

async fn load_accessible_file(
    state: &AppState,
    user_id: &str,
    id: &str,
    required: ProjectRole,
) -> Result<file::Model, ApiError> {
    let model = file::Entity::find_by_id(id.to_string())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    require_project_role(state, user_id, &model.project_id, required).await?;
    Ok(model)
}

pub(crate) async fn owned_project_ids(
    state: &AppState,
    user_id: &str,
) -> Result<Vec<String>, ApiError> {
    accessible_project_ids(state, user_id).await
}

pub(crate) fn parse_date_filter(value: Option<&str>) -> Result<Option<DateTime<Utc>>, ApiError> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let dt = DateTime::parse_from_rfc3339(value)
        .map_err(|_| ApiError::Validation("date filters must be RFC3339 timestamps".into()))?;
    Ok(Some(dt.with_timezone(&Utc)))
}

fn normalize_folder_filter(value: Option<&str>) -> Result<Option<String>, ApiError> {
    let Some(value) = value
        .map(str::trim)
        .map(|value| value.trim_matches('/'))
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    if value.contains('\\')
        || value.split('/').any(|part| {
            part.is_empty() || part == "." || part == ".." || part.chars().any(char::is_control)
        })
    {
        return Err(ApiError::Validation("folder filter is invalid".into()));
    }
    Ok(Some(value.to_string()))
}
