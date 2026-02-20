use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CubeModelStats::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CubeModelStats::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CubeModelStats::CubeId).integer().not_null())
                    .col(
                        ColumnDef::new(CubeModelStats::MemoryGroup)
                            .string_len(64)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(CubeModelStats::ModelName)
                            .string_len(100)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(CubeModelStats::ActionType)
                            .string_len(6)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(CubeModelStats::InputTokens)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(CubeModelStats::OutputTokens)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(CubeModelStats::ApxId).integer().not_null())
                    .col(ColumnDef::new(CubeModelStats::VdrId).integer().not_null())
                    .col(
                        ColumnDef::new(CubeModelStats::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CubeModelStats::UpdatedAt)
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
                    .name("idx_cube_mg_model_action")
                    .table(CubeModelStats::Table)
                    .col(CubeModelStats::CubeId)
                    .col(CubeModelStats::MemoryGroup)
                    .col(CubeModelStats::ModelName)
                    .col(CubeModelStats::ActionType)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_model_stat_apxid_vdrid_idx")
                    .table(CubeModelStats::Table)
                    .col(CubeModelStats::ApxId)
                    .col(CubeModelStats::VdrId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CubeModelStats::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CubeModelStats {
    Table,
    Id,
    CubeId,
    MemoryGroup,
    ModelName,
    ActionType,
    InputTokens,
    OutputTokens,
    ApxId,
    VdrId,
    CreatedAt,
    UpdatedAt,
}
