use std::collections::HashSet;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::entities::{project, project_member};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRole {
    Viewer,
    Editor,
    Admin,
    Owner,
}

impl ProjectRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Editor => "editor",
            Self::Admin => "admin",
            Self::Owner => "owner",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ApiError> {
        match value {
            "viewer" => Ok(Self::Viewer),
            "editor" => Ok(Self::Editor),
            "admin" => Ok(Self::Admin),
            "owner" => Ok(Self::Owner),
            _ => Err(ApiError::Validation(
                "role must be viewer, editor, or admin".into(),
            )),
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Viewer => 0,
            Self::Editor => 1,
            Self::Admin => 2,
            Self::Owner => 3,
        }
    }

    pub fn allows(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }
}

pub async fn project_role(
    state: &AppState,
    user_id: &str,
    project_id: &str,
) -> ApiResult<ProjectRole> {
    let project = project::Entity::find_by_id(project_id.to_string())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if project.user_id == user_id {
        return Ok(ProjectRole::Owner);
    }

    let membership =
        project_member::Entity::find_by_id((project_id.to_string(), user_id.to_string()))
            .one(&state.db)
            .await?
            .ok_or(ApiError::Forbidden)?;
    ProjectRole::parse(&membership.role).map_err(|_| ApiError::Forbidden)
}

pub async fn require_project_role(
    state: &AppState,
    user_id: &str,
    project_id: &str,
    required: ProjectRole,
) -> ApiResult<ProjectRole> {
    let role = project_role(state, user_id, project_id).await?;
    if !role.allows(required) {
        return Err(ApiError::Forbidden);
    }
    Ok(role)
}

pub async fn accessible_project_ids(state: &AppState, user_id: &str) -> ApiResult<Vec<String>> {
    let owned = project::Entity::find()
        .filter(project::Column::UserId.eq(user_id.to_string()))
        .all(&state.db)
        .await?;
    let memberships = project_member::Entity::find()
        .filter(project_member::Column::UserId.eq(user_id.to_string()))
        .all(&state.db)
        .await?;

    let mut ids = HashSet::new();
    ids.extend(owned.into_iter().map(|project| project.id));
    ids.extend(memberships.into_iter().map(|member| member.project_id));
    let mut ids = ids.into_iter().collect::<Vec<_>>();
    ids.sort();
    Ok(ids)
}
