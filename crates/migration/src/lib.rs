use sea_orm_migration::prelude::*;

mod m20260523_000001_create_core_tables;
mod m20260523_000002_add_upload_preset_storage_connection;
mod m20260524_000001_create_webhook_delivery_logs;

pub use sea_orm_migration::MigratorTrait;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260523_000001_create_core_tables::Migration),
            Box::new(m20260523_000002_add_upload_preset_storage_connection::Migration),
            Box::new(m20260524_000001_create_webhook_delivery_logs::Migration),
        ]
    }
}
