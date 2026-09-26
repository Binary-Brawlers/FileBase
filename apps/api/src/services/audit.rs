use axum::http::HeaderMap;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{json, Value as JsonValue};
use uuid::Uuid;

use crate::entities::audit_log;
use crate::error::ApiResult;
use crate::services::jwt::Claims;
use crate::state::AppState;

pub struct AuditEvent<'a> {
    pub actor_type: &'a str,
    pub actor_id: Option<&'a str>,
    pub actor_email: Option<&'a str>,
    pub action: &'a str,
    pub status: &'a str,
    pub project_id: Option<&'a str>,
    pub resource_type: Option<&'a str>,
    pub resource_id: Option<&'a str>,
    pub ip_address: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: JsonValue,
}

impl<'a> AuditEvent<'a> {
    pub fn user(claims: &'a Claims, action: &'a str) -> Self {
        Self::user_identity(&claims.sub, &claims.email, action)
    }

    pub fn user_identity(user_id: &'a str, email: &'a str, action: &'a str) -> Self {
        Self {
            actor_type: "user",
            actor_id: Some(user_id),
            actor_email: Some(email),
            action,
            status: "success",
            project_id: None,
            resource_type: None,
            resource_id: None,
            ip_address: None,
            user_agent: None,
            metadata: json!({}),
        }
    }

    pub fn api_key(api_key_id: &'a str, project_id: Option<&'a str>, action: &'a str) -> Self {
        Self {
            actor_type: "api_key",
            actor_id: Some(api_key_id),
            actor_email: None,
            action,
            status: "success",
            project_id,
            resource_type: None,
            resource_id: None,
            ip_address: None,
            user_agent: None,
            metadata: json!({}),
        }
    }

    pub fn system(action: &'a str) -> Self {
        Self {
            actor_type: "system",
            actor_id: None,
            actor_email: None,
            action,
            status: "success",
            project_id: None,
            resource_type: None,
            resource_id: None,
            ip_address: None,
            user_agent: None,
            metadata: json!({}),
        }
    }

    pub fn with_status(mut self, status: &'a str) -> Self {
        self.status = status;
        self
    }

    pub fn with_project(mut self, project_id: &'a str) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub fn with_resource(mut self, resource_type: &'a str, resource_id: &'a str) -> Self {
        self.resource_type = Some(resource_type);
        self.resource_id = Some(resource_id);
        self
    }

    pub fn with_actor_email(mut self, email: &'a str) -> Self {
        self.actor_email = Some(email);
        self
    }

    pub fn with_metadata(mut self, metadata: JsonValue) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_client(mut self, client: &'a ClientContext) -> Self {
        self.ip_address = client.ip_address.as_deref();
        self.user_agent = client.user_agent.as_deref();
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClientContext {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

pub fn client_context(headers: &HeaderMap) -> ClientContext {
    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        });
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.chars().take(400).collect::<String>());
    ClientContext {
        ip_address,
        user_agent,
    }
}

pub async fn record(state: &AppState, event: AuditEvent<'_>) -> ApiResult<()> {
    tracing::info!(
        action = %event.action,
        status = %event.status,
        actor_type = %event.actor_type,
        actor_id = event.actor_id.unwrap_or("-"),
        project_id = event.project_id.unwrap_or("-"),
        "audit"
    );
    audit_log::ActiveModel {
        id: Set(format!("audit_{}", Uuid::new_v4().simple())),
        actor_type: Set(event.actor_type.to_string()),
        actor_id: Set(event.actor_id.map(str::to_string)),
        actor_email: Set(event.actor_email.map(str::to_string)),
        project_id: Set(event.project_id.map(str::to_string)),
        action: Set(event.action.to_string()),
        resource_type: Set(event.resource_type.map(str::to_string)),
        resource_id: Set(event.resource_id.map(str::to_string)),
        status: Set(event.status.to_string()),
        ip_address: Set(event.ip_address.map(str::to_string)),
        user_agent: Set(event.user_agent.map(str::to_string)),
        metadata_json: Set(event.metadata),
        created_at: Set(Utc::now().into()),
    }
    .insert(&state.db)
    .await?;
    Ok(())
}

pub async fn record_best_effort(state: &AppState, event: AuditEvent<'_>) {
    if let Err(error) = record(state, event).await {
        tracing::error!(error = ?error, "failed to persist audit log entry");
    }
}

pub async fn prune_older_than(state: &AppState, days: i64) -> ApiResult<u64> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let cutoff = Utc::now() - chrono::Duration::days(days.max(1));
    let result = audit_log::Entity::delete_many()
        .filter(audit_log::Column::CreatedAt.lt(cutoff))
        .exec(&state.db)
        .await?;
    Ok(result.rows_affected)
}
