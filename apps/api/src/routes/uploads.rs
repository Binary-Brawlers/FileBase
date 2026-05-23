use axum::{
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use filebase_image_processing::process_image;
use filebase_storage::UploadInput;
use rand::{distributions::Alphanumeric, Rng};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};
use uuid::Uuid;

use crate::entities::{
    api_key, file, storage_connection, upload_log, upload_preset, upload_session,
};
use crate::error::{ApiError, ApiResult};
use crate::services::storage_factory;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SignRequest {
    pub project_id: Option<String>,
    pub preset_id: Option<String>,
    pub preset: Option<String>,
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct UploadSessionView {
    pub id: String,
    pub project_id: String,
    pub preset_id: String,
    pub upload_url: String,
    pub token: String,
    pub expires_at: String,
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
    pub duplicate: bool,
    pub duplicate_of_file_id: Option<String>,
    pub created_at: String,
}

impl From<file::Model> for FileView {
    fn from(m: file::Model) -> Self {
        let duplicate = m.duplicate_of_file_id.is_some();
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
            duplicate,
            duplicate_of_file_id: m.duplicate_of_file_id,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

struct UploadedPart {
    temp_path: PathBuf,
    size: u64,
    hash: String,
    magic_bytes: Vec<u8>,
    filename: String,
    content_type: Option<String>,
    preset_id: Option<String>,
    preset: Option<String>,
    project_id: Option<String>,
}

pub async fn sign(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SignRequest>,
) -> ApiResult<Response> {
    let key = authenticate_api_key(&state, &headers).await?;
    let project_id = payload.project_id.unwrap_or_else(|| key.project_id.clone());
    if project_id != key.project_id {
        return Err(ApiError::Forbidden);
    }
    let preset = load_preset(
        &state,
        &project_id,
        payload.preset_id.as_deref(),
        payload.preset.as_deref(),
    )
    .await?;
    let expires_in = payload.expires_in_seconds.unwrap_or(900).clamp(60, 3600);
    let token = format!("fb_upload_{}", random_string(48));
    let now = Utc::now();
    let expires_at = now + Duration::seconds(expires_in);
    let session_id = new_id("session");

    upload_session::ActiveModel {
        id: Set(session_id.clone()),
        project_id: Set(project_id.clone()),
        preset_id: Set(preset.id.clone()),
        token_hash: Set(hash_secret(&token)),
        folder: Set(preset.folder.clone()),
        allowed_mime_types: Set(preset.allowed_mime_types.clone()),
        max_file_size: Set(preset.max_file_size),
        expires_at: Set(expires_at.into()),
        used_at: Set(None),
        created_at: Set(now.into()),
    }
    .insert(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "data": UploadSessionView {
                id: session_id.clone(),
                project_id,
                preset_id: preset.id,
                upload_url: format!("{}/uploads/{}", state.config.app_url.trim_end_matches('/'), session_id),
                token,
                expires_at: expires_at.to_rfc3339(),
            }
        })),
    )
        .into_response())
}

pub async fn direct_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<Response> {
    let key = authenticate_api_key(&state, &headers).await?;
    let input = read_multipart(multipart, state.config.max_upload_size).await?;
    let project_id = input
        .project_id
        .clone()
        .unwrap_or_else(|| key.project_id.clone());
    if project_id != key.project_id {
        cleanup_upload_temp(&input).await?;
        return Err(ApiError::Forbidden);
    }
    let preset = match load_preset(
        &state,
        &project_id,
        input.preset_id.as_deref(),
        input.preset.as_deref(),
    )
    .await
    {
        Ok(preset) => preset,
        Err(e) => {
            cleanup_upload_temp(&input).await?;
            return Err(e);
        }
    };
    let result = process_upload(&state, &preset, &input, None).await;
    cleanup_upload_temp(&input).await?;
    let result = result?;
    Ok((StatusCode::CREATED, Json(json!({ "data": result }))).into_response())
}

pub async fn session_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    multipart: Multipart,
) -> ApiResult<Response> {
    let token = extract_bearer_or_key(&headers).ok_or(ApiError::Unauthorized)?;
    let session = upload_session::Entity::find_by_id(session_id)
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if session.token_hash != hash_secret(&token) {
        return Err(ApiError::Unauthorized);
    }
    if session.used_at.is_some() {
        return Err(ApiError::Conflict(
            "upload session has already been used".into(),
        ));
    }
    if session.expires_at < Utc::now().fixed_offset() {
        return Err(ApiError::Unauthorized);
    }
    let preset = upload_preset::Entity::find_by_id(session.preset_id.clone())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if preset.project_id != session.project_id {
        return Err(ApiError::Forbidden);
    }
    let input = read_multipart(multipart, state.config.max_upload_size).await?;
    let result = process_upload(&state, &preset, &input, Some(session.clone())).await;
    cleanup_upload_temp(&input).await?;
    let result = result?;

    let mut active = session.into_active_model();
    active.used_at = Set(Some(Utc::now().into()));
    active.update(&state.db).await?;

    Ok((StatusCode::CREATED, Json(json!({ "data": result }))).into_response())
}

async fn process_upload(
    state: &AppState,
    preset: &upload_preset::Model,
    input: &UploadedPart,
    session: Option<upload_session::Model>,
) -> Result<FileView, ApiError> {
    let size =
        i64::try_from(input.size).map_err(|_| ApiError::Validation("file is too large".into()))?;
    if size <= 0 {
        return Err(ApiError::Validation("file is required".into()));
    }
    if size > preset.max_file_size {
        return Err(ApiError::Validation(
            "file exceeds preset max_file_size".into(),
        ));
    }
    if size as u64 > state.config.max_upload_size {
        return Err(ApiError::Validation(
            "file exceeds server max upload size".into(),
        ));
    }

    let original_name = safe_original_name(&input.filename)?;
    let original_extension = extension_from_name(&original_name)?;
    validate_extension(&preset.allowed_extensions, &original_extension)?;
    let input_mime_type = validate_mime(
        &preset.allowed_mime_types,
        input.content_type.as_deref(),
        &input.magic_bytes,
    )?;
    if let Some(session) = &session {
        validate_mime(
            &session.allowed_mime_types,
            Some(&input_mime_type),
            &input.magic_bytes,
        )?;
        if size > session.max_file_size {
            return Err(ApiError::Validation(
                "file exceeds upload session max_file_size".into(),
            ));
        }
    }

    let original_bytes = fs::read(&input.temp_path)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("read uploaded temp file: {e}")))?;
    let processed = process_image(
        &original_bytes,
        &input_mime_type,
        &original_extension,
        &preset.transformations_json,
    )
    .map_err(|e| ApiError::Validation(e.to_string()))?;

    let mut bytes = original_bytes;
    let mut mime_type = input_mime_type.clone();
    let mut extension = original_extension.clone();
    let mut transformation_metadata = json!({});
    let mut original_to_preserve = None;
    let mut thumbnail_to_upload = None;

    if let Some(processed) = processed {
        bytes = processed.bytes;
        mime_type = processed.mime_type;
        extension = processed.extension;
        transformation_metadata = processed.metadata;
        original_to_preserve = processed.original;
        thumbnail_to_upload = processed.thumbnail;
    }

    let output_size = i64::try_from(bytes.len())
        .map_err(|_| ApiError::Validation("processed file is too large".into()))?;
    if output_size > preset.max_file_size {
        return Err(ApiError::Validation(
            "processed file exceeds preset max_file_size".into(),
        ));
    }
    if output_size as u64 > state.config.max_upload_size {
        return Err(ApiError::Validation(
            "processed file exceeds server max upload size".into(),
        ));
    }

    let hash = if transformation_metadata == json!({}) {
        input.hash.clone()
    } else {
        hex_digest(&bytes)
    };
    if let Some(existing) = file::Entity::find()
        .filter(file::Column::ProjectId.eq(preset.project_id.clone()))
        .filter(file::Column::Hash.eq(hash.clone()))
        .filter(file::Column::Status.eq("uploaded"))
        .order_by_asc(file::Column::CreatedAt)
        .one(&state.db)
        .await?
    {
        match preset.duplicate_strategy.as_str() {
            "return_existing" => {
                write_log(
                    state,
                    &preset.project_id,
                    Some(&existing.id),
                    "file.duplicate_detected",
                    "success",
                    Some("returned existing file"),
                    json!({ "duplicateOfFileId": existing.id }),
                )
                .await?;
                return Ok(FileView::from(existing));
            }
            "reject_duplicate" => {
                write_log(
                    state,
                    &preset.project_id,
                    Some(&existing.id),
                    "file.duplicate_detected",
                    "rejected",
                    Some("duplicate file rejected"),
                    json!({ "duplicateOfFileId": existing.id }),
                )
                .await?;
                return Err(ApiError::Conflict(
                    "duplicate file rejected by preset".into(),
                ));
            }
            _ => {}
        }
    }

    let connection = load_storage_connection(state, preset).await?;
    let adapter = storage_factory::build_adapter(&connection, &state.config.encryption_key)?;
    let saved_name =
        generate_saved_name(&preset.filename_strategy, &original_name, &extension, &hash);
    let path = join_relative(&preset.folder, &saved_name)?;
    let uploaded = adapter
        .upload(UploadInput {
            path: path.clone(),
            bytes,
            content_type: Some(mime_type.clone()),
        })
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    let saved_stem = saved_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(&saved_name);
    let mut original_metadata = json!(null);
    if let Some(original) = original_to_preserve {
        let original_name = format!("{saved_stem}-original.{}", original.extension);
        let original_path = join_relative(&format!("{}/originals", preset.folder), &original_name)?;
        let original_uploaded = adapter
            .upload(UploadInput {
                path: original_path,
                bytes: original.bytes,
                content_type: Some(original.mime_type.clone()),
            })
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
        original_metadata = json!({
            "path": original_uploaded.path,
            "url": original_uploaded.url,
            "mimeType": original.mime_type,
            "extension": original.extension,
            "size": original.size
        });
    }

    let mut thumbnail_metadata = json!(null);
    if let Some(thumbnail) = thumbnail_to_upload {
        let thumbnail_name = format!("{saved_stem}-thumb.{}", thumbnail.extension);
        let thumbnail_path =
            join_relative(&format!("{}/thumbnails", preset.folder), &thumbnail_name)?;
        let thumbnail_uploaded = adapter
            .upload(UploadInput {
                path: thumbnail_path,
                bytes: thumbnail.bytes,
                content_type: Some(thumbnail.mime_type.clone()),
            })
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
        thumbnail_metadata = json!({
            "path": thumbnail_uploaded.path,
            "url": thumbnail_uploaded.url,
            "mimeType": thumbnail.mime_type,
            "extension": thumbnail.extension,
            "width": thumbnail.width,
            "height": thumbnail.height,
            "size": thumbnail.size
        });
    }

    let now = Utc::now().into();
    let inserted = file::ActiveModel {
        id: Set(new_id("file")),
        project_id: Set(preset.project_id.clone()),
        storage_connection_id: Set(connection.id),
        original_name: Set(original_name),
        saved_name: Set(saved_name),
        mime_type: Set(mime_type),
        extension: Set(extension),
        size: Set(uploaded.size as i64),
        hash: Set(hash),
        folder: Set(preset.folder.clone()),
        path: Set(uploaded.path),
        url: Set(uploaded.url),
        status: Set("uploaded".to_string()),
        duplicate_of_file_id: Set(None),
        metadata_json: Set(json!({
            "presetId": preset.id,
            "sessionId": session.map(|s| s.id),
            "transformations": transformation_metadata,
            "original": original_metadata,
            "thumbnail": thumbnail_metadata
        })),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&state.db)
    .await?;
    write_log(
        state,
        &preset.project_id,
        Some(&inserted.id),
        "file.uploaded",
        "success",
        None,
        json!({ "path": inserted.path, "url": inserted.url }),
    )
    .await?;
    Ok(FileView::from(inserted))
}

async fn authenticate_api_key(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<api_key::Model, ApiError> {
    let secret = extract_bearer_or_key(headers).ok_or(ApiError::Unauthorized)?;
    let hash = hash_secret(&secret);
    let key = api_key::Entity::find()
        .filter(api_key::Column::KeyHash.eq(hash))
        .filter(api_key::Column::RevokedAt.is_null())
        .one(&state.db)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    let mut active = key.clone().into_active_model();
    active.last_used_at = Set(Some(Utc::now().into()));
    active.update(&state.db).await?;
    Ok(key)
}

fn extract_bearer_or_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(str::to_string)
        })
}

async fn load_preset(
    state: &AppState,
    project_id: &str,
    preset_id: Option<&str>,
    preset_name: Option<&str>,
) -> Result<upload_preset::Model, ApiError> {
    let mut query = upload_preset::Entity::find()
        .filter(upload_preset::Column::ProjectId.eq(project_id.to_string()));
    query = if let Some(id) = preset_id {
        query.filter(upload_preset::Column::Id.eq(id.to_string()))
    } else if let Some(name) = preset_name {
        query.filter(upload_preset::Column::Name.eq(name.to_string()))
    } else {
        return Err(ApiError::Validation(
            "preset_id or preset is required".into(),
        ));
    };
    query.one(&state.db).await?.ok_or(ApiError::NotFound)
}

async fn load_storage_connection(
    state: &AppState,
    preset: &upload_preset::Model,
) -> Result<storage_connection::Model, ApiError> {
    if let Some(id) = &preset.storage_connection_id {
        let connection = storage_connection::Entity::find_by_id(id.clone())
            .one(&state.db)
            .await?
            .ok_or(ApiError::NotFound)?;
        if connection.project_id != preset.project_id {
            return Err(ApiError::Forbidden);
        }
        return Ok(connection);
    }
    storage_connection::Entity::find()
        .filter(storage_connection::Column::ProjectId.eq(preset.project_id.clone()))
        .order_by_asc(storage_connection::Column::CreatedAt)
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::Validation("project has no storage connection".into()))
}

async fn read_multipart(
    mut multipart: Multipart,
    max_upload_size: u64,
) -> Result<UploadedPart, ApiError> {
    let mut temp_path = None;
    let mut size = 0_u64;
    let mut hash = String::new();
    let mut magic_bytes = Vec::new();
    let mut filename = String::new();
    let mut content_type = None;
    let mut preset_id = None;
    let mut preset = None;
    let mut project_id = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            if temp_path.is_some() {
                return Err(ApiError::Validation(
                    "multipart request must contain one file field".into(),
                ));
            }
            filename = field.file_name().unwrap_or("upload.bin").to_string();
            content_type = field.content_type().map(str::to_string);
            let streamed = stream_file_field(field, max_upload_size).await?;
            temp_path = Some(streamed.temp_path);
            size = streamed.size;
            hash = streamed.hash;
            magic_bytes = streamed.magic_bytes;
        } else {
            let value = field
                .text()
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            match name.as_str() {
                "preset_id" => preset_id = Some(value),
                "preset" => preset = Some(value),
                "project_id" => project_id = Some(value),
                _ => {}
            }
        }
    }
    let temp_path =
        temp_path.ok_or_else(|| ApiError::Validation("multipart file field is required".into()))?;
    if size == 0 {
        let _ = fs::remove_file(&temp_path).await;
        return Err(ApiError::Validation(
            "multipart file field is required".into(),
        ));
    }
    Ok(UploadedPart {
        temp_path,
        size,
        hash,
        magic_bytes,
        filename,
        content_type,
        preset_id,
        preset,
        project_id,
    })
}

struct StreamedFile {
    temp_path: PathBuf,
    size: u64,
    hash: String,
    magic_bytes: Vec<u8>,
}

async fn stream_file_field(
    mut field: axum::extract::multipart::Field<'_>,
    max_upload_size: u64,
) -> Result<StreamedFile, ApiError> {
    let temp_path =
        std::env::temp_dir().join(format!("filebase-upload-{}.tmp", Uuid::new_v4().simple()));
    let mut file = File::create(&temp_path)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("create upload temp file: {e}")))?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut magic_bytes = Vec::with_capacity(16);

    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    {
        size = size
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| ApiError::Validation("file is too large".into()))?;
        if size > max_upload_size {
            let _ = fs::remove_file(&temp_path).await;
            return Err(ApiError::Validation(
                "file exceeds server max upload size".into(),
            ));
        }
        if magic_bytes.len() < 16 {
            let remaining = 16 - magic_bytes.len();
            magic_bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        }
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("write upload temp file: {e}")))?;
    }
    file.flush()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("flush upload temp file: {e}")))?;

    Ok(StreamedFile {
        temp_path,
        size,
        hash: hex_digest_from_hasher(hasher),
        magic_bytes,
    })
}

async fn cleanup_upload_temp(input: &UploadedPart) -> Result<(), ApiError> {
    match fs::remove_file(&input.temp_path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ApiError::Internal(anyhow::anyhow!(
            "remove upload temp file: {e}"
        ))),
    }
}

fn validate_mime(
    allowed: &JsonValue,
    content_type: Option<&str>,
    bytes: &[u8],
) -> Result<String, ApiError> {
    let mime = content_type
        .unwrap_or("application/octet-stream")
        .to_lowercase();
    let magic = magic_mime(bytes);
    if let Some(magic) = magic {
        if mime != "application/octet-stream" && mime != magic {
            return Err(ApiError::Validation(
                "file content does not match MIME type".into(),
            ));
        }
    }
    let effective = magic.unwrap_or(mime.as_str()).to_string();
    let allowed = string_array(allowed)?;
    if !allowed.is_empty() && !allowed.iter().any(|v| v == &effective) {
        return Err(ApiError::Validation(
            "MIME type is not allowed by preset".into(),
        ));
    }
    Ok(effective)
}

fn validate_extension(allowed: &JsonValue, extension: &str) -> Result<(), ApiError> {
    let allowed = string_array(allowed)?;
    if allowed.is_empty() {
        return Ok(());
    }
    if allowed
        .iter()
        .any(|v| v.trim_start_matches('.') == extension)
    {
        return Ok(());
    }
    Err(ApiError::Validation(
        "file extension is not allowed by preset".into(),
    ))
}

fn string_array(value: &JsonValue) -> Result<Vec<String>, ApiError> {
    value
        .as_array()
        .ok_or_else(|| ApiError::Validation("preset rules must be arrays".into()))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(|s| s.to_lowercase())
                .ok_or_else(|| ApiError::Validation("preset rules must contain strings".into()))
        })
        .collect()
}

fn magic_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Some("image/png");
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.starts_with(b"%PDF-") {
        return Some("application/pdf");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

fn safe_original_name(name: &str) -> Result<String, ApiError> {
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    if name.is_empty() || name == "." || name == ".." {
        return Err(ApiError::Validation("filename is invalid".into()));
    }
    Ok(name.to_string())
}

fn extension_from_name(name: &str) -> Result<String, ApiError> {
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_lowercase())
        .filter(|ext| !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric()))
        .ok_or_else(|| ApiError::Validation("file must have a simple extension".into()))
}

fn generate_saved_name(strategy: &str, original: &str, extension: &str, hash: &str) -> String {
    let stem = original
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(original);
    match strategy {
        "uuid" => format!("{}.{}", Uuid::new_v4().simple(), extension),
        "hash" => format!("{}.{}", hash, extension),
        "timestamp_random" => format!(
            "{}-{}.{}",
            Utc::now().timestamp_millis(),
            random_string(8),
            extension
        ),
        "random" => format!("{}.{}", random_string(20), extension),
        "original_suffix" => format!("{}-{}.{}", slugify(stem), random_string(8), extension),
        _ => format!("{}-{}.{}", slugify(stem), random_string(8), extension),
    }
}

fn slugify(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        "file".to_string()
    } else {
        out.to_string()
    }
}

fn join_relative(folder: &str, filename: &str) -> Result<String, ApiError> {
    let folder = folder.trim().trim_matches('/');
    if folder.is_empty()
        || folder.contains("..")
        || filename.contains('/')
        || filename.contains('\\')
    {
        return Err(ApiError::Validation("upload path is invalid".into()));
    }
    Ok(format!("{folder}/{filename}"))
}

async fn write_log(
    state: &AppState,
    project_id: &str,
    file_id: Option<&str>,
    event: &str,
    status: &str,
    message: Option<&str>,
    metadata: JsonValue,
) -> Result<(), ApiError> {
    upload_log::ActiveModel {
        id: Set(new_id("log")),
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

fn hash_secret(secret: &str) -> String {
    hex_digest(secret.as_bytes())
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_digest_from_hasher(hasher: Sha256) -> String {
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}
