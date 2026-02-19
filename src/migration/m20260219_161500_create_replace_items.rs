use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // `replace_items` テーブルの作成
        manager
            .create_table(
                Table::create()
                    .table(ReplaceItems::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ReplaceItems::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ReplaceItems::ReplaceId).uuid().not_null())
                    .col(ColumnDef::new(ReplaceItems::ApxId).integer().not_null())
                    .col(ColumnDef::new(ReplaceItems::VdrId).integer().not_null())
                    .col(ColumnDef::new(ReplaceItems::Key).string().not_null())
                    .col(ColumnDef::new(ReplaceItems::Texts).json().not_null())
                    .col(
                        ColumnDef::new(ReplaceItems::Rank)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ReplaceItems::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ReplaceItems::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_replace_items_replace_id")
                            .from(ReplaceItems::Table, ReplaceItems::ReplaceId)
                            .to(Alias::new("replaces"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 検索効率化のためのインデックス
        manager
            .create_index(
                Index::create()
                    .name("idx_replace_items_lookup")
                    .table(ReplaceItems::Table)
                    .col(ReplaceItems::ReplaceId)
                    .col(ReplaceItems::Key)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_replace_items_apx_vdr")
                    .table(ReplaceItems::Table)
                    .col(ReplaceItems::ApxId)
                    .col(ReplaceItems::VdrId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ReplaceItems::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ReplaceItems {
    /// テーブル名: replace_items
    Table,
    /// 自動増分ID
    Id,
    /// 所属辞書セットID (replaces.id)
    ReplaceId,
    /// 所属APX ID
    ApxId,
    /// 所属VDR ID
    VdrId,
    /// 置換対象のキーテキスト
    Key,
    /// 置換後の候補テキスト（JSON配列）
    Texts,
    /// 適用順序
    Rank,
    /// 作成日時
    CreatedAt,
    /// 更新日時
    UpdatedAt,
}
