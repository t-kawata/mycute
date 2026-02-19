use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Forums::Table)
                    .if_not_exists()
                    .col(pk_uuid(Forums::Id))
                    .col(string(Forums::Name))
                    .col(string(Forums::Description))
                    .col(integer(Forums::InitialBalance)) // 2-30
                    .col(date_time(Forums::CreatedAt))
                    .col(date_time(Forums::UpdatedAt))
                    .col(date_time_null(Forums::DeletedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Forums::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Forums {
    /// テーブル名
    Table,
    /// フォーラムID (UUID)
    Id,
    /// フォーラム名
    Name,
    /// フォーラム説明
    Description,
    /// 初期投票可能セット数 (2-30)
    InitialBalance,
    /// レコード作成日時
    CreatedAt,
    /// レコード更新日時
    UpdatedAt,
    /// 論理削除日時
    DeletedAt,
}
