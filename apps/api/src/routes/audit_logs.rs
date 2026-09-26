use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::entities::audit_log;
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::routes::files::{owned_project_ids, parse_date_filter};
use crate::state::AppState;

const DEFAULT_LIMIT: u64 = 50;
const MAX_LIMIT: u64 = 200;

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub project_id: Option<String>,
    pub actor_id: Option<String>,
    pub action: Option<String>,
    pub status: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogView {
    pub id: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub actor_email: Option<String>,
    pub project_id: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub status: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

impl From<audit_log::Model> for AuditLogView {
    fn from(model: audit_log::Model) -> Self {
        Self {
            id: model.id,
            actor_type: model.actor_type,
            actor_id: model.actor_id,
            actor_email: model.actor_email,
            project_id: model.project_id,
            action: model.action,
            resource_type: model.resource_type,
            resource_id: model.resource_id,
            status: model.status,
            ip_address: model.ip_address,
            user_agent: model.user_agent,
            metadata: model.metadata_json,
            created_at: model.created_at.to_rfc3339(),
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AuditLogQuery>,
) -> ApiResult<Response> {
    let project_ids = owned_project_ids(&state, &auth.claims.sub).await?;
    if let Some(project_id) = &query.project_id {
        if !project_ids.iter().any(|id| id == project_id) {
            return Err(ApiError::Forbidden);
        }
    }
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = query.offset.unwrap_or(0);
    let from = parse_date_filter(query.from.as_deref())?;
    let to = parse_date_filter(query.to.as_deref())?;

    let mut find = audit_log::Entity::find();
    if let Some(project_id) = &query.project_id {
        find = find.filter(audit_log::Column::ProjectId.eq(project_id.clone()));
    } else if !project_ids.is_empty() {
        find = find.filter(
            Condition::any()
                .add(audit_log::Column::ProjectId.is_in(project_ids))
                .add(audit_log::Column::ActorId.eq(auth.claims.sub.clone())),
        );
    } else {
        find = find.filter(audit_log::Column::ActorId.eq(auth.claims.sub.clone()));
    }
    if let Some(actor_id) = &query.actor_id {
        find = find.filter(audit_log::Column::ActorId.eq(actor_id.clone()));
    }
    if let Some(action) = &query.action {
        find = find.filter(audit_log::Column::Action.eq(action.clone()));
    }
    if let Some(status) = &query.status {
        find = find.filter(audit_log::Column::Status.eq(status.clone()));
    }
    if let Some(from) = from {
        find = find.filter(audit_log::Column::CreatedAt.gte(from));
    }
    if let Some(to) = to {
        find = find.filter(audit_log::Column::CreatedAt.lte(to));
    }

    let rows = find
        .order_by_desc(audit_log::Column::CreatedAt)
        .offset(offset)
        .limit(limit)
        .all(&state.db)
        .await?;
    let views: Vec<AuditLogView> = rows.into_iter().map(AuditLogView::from).collect();

    Ok(Json(json!({
        "data": views,
        "meta": { "limit": limit, "offset": offset }
    }))
    .into_response())
}
