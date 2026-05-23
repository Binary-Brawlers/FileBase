use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UploadPresets::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(UploadPresets::StorageConnectionId)
                            .string()
                            .null()
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_upload_presets_storage_connection_id")
                    .table(UploadPresets::Table)
                    .col(UploadPresets::StorageConnectionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .if_exists()
                    .name("idx_upload_presets_storage_connection_id")
                    .table(UploadPresets::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(UploadPresets::Table)
                    .drop_column(UploadPresets::StorageConnectionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum UploadPresets {
    Table,
    StorageConnectionId,
}
