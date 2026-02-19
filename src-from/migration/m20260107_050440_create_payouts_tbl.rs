use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Usr（個人）単位の現金プール分配実行履歴
        manager.create_table(
            Table::create()
                .table(Payout::Table)
                .if_not_exists()
                .col(pk_auto(Payout::Id))
                .col(unsigned(Payout::PoolID).not_null().default(0))
                .col(unsigned(Payout::FlushID).not_null().default(0))
                .col(unsigned(Payout::UsrID).not_null().default(0))
                .col(unsigned(Payout::Points).not_null().default(0))
                .col(ColumnDef::new(Payout::Share).decimal_len(5, 5).not_null().default(0.0))
                .col(unsigned(Payout::Amount).not_null().default(0))
                .col(unsigned(Payout::ApxID).not_null().default(0))
                .col(unsigned(Payout::VdrID).not_null().default(0))
                .col(ColumnDef::new(Payout::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Payout::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("payout_apxid_vdrid_usrid_idx")
                .table(Payout::Table)
                .col(Payout::ApxID)
                .col(Payout::VdrID)
                .col(Payout::UsrID)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Payout::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Payout {
    #[sea_orm(iden = "payouts")]
    Table,
    Id,
    /// 分配元の PoolID
    PoolID,
    /// 分配元の FlushID
    FlushID,
    /// 還元対象 個人 UsrID
    UsrID,
    /// 分配時の個人ポイント残高（Usr.Points）
    Points,
    /// 分配総額に対する自分の取り分割合（Payout.Points / Flush.Points）
    Share,
    /// 現金化された分配金額
    Amount,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
