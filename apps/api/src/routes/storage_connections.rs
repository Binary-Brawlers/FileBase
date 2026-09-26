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

use crate::entities::storage_connection;
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::services::audit::{self, AuditEvent};
use crate::services::authorization::{accessible_project_ids, require_project_role, ProjectRole};
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
    S3 {
        bucket: String,
        region: String,
        endpoint: Option<String>,
        access_key: String,
        secret_key: String,
        force_path_style: Option<bool>,
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
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub force_path_style: Option<bool>,
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
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub force_path_style: bool,
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
            bucket: m.bucket,
            region: m.region,
            force_path_style: m.force_path_style,
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
    require_project_role(
        &state,
        &auth.claims.sub,
        &payload.project_id,
        ProjectRole::Admin,
    )
    .await?;

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
    audit::record(
        &state,
        AuditEvent::user(&auth.claims, "storage_connection.created")
            .with_project(&inserted.project_id)
            .with_resource("storage_connection", &inserted.id)
            .with_metadata(json!({ "type": inserted.r#type })),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": StorageConnectionView::from(inserted) })),
    )
        .into_response())
}

pub async fn list(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Response> {
    let ids = accessible_project_ids(&state, &auth.claims.sub).await?;
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
    let model = load_accessible(&state, &auth.claims.sub, &id, ProjectRole::Viewer).await?;
    Ok(Json(json!({ "data": StorageConnectionView::from(model) })).into_response())
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateRequest>,
) -> ApiResult<Response> {
    let model = load_accessible(&state, &auth.claims.sub, &id, ProjectRole::Admin).await?;
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
    if let Some(bucket) = payload.bucket {
        active.bucket = Set(Some(bucket));
    }
    if let Some(region) = payload.region {
        active.region = Set(Some(region));
    }
    if let Some(force_path_style) = payload.force_path_style {
        active.force_path_style = Set(force_path_style);
    }
    if let Some(base_path) = payload.base_path {
        active.base_path = Set(base_path);
    }
    if let Some(public_base_url) = payload.public_base_url {
        active.public_base_url = Set(public_base_url);
    }
    active.updated_at = Set(Utc::now().into());

    let saved = active.update(&state.db).await?;
    audit::record(
        &state,
        AuditEvent::user(&auth.claims, "storage_connection.updated")
            .with_project(&saved.project_id)
            .with_resource("storage_connection", &saved.id)
            .with_metadata(json!({ "type": saved.r#type })),
    )
    .await?;
    Ok(Json(json!({ "data": StorageConnectionView::from(saved) })).into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_accessible(&state, &auth.claims.sub, &id, ProjectRole::Admin).await?;
    let project_id = model.project_id.clone();
    let storage_type = model.r#type.clone();
    storage_connection::Entity::delete_by_id(model.id)
        .exec(&state.db)
        .await?;
    audit::record(
        &state,
        AuditEvent::user(&auth.claims, "storage_connection.deleted")
            .with_project(&project_id)
            .with_resource("storage_connection", &id)
            .with_metadata(json!({ "type": storage_type })),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn test(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let model = load_accessible(&state, &auth.claims.sub, &id, ProjectRole::Admin).await?;
    let adapter = storage_factory::build_adapter(&model, &state.config.encryption_key, None)?;
    let body = match adapter.health_check().await {
        Ok(()) => {
            audit::record(
                &state,
                AuditEvent::user(&auth.claims, "storage_connection.test_succeeded")
                    .with_project(&model.project_id)
                    .with_resource("storage_connection", &id),
            )
            .await?;
            json!({ "data": { "ok": true } })
        }
        Err(e) => {
            audit::record_best_effort(
                &state,
                AuditEvent::user(&auth.claims, "storage_connection.test_failed")
                    .with_status("failure")
                    .with_project(&model.project_id)
                    .with_resource("storage_connection", &id)
                    .with_metadata(json!({ "error": e.to_string() })),
            )
            .await;
            json!({ "data": { "ok": false, "message": e.to_string() } })
        }
    };
    Ok(Json(body).into_response())
}

async fn load_accessible(
    state: &AppState,
    user_id: &str,
    id: &str,
    required: ProjectRole,
) -> Result<storage_connection::Model, ApiError> {
    let model = storage_connection::Entity::find_by_id(id.to_string())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    require_project_role(state, user_id, &model.project_id, required).await?;
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
        bucket,
        region,
        force_path_style,
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
            None,
            None,
            false,
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
            None,
            None,
            false,
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
                None,
                None,
                false,
                base_path,
                public_base_url,
            )
        }
        StorageInput::S3 {
            bucket,
            region,
            endpoint,
            access_key,
            secret_key,
            force_path_style,
            base_path,
            public_base_url,
        } => (
            "s3".to_string(),
            endpoint,
            None,
            Some(access_key),
            Some(crypto::encrypt(&secret_key, encryption_key)?),
            None,
            Some(bucket),
            Some(region),
            force_path_style.unwrap_or(false),
            base_path,
            public_base_url,
        ),
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
        bucket: Set(bucket),
        region: Set(region),
        force_path_style: Set(force_path_style),
        base_path: Set(base_path),
        public_base_url: Set(public_base_url),
        created_at: Set(now),
        updated_at: Set(now),
    })
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}
