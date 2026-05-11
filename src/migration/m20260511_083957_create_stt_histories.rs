use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SttHistories::Table)
                    .if_not_exists()
                    .col(pk_auto(SttHistories::Id))
                    .col(string(SttHistories::Text))
                    .col(
                        ColumnDef::new(SttHistories::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(SttHistories::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SttHistories::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SttHistories {
    /// テーブル名
    Table,
    /// プライマリキー（自動採番）
    Id,
    /// 音声認識されたテキスト本文
    Text,
    /// レコード作成日時
    CreatedAt,
    /// レコード更新日時
    UpdatedAt,
}
