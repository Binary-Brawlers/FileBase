use std::collections::BTreeMap;

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, FixedOffset};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::entities::{file, upload_preset};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::routes::files::owned_project_ids;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub project_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FolderView {
    pub project_id: String,
    pub path: String,
    pub name: String,
    pub parent: Option<String>,
    pub file_count: u64,
    pub direct_file_count: u64,
    pub total_size: i64,
    pub preset_count: u64,
    pub latest_upload_at: Option<String>,
}

#[derive(Default)]
struct FolderAggregate {
    file_count: u64,
    direct_file_count: u64,
    total_size: i64,
    preset_count: u64,
    latest_upload_at: Option<DateTime<FixedOffset>>,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListQuery>,
) -> ApiResult<Response> {
    let project_ids = owned_project_ids(&state, &auth.claims.sub).await?;
    if project_ids.is_empty() {
        return Ok(Json(json!({ "data": Vec::<FolderView>::new() })).into_response());
    }
    if let Some(project_id) = &query.project_id {
        if !project_ids.iter().any(|id| id == project_id) {
            return Err(ApiError::Forbidden);
        }
    }

    let selected_project_ids = query.project_id.map(|id| vec![id]).unwrap_or(project_ids);
    let files = file::Entity::find()
        .filter(file::Column::ProjectId.is_in(selected_project_ids.clone()))
        .all(&state.db)
        .await?;
    let presets = upload_preset::Entity::find()
        .filter(upload_preset::Column::ProjectId.is_in(selected_project_ids))
        .all(&state.db)
        .await?;

    let mut folders: BTreeMap<(String, String), FolderAggregate> = BTreeMap::new();
    for row in files {
        let folder = normalize_stored_folder(&row.folder);
        if folder.is_empty() {
            continue;
        }
        let ancestors = ancestors(&folder);
        for path in &ancestors {
            let aggregate = folders
                .entry((row.project_id.clone(), path.clone()))
                .or_default();
            aggregate.file_count += 1;
            aggregate.total_size += row.size;
            if path == &folder {
                aggregate.direct_file_count += 1;
            }
            if aggregate
                .latest_upload_at
                .as_ref()
                .is_none_or(|latest| row.created_at > *latest)
            {
                aggregate.latest_upload_at = Some(row.created_at);
            }
        }
    }
    for preset in presets {
        let folder = normalize_stored_folder(&preset.folder);
        if folder.is_empty() {
            continue;
        }
        for path in ancestors(&folder) {
            let aggregate = folders
                .entry((preset.project_id.clone(), path.clone()))
                .or_default();
            if path == folder {
                aggregate.preset_count += 1;
            }
        }
    }

    let view = folders
        .into_iter()
        .map(|((project_id, path), aggregate)| FolderView {
            project_id,
            name: path.rsplit('/').next().unwrap_or(&path).to_string(),
            parent: path.rsplit_once('/').map(|(parent, _)| parent.to_string()),
            path,
            file_count: aggregate.file_count,
            direct_file_count: aggregate.direct_file_count,
            total_size: aggregate.total_size,
            preset_count: aggregate.preset_count,
            latest_upload_at: aggregate.latest_upload_at.map(|date| date.to_rfc3339()),
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({ "data": view })).into_response())
}

fn normalize_stored_folder(folder: &str) -> String {
    folder.trim().trim_matches('/').to_string()
}

fn ancestors(folder: &str) -> Vec<String> {
    let mut path = String::new();
    folder
        .split('/')
        .filter(|part| !part.is_empty())
        .map(|part| {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
            path.clone()
        })
        .collect()
}
