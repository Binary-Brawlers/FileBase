use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{integer, json, string, timestamp};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebhookDeliveryLogs::Table)
                    .if_not_exists()
                    .col(string(WebhookDeliveryLogs::Id).primary_key())
                    .col(string(WebhookDeliveryLogs::WebhookId))
                    .col(string(WebhookDeliveryLogs::ProjectId))
                    .col(ColumnDef::new(WebhookDeliveryLogs::FileId).string().null())
                    .col(string(WebhookDeliveryLogs::Event))
                    .col(string(WebhookDeliveryLogs::Status))
                    .col(integer(WebhookDeliveryLogs::Attempt))
                    .col(
                        ColumnDef::new(WebhookDeliveryLogs::StatusCode)
                            .integer()
                            .null(),
                    )
                    .col(ColumnDef::new(WebhookDeliveryLogs::Error).text().null())
                    .col(json(WebhookDeliveryLogs::RequestJson))
                    .col(json(WebhookDeliveryLogs::ResponseJson))
                    .col(timestamp(WebhookDeliveryLogs::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(WebhookDeliveryLogs::Table, WebhookDeliveryLogs::WebhookId)
                            .to(Webhooks::Table, Webhooks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(WebhookDeliveryLogs::Table, WebhookDeliveryLogs::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(WebhookDeliveryLogs::Table, WebhookDeliveryLogs::FileId)
                            .to(Files::Table, Files::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_webhook_delivery_logs_webhook_id")
                    .table(WebhookDeliveryLogs::Table)
                    .col(WebhookDeliveryLogs::WebhookId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_webhook_delivery_logs_project_id")
                    .table(WebhookDeliveryLogs::Table)
                    .col(WebhookDeliveryLogs::ProjectId)
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
                    .table(WebhookDeliveryLogs::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum WebhookDeliveryLogs {
    Table,
    Id,
    WebhookId,
    ProjectId,
    FileId,
    Event,
    Status,
    Attempt,
    StatusCode,
    Error,
    RequestJson,
    ResponseJson,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Webhooks {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Files {
    Table,
    Id,
}
