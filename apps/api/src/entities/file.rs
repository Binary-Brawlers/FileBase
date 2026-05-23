use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "files")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub project_id: String,
    pub storage_connection_id: String,
    pub original_name: String,
    pub saved_name: String,
    pub mime_type: String,
    pub extension: String,
    pub size: i64,
    pub hash: String,
    pub folder: String,
    pub path: String,
    pub url: String,
    pub status: String,
    pub duplicate_of_file_id: Option<String>,
    pub metadata_json: Json,
    pub created_at: ChronoDateTimeWithTimeZone,
    pub updated_at: ChronoDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
