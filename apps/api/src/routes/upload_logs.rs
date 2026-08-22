use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::entities::{file, upload_log};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::routes::files::{owned_project_ids, parse_date_filter};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub project_id: Option<String>,
    pub file_id: Option<String>,
    pub event: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct UploadLogView {
    pub id: String,
    pub project_id: String,
    pub file_id: Option<String>,
    pub file_name: Option<String>,
    pub event: String,
    pub status: String,
    pub message: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

impl UploadLogView {
    pub fn from_model(model: upload_log::Model, file_name: Option<String>) -> Self {
        Self {
            id: model.id,
            project_id: model.project_id,
            file_id: model.file_id,
            file_name,
            event: model.event,
            status: model.status,
            message: model.message,
            metadata: model.metadata_json,
            created_at: model.created_at.to_rfc3339(),
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
        return Ok(Json(json!({ "data": Vec::<UploadLogView>::new() })).into_response());
    }
    if let Some(project_id) = &query.project_id {
        if !project_ids.iter().any(|id| id == project_id) {
            return Err(ApiError::Forbidden);
        }
    }

    let mut db_query = upload_log::Entity::find()
        .filter(upload_log::Column::ProjectId.is_in(project_ids))
        .order_by_desc(upload_log::Column::CreatedAt);
    if let Some(project_id) = query.project_id {
        db_query = db_query.filter(upload_log::Column::ProjectId.eq(project_id));
    }
    if let Some(file_id) = clean_filter(query.file_id) {
        db_query = db_query.filter(upload_log::Column::FileId.eq(file_id));
    }
    if let Some(event) = clean_filter(query.event) {
        db_query = db_query.filter(upload_log::Column::Event.eq(event));
    }
    if let Some(status) = clean_filter(query.status) {
        db_query = db_query.filter(upload_log::Column::Status.eq(status));
    }
    if let Some(from) = parse_date_filter(query.from.as_deref())? {
        db_query = db_query.filter(upload_log::Column::CreatedAt.gte(from));
    }
    if let Some(to) = parse_date_filter(query.to.as_deref())? {
        db_query = db_query.filter(upload_log::Column::CreatedAt.lte(to));
    }
    if let Some(search) = clean_filter(query.search) {
        db_query = db_query.filter(
            Condition::any()
                .add(upload_log::Column::Event.contains(search.clone()))
                .add(upload_log::Column::Status.contains(search.clone()))
                .add(upload_log::Column::Message.contains(search.clone()))
                .add(upload_log::Column::FileId.contains(search)),
        );
    }

    let rows = db_query
        .limit(query.limit.unwrap_or(200).clamp(1, 500))
        .all(&state.db)
        .await?;
    let file_ids = rows
        .iter()
        .filter_map(|row| row.file_id.clone())
        .collect::<HashSet<_>>();
    let file_names = if file_ids.is_empty() {
        HashMap::new()
    } else {
        file::Entity::find()
            .filter(file::Column::Id.is_in(file_ids))
            .all(&state.db)
            .await?
            .into_iter()
            .map(|file| (file.id, file.original_name))
            .collect::<HashMap<_, _>>()
    };
    let view = rows
        .into_iter()
        .map(|row| {
            let file_name = row
                .file_id
                .as_ref()
                .and_then(|id| file_names.get(id))
                .cloned()
                .or_else(|| {
                    row.metadata_json
                        .get("originalName")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                });
            UploadLogView::from_model(row, file_name)
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({ "data": view })).into_response())
}

pub async fn record(
    state: &AppState,
    project_id: &str,
    file_id: Option<&str>,
    event: &str,
    status: &str,
    message: Option<&str>,
    metadata: Value,
) -> Result<(), ApiError> {
    upload_log::ActiveModel {
        id: Set(format!("log_{}", Uuid::new_v4().simple())),
        project_id: Set(project_id.to_string()),
        file_id: Set(file_id.map(str::to_string)),
        event: Set(event.to_string()),
        status: Set(status.to_string()),
        message: Set(message.map(str::to_string)),
        metadata_json: Set(metadata),
        created_at: Set(Utc::now().into()),
    }
    .insert(&state.db)
    .await?;
    Ok(())
}

fn clean_filter(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
