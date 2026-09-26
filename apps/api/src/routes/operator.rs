use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use axum::{
    extract::{Path as AxumPath, Query, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use filebase_core::jobs::JobQueue;
use filebase_migration::MigratorTrait;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, PaginatorTrait, QueryFilter,
    Statement,
};
use serde::Deserialize;
use serde_json::json;

use crate::entities::{file, project, project_member, upload_chunk, upload_session};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::routes::transform::transform_cache_dir;
use crate::routes::upload_chunks::{chunks_root, delete_session_chunks};
use crate::services::audit;
use crate::state::AppState;

const MAX_WALK_ENTRIES: usize = 200_000;

#[derive(Debug, Deserialize)]
pub struct FailedJobsQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CleanupRequest {
    pub scope: Option<String>,
    pub older_than_hours: Option<i64>,
    pub older_than_days: Option<i64>,
}

pub async fn require_operator(state: &AppState, user_id: &str) -> ApiResult<()> {
    let owned = project::Entity::find()
        .filter(project::Column::UserId.eq(user_id.to_string()))
        .count(&state.db)
        .await?;
    if owned > 0 {
        return Ok(());
    }
    let admin_memberships = project_member::Entity::find()
        .filter(project_member::Column::UserId.eq(user_id.to_string()))
        .filter(project_member::Column::Role.is_in(["owner", "admin"]))
        .count(&state.db)
        .await?;
    if admin_memberships > 0 {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn diagnostics(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Response> {
    require_operator(&state, &auth.claims.sub).await?;
    let queue = JobQueue::new(state.redis.clone());
    let depths = queue.depths().await.ok();
    let redis_ok = depths.is_some();

    let database_ok = state.db.ping().await.is_ok();
    let database_size = scalar_i64(
        &state.db,
        "SELECT pg_database_size(current_database()) AS value",
    )
    .await;
    let users = crate::entities::user::Entity::find()
        .count(&state.db)
        .await?;
    let projects = project::Entity::find().count(&state.db).await?;
    let files = file::Entity::find().count(&state.db).await?;
    let storage_bytes = scalar_i64(
        &state.db,
        "SELECT COALESCE(SUM(size), 0)::bigint AS value FROM files",
    )
    .await
    .unwrap_or(0);
    let pending_sessions = upload_session::Entity::find()
        .filter(upload_session::Column::UsedAt.is_null())
        .count(&state.db)
        .await?;
    let chunk_rows = upload_chunk::Entity::find().count(&state.db).await?;

    let storage_path = PathBuf::from(state.config.local_storage_path.clone());
    let scan = tokio::task::spawn_blocking(move || local_storage_stats(&storage_path))
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage scan failed: {e}")))?;

    let media = json!({
        "ffmpegAvailable": filebase_video_processing::binary_available(&state.config.ffmpeg_path).await,
        "ffprobeAvailable": filebase_video_processing::binary_available(&state.config.ffprobe_path).await,
    });

    Ok(Json(json!({
        "data": {
            "version": env!("CARGO_PKG_VERSION"),
            "uptimeSeconds": state.started_at.elapsed().as_secs(),
            "database": {
                "ok": database_ok,
                "sizeBytes": database_size,
                "users": users,
                "projects": projects,
                "files": files,
                "fileBytes": storage_bytes,
            },
            "redis": {
                "ok": redis_ok,
                "pendingJobs": depths.as_ref().map(|d| d.pending),
                "processingJobs": depths.as_ref().map(|d| d.processing),
                "failedJobs": depths.as_ref().map(|d| d.failed),
            },
            "storage": {
                "localPath": state.config.local_storage_path,
                "localFiles": scan.local_files,
                "localBytes": scan.local_bytes,
                "tempFiles": scan.temp_files,
                "tempBytes": scan.temp_bytes,
                "pendingUploadSessions": pending_sessions,
                "chunkRows": chunk_rows,
                "chunkBytes": scan.chunk_bytes,
                "transformCacheFiles": scan.transform_files,
                "transformCacheBytes": scan.transform_bytes,
            },
            "media": media,
            "limits": {
                "maxUploadSize": state.config.max_upload_size,
                "uploadChunkSize": state.config.upload_chunk_size,
                "authRateLimitPerMinute": state.config.auth_rate_limit_per_minute,
                "uploadRateLimitPerMinute": state.config.upload_rate_limit_per_minute,
            }
        }
    }))
    .into_response())
}

pub async fn failed_jobs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<FailedJobsQuery>,
) -> ApiResult<Response> {
    require_operator(&state, &auth.claims.sub).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let queue = JobQueue::new(state.redis.clone());
    let jobs = queue
        .list_failed(limit)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(json!({ "data": jobs })).into_response())
}

pub async fn retry_job(
    State(state): State<AppState>,
    auth: AuthUser,
    AxumPath(job_id): AxumPath<String>,
) -> ApiResult<Response> {
    require_operator(&state, &auth.claims.sub).await?;
    let queue = JobQueue::new(state.redis.clone());
    match queue
        .retry_failed(&job_id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
    {
        Some(job) => Ok(Json(json!({ "data": job })).into_response()),
        None => Err(ApiError::NotFound),
    }
}

pub async fn delete_job(
    State(state): State<AppState>,
    auth: AuthUser,
    AxumPath(job_id): AxumPath<String>,
) -> ApiResult<Response> {
    require_operator(&state, &auth.claims.sub).await?;
    let queue = JobQueue::new(state.redis.clone());
    let removed = queue
        .remove_failed(&job_id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    if !removed {
        return Err(ApiError::NotFound);
    }
    Ok(Json(json!({ "data": { "jobId": job_id, "removed": true } })).into_response())
}

pub async fn cleanup(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CleanupRequest>,
) -> ApiResult<Response> {
    require_operator(&state, &auth.claims.sub).await?;
    let scope = payload.scope.as_deref().unwrap_or("all");
    let mut removed_count = 0_u64;
    let mut removed_bytes = 0_u64;

    if matches!(scope, "temp" | "all") {
        let hours = payload.older_than_hours.unwrap_or(1).max(0);
        let (count, bytes) = cleanup_temp_uploads(hours).await?;
        removed_count += count;
        removed_bytes += bytes;
    }
    if matches!(scope, "sessions" | "all") {
        removed_count += cleanup_expired_sessions(&state).await?;
    }
    if matches!(scope, "chunks" | "all") {
        removed_count += cleanup_stale_chunks(&state).await?;
    }
    if matches!(scope, "transform_cache" | "all") {
        let (count, bytes) = cleanup_transform_cache(payload.older_than_hours.unwrap_or(0)).await?;
        removed_count += count;
        removed_bytes += bytes;
    }
    if scope == "audit_logs" {
        let days = payload.older_than_days.unwrap_or(90).max(1);
        removed_count += audit::prune_older_than(&state, days).await?;
    }
    if !matches!(
        scope,
        "temp" | "sessions" | "chunks" | "transform_cache" | "audit_logs" | "all"
    ) {
        return Err(ApiError::Validation(format!(
            "unknown cleanup scope: {scope}"
        )));
    }

    audit::record(
        &state,
        crate::services::audit::AuditEvent::user_identity(
            &auth.claims.sub,
            &auth.claims.email,
            "maintenance.cleanup",
        )
        .with_metadata(json!({
            "scope": scope,
            "removedCount": removed_count,
            "removedBytes": removed_bytes
        })),
    )
    .await?;

    Ok(Json(json!({
        "data": {
            "scope": scope,
            "removedCount": removed_count,
            "removedBytes": removed_bytes
        }
    }))
    .into_response())
}

pub async fn upgrade_check(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Response> {
    require_operator(&state, &auth.claims.sub).await?;

    let pending = filebase_migration::Migrator::get_pending_migrations(&state.db).await?;
    let applied = filebase_migration::Migrator::get_applied_migrations(&state.db).await?;
    let pending_names: Vec<String> = pending
        .iter()
        .map(|migration| migration.name().to_string())
        .collect();

    let mut checks: Vec<serde_json::Value> = Vec::new();
    checks.push(upgrade_check_entry(
        "database_migrations",
        if pending.is_empty() { "ok" } else { "warning" },
        format!("{} applied, {} pending", applied.len(), pending.len()),
    ));
    checks.push(upgrade_check_entry(
        "jwt_secret_strength",
        if state.config.jwt_secret.len() >= 32 {
            "ok"
        } else {
            "warning"
        },
        format!("JWT_SECRET is {} characters", state.config.jwt_secret.len()),
    ));
    checks.push(upgrade_check_entry(
        "encryption_key_strength",
        if state.config.encryption_key.len() >= 16 {
            "ok"
        } else {
            "warning"
        },
        format!(
            "ENCRYPTION_KEY is {} characters",
            state.config.encryption_key.len()
        ),
    ));

    let storage_path = PathBuf::from(state.config.local_storage_path.clone());
    let storage_path_for_scan = storage_path.clone();
    let (writable, free_bytes) = tokio::task::spawn_blocking(move || {
        let writable = std::fs::create_dir_all(&storage_path_for_scan).is_ok();
        let free = fs2::available_space(&storage_path_for_scan).ok();
        (writable, free)
    })
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("disk check failed: {e}")))?;
    checks.push(upgrade_check_entry(
        "local_storage",
        if !writable {
            "error"
        } else if free_bytes.unwrap_or(u64::MAX) < 1024 * 1024 * 1024 {
            "warning"
        } else {
            "ok"
        },
        match free_bytes {
            Some(free) => format!(
                "{} writable, {} free",
                storage_path.display(),
                crate::routes::operator::format_bytes(free)
            ),
            None => format!("{} writable, free space unknown", storage_path.display()),
        },
    ));

    let queue = JobQueue::new(state.redis.clone());
    checks.push(upgrade_check_entry(
        "redis",
        if queue.depths().await.is_ok() {
            "ok"
        } else {
            "warning"
        },
        "Redis job queue connectivity".to_string(),
    ));

    let ffmpeg = filebase_video_processing::binary_available(&state.config.ffmpeg_path).await;
    checks.push(upgrade_check_entry(
        "ffmpeg",
        if ffmpeg { "ok" } else { "warning" },
        format!("ffmpeg at {}", state.config.ffmpeg_path.display()),
    ));

    let cache_dir = transform_cache_dir();
    let cache_writable = std::fs::create_dir_all(&cache_dir).is_ok();
    checks.push(upgrade_check_entry(
        "transform_cache",
        if cache_writable { "ok" } else { "warning" },
        format!("{} writable", cache_dir.display()),
    ));

    let upgrade_safe = checks
        .iter()
        .all(|check| check.get("status").and_then(|s| s.as_str()) != Some("error"));

    Ok(Json(json!({
        "data": {
            "currentVersion": env!("CARGO_PKG_VERSION"),
            "upgradeSafe": upgrade_safe,
            "appliedMigrations": applied.len(),
            "pendingMigrations": pending.len(),
            "pendingMigrationNames": pending_names,
            "checks": checks,
        }
    }))
    .into_response())
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

fn upgrade_check_entry(name: &str, status: &str, message: String) -> serde_json::Value {
    json!({ "name": name, "status": status, "message": message })
}

async fn cleanup_temp_uploads(older_than_hours: i64) -> ApiResult<(u64, u64)> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(
            (older_than_hours as u64).saturating_mul(3600),
        ))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let dir = std::env::temp_dir();
    let entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(_) => return Ok((0, 0)),
    };
    let mut count = 0_u64;
    let mut bytes = 0_u64;
    let mut entries = entries;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("read temp dir: {e}")))?
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("filebase-upload-") && !name.starts_with("filebase-assembled-") {
            continue;
        }
        let Ok(metadata) = entry.metadata().await else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified > cutoff {
            continue;
        }
        let size = metadata.len();
        if tokio::fs::remove_file(entry.path()).await.is_ok() {
            count += 1;
            bytes += size;
        }
    }
    Ok((count, bytes))
}

async fn cleanup_expired_sessions(state: &AppState) -> ApiResult<u64> {
    let now = Utc::now();
    let expired = upload_session::Entity::find()
        .filter(upload_session::Column::ExpiresAt.lt(now.fixed_offset()))
        .filter(upload_session::Column::UsedAt.is_null())
        .all(&state.db)
        .await?;
    let mut removed = 0_u64;
    for session in expired {
        let _ = delete_session_chunks(state, &session.id).await;
        upload_session::Entity::delete_by_id(session.id)
            .exec(&state.db)
            .await?;
        removed += 1;
    }
    Ok(removed)
}

async fn cleanup_stale_chunks(state: &AppState) -> ApiResult<u64> {
    let mut removed = 0_u64;
    let rows = upload_chunk::Entity::find().all(&state.db).await?;
    for row in rows {
        if !Path::new(&row.temp_path).exists() {
            upload_chunk::Entity::delete_by_id(row.id)
                .exec(&state.db)
                .await?;
            removed += 1;
        }
    }
    let root = chunks_root();
    if let Ok(mut entries) = tokio::fs::read_dir(&root).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let exists = upload_session::Entity::find_by_id(name.clone())
                .count(&state.db)
                .await
                .unwrap_or(0)
                > 0;
            if !exists && tokio::fs::remove_dir_all(entry.path()).await.is_ok() {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

async fn scalar_i64(db: &sea_orm::DatabaseConnection, sql: &str) -> Option<i64> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [],
        ))
        .await
        .ok()?
        .or(None)?;
    row.try_get::<i64>("", "value").ok()
}

struct StorageScan {
    local_files: u64,
    local_bytes: u64,
    temp_files: u64,
    temp_bytes: u64,
    chunk_bytes: u64,
    transform_files: u64,
    transform_bytes: u64,
}

fn local_storage_stats(path: &Path) -> StorageScan {
    let (local_files, local_bytes) = walk_dir(path, MAX_WALK_ENTRIES);
    let temp_dir = std::env::temp_dir();
    let mut temp_files = 0_u64;
    let mut temp_bytes = 0_u64;
    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("filebase-upload-") && !name.starts_with("filebase-assembled-") {
                continue;
            }
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    temp_files += 1;
                    temp_bytes += metadata.len();
                }
            }
        }
    }
    let (_, chunk_bytes) = walk_dir(&chunks_root(), MAX_WALK_ENTRIES);
    let (transform_files, transform_bytes) = walk_dir(&transform_cache_dir(), MAX_WALK_ENTRIES);
    StorageScan {
        local_files,
        local_bytes,
        temp_files,
        temp_bytes,
        chunk_bytes,
        transform_files,
        transform_bytes,
    }
}

async fn cleanup_transform_cache(older_than_hours: i64) -> ApiResult<(u64, u64)> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(
            (older_than_hours.max(0) as u64).saturating_mul(3600),
        ))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let dir = transform_cache_dir();
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(_) => return Ok((0, 0)),
    };
    let mut count = 0_u64;
    let mut bytes = 0_u64;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("read transform cache: {e}")))?
    {
        let Ok(metadata) = entry.metadata().await else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified > cutoff {
            continue;
        }
        let size = metadata.len();
        let is_binary = entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "bin");
        if tokio::fs::remove_file(entry.path()).await.is_ok() && is_binary {
            count += 1;
            bytes += size;
        }
    }
    Ok((count, bytes))
}

fn walk_dir(root: &Path, max_entries: usize) -> (u64, u64) {
    let mut files = 0_u64;
    let mut bytes = 0_u64;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if files as usize >= max_entries {
                return (files, bytes);
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                stack.push(entry.path());
            } else {
                files += 1;
                bytes += metadata.len();
            }
        }
    }
    (files, bytes)
}
