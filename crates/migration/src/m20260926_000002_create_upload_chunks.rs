use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_integer, integer, string, text};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UploadChunks::Table)
                    .if_not_exists()
                    .col(string(UploadChunks::Id).primary_key())
                    .col(string(UploadChunks::SessionId))
                    .col(integer(UploadChunks::ChunkIndex))
                    .col(big_integer(UploadChunks::Size))
                    .col(string(UploadChunks::Hash))
                    .col(text(UploadChunks::TempPath))
                    .col(
                        ColumnDef::new(UploadChunks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UploadChunks::Table, UploadChunks::SessionId)
                            .to(UploadSessions::Table, UploadSessions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_upload_chunks_session_id")
                    .table(UploadChunks::Table)
                    .col(UploadChunks::SessionId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_upload_chunks_session_index")
                    .table(UploadChunks::Table)
                    .col(UploadChunks::SessionId)
                    .col(UploadChunks::ChunkIndex)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(UploadChunks::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum UploadChunks {
    Table,
    Id,
    SessionId,
    ChunkIndex,
    Size,
    Hash,
    TempPath,
    CreatedAt,
}

#[derive(DeriveIden)]
enum UploadSessions {
    Table,
    Id,
}
