use sea_orm_migration::prelude::*;

mod m20260523_000001_create_core_tables;

pub use sea_orm_migration::MigratorTrait;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260523_000001_create_core_tables::Migration)]
    }
}
