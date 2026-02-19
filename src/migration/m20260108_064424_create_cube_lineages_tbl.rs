use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CubeLineages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CubeLineages::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CubeLineages::CubeId).integer().not_null())
                    .col(ColumnDef::new(CubeLineages::AncestorUuid).string_len(36).not_null().default(""))
                    .col(ColumnDef::new(CubeLineages::AncestorOwner).string_len(50).not_null().default(""))
                    .col(ColumnDef::new(CubeLineages::ExportedAt).big_integer().not_null().default(0))
                    .col(ColumnDef::new(CubeLineages::Generation).integer().not_null().default(0))
                    .col(ColumnDef::new(CubeLineages::ApxId).integer().not_null())
                    .col(ColumnDef::new(CubeLineages::VdrId).integer().not_null())
                    .col(
                        ColumnDef::new(CubeLineages::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CubeLineages::UpdatedAt)
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
                    .name("lineage_cube_idx")
                    .table(CubeLineages::Table)
                    .col(CubeLineages::CubeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("lineage_apxid_vdrid_idx")
                    .table(CubeLineages::Table)
                    .col(CubeLineages::ApxId)
                    .col(CubeLineages::VdrId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CubeLineages::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CubeLineages {
    Table,
    Id,
    CubeId,
    AncestorUuid,
    AncestorOwner,
    ExportedAt,
    Generation,
    ApxId,
    VdrId,
    CreatedAt,
    UpdatedAt,
}
