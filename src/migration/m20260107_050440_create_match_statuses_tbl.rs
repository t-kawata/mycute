use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 求人のアプローチ情報のステータスを時系列で終えるようにするためのテーブル
        manager.create_table(
            Table::create()
                .table(MatchStatus::Table)
                .if_not_exists()
                .col(pk_auto(MatchStatus::Id))
                .col(unsigned(MatchStatus::JobID).not_null().default(0))
                .col(unsigned(MatchStatus::MatchID).not_null().default(0))
                .col(unsigned(MatchStatus::From).not_null().default(0))
                .col(unsigned(MatchStatus::To).not_null().default(0))
                .col(tiny_unsigned(MatchStatus::Status).not_null().default(0))
                .col(boolean(MatchStatus::IsTmp).not_null().default(true))
                .col(unsigned(MatchStatus::ApxID).not_null())
                .col(unsigned(MatchStatus::VdrID).not_null())
                .col(ColumnDef::new(MatchStatus::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(MatchStatus::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("matchstatus_apxid_vdrid_jobid_machid_idx")
                .table(MatchStatus::Table)
                .col(MatchStatus::ApxID)
                .col(MatchStatus::VdrID)
                .col(MatchStatus::JobID)
                .col(MatchStatus::MatchID)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(MatchStatus::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum MatchStatus {
    #[sea_orm(iden = "match_statuses")]
    Table,
    Id,
    /// 求人の JobID
    JobID,
    /// 求人のアプローチ情報の MatchID
    MatchID,
    /// 求人を発行した法人の UsrID
    From,
    /// 求人のアプローチを受けた個人の UsrID
    To,
    /// 1:アプローチ, 2:面談設定, 3:面談実行, 4:採用成功
    Status,
    /// 個人によって「仮予定」とした場合 true、確定したら false
    IsTmp,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
