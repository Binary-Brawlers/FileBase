use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use serde_json::json;

use crate::entities::project;
use crate::error::ApiResult;
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ProjectView {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
}

impl From<project::Model> for ProjectView {
    fn from(p: project::Model) -> Self {
        Self {
            id: p.id,
            name: p.name,
            slug: p.slug,
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

pub async fn list(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Response> {
    let rows = project::Entity::find()
        .filter(project::Column::UserId.eq(auth.claims.sub.clone()))
        .order_by_asc(project::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let view: Vec<ProjectView> = rows.into_iter().map(ProjectView::from).collect();
    Ok(Json(json!({ "data": view })).into_response())
}
