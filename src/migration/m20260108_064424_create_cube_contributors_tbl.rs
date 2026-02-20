use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CubeContributors::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CubeContributors::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CubeContributors::CubeId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CubeContributors::MemoryGroup)
                            .string_len(64)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(CubeContributors::ContributorName)
                            .string_len(100)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(CubeContributors::ModelName)
                            .string_len(100)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(CubeContributors::InputTokens)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(CubeContributors::OutputTokens)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(CubeContributors::ApxId).integer().not_null())
                    .col(ColumnDef::new(CubeContributors::VdrId).integer().not_null())
                    .col(
                        ColumnDef::new(CubeContributors::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CubeContributors::UpdatedAt)
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
                    .name("idx_cube_mg_contrib_model")
                    .table(CubeContributors::Table)
                    .col(CubeContributors::CubeId)
                    .col(CubeContributors::MemoryGroup)
                    .col(CubeContributors::ContributorName)
                    .col(CubeContributors::ModelName)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_contrib_apxid_vdrid_idx")
                    .table(CubeContributors::Table)
                    .col(CubeContributors::ApxId)
                    .col(CubeContributors::VdrId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CubeContributors::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CubeContributors {
    Table,
    Id,
    CubeId,
    MemoryGroup,
    ContributorName,
    ModelName,
    InputTokens,
    OutputTokens,
    ApxId,
    VdrId,
    CreatedAt,
    UpdatedAt,
}
