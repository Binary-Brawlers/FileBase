use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProjectMembers::Table)
                    .if_not_exists()
                    .col(string(ProjectMembers::ProjectId))
                    .col(string(ProjectMembers::UserId))
                    .col(string(ProjectMembers::Role))
                    .col(timestamp(ProjectMembers::CreatedAt))
                    .col(timestamp(ProjectMembers::UpdatedAt))
                    .primary_key(
                        Index::create()
                            .col(ProjectMembers::ProjectId)
                            .col(ProjectMembers::UserId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ProjectMembers::Table, ProjectMembers::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ProjectMembers::Table, ProjectMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ProjectInvitations::Table)
                    .if_not_exists()
                    .col(string(ProjectInvitations::Id).primary_key())
                    .col(string(ProjectInvitations::ProjectId))
                    .col(string(ProjectInvitations::Email))
                    .col(string(ProjectInvitations::Role))
                    .col(string(ProjectInvitations::TokenHash).unique_key())
                    .col(string(ProjectInvitations::InvitedBy))
                    .col(timestamp(ProjectInvitations::ExpiresAt))
                    .col(optional_timestamp(ProjectInvitations::AcceptedAt))
                    .col(timestamp(ProjectInvitations::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(ProjectInvitations::Table, ProjectInvitations::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ProjectInvitations::Table, ProjectInvitations::InvitedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_members_user_id")
                    .table(ProjectMembers::Table)
                    .col(ProjectMembers::UserId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_invitations_project_id")
                    .table(ProjectInvitations::Table)
                    .col(ProjectInvitations::ProjectId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_invitations_email")
                    .table(ProjectInvitations::Table)
                    .col(ProjectInvitations::Email)
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO project_members (project_id, user_id, role, created_at, updated_at) \
                 SELECT id, user_id, 'owner', created_at, updated_at FROM projects \
                 ON CONFLICT (project_id, user_id) DO NOTHING",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ProjectInvitations::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(ProjectMembers::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

fn string<T: Iden + 'static>(name: T) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    column.string().not_null();
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

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ProjectMembers {
    Table,
    ProjectId,
    UserId,
    Role,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ProjectInvitations {
    Table,
    Id,
    ProjectId,
    Email,
    Role,
    TokenHash,
    InvitedBy,
    ExpiresAt,
    AcceptedAt,
    CreatedAt,
}
