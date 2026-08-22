use std::collections::{BTreeMap, HashMap, HashSet};

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Days, Duration, FixedOffset, Utc};
use sea_orm::{ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QuerySelect};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::entities::{file, storage_connection, upload_log, upload_preset};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::routes::files::{owned_project_ids, parse_date_filter};
use crate::state::AppState;

const MAX_RANGE_DAYS: i64 = 366;

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub project_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsView {
    pub scope: AnalyticsScope,
    pub totals: AnalyticsTotals,
    pub period: PeriodMetrics,
    pub trend: Vec<TrendPoint>,
    pub mime_types: Vec<Breakdown>,
    pub storage_types: Vec<Breakdown>,
    pub folders: Vec<Breakdown>,
    pub outcomes: Vec<OutcomeBreakdown>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsScope {
    pub project_id: Option<String>,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsTotals {
    pub files: u64,
    pub storage_bytes: i64,
    pub projects: u64,
    pub folders: u64,
}

#[derive(Debug, Serialize)]
pub struct PeriodMetrics {
    pub uploads: u64,
    pub uploaded_bytes: i64,
    pub average_file_size: i64,
    pub duplicate_events: u64,
    pub failure_events: u64,
    pub success_rate: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TrendPoint {
    pub date: String,
    pub uploads: u64,
    pub bytes: i64,
}

#[derive(Debug, Serialize)]
pub struct Breakdown {
    pub key: String,
    pub count: u64,
    pub bytes: i64,
}

#[derive(Debug, Serialize)]
pub struct OutcomeBreakdown {
    pub status: String,
    pub count: u64,
}

#[derive(Debug, FromQueryResult)]
struct FileMetricRow {
    id: String,
    project_id: String,
    storage_connection_id: String,
    mime_type: String,
    folder: String,
    size: i64,
    created_at: DateTime<FixedOffset>,
}

#[derive(Debug, FromQueryResult)]
struct LogMetricRow {
    file_id: Option<String>,
    event: String,
    status: String,
    metadata_json: Value,
    created_at: DateTime<FixedOffset>,
}

#[derive(Debug, FromQueryResult)]
struct PresetFolderRow {
    project_id: String,
    folder: String,
}

pub async fn summary(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AnalyticsQuery>,
) -> ApiResult<Response> {
    let owned_ids = owned_project_ids(&state, &auth.claims.sub).await?;
    if let Some(project_id) = &query.project_id {
        if !owned_ids.iter().any(|id| id == project_id) {
            return Err(ApiError::Forbidden);
        }
    }
    let project_ids = query
        .project_id
        .as_ref()
        .map(|id| vec![id.clone()])
        .unwrap_or(owned_ids);
    let now = Utc::now();
    let to = parse_date_filter(query.to.as_deref())?.unwrap_or(now);
    let from = parse_date_filter(query.from.as_deref())?.unwrap_or(to - Duration::days(29));
    validate_range(from, to)?;

    if project_ids.is_empty() {
        return Ok(Json(json!({
            "data": empty_view(query.project_id, from, to)
        }))
        .into_response());
    }

    let files = file::Entity::find()
        .filter(file::Column::ProjectId.is_in(project_ids.clone()))
        .select_only()
        .columns([
            file::Column::Id,
            file::Column::ProjectId,
            file::Column::StorageConnectionId,
            file::Column::MimeType,
            file::Column::Folder,
            file::Column::Size,
            file::Column::CreatedAt,
        ])
        .into_model::<FileMetricRow>()
        .all(&state.db)
        .await?;
    let logs = upload_log::Entity::find()
        .filter(upload_log::Column::ProjectId.is_in(project_ids.clone()))
        .filter(upload_log::Column::CreatedAt.gte(from))
        .filter(upload_log::Column::CreatedAt.lte(to))
        .select_only()
        .columns([
            upload_log::Column::FileId,
            upload_log::Column::Event,
            upload_log::Column::Status,
            upload_log::Column::MetadataJson,
            upload_log::Column::CreatedAt,
        ])
        .into_model::<LogMetricRow>()
        .all(&state.db)
        .await?;
    let connections = storage_connection::Entity::find()
        .filter(storage_connection::Column::ProjectId.is_in(project_ids.clone()))
        .all(&state.db)
        .await?;
    let preset_folders = upload_preset::Entity::find()
        .filter(upload_preset::Column::ProjectId.is_in(project_ids.clone()))
        .select_only()
        .columns([
            upload_preset::Column::ProjectId,
            upload_preset::Column::Folder,
        ])
        .into_model::<PresetFolderRow>()
        .all(&state.db)
        .await?;

    let connection_types = connections
        .into_iter()
        .map(|connection| (connection.id, connection.r#type))
        .collect::<HashMap<_, _>>();
    let period_files = files
        .iter()
        .filter(|row| {
            let created_at = row.created_at.with_timezone(&Utc);
            created_at >= from && created_at <= to
        })
        .collect::<Vec<_>>();

    let total_storage_bytes = files.iter().map(|row| row.size).sum::<i64>();
    let file_sizes = files
        .iter()
        .map(|row| (row.id.as_str(), row.size))
        .collect::<HashMap<_, _>>();
    let successful_uploads = logs
        .iter()
        .filter(|row| row.event == "file.uploaded" && row.status == "success")
        .collect::<Vec<_>>();
    let uploaded_bytes = successful_uploads
        .iter()
        .map(|row| upload_log_bytes(row, &file_sizes))
        .sum::<i64>();
    let uploads = successful_uploads.len() as u64;
    let duplicate_events = logs
        .iter()
        .filter(|row| row.event == "file.duplicate_detected")
        .count() as u64;
    let failure_events = logs
        .iter()
        .filter(|row| {
            !row.event.starts_with("webhook.")
                && (row.status == "failed" || row.status == "rejected")
        })
        .count() as u64;
    let measured_outcomes = uploads + failure_events;
    let success_rate = (measured_outcomes > 0)
        .then(|| round_percent(uploads as f64 / measured_outcomes as f64 * 100.0));

    let mut folder_paths = HashSet::new();
    for row in &files {
        insert_folder_ancestors(&mut folder_paths, &row.project_id, &row.folder);
    }
    for preset in preset_folders {
        insert_folder_ancestors(&mut folder_paths, &preset.project_id, &preset.folder);
    }

    let view = AnalyticsView {
        scope: AnalyticsScope {
            project_id: query.project_id,
            from: from.to_rfc3339(),
            to: to.to_rfc3339(),
        },
        totals: AnalyticsTotals {
            files: files.len() as u64,
            storage_bytes: total_storage_bytes,
            projects: project_ids.len() as u64,
            folders: folder_paths.len() as u64,
        },
        period: PeriodMetrics {
            uploads,
            uploaded_bytes,
            average_file_size: if uploads > 0 {
                uploaded_bytes / uploads as i64
            } else {
                0
            },
            duplicate_events,
            failure_events,
            success_rate,
        },
        trend: build_trend(&successful_uploads, &file_sizes, from, to),
        mime_types: build_breakdown(&period_files, |row| row.mime_type.clone(), 10),
        storage_types: build_breakdown(
            &period_files,
            |row| {
                connection_types
                    .get(&row.storage_connection_id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string())
            },
            10,
        ),
        folders: build_breakdown(&period_files, |row| row.folder.clone(), 10),
        outcomes: build_outcomes(&logs),
    };

    Ok(Json(json!({ "data": view })).into_response())
}

fn empty_view(project_id: Option<String>, from: DateTime<Utc>, to: DateTime<Utc>) -> AnalyticsView {
    AnalyticsView {
        scope: AnalyticsScope {
            project_id,
            from: from.to_rfc3339(),
            to: to.to_rfc3339(),
        },
        totals: AnalyticsTotals {
            files: 0,
            storage_bytes: 0,
            projects: 0,
            folders: 0,
        },
        period: PeriodMetrics {
            uploads: 0,
            uploaded_bytes: 0,
            average_file_size: 0,
            duplicate_events: 0,
            failure_events: 0,
            success_rate: None,
        },
        trend: build_trend(&[], &HashMap::new(), from, to),
        mime_types: Vec::new(),
        storage_types: Vec::new(),
        folders: Vec::new(),
        outcomes: Vec::new(),
    }
}

fn validate_range(from: DateTime<Utc>, to: DateTime<Utc>) -> Result<(), ApiError> {
    if from > to {
        return Err(ApiError::Validation(
            "analytics from must be before to".into(),
        ));
    }
    if to - from > Duration::days(MAX_RANGE_DAYS) {
        return Err(ApiError::Validation(format!(
            "analytics range cannot exceed {MAX_RANGE_DAYS} days"
        )));
    }
    Ok(())
}

fn build_trend(
    logs: &[&LogMetricRow],
    file_sizes: &HashMap<&str, i64>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Vec<TrendPoint> {
    let mut values = BTreeMap::<String, (u64, i64)>::new();
    for row in logs {
        let date = row
            .created_at
            .with_timezone(&Utc)
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let entry = values.entry(date).or_default();
        entry.0 += 1;
        entry.1 += upload_log_bytes(row, file_sizes);
    }

    let mut date = from.date_naive();
    let last = to.date_naive();
    let mut trend = Vec::new();
    while date <= last {
        let key = date.format("%Y-%m-%d").to_string();
        let (uploads, bytes) = values.remove(&key).unwrap_or_default();
        trend.push(TrendPoint {
            date: key,
            uploads,
            bytes,
        });
        let Some(next) = date.checked_add_days(Days::new(1)) else {
            break;
        };
        date = next;
    }
    trend
}

fn upload_log_bytes(row: &LogMetricRow, file_sizes: &HashMap<&str, i64>) -> i64 {
    row.metadata_json
        .get("size")
        .and_then(Value::as_i64)
        .or_else(|| {
            row.file_id
                .as_deref()
                .and_then(|file_id| file_sizes.get(file_id).copied())
        })
        .unwrap_or(0)
}

fn build_breakdown<F>(files: &[&FileMetricRow], key_for: F, limit: usize) -> Vec<Breakdown>
where
    F: Fn(&FileMetricRow) -> String,
{
    let mut values = HashMap::<String, (u64, i64)>::new();
    for row in files {
        let key = key_for(row);
        let key = if key.trim().is_empty() {
            "unassigned".to_string()
        } else {
            key
        };
        let entry = values.entry(key).or_default();
        entry.0 += 1;
        entry.1 += row.size;
    }
    let mut breakdown = values
        .into_iter()
        .map(|(key, (count, bytes))| Breakdown { key, count, bytes })
        .collect::<Vec<_>>();
    breakdown.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| right.bytes.cmp(&left.bytes))
            .then_with(|| left.key.cmp(&right.key))
    });
    breakdown.truncate(limit);
    breakdown
}

fn build_outcomes(logs: &[LogMetricRow]) -> Vec<OutcomeBreakdown> {
    let mut values = HashMap::<String, u64>::new();
    for row in logs {
        *values.entry(row.status.clone()).or_default() += 1;
    }
    let mut outcomes = values
        .into_iter()
        .map(|(status, count)| OutcomeBreakdown { status, count })
        .collect::<Vec<_>>();
    outcomes.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.status.cmp(&right.status))
    });
    outcomes
}

fn insert_folder_ancestors(
    folders: &mut HashSet<(String, String)>,
    project_id: &str,
    folder: &str,
) {
    let mut path = String::new();
    for part in folder
        .trim()
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
    {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(part);
        folders.insert((project_id.to_string(), path.clone()));
    }
}

fn round_percent(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
