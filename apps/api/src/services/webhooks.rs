use chrono::Utc;
use filebase_core::jobs::{JobEnvelope, JobKind, JobQueue};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::entities::{upload_log, webhook};
use crate::error::ApiError;
use crate::state::AppState;

pub async fn emit_file_event(
    state: &AppState,
    project_id: &str,
    file_id: Option<&str>,
    event: &str,
    data: Value,
) -> Result<(), ApiError> {
    let hooks = webhook::Entity::find()
        .filter(webhook::Column::ProjectId.eq(project_id.to_string()))
        .filter(webhook::Column::IsActive.eq(true))
        .all(&state.db)
        .await?;

    if hooks.is_empty() {
        return Ok(());
    }

    let queue = JobQueue::new(state.redis.clone());
    for hook in hooks.into_iter().filter(|hook| subscribes_to(hook, event)) {
        let payload = json!({
            "webhookId": hook.id,
            "projectId": project_id,
            "fileId": file_id,
            "event": event,
            "url": hook.url,
            "secret": hook.secret,
            "createdAt": Utc::now().to_rfc3339(),
            "data": data,
        });
        let job = JobEnvelope::new(JobKind::SendWebhook, payload).with_max_attempts(5);
        queue
            .enqueue(&job)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
        write_delivery_log(
            state,
            project_id,
            file_id,
            event,
            "queued",
            None,
            json!({
                "jobId": job.id,
                "webhookId": hook.id,
            }),
        )
        .await?;
    }

    Ok(())
}

fn subscribes_to(hook: &webhook::Model, event: &str) -> bool {
    hook.events
        .as_array()
        .is_some_and(|events| events.iter().any(|value| value.as_str() == Some(event)))
}

async fn write_delivery_log(
    state: &AppState,
    project_id: &str,
    file_id: Option<&str>,
    event: &str,
    status: &str,
    message: Option<&str>,
    metadata: Value,
) -> Result<(), ApiError> {
    upload_log::ActiveModel {
        id: Set(format!("log_{}", Uuid::new_v4().simple())),
        project_id: Set(project_id.to_string()),
        file_id: Set(file_id.map(str::to_string)),
        event: Set(format!("webhook.{event}")),
        status: Set(status.to_string()),
        message: Set(message.map(str::to_string)),
        metadata_json: Set(metadata),
        created_at: Set(Utc::now().into()),
    }
    .insert(&state.db)
    .await?;
    Ok(())
}
