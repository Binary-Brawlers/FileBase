use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(text_id(Users::Id).primary_key())
                    .col(string(Users::Name))
                    .col(string(Users::Email).unique_key())
                    .col(string(Users::PasswordHash))
                    .col(timestamp(Users::CreatedAt))
                    .col(timestamp(Users::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(text_id(Projects::Id).primary_key())
                    .col(text_id(Projects::UserId))
                    .col(string(Projects::Name))
                    .col(string(Projects::Slug).unique_key())
                    .col(timestamp(Projects::CreatedAt))
                    .col(timestamp(Projects::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Projects::Table, Projects::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(text_id(ApiKeys::Id).primary_key())
                    .col(text_id(ApiKeys::ProjectId))
                    .col(string(ApiKeys::Name))
                    .col(string(ApiKeys::KeyHash))
                    .col(string(ApiKeys::Prefix))
                    .col(optional_timestamp(ApiKeys::LastUsedAt))
                    .col(timestamp(ApiKeys::CreatedAt))
                    .col(optional_timestamp(ApiKeys::RevokedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(ApiKeys::Table, ApiKeys::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(StorageConnections::Table)
                    .if_not_exists()
                    .col(text_id(StorageConnections::Id).primary_key())
                    .col(text_id(StorageConnections::ProjectId))
                    .col(string(StorageConnections::Type))
                    .col(nullable_string(StorageConnections::Host))
                    .col(ColumnDef::new(StorageConnections::Port).integer().null())
                    .col(nullable_string(StorageConnections::Username))
                    .col(nullable_text(StorageConnections::EncryptedPassword))
                    .col(nullable_text(StorageConnections::EncryptedPrivateKey))
                    .col(string(StorageConnections::BasePath))
                    .col(string(StorageConnections::PublicBaseUrl))
                    .col(timestamp(StorageConnections::CreatedAt))
                    .col(timestamp(StorageConnections::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(StorageConnections::Table, StorageConnections::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UploadPresets::Table)
                    .if_not_exists()
                    .col(text_id(UploadPresets::Id).primary_key())
                    .col(text_id(UploadPresets::ProjectId))
                    .col(
                        ColumnDef::new(UploadPresets::StorageConnectionId)
                            .string()
                            .null(),
                    )
                    .col(string(UploadPresets::Name))
                    .col(string(UploadPresets::Folder))
                    .col(json(UploadPresets::AllowedMimeTypes))
                    .col(json(UploadPresets::AllowedExtensions))
                    .col(
                        ColumnDef::new(UploadPresets::MaxFileSize)
                            .big_integer()
                            .not_null(),
                    )
                    .col(string(UploadPresets::DuplicateStrategy))
                    .col(string(UploadPresets::FilenameStrategy))
                    .col(json(UploadPresets::TransformationsJson))
                    .col(timestamp(UploadPresets::CreatedAt))
                    .col(timestamp(UploadPresets::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(UploadPresets::Table, UploadPresets::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UploadPresets::Table, UploadPresets::StorageConnectionId)
                            .to(StorageConnections::Table, StorageConnections::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UploadSessions::Table)
                    .if_not_exists()
                    .col(text_id(UploadSessions::Id).primary_key())
                    .col(text_id(UploadSessions::ProjectId))
                    .col(text_id(UploadSessions::PresetId))
                    .col(string(UploadSessions::TokenHash))
                    .col(string(UploadSessions::Folder))
                    .col(json(UploadSessions::AllowedMimeTypes))
                    .col(
                        ColumnDef::new(UploadSessions::MaxFileSize)
                            .big_integer()
                            .not_null(),
                    )
                    .col(timestamp(UploadSessions::ExpiresAt))
                    .col(optional_timestamp(UploadSessions::UsedAt))
                    .col(timestamp(UploadSessions::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(UploadSessions::Table, UploadSessions::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UploadSessions::Table, UploadSessions::PresetId)
                            .to(UploadPresets::Table, UploadPresets::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Files::Table)
                    .if_not_exists()
                    .col(text_id(Files::Id).primary_key())
                    .col(text_id(Files::ProjectId))
                    .col(text_id(Files::StorageConnectionId))
                    .col(string(Files::OriginalName))
                    .col(string(Files::SavedName))
                    .col(string(Files::MimeType))
                    .col(string(Files::Extension))
                    .col(ColumnDef::new(Files::Size).big_integer().not_null())
                    .col(string(Files::Hash))
                    .col(string(Files::Folder))
                    .col(string(Files::Path))
                    .col(string(Files::Url))
                    .col(string(Files::Status))
                    .col(ColumnDef::new(Files::DuplicateOfFileId).string().null())
                    .col(json(Files::MetadataJson))
                    .col(timestamp(Files::CreatedAt))
                    .col(timestamp(Files::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Files::Table, Files::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Files::Table, Files::StorageConnectionId)
                            .to(StorageConnections::Table, StorageConnections::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Files::Table, Files::DuplicateOfFileId)
                            .to(Files::Table, Files::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Webhooks::Table)
                    .if_not_exists()
                    .col(text_id(Webhooks::Id).primary_key())
                    .col(text_id(Webhooks::ProjectId))
                    .col(string(Webhooks::Url))
                    .col(string(Webhooks::Secret))
                    .col(json(Webhooks::Events))
                    .col(
                        ColumnDef::new(Webhooks::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(timestamp(Webhooks::CreatedAt))
                    .col(timestamp(Webhooks::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Webhooks::Table, Webhooks::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UploadLogs::Table)
                    .if_not_exists()
                    .col(text_id(UploadLogs::Id).primary_key())
                    .col(text_id(UploadLogs::ProjectId))
                    .col(ColumnDef::new(UploadLogs::FileId).string().null())
                    .col(string(UploadLogs::Event))
                    .col(string(UploadLogs::Status))
                    .col(nullable_text(UploadLogs::Message))
                    .col(json(UploadLogs::MetadataJson))
                    .col(timestamp(UploadLogs::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(UploadLogs::Table, UploadLogs::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UploadLogs::Table, UploadLogs::FileId)
                            .to(Files::Table, Files::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        create_index(
            manager,
            Projects::Table,
            "idx_projects_user_id",
            Projects::UserId,
        )
        .await?;
        create_index(
            manager,
            ApiKeys::Table,
            "idx_api_keys_project_id",
            ApiKeys::ProjectId,
        )
        .await?;
        create_index(
            manager,
            StorageConnections::Table,
            "idx_storage_connections_project_id",
            StorageConnections::ProjectId,
        )
        .await?;
        create_index(
            manager,
            UploadPresets::Table,
            "idx_upload_presets_project_id",
            UploadPresets::ProjectId,
        )
        .await?;
        create_index(
            manager,
            UploadSessions::Table,
            "idx_upload_sessions_project_id",
            UploadSessions::ProjectId,
        )
        .await?;
        create_index(
            manager,
            Files::Table,
            "idx_files_project_id",
            Files::ProjectId,
        )
        .await?;
        create_index(manager, Files::Table, "idx_files_project_hash", Files::Hash).await?;
        create_index(
            manager,
            Webhooks::Table,
            "idx_webhooks_project_id",
            Webhooks::ProjectId,
        )
        .await?;
        create_index(
            manager,
            UploadLogs::Table,
            "idx_upload_logs_project_id",
            UploadLogs::ProjectId,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(drop_table(UploadLogs::Table)).await?;
        manager.drop_table(drop_table(Webhooks::Table)).await?;
        manager.drop_table(drop_table(Files::Table)).await?;
        manager
            .drop_table(drop_table(UploadSessions::Table))
            .await?;
        manager.drop_table(drop_table(UploadPresets::Table)).await?;
        manager
            .drop_table(drop_table(StorageConnections::Table))
            .await?;
        manager.drop_table(drop_table(ApiKeys::Table)).await?;
        manager.drop_table(drop_table(Projects::Table)).await?;
        manager.drop_table(drop_table(Users::Table)).await?;

        Ok(())
    }
}

fn text_id<T: Iden + 'static>(name: T) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.string().not_null();
    column
}

fn string<T: Iden + 'static>(name: T) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.string().not_null();
    column
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

fn timestamp<T: Iden + 'static>(name: T) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column
        .timestamp_with_time_zone()
        .not_null()
        .default(Expr::current_timestamp());
    column
}

fn optional_timestamp<T: Iden + 'static>(name: T) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.timestamp_with_time_zone().null();
    column
}

fn json<T: Iden + 'static>(name: T) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column
        .json_binary()
        .not_null()
        .default(Expr::cust("'{}'::jsonb"));
    column
}

fn drop_table<T: Iden + 'static>(table: T) -> TableDropStatement {
    Table::drop().table(table).if_exists().to_owned()
}

async fn create_index<T, C>(
    manager: &SchemaManager<'_>,
    table: T,
    name: &str,
    column: C,
) -> Result<(), DbErr>
where
    T: Iden + 'static,
    C: Iden + 'static,
{
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name(name)
                .table(table)
                .col(column)
                .to_owned(),
        )
        .await
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Name,
    Email,
    PasswordHash,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
    UserId,
    Name,
    Slug,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    Id,
    ProjectId,
    Name,
    KeyHash,
    Prefix,
    LastUsedAt,
    CreatedAt,
    RevokedAt,
}

#[derive(DeriveIden)]
enum StorageConnections {
    Table,
    Id,
    ProjectId,
    Type,
    Host,
    Port,
    Username,
    EncryptedPassword,
    EncryptedPrivateKey,
    BasePath,
    PublicBaseUrl,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UploadPresets {
    Table,
    Id,
    ProjectId,
    StorageConnectionId,
    Name,
    Folder,
    AllowedMimeTypes,
    AllowedExtensions,
    MaxFileSize,
    DuplicateStrategy,
    FilenameStrategy,
    TransformationsJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UploadSessions {
    Table,
    Id,
    ProjectId,
    PresetId,
    TokenHash,
    Folder,
    AllowedMimeTypes,
    MaxFileSize,
    ExpiresAt,
    UsedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Files {
    Table,
    Id,
    ProjectId,
    StorageConnectionId,
    OriginalName,
    SavedName,
    MimeType,
    Extension,
    Size,
    Hash,
    Folder,
    Path,
    Url,
    Status,
    DuplicateOfFileId,
    MetadataJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Webhooks {
    Table,
    Id,
    ProjectId,
    Url,
    Secret,
    Events,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UploadLogs {
    Table,
    Id,
    ProjectId,
    FileId,
    Event,
    Status,
    Message,
    MetadataJson,
    CreatedAt,
}
