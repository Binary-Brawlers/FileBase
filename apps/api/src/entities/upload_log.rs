use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "upload_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub project_id: String,
    pub file_id: Option<String>,
    pub event: String,
    pub status: String,
    pub message: Option<String>,
    pub metadata_json: Json,
    pub created_at: ChronoDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
