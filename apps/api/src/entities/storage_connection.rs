use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "storage_connections")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub project_id: String,
    pub r#type: String,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    #[serde(skip_serializing)]
    pub encrypted_password: Option<String>,
    #[serde(skip_serializing)]
    pub encrypted_private_key: Option<String>,
    pub base_path: String,
    pub public_base_url: String,
    pub created_at: ChronoDateTimeWithTimeZone,
    pub updated_at: ChronoDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
