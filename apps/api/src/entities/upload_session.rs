use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "upload_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub project_id: String,
    pub preset_id: String,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub folder: String,
    pub allowed_mime_types: Json,
    pub max_file_size: i64,
    pub expires_at: ChronoDateTimeWithTimeZone,
    pub used_at: Option<ChronoDateTimeWithTimeZone>,
    pub created_at: ChronoDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
