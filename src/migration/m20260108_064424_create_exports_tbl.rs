use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Exports::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Exports::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Exports::CubeId).integer().not_null())
                    .col(
                        ColumnDef::new(Exports::NewUuid)
                            .string_len(36)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(Exports::Hash)
                            .string_len(64)
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Exports::PrivateKey).text().not_null())
                    .col(ColumnDef::new(Exports::ApxId).integer().not_null())
                    .col(ColumnDef::new(Exports::VdrId).integer().not_null())
                    .col(
                        ColumnDef::new(Exports::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Exports::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("export_cube_idx")
                    .table(Exports::Table)
                    .col(Exports::CubeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("export_new_uuid_idx")
                    .table(Exports::Table)
                    .col(Exports::NewUuid)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("export_apxid_vdrid_idx")
                    .table(Exports::Table)
                    .col(Exports::ApxId)
                    .col(Exports::VdrId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Exports::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Exports {
    Table,
    Id,
    CubeId,
    NewUuid,
    Hash,
    PrivateKey,
    ApxId,
    VdrId,
    CreatedAt,
    UpdatedAt,
}
