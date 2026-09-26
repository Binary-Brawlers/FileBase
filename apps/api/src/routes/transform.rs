use std::path::{Path as FsPath, PathBuf};
use std::time::{Duration, SystemTime};

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use filebase_image_processing::{transform_image, ImageProcessingPreset, ResizeMode};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};

use crate::entities::{file, storage_connection, upload_preset};
use crate::error::{ApiError, ApiResult};
use crate::services::storage_factory;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransformQuery {
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub fit: Option<String>,
    pub format: Option<String>,
    pub q: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheMeta {
    mime_type: String,
    extension: String,
    width: u32,
    height: u32,
}

pub(crate) fn transform_cache_dir() -> PathBuf {
    std::env::temp_dir().join("filebase-transform-cache")
}

pub async fn fetch(
    State(state): State<AppState>,
    Path((preset_key, file_id)): Path<(String, String)>,
    Query(query): Query<TransformQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    validate_query(&state, &query)?;

    let record = file::Entity::find_by_id(file_id)
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if record.status != "uploaded" {
        return Err(ApiError::NotFound);
    }

    let preset = find_preset(&state, &preset_key, &record.project_id).await?;
    if record.project_id != preset.project_id {
        return Err(ApiError::NotFound);
    }
    let parsed: ImageProcessingPreset = serde_json::from_value(preset.transformations_json.clone())
        .map_err(|e| ApiError::Validation(e.to_string()))?;
    if !parsed.url_transforms.enabled {
        return Err(ApiError::Forbidden);
    }

    let key = cache_key(&preset.id, &record.id, &record.hash, &query);
    let etag = format!("\"{key}\"");
    if let Some(value) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
    {
        if value == etag {
            return Ok(not_modified(&etag));
        }
    }

    let cache_dir = transform_cache_dir();
    let cache_file = cache_dir.join(format!("{key}.bin"));
    let meta_file = cache_dir.join(format!("{key}.json"));
    if let Some((bytes, meta)) = read_cache(
        &cache_file,
        &meta_file,
        state.config.transform_cache_ttl_seconds,
    )
    .await
    {
        return Ok(transform_response(
            bytes,
            &meta,
            &etag,
            state.config.transform_cache_ttl_seconds,
            "hit",
        ));
    }

    let merged = build_transformations(&preset.transformations_json, &query)?;
    let connection = storage_connection::Entity::find_by_id(record.storage_connection_id.clone())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    let adapter = storage_factory::build_adapter(&connection, &state.config.encryption_key, None)?;
    let source = adapter
        .download(&record.path)
        .await
        .map_err(|error| match error {
            filebase_storage::StorageError::Backend(message) if message.contains("not found") => {
                ApiError::NotFound
            }
            other => ApiError::Internal(anyhow::anyhow!(other)),
        })?;
    if source.len() as u64 > state.config.max_upload_size {
        return Err(ApiError::Validation(
            "source file is too large for dynamic transforms".into(),
        ));
    }

    let transformed = transform_image(&source, &record.mime_type, &record.extension, &merged)
        .map_err(|e| ApiError::Validation(e.to_string()))?
        .ok_or_else(|| {
            ApiError::Validation("file type does not support dynamic transforms".into())
        })?;

    let meta = CacheMeta {
        mime_type: transformed.mime_type,
        extension: transformed.extension,
        width: transformed.width,
        height: transformed.height,
    };
    if tokio::fs::create_dir_all(&cache_dir).await.is_ok() {
        let _ = tokio::fs::write(&cache_file, &transformed.bytes).await;
        if let Ok(serialized) = serde_json::to_vec(&meta) {
            let _ = tokio::fs::write(&meta_file, serialized).await;
        }
    }

    Ok(transform_response(
        transformed.bytes,
        &meta,
        &etag,
        state.config.transform_cache_ttl_seconds,
        "miss",
    ))
}

async fn find_preset(
    state: &AppState,
    key: &str,
    project_id: &str,
) -> Result<upload_preset::Model, ApiError> {
    if let Some(preset) = upload_preset::Entity::find_by_id(key.to_string())
        .one(&state.db)
        .await?
    {
        return Ok(preset);
    }
    upload_preset::Entity::find()
        .filter(upload_preset::Column::Name.eq(key.to_string()))
        .filter(upload_preset::Column::ProjectId.eq(project_id.to_string()))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)
}

fn validate_query(state: &AppState, query: &TransformQuery) -> Result<(), ApiError> {
    let max = state.config.transform_max_dimension;
    if let Some(width) = query.w {
        if width == 0 || width > max {
            return Err(ApiError::Validation(format!(
                "w must be between 1 and {max}"
            )));
        }
    }
    if let Some(height) = query.h {
        if height == 0 || height > max {
            return Err(ApiError::Validation(format!(
                "h must be between 1 and {max}"
            )));
        }
    }
    if query.fit.is_some() && query.w.is_none() && query.h.is_none() {
        return Err(ApiError::Validation("fit requires w or h".into()));
    }
    if let Some(fit) = query.fit.as_deref() {
        if fit != "fit" && fit != "fill" {
            return Err(ApiError::Validation("fit must be fit or fill".into()));
        }
    }
    if let Some(format) = query.format.as_deref() {
        if !matches!(format, "original" | "jpeg" | "png" | "webp" | "avif") {
            return Err(ApiError::Validation(
                "format must be original, jpeg, png, webp, or avif".into(),
            ));
        }
    }
    if let Some(quality) = query.q {
        if !(1..=100).contains(&quality) {
            return Err(ApiError::Validation("q must be between 1 and 100".into()));
        }
    }
    Ok(())
}

fn build_transformations(
    preset: &JsonValue,
    query: &TransformQuery,
) -> Result<JsonValue, ApiError> {
    let mut merged = preset.as_object().cloned().unwrap_or_default();
    merged.insert("enabled".to_string(), json!(true));
    if query.w.is_some() || query.h.is_some() {
        let mode = match query.fit.as_deref() {
            Some("fill") => ResizeMode::Fill,
            _ => ResizeMode::Fit,
        };
        merged.insert(
            "resize".to_string(),
            json!({ "width": query.w, "height": query.h, "mode": mode }),
        );
    }
    if let Some(format) = &query.format {
        merged.insert("format".to_string(), json!(format));
    }
    if let Some(quality) = query.q {
        merged.insert("quality".to_string(), json!(quality));
    }
    merged.remove("thumbnail");
    merged.remove("thumbnails");
    merged.remove("preserve_original");
    Ok(JsonValue::Object(merged))
}

fn cache_key(preset_id: &str, file_id: &str, file_hash: &str, query: &TransformQuery) -> String {
    let mut hasher = Sha256::new();
    hasher.update(preset_id.as_bytes());
    hasher.update(b":");
    hasher.update(file_id.as_bytes());
    hasher.update(b":");
    hasher.update(file_hash.as_bytes());
    hasher.update(b":");
    hasher.update(serde_json::to_vec(query).unwrap_or_default());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn read_cache(
    cache_file: &FsPath,
    meta_file: &FsPath,
    ttl_seconds: u64,
) -> Option<(Vec<u8>, CacheMeta)> {
    let metadata = tokio::fs::metadata(cache_file).await.ok()?;
    let modified = metadata.modified().ok()?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);
    if age.as_secs() > ttl_seconds {
        return None;
    }
    let bytes = tokio::fs::read(cache_file).await.ok()?;
    let meta: CacheMeta = serde_json::from_slice(&tokio::fs::read(meta_file).await.ok()?).ok()?;
    Some((bytes, meta))
}

fn transform_response(
    bytes: Vec<u8>,
    meta: &CacheMeta,
    etag: &str,
    ttl_seconds: u64,
    cache_status: &'static str,
) -> Response {
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        meta.mime_type
            .parse()
            .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
    );
    if let Ok(value) = etag.parse() {
        headers.insert(header::ETAG, value);
    }
    if let Ok(value) = format!("public, max-age={ttl_seconds}").parse() {
        headers.insert(header::CACHE_CONTROL, value);
    }
    headers.insert("x-filebase-transform-cache", cache_status.parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("inline; filename=\"transformed.{}\"", meta.extension)
            .parse()
            .unwrap(),
    );
    response
}

fn not_modified(etag: &str) -> Response {
    let mut response = StatusCode::NOT_MODIFIED.into_response();
    if let Ok(value) = etag.parse() {
        response.headers_mut().insert(header::ETAG, value);
    }
    response
}
