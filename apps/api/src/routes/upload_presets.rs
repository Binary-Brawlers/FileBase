use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use crate::entities::{project, storage_connection, upload_preset};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub storage_connection_id: Option<String>,
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

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateRequest>,
) -> ApiResult<Response> {
    let model = upload_preset::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    ensure_project_owner(&state, &auth.claims.sub, &model.project_id).await?;

    let mut active = model.into_active_model();

    if let Some(connection_id) = payload.storage_connection_id {
        let project_id = match &active.project_id {
            sea_orm::ActiveValue::Unchanged(v) | sea_orm::ActiveValue::Set(v) => v.clone(),
            _ => return Err(ApiError::Internal(anyhow::anyhow!("missing project id"))),
        };
        let target = storage_connection::Entity::find_by_id(connection_id.clone())
            .one(&state.db)
            .await?
            .ok_or_else(|| ApiError::Validation("storage connection not found".into()))?;
        if target.project_id != project_id {
            return Err(ApiError::Validation(
                "storage connection belongs to a different project".into(),
            ));
        }
        active.storage_connection_id = Set(Some(connection_id));
    }

    active.updated_at = Set(Utc::now().into());
    let saved = active.update(&state.db).await?;
    Ok(Json(json!({ "data": PresetView::from(saved) })).into_response())
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
