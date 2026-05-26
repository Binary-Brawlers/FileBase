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

use crate::entities::{project, storage_connection};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::services::{crypto, storage_factory};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StorageInput {
    Local {
        base_path: String,
        public_base_url: String,
    },
    Ftp {
        host: String,
        port: Option<i32>,
        username: String,
        password: String,
        base_path: String,
        public_base_url: String,
    },
    Sftp {
        host: String,
        port: Option<i32>,
        username: String,
        password: Option<String>,
        private_key: Option<String>,
        base_path: String,
        public_base_url: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    pub project_id: String,
    #[serde(flatten)]
    pub storage: StorageInput,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub base_path: Option<String>,
    pub public_base_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StorageConnectionView {
    pub id: String,
    pub project_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    pub has_password: bool,
    pub has_private_key: bool,
    pub base_path: String,
    pub public_base_url: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<storage_connection::Model> for StorageConnectionView {
    fn from(m: storage_connection::Model) -> Self {
        Self {
            id: m.id,
            project_id: m.project_id,
            kind: m.r#type,
            host: m.host,
            port: m.port,
            username: m.username,
            has_password: m.encrypted_password.is_some(),
            has_private_key: m.encrypted_private_key.is_some(),
            base_path: m.base_path,
            public_base_url: m.public_base_url,
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

    let id = new_id("stc");
    let now = Utc::now().into();
    let model = build_create_model(
        payload.storage,
        &payload.project_id,
        &id,
        &state.config.encryption_key,
        now,
    )?;
    let inserted = model.insert(&state.db).await?;
    tracing::info!(
        user_id = %auth.claims.sub,
        project_id = %payload.project_id,
        storage_connection_id = %inserted.id,
        storage_type = %inserted.r#type,
        "audit.storage_connection.created"
    );
    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": StorageConnectionView::from(inserted) })),
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
        return Ok(Json(json!({ "data": Vec::<StorageConnectionView>::new() })).into_response());
    }
    let rows = storage_connection::Entity::find()
        .filter(storage_connection::Column::ProjectId.is_in(ids))
        .order_by_asc(storage_connection::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let view: Vec<StorageConnectionView> =
        rows.into_iter().map(StorageConnectionView::from).collect();
    Ok(Json(json!({ "data": view })).into_response())
}

pub async fn get(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_owned(&state, &auth.claims.sub, &id).await?;
    Ok(Json(json!({ "data": StorageConnectionView::from(model) })).into_response())
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateRequest>,
) -> ApiResult<Response> {
    let model = load_owned(&state, &auth.claims.sub, &id).await?;
    let key = state.config.encryption_key.clone();
    let mut active = model.into_active_model();

    if let Some(host) = payload.host {
        active.host = Set(Some(host));
    }
    if let Some(port) = payload.port {
        active.port = Set(Some(port));
    }
    if let Some(username) = payload.username {
        active.username = Set(Some(username));
    }
    if let Some(password) = payload.password {
        let encrypted = if password.is_empty() {
            None
        } else {
            Some(crypto::encrypt(&password, &key)?)
        };
        active.encrypted_password = Set(encrypted);
    }
    if let Some(private_key) = payload.private_key {
        let encrypted = if private_key.is_empty() {
            None
        } else {
            Some(crypto::encrypt(&private_key, &key)?)
        };
        active.encrypted_private_key = Set(encrypted);
    }
    if let Some(base_path) = payload.base_path {
        active.base_path = Set(base_path);
    }
    if let Some(public_base_url) = payload.public_base_url {
        active.public_base_url = Set(public_base_url);
    }
    active.updated_at = Set(Utc::now().into());

    let saved = active.update(&state.db).await?;
    tracing::info!(
        user_id = %auth.claims.sub,
        storage_connection_id = %saved.id,
        storage_type = %saved.r#type,
        "audit.storage_connection.updated"
    );
    Ok(Json(json!({ "data": StorageConnectionView::from(saved) })).into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_owned(&state, &auth.claims.sub, &id).await?;
    let project_id = model.project_id.clone();
    let storage_type = model.r#type.clone();
    storage_connection::Entity::delete_by_id(model.id)
        .exec(&state.db)
        .await?;
    tracing::info!(
        user_id = %auth.claims.sub,
        storage_connection_id = %id,
        project_id = %project_id,
        storage_type = %storage_type,
        "audit.storage_connection.deleted"
    );
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn test(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_owned(&state, &auth.claims.sub, &id).await?;
    let adapter = storage_factory::build_adapter(&model, &state.config.encryption_key)?;
    let body = match adapter.health_check().await {
        Ok(()) => {
            tracing::info!(
                user_id = %auth.claims.sub,
                storage_connection_id = %id,
                "audit.storage_connection.test_succeeded"
            );
            json!({ "data": { "ok": true } })
        }
        Err(e) => {
            tracing::warn!(
                user_id = %auth.claims.sub,
                storage_connection_id = %id,
                error = %e,
                "audit.storage_connection.test_failed"
            );
            json!({ "data": { "ok": false, "message": e.to_string() } })
        }
    };
    Ok(Json(body).into_response())
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
) -> Result<storage_connection::Model, ApiError> {
    let model = storage_connection::Entity::find_by_id(id.to_string())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    ensure_project_owner(state, user_id, &model.project_id).await?;
    Ok(model)
}

fn build_create_model(
    input: StorageInput,
    project_id: &str,
    id: &str,
    encryption_key: &str,
    now: sea_orm::prelude::ChronoDateTimeWithTimeZone,
) -> Result<storage_connection::ActiveModel, ApiError> {
    let (
        r#type,
        host,
        port,
        username,
        encrypted_password,
        encrypted_private_key,
        base_path,
        public_base_url,
    ) = match input {
        StorageInput::Local {
            base_path,
            public_base_url,
        } => (
            "local".to_string(),
            None,
            None,
            None,
            None,
            None,
            base_path,
            public_base_url,
        ),
        StorageInput::Ftp {
            host,
            port,
            username,
            password,
            base_path,
            public_base_url,
        } => (
            "ftp".to_string(),
            Some(host),
            Some(port.unwrap_or(21)),
            Some(username),
            Some(crypto::encrypt(&password, encryption_key)?),
            None,
            base_path,
            public_base_url,
        ),
        StorageInput::Sftp {
            host,
            port,
            username,
            password,
            private_key,
            base_path,
            public_base_url,
        } => {
            if password.is_none() && private_key.is_none() {
                return Err(ApiError::Validation(
                    "sftp requires password or private_key".into(),
                ));
            }
            (
                "sftp".to_string(),
                Some(host),
                Some(port.unwrap_or(22)),
                Some(username),
                password
                    .as_deref()
                    .map(|p| crypto::encrypt(p, encryption_key))
                    .transpose()?,
                private_key
                    .as_deref()
                    .map(|k| crypto::encrypt(k, encryption_key))
                    .transpose()?,
                base_path,
                public_base_url,
            )
        }
    };

    Ok(storage_connection::ActiveModel {
        id: Set(id.to_string()),
        project_id: Set(project_id.to_string()),
        r#type: Set(r#type),
        host: Set(host),
        port: Set(port),
        username: Set(username),
        encrypted_password: Set(encrypted_password),
        encrypted_private_key: Set(encrypted_private_key),
        base_path: Set(base_path),
        public_base_url: Set(public_base_url),
        created_at: Set(now),
        updated_at: Set(now),
    })
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}
