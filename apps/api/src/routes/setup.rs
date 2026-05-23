use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, Set};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json_};
use uuid::Uuid;

use crate::entities::{project, storage_connection, upload_preset, user};
use crate::error::{ApiError, ApiResult};
use crate::services::{crypto, password};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub setup_required: bool,
}

pub async fn status(State(state): State<AppState>) -> ApiResult<Json<Json_>> {
    let count = user::Entity::find().count(&state.db).await?;
    Ok(Json(json!({
        "data": StatusResponse {
            setup_required: count == 0,
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct AdminInput {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectInput {
    pub name: String,
    pub slug: Option<String>,
}

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
pub struct PresetInput {
    pub name: Option<String>,
    pub folder: Option<String>,
    pub allowed_mime_types: Option<Vec<String>>,
    pub allowed_extensions: Option<Vec<String>>,
    pub max_file_size: Option<i64>,
    pub duplicate_strategy: Option<String>,
    pub filename_strategy: Option<String>,
    pub transformations: Option<Json_>,
}

#[derive(Debug, Deserialize)]
pub struct InitializeRequest {
    pub admin: AdminInput,
    pub project: ProjectInput,
    pub storage: StorageInput,
    pub preset: Option<PresetInput>,
}

#[derive(Debug, Serialize)]
pub struct InitializeResponse {
    pub user_id: String,
    pub project_id: String,
    pub storage_connection_id: String,
    pub upload_preset_id: String,
}

pub async fn initialize(
    State(state): State<AppState>,
    Json(payload): Json<InitializeRequest>,
) -> ApiResult<impl IntoResponse> {
    let existing = user::Entity::find().count(&state.db).await?;
    if existing > 0 {
        return Err(ApiError::Conflict("setup already completed".into()));
    }

    validate_admin(&payload.admin)?;

    let now = Utc::now().into();
    let user_id = new_id("usr");
    let project_id = new_id("prj");
    let storage_id = new_id("stc");
    let preset_id = new_id("ups");

    let password_hash = password::hash(&payload.admin.password)?;

    user::ActiveModel {
        id: Set(user_id.clone()),
        name: Set(payload.admin.name.trim().to_string()),
        email: Set(payload.admin.email.trim().to_lowercase()),
        password_hash: Set(password_hash),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&state.db)
    .await?;

    let slug = payload
        .project
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&payload.project.name));
    if slug.is_empty() {
        return Err(ApiError::Validation("project slug cannot be empty".into()));
    }

    project::ActiveModel {
        id: Set(project_id.clone()),
        user_id: Set(user_id.clone()),
        name: Set(payload.project.name.trim().to_string()),
        slug: Set(slug),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&state.db)
    .await?;

    let sc = build_storage(
        &payload.storage,
        &project_id,
        &storage_id,
        &state.config.encryption_key,
        now,
    )?;
    sc.insert(&state.db).await?;

    let preset_model = build_preset(payload.preset, &project_id, &preset_id, &storage_id, now)?;
    preset_model.insert(&state.db).await?;

    Ok(Json(json!({
        "data": InitializeResponse {
            user_id,
            project_id,
            storage_connection_id: storage_id,
            upload_preset_id: preset_id,
        }
    })))
}

fn validate_admin(admin: &AdminInput) -> Result<(), ApiError> {
    if admin.name.trim().is_empty() {
        return Err(ApiError::Validation("admin name is required".into()));
    }
    if !admin.email.contains('@') {
        return Err(ApiError::Validation("admin email is invalid".into()));
    }
    if admin.password.len() < 8 {
        return Err(ApiError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }
    Ok(())
}

fn build_storage(
    input: &StorageInput,
    project_id: &str,
    storage_id: &str,
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
            base_path.clone(),
            public_base_url.clone(),
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
            Some(host.clone()),
            Some(port.unwrap_or(21)),
            Some(username.clone()),
            Some(crypto::encrypt(password, encryption_key)?),
            None,
            base_path.clone(),
            public_base_url.clone(),
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
                Some(host.clone()),
                Some(port.unwrap_or(22)),
                Some(username.clone()),
                password
                    .as_deref()
                    .map(|p| crypto::encrypt(p, encryption_key))
                    .transpose()?,
                private_key
                    .as_deref()
                    .map(|k| crypto::encrypt(k, encryption_key))
                    .transpose()?,
                base_path.clone(),
                public_base_url.clone(),
            )
        }
    };

    Ok(storage_connection::ActiveModel {
        id: Set(storage_id.to_string()),
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

fn build_preset(
    input: Option<PresetInput>,
    project_id: &str,
    preset_id: &str,
    storage_connection_id: &str,
    now: sea_orm::prelude::ChronoDateTimeWithTimeZone,
) -> Result<upload_preset::ActiveModel, ApiError> {
    let input = input.unwrap_or(PresetInput {
        name: None,
        folder: None,
        allowed_mime_types: None,
        allowed_extensions: None,
        max_file_size: None,
        duplicate_strategy: None,
        filename_strategy: None,
        transformations: None,
    });

    let duplicate_strategy = input
        .duplicate_strategy
        .unwrap_or_else(|| "return_existing".into());
    match duplicate_strategy.as_str() {
        "return_existing" | "upload_new_copy" | "reject_duplicate" => {}
        other => {
            return Err(ApiError::Validation(format!(
                "invalid duplicate_strategy: {other}"
            )))
        }
    }

    Ok(upload_preset::ActiveModel {
        id: Set(preset_id.to_string()),
        project_id: Set(project_id.to_string()),
        storage_connection_id: Set(Some(storage_connection_id.to_string())),
        name: Set(input.name.unwrap_or_else(|| "default".into())),
        folder: Set(input.folder.unwrap_or_else(|| "uploads".into())),
        allowed_mime_types: Set(json!(input.allowed_mime_types.unwrap_or_default())),
        allowed_extensions: Set(json!(input.allowed_extensions.unwrap_or_default())),
        max_file_size: Set(input.max_file_size.unwrap_or(10_485_760)),
        duplicate_strategy: Set(duplicate_strategy),
        filename_strategy: Set(input
            .filename_strategy
            .unwrap_or_else(|| "slug_random".into())),
        transformations_json: Set(input.transformations.unwrap_or_else(|| json!({}))),
        created_at: Set(now),
        updated_at: Set(now),
    })
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
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
