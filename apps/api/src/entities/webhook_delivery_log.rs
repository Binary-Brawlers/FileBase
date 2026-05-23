use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook_delivery_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub webhook_id: String,
    pub project_id: String,
    pub file_id: Option<String>,
    pub event: String,
    pub status: String,
    pub attempt: i32,
    pub status_code: Option<i32>,
    pub error: Option<String>,
    pub request_json: Json,
    pub response_json: Json,
    pub created_at: ChronoDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
