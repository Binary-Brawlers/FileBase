use axum::{
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};
use uuid::Uuid;

use crate::entities::{upload_chunk, upload_preset, upload_session};
use crate::error::{ApiError, ApiResult};
use crate::routes::uploads::{
    cleanup_upload_temp, extract_bearer_or_key, hash_secret, hex_digest_from_hasher, new_id,
    process_upload, stream_file_field, UploadedPart,
};
use crate::services::audit::{self, AuditEvent};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ChunkStatusView {
    pub session_id: String,
    pub project_id: String,
    pub preset_id: String,
    pub used: bool,
    pub expires_at: String,
    pub max_file_size: i64,
    pub chunk_size: u64,
    pub received_chunks: Vec<i32>,
    pub received_bytes: i64,
}

#[derive(Debug, Serialize)]
pub struct ChunkAcceptedView {
    pub session_id: String,
    pub chunk_index: i32,
    pub size: i64,
    pub received_chunks: usize,
    pub received_bytes: i64,
}

#[derive(Debug, Deserialize)]
pub struct CompleteChunkedRequest {
    pub filename: String,
    pub content_type: Option<String>,
}

pub(crate) fn chunks_root() -> PathBuf {
    std::env::temp_dir().join("filebase-chunks")
}

pub(crate) fn chunk_dir(session_id: &str) -> PathBuf {
    chunks_root().join(session_id)
}

async fn authorize_session(
    state: &AppState,
    headers: &HeaderMap,
    session_id: &str,
) -> Result<upload_session::Model, ApiError> {
    let token = extract_bearer_or_key(headers).ok_or(ApiError::Unauthorized)?;
    let session = upload_session::Entity::find_by_id(session_id.to_string())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if session.token_hash != hash_secret(&token) {
        return Err(ApiError::Unauthorized);
    }
    if session.expires_at < Utc::now().fixed_offset() {
        return Err(ApiError::Unauthorized);
    }
    Ok(session)
}

pub async fn upload_chunk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    mut multipart: Multipart,
) -> ApiResult<Response> {
    let session = authorize_session(&state, &headers, &session_id).await?;
    if session.used_at.is_some() {
        return Err(ApiError::Conflict(
            "upload session has already been used".into(),
        ));
    }

    let mut chunk_index: Option<i32> = None;
    let mut streamed: Option<PathBuf> = None;
    let mut chunk_size = 0_u64;
    let mut chunk_hash = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "chunk" {
            if streamed.is_some() {
                return Err(ApiError::Validation(
                    "multipart request must contain one chunk field".into(),
                ));
            }
            let part = stream_file_field(field, state.config.max_upload_size).await?;
            streamed = Some(part.temp_path);
            chunk_size = part.size;
            chunk_hash = part.hash;
        } else if name == "chunk_index" {
            let value = field
                .text()
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            let parsed = match value.trim().parse::<i32>() {
                Ok(parsed) => parsed,
                Err(_) => {
                    cleanup_optional(&streamed);
                    return Err(ApiError::Validation(
                        "chunk_index must be an integer".into(),
                    ));
                }
            };
            if parsed < 0 {
                cleanup_optional(&streamed);
                return Err(ApiError::Validation(
                    "chunk_index must be zero or greater".into(),
                ));
            }
            chunk_index = Some(parsed);
        }
    }

    let chunk_index = chunk_index.ok_or_else(|| {
        cleanup_optional(&streamed);
        ApiError::Validation("chunk_index is required".into())
    })?;
    let temp_path =
        streamed.ok_or_else(|| ApiError::Validation("multipart chunk field is required".into()))?;
    if chunk_size == 0 {
        cleanup_optional(&Some(temp_path));
        return Err(ApiError::Validation("chunk must not be empty".into()));
    }

    let existing = upload_chunk::Entity::find()
        .filter(upload_chunk::Column::SessionId.eq(session.id.clone()))
        .all(&state.db)
        .await?;
    let other_bytes: i64 = existing
        .iter()
        .filter(|row| row.chunk_index != chunk_index)
        .map(|row| row.size)
        .sum();
    let total = other_bytes
        .checked_add(i64::try_from(chunk_size).unwrap_or(i64::MAX))
        .ok_or_else(|| ApiError::Validation("chunk is too large".into()))?;
    if total > session.max_file_size {
        cleanup_optional(&Some(temp_path));
        return Err(ApiError::Validation(
            "chunks exceed upload session max_file_size".into(),
        ));
    }

    let dir = chunk_dir(&session.id);
    fs::create_dir_all(&dir)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("create chunk directory: {e}")))?;
    let target = dir.join(format!("{chunk_index}.part"));
    move_file(&temp_path, &target).await?;

    upload_chunk::Entity::delete_many()
        .filter(upload_chunk::Column::SessionId.eq(session.id.clone()))
        .filter(upload_chunk::Column::ChunkIndex.eq(chunk_index))
        .exec(&state.db)
        .await?;
    upload_chunk::ActiveModel {
        id: Set(new_id("chunk")),
        session_id: Set(session.id.clone()),
        chunk_index: Set(chunk_index),
        size: Set(i64::try_from(chunk_size).unwrap_or(i64::MAX)),
        hash: Set(chunk_hash),
        temp_path: Set(target.to_string_lossy().to_string()),
        created_at: Set(Utc::now().into()),
    }
    .insert(&state.db)
    .await?;

    let rows = upload_chunk::Entity::find()
        .filter(upload_chunk::Column::SessionId.eq(session.id.clone()))
        .order_by_asc(upload_chunk::Column::ChunkIndex)
        .all(&state.db)
        .await?;
    let received_bytes: i64 = rows.iter().map(|row| row.size).sum();

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "data": ChunkAcceptedView {
                session_id: session.id,
                chunk_index,
                size: i64::try_from(chunk_size).unwrap_or(i64::MAX),
                received_chunks: rows.len(),
                received_bytes,
            }
        })),
    )
        .into_response())
}

pub async fn list_chunks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> ApiResult<Response> {
    let session = authorize_session(&state, &headers, &session_id).await?;
    let rows = upload_chunk::Entity::find()
        .filter(upload_chunk::Column::SessionId.eq(session.id.clone()))
        .order_by_asc(upload_chunk::Column::ChunkIndex)
        .all(&state.db)
        .await?;
    let received_chunks: Vec<i32> = rows.iter().map(|row| row.chunk_index).collect();
    let received_bytes: i64 = rows.iter().map(|row| row.size).sum();

    Ok(Json(json!({
        "data": ChunkStatusView {
            session_id: session.id,
            project_id: session.project_id,
            preset_id: session.preset_id,
            used: session.used_at.is_some(),
            expires_at: session.expires_at.to_rfc3339(),
            max_file_size: session.max_file_size,
            chunk_size: state.config.upload_chunk_size,
            received_chunks,
            received_bytes,
        }
    }))
    .into_response())
}

pub async fn abort_chunks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> ApiResult<Response> {
    let session = authorize_session(&state, &headers, &session_id).await?;
    if session.used_at.is_some() {
        return Err(ApiError::Conflict(
            "upload session has already been used".into(),
        ));
    }
    let removed = delete_session_chunks(&state, &session.id).await?;
    Ok(Json(json!({ "data": { "removedChunks": removed } })).into_response())
}

pub async fn complete_chunked_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(payload): Json<CompleteChunkedRequest>,
) -> ApiResult<Response> {
    let session = authorize_session(&state, &headers, &session_id).await?;
    if session.used_at.is_some() {
        return Err(ApiError::Conflict(
            "upload session has already been used".into(),
        ));
    }
    if payload.filename.trim().is_empty() {
        return Err(ApiError::Validation("filename is required".into()));
    }

    let now = Utc::now();
    let reserved = upload_session::Entity::update_many()
        .col_expr(
            upload_session::Column::UsedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(upload_session::Column::Id.eq(session.id.clone()))
        .filter(upload_session::Column::UsedAt.is_null())
        .filter(upload_session::Column::ExpiresAt.gte(now.fixed_offset()))
        .exec(&state.db)
        .await?;
    if reserved.rows_affected != 1 {
        return Err(ApiError::Conflict(
            "upload session has already been used".into(),
        ));
    }

    let result = assemble_and_process(&state, &session, &payload).await;
    match result {
        Ok(file) => {
            delete_session_chunks(&state, &session.id).await?;
            audit::record(
                &state,
                AuditEvent::system("upload.chunked_succeeded")
                    .with_project(&session.project_id)
                    .with_resource("upload_session", &session.id)
                    .with_metadata(json!({ "fileId": file.id, "path": file.path, "chunks": true })),
            )
            .await?;
            Ok((StatusCode::CREATED, Json(json!({ "data": file }))).into_response())
        }
        Err(error) => {
            let mut active = session.clone().into_active_model();
            active.used_at = Set(None);
            let _ = active.update(&state.db).await;
            audit::record_best_effort(
                &state,
                AuditEvent::system("upload.chunked_failed")
                    .with_status("failure")
                    .with_project(&session.project_id)
                    .with_resource("upload_session", &session.id)
                    .with_metadata(json!({ "error": error.to_string() })),
            )
            .await;
            Err(error)
        }
    }
}

async fn assemble_and_process(
    state: &AppState,
    session: &upload_session::Model,
    payload: &CompleteChunkedRequest,
) -> Result<crate::routes::uploads::FileView, ApiError> {
    let preset = upload_preset::Entity::find_by_id(session.preset_id.clone())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if preset.project_id != session.project_id {
        return Err(ApiError::Forbidden);
    }

    let rows = upload_chunk::Entity::find()
        .filter(upload_chunk::Column::SessionId.eq(session.id.clone()))
        .order_by_asc(upload_chunk::Column::ChunkIndex)
        .all(&state.db)
        .await?;
    if rows.is_empty() {
        return Err(ApiError::Validation("no chunks were uploaded".into()));
    }
    for (expected, row) in rows.iter().enumerate() {
        if row.chunk_index != expected as i32 {
            return Err(ApiError::Validation(format!(
                "missing chunk {} before chunk {}",
                expected, row.chunk_index
            )));
        }
    }

    let assembled_path = std::env::temp_dir().join(format!(
        "filebase-assembled-{}.tmp",
        Uuid::new_v4().simple()
    ));
    let mut file = File::create(&assembled_path)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("create assembled upload: {e}")))?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut magic_bytes = Vec::with_capacity(16);

    for row in &rows {
        let bytes = match fs::read(&row.temp_path).await {
            Ok(bytes) => bytes,
            Err(e) => {
                let _ = fs::remove_file(&assembled_path).await;
                return Err(ApiError::Validation(format!(
                    "chunk {} is no longer available: {e}",
                    row.chunk_index
                )));
            }
        };
        size = size.saturating_add(bytes.len() as u64);
        if size > state.config.max_upload_size {
            let _ = fs::remove_file(&assembled_path).await;
            return Err(ApiError::Validation(
                "assembled file exceeds server max upload size".into(),
            ));
        }
        if magic_bytes.len() < 16 {
            let remaining = 16 - magic_bytes.len();
            magic_bytes.extend_from_slice(&bytes[..bytes.len().min(remaining)]);
        }
        hasher.update(&bytes);
        if let Err(e) = file.write_all(&bytes).await {
            let _ = fs::remove_file(&assembled_path).await;
            return Err(ApiError::Internal(anyhow::anyhow!(
                "write assembled upload: {e}"
            )));
        }
    }
    if let Err(e) = file.flush().await {
        let _ = fs::remove_file(&assembled_path).await;
        return Err(ApiError::Internal(anyhow::anyhow!(
            "flush assembled upload: {e}"
        )));
    }
    drop(file);

    if size == 0 {
        let _ = fs::remove_file(&assembled_path).await;
        return Err(ApiError::Validation("assembled file is empty".into()));
    }

    let input = UploadedPart {
        temp_path: assembled_path,
        size,
        hash: hex_digest_from_hasher(hasher),
        magic_bytes,
        filename: payload.filename.clone(),
        content_type: payload.content_type.clone(),
        preset_id: Some(session.preset_id.clone()),
        preset: None,
        project_id: Some(session.project_id.clone()),
    };
    let result = process_upload(state, &preset, &input, Some(session.clone())).await;
    cleanup_upload_temp(&input).await?;
    result
}

pub(crate) async fn delete_session_chunks(
    state: &AppState,
    session_id: &str,
) -> Result<u64, ApiError> {
    let rows = upload_chunk::Entity::find()
        .filter(upload_chunk::Column::SessionId.eq(session_id.to_string()))
        .all(&state.db)
        .await?;
    let removed = rows.len() as u64;
    for row in rows {
        let _ = fs::remove_file(&row.temp_path).await;
    }
    upload_chunk::Entity::delete_many()
        .filter(upload_chunk::Column::SessionId.eq(session_id.to_string()))
        .exec(&state.db)
        .await?;
    let _ = fs::remove_dir_all(chunk_dir(session_id)).await;
    Ok(removed)
}

async fn move_file(source: &PathBuf, target: &PathBuf) -> Result<(), ApiError> {
    match fs::rename(source, target).await {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, target)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("store chunk: {e}")))?;
            let _ = fs::remove_file(source).await;
            Ok(())
        }
    }
}

fn cleanup_optional(path: &Option<PathBuf>) {
    if let Some(path) = path {
        let _ = std::fs::remove_file(path);
    }
}
