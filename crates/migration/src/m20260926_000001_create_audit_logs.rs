use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{json, string};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuditLogs::Table)
                    .if_not_exists()
                    .col(string(AuditLogs::Id).primary_key())
                    .col(string(AuditLogs::ActorType))
                    .col(nullable_string(AuditLogs::ActorId))
                    .col(nullable_string(AuditLogs::ActorEmail))
                    .col(nullable_string(AuditLogs::ProjectId))
                    .col(string(AuditLogs::Action))
                    .col(nullable_string(AuditLogs::ResourceType))
                    .col(nullable_string(AuditLogs::ResourceId))
                    .col(string(AuditLogs::Status))
                    .col(nullable_string(AuditLogs::IpAddress))
                    .col(nullable_text(AuditLogs::UserAgent))
                    .col(json(AuditLogs::MetadataJson))
                    .col(
                        ColumnDef::new(AuditLogs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        for (name, column) in [
            ("idx_audit_logs_created_at", AuditLogs::CreatedAt),
            ("idx_audit_logs_actor_id", AuditLogs::ActorId),
            ("idx_audit_logs_project_id", AuditLogs::ProjectId),
            ("idx_audit_logs_action", AuditLogs::Action),
        ] {
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name(name)
                        .table(AuditLogs::Table)
                        .col(column)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().if_exists().table(AuditLogs::Table).to_owned())
            .await?;
        Ok(())
    }
}

fn nullable_string<T: Iden + 'static>(name: T) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.string().null();
    column
}

fn nullable_text<T: Iden + 'static>(name: T) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.text().null();
    column
}

#[derive(DeriveIden)]
enum AuditLogs {
    Table,
    Id,
    ActorType,
    ActorId,
    ActorEmail,
    ProjectId,
    Action,
    ResourceType,
    ResourceId,
    Status,
    IpAddress,
    UserAgent,
    MetadataJson,
    CreatedAt,
}
