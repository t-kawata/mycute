use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Vdr単位の現金プールの分配実行履歴
        manager.create_table(
            Table::create()
                .table(Flush::Table)
                .if_not_exists()
                .col(pk_auto(Flush::Id))
                .col(unsigned(Flush::PoolID).not_null().default(0))
                .col(unsigned(Flush::Total).not_null().default(0))
                .col(ColumnDef::new(Flush::FlushFeeRate).decimal_len(5, 5).not_null().default(0.0))
                .col(unsigned(Flush::Points).not_null().default(0))
                .col(unsigned(Flush::ApxID).not_null().default(0))
                .col(unsigned(Flush::VdrID).not_null().default(0))
                .col(ColumnDef::new(Flush::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Flush::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("flush_apxid_vdrid_poolid_idx")
                .table(Flush::Table)
                .col(Flush::ApxID)
                .col(Flush::VdrID)
                .col(Flush::PoolID)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Flush::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Flush {
    #[sea_orm(iden = "flushes")]
    Table,
    Id,
    /// 分配元の PoolID
    PoolID,
    /// 分配実行額（多くの場合、その時点の Pool.Remain）
    Total,
    /// 分配実行時の事務費用割引率記録
    FlushFeeRate,
    /// 分配実行時の Vdr 内全ユーザーの Usr.Points 合計
    Points,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
