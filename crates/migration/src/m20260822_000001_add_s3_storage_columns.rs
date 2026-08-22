use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(StorageConnections::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(StorageConnections::Bucket)
                            .text()
                            .null()
                            .to_owned(),
                    )
                    .add_column_if_not_exists(
                        ColumnDef::new(StorageConnections::Region)
                            .text()
                            .null()
                            .to_owned(),
                    )
                    .add_column_if_not_exists(
                        ColumnDef::new(StorageConnections::ForcePathStyle)
                            .boolean()
                            .not_null()
                            .default(false)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(StorageConnections::Table)
                    .drop_column(StorageConnections::ForcePathStyle)
                    .drop_column(StorageConnections::Region)
                    .drop_column(StorageConnections::Bucket)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum StorageConnections {
    Table,
    Bucket,
    Region,
    ForcePathStyle,
}
