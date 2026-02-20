use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BurnedKeys::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BurnedKeys::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(BurnedKeys::KeyId)
                            .string_len(36)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(BurnedKeys::UsedByUsrId)
                            .string_len(36)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(BurnedKeys::UsedForCubeUuid)
                            .string_len(36)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(BurnedKeys::BurnType)
                            .string_len(6)
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(BurnedKeys::ApxId).integer().not_null())
                    .col(ColumnDef::new(BurnedKeys::VdrId).integer().not_null())
                    .col(
                        ColumnDef::new(BurnedKeys::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(BurnedKeys::UpdatedAt)
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
                    .name("burned_apxid_vdrid_idx")
                    .table(BurnedKeys::Table)
                    .col(BurnedKeys::ApxId)
                    .col(BurnedKeys::VdrId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BurnedKeys::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BurnedKeys {
    Table,
    Id,
    KeyId,
    UsedByUsrId,
    UsedForCubeUuid,
    BurnType,
    ApxId,
    VdrId,
    CreatedAt,
    UpdatedAt,
}
