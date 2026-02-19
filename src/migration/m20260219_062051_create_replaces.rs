use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create `replaces` table
        manager
            .create_table(
                Table::create()
                    .table(Replaces::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Replaces::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Replaces::ApxId).integer().not_null())
                    .col(ColumnDef::new(Replaces::VdrId).integer().not_null())
                    .col(ColumnDef::new(Replaces::Name).string().not_null())
                    .col(ColumnDef::new(Replaces::Description).string())
                    .col(
                        ColumnDef::new(Replaces::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Replaces::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Replaces::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for `replaces`
        manager
            .create_index(
                Index::create()
                    .name("idx_replaces_apx_vdr")
                    .table(Replaces::Table)
                    .col(Replaces::ApxId)
                    .col(Replaces::VdrId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Replaces::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Replaces {
    /// テーブル名: replaces
    Table,
    /// 固定ID (UUID)
    Id,
    /// 所属APX ID
    ApxId,
    /// 所属VDR ID
    VdrId,
    /// 辞書名
    Name,
    /// 説明文
    Description,
    /// 有効フラグ
    IsActive,
    /// 作成日時
    CreatedAt,
    /// 更新日時
    UpdatedAt,
}
