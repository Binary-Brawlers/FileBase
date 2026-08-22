use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "project_invitations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub project_id: String,
    pub email: String,
    pub role: String,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub invited_by: String,
    pub expires_at: ChronoDateTimeWithTimeZone,
    pub accepted_at: Option<ChronoDateTimeWithTimeZone>,
    pub created_at: ChronoDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
