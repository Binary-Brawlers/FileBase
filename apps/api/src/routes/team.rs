use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::entities::{project, project_invitation, project_member, user};
use crate::error::{ApiError, ApiResult};
use crate::middleware::auth::AuthUser;
use crate::routes::auth::{session_cookie, PublicUser};
use crate::services::{
    audit::{self, AuditEvent},
    authorization::{require_project_role, ProjectRole},
    jwt::{issue_token, TOKEN_TTL_HOURS},
    password,
};
use crate::state::AppState;

const INVITATION_TTL_DAYS: i64 = 7;

#[derive(Debug, Serialize)]
pub struct MemberView {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: ProjectRole,
    pub joined_at: String,
}

#[derive(Debug, Serialize)]
pub struct InvitationView {
    pub id: String,
    pub project_id: String,
    pub email: String,
    pub role: ProjectRole,
    pub invited_by: String,
    pub inviter_name: String,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreatedInvitationView {
    #[serde(flatten)]
    pub invitation: InvitationView,
    pub accept_token: String,
    pub accept_url: String,
}

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub email: String,
    pub role: ProjectRole,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRequest {
    pub role: ProjectRole,
}

#[derive(Debug, Deserialize)]
pub struct AcceptInvitationRequest {
    pub token: String,
    pub name: Option<String>,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct PreviewInvitationRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct InvitationPreview {
    pub project_name: String,
    pub email: String,
    pub role: ProjectRole,
    pub expires_at: String,
    pub existing_account: bool,
}

#[derive(Debug, Serialize)]
pub struct AcceptedInvitationView {
    pub token: String,
    pub user: PublicUser,
    pub project_id: String,
    pub role: ProjectRole,
}

pub async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<String>,
) -> ApiResult<Response> {
    require_project_role(&state, &auth.claims.sub, &project_id, ProjectRole::Viewer).await?;

    let members = project_member::Entity::find()
        .filter(project_member::Column::ProjectId.eq(project_id))
        .order_by_asc(project_member::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let users = user::Entity::find()
        .filter(
            user::Column::Id.is_in(
                members
                    .iter()
                    .map(|member| member.user_id.clone())
                    .collect::<Vec<_>>(),
            ),
        )
        .all(&state.db)
        .await?;
    let users = users
        .into_iter()
        .map(|user| (user.id.clone(), user))
        .collect::<HashMap<_, _>>();

    let view = members
        .into_iter()
        .filter_map(|member| {
            let user = users.get(&member.user_id)?;
            let role = ProjectRole::parse(&member.role).ok()?;
            Some(MemberView {
                user_id: user.id.clone(),
                name: user.name.clone(),
                email: user.email.clone(),
                role,
                joined_at: member.created_at.to_rfc3339(),
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({ "data": view })).into_response())
}

pub async fn update_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((project_id, user_id)): Path<(String, String)>,
    Json(payload): Json<UpdateMemberRequest>,
) -> ApiResult<Response> {
    require_project_role(&state, &auth.claims.sub, &project_id, ProjectRole::Admin).await?;
    reject_owner_assignment(payload.role)?;

    let member = project_member::Entity::find_by_id((project_id, user_id))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if member.role == ProjectRole::Owner.as_str() {
        return Err(ApiError::Forbidden);
    }

    let mut active = member.into_active_model();
    active.role = Set(payload.role.as_str().to_string());
    active.updated_at = Set(Utc::now().into());
    let saved = active.update(&state.db).await?;
    audit::record(
        &state,
        AuditEvent::user(&auth.claims, "project_member.updated")
            .with_project(&saved.project_id)
            .with_resource("project_member", &saved.user_id)
            .with_metadata(json!({ "role": saved.role })),
    )
    .await?;
    Ok(Json(json!({ "data": { "role": payload.role } })).into_response())
}

pub async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((project_id, user_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    require_project_role(&state, &auth.claims.sub, &project_id, ProjectRole::Admin).await?;

    let member = project_member::Entity::find_by_id((project_id.clone(), user_id.clone()))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if member.role == ProjectRole::Owner.as_str() {
        return Err(ApiError::Forbidden);
    }
    project_member::Entity::delete_by_id((project_id.clone(), user_id.clone()))
        .exec(&state.db)
        .await?;
    audit::record(
        &state,
        AuditEvent::user(&auth.claims, "project_member.removed")
            .with_project(&project_id)
            .with_resource("project_member", &user_id),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn list_invitations(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<String>,
) -> ApiResult<Response> {
    require_project_role(&state, &auth.claims.sub, &project_id, ProjectRole::Admin).await?;
    let rows = project_invitation::Entity::find()
        .filter(project_invitation::Column::ProjectId.eq(project_id))
        .filter(project_invitation::Column::AcceptedAt.is_null())
        .filter(project_invitation::Column::ExpiresAt.gt(Utc::now()))
        .order_by_desc(project_invitation::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let view = invitation_views(&state, rows).await?;
    Ok(Json(json!({ "data": view })).into_response())
}

pub async fn create_invitation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<String>,
    Json(payload): Json<InviteRequest>,
) -> ApiResult<Response> {
    require_project_role(&state, &auth.claims.sub, &project_id, ProjectRole::Admin).await?;
    reject_owner_assignment(payload.role)?;
    let email = normalize_email(&payload.email)?;

    if let Some(existing_user) = user::Entity::find()
        .filter(user::Column::Email.eq(email.clone()))
        .one(&state.db)
        .await?
    {
        if project_member::Entity::find_by_id((project_id.clone(), existing_user.id))
            .one(&state.db)
            .await?
            .is_some()
        {
            return Err(ApiError::Conflict(
                "that user is already a project member".into(),
            ));
        }
    }

    let pending = project_invitation::Entity::find()
        .filter(project_invitation::Column::ProjectId.eq(project_id.clone()))
        .filter(project_invitation::Column::Email.eq(email.clone()))
        .filter(project_invitation::Column::AcceptedAt.is_null())
        .filter(project_invitation::Column::ExpiresAt.gt(Utc::now()))
        .one(&state.db)
        .await?;
    if pending.is_some() {
        return Err(ApiError::Conflict(
            "an active invitation already exists for that email".into(),
        ));
    }

    let token = generate_invitation_token();
    let now = Utc::now();
    let inserted = project_invitation::ActiveModel {
        id: Set(new_id("inv")),
        project_id: Set(project_id.clone()),
        email: Set(email),
        role: Set(payload.role.as_str().to_string()),
        token_hash: Set(hash_token(&token)),
        invited_by: Set(auth.claims.sub.clone()),
        expires_at: Set((now + Duration::days(INVITATION_TTL_DAYS)).into()),
        accepted_at: Set(None),
        created_at: Set(now.into()),
    }
    .insert(&state.db)
    .await?;

    let invitation = invitation_views(&state, vec![inserted])
        .await?
        .pop()
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("inviter not found")))?;
    let accept_url = format!(
        "{}/accept-invite#token={token}",
        state.config.dashboard_url.trim_end_matches('/')
    );
    audit::record(
        &state,
        AuditEvent::user(&auth.claims, "project_invitation.created")
            .with_project(&project_id)
            .with_resource("project_invitation", &invitation.id)
            .with_metadata(json!({ "role": invitation.role.as_str(), "email": invitation.email })),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "data": CreatedInvitationView {
                invitation,
                accept_token: token,
                accept_url,
            }
        })),
    )
        .into_response())
}

pub async fn revoke_invitation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((project_id, invitation_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    require_project_role(&state, &auth.claims.sub, &project_id, ProjectRole::Admin).await?;
    let invitation = project_invitation::Entity::find_by_id(invitation_id.clone())
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if invitation.project_id != project_id || invitation.accepted_at.is_some() {
        return Err(ApiError::NotFound);
    }
    project_invitation::Entity::delete_by_id(invitation_id.clone())
        .exec(&state.db)
        .await?;
    audit::record(
        &state,
        AuditEvent::user(&auth.claims, "project_invitation.revoked")
            .with_project(&project_id)
            .with_resource("project_invitation", &invitation_id),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn preview_invitation(
    State(state): State<AppState>,
    Json(payload): Json<PreviewInvitationRequest>,
) -> ApiResult<Response> {
    let invitation = load_active_invitation(&state, &payload.token).await?;
    let project = project::Entity::find_by_id(invitation.project_id)
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    let existing_account = user::Entity::find()
        .filter(user::Column::Email.eq(invitation.email.clone()))
        .one(&state.db)
        .await?
        .is_some();
    Ok(Json(json!({
        "data": InvitationPreview {
            project_name: project.name,
            email: invitation.email,
            role: ProjectRole::parse(&invitation.role)?,
            expires_at: invitation.expires_at.to_rfc3339(),
            existing_account,
        }
    }))
    .into_response())
}

pub async fn accept_invitation(
    State(state): State<AppState>,
    Json(payload): Json<AcceptInvitationRequest>,
) -> ApiResult<Response> {
    let invitation = load_active_invitation(&state, &payload.token).await?;
    let role = ProjectRole::parse(&invitation.role)?;
    reject_owner_assignment(role)?;

    let existing_user = user::Entity::find()
        .filter(user::Column::Email.eq(invitation.email.clone()))
        .one(&state.db)
        .await?;
    let new_user = existing_user.is_none();
    let account = match existing_user {
        Some(user) if password::verify(&payload.password, &user.password_hash) => user,
        Some(_) => return Err(ApiError::Unauthorized),
        None => {
            let name = validate_name(payload.name.as_deref())?;
            validate_password(&payload.password)?;
            let now = Utc::now().into();
            user::Model {
                id: new_id("usr"),
                name,
                email: invitation.email.clone(),
                password_hash: password::hash(&payload.password)?,
                created_at: now,
                updated_at: now,
            }
        }
    };

    if project_member::Entity::find_by_id((invitation.project_id.clone(), account.id.clone()))
        .one(&state.db)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(
            "this account already belongs to the project".into(),
        ));
    }

    let transaction = state.db.begin().await?;
    if new_user {
        user::ActiveModel {
            id: Set(account.id.clone()),
            name: Set(account.name.clone()),
            email: Set(account.email.clone()),
            password_hash: Set(account.password_hash.clone()),
            created_at: Set(account.created_at),
            updated_at: Set(account.updated_at),
        }
        .insert(&transaction)
        .await?;
    }
    let now = Utc::now().into();
    project_member::ActiveModel {
        project_id: Set(invitation.project_id.clone()),
        user_id: Set(account.id.clone()),
        role: Set(role.as_str().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&transaction)
    .await?;
    let project_id = invitation.project_id.clone();
    let invitation_id = invitation.id.clone();
    let mut active_invitation = invitation.into_active_model();
    active_invitation.accepted_at = Set(Some(now));
    active_invitation.update(&transaction).await?;
    transaction.commit().await?;

    let token = issue_token(&state.config.jwt_secret, &account.id, &account.email)?;
    audit::record(
        &state,
        AuditEvent::user_identity(&account.id, &account.email, "project_invitation.accepted")
            .with_project(&project_id)
            .with_resource("project_invitation", &invitation_id)
            .with_metadata(json!({ "role": role.as_str() })),
    )
    .await?;
    let cookie = session_cookie(&state, Some(&token), TOKEN_TTL_HOURS * 3600);
    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(json!({
            "data": AcceptedInvitationView {
                token,
                user: account.into(),
                project_id,
                role,
            }
        })),
    )
        .into_response())
}

async fn load_active_invitation(
    state: &AppState,
    token: &str,
) -> ApiResult<project_invitation::Model> {
    if token.trim().is_empty() {
        return Err(ApiError::NotFound);
    }
    project_invitation::Entity::find()
        .filter(project_invitation::Column::TokenHash.eq(hash_token(token)))
        .filter(project_invitation::Column::AcceptedAt.is_null())
        .filter(project_invitation::Column::ExpiresAt.gt(Utc::now()))
        .one(&state.db)
        .await?
        .ok_or(ApiError::NotFound)
}

async fn invitation_views(
    state: &AppState,
    rows: Vec<project_invitation::Model>,
) -> ApiResult<Vec<InvitationView>> {
    let inviters = user::Entity::find()
        .filter(
            user::Column::Id.is_in(
                rows.iter()
                    .map(|invitation| invitation.invited_by.clone())
                    .collect::<Vec<_>>(),
            ),
        )
        .all(&state.db)
        .await?
        .into_iter()
        .map(|user| (user.id, user.name))
        .collect::<HashMap<_, _>>();
    rows.into_iter()
        .map(|invitation| {
            Ok(InvitationView {
                id: invitation.id,
                project_id: invitation.project_id,
                email: invitation.email,
                role: ProjectRole::parse(&invitation.role)?,
                inviter_name: inviters
                    .get(&invitation.invited_by)
                    .cloned()
                    .unwrap_or_else(|| "Unknown user".into()),
                invited_by: invitation.invited_by,
                expires_at: invitation.expires_at.to_rfc3339(),
                created_at: invitation.created_at.to_rfc3339(),
            })
        })
        .collect()
}

fn reject_owner_assignment(role: ProjectRole) -> ApiResult<()> {
    if role == ProjectRole::Owner {
        return Err(ApiError::Validation(
            "project ownership cannot be assigned through team roles".into(),
        ));
    }
    Ok(())
}

fn normalize_email(email: &str) -> ApiResult<String> {
    let email = email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') || email.len() > 254 {
        return Err(ApiError::Validation("email is invalid".into()));
    }
    Ok(email)
}

fn validate_name(name: Option<&str>) -> ApiResult<String> {
    let name = name.map(str::trim).filter(|value| !value.is_empty());
    match name {
        Some(name) if name.len() <= 120 => Ok(name.to_string()),
        _ => Err(ApiError::Validation(
            "name is required for a new account".into(),
        )),
    }
}

fn validate_password(password: &str) -> ApiResult<()> {
    if password.len() < 8 {
        return Err(ApiError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }
    Ok(())
}

fn generate_invitation_token() -> String {
    let suffix = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect::<String>();
    format!("fbi_{suffix}")
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}
