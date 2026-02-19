use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 個人に対して行なった求人のアプローチ情報
        manager.create_table(
            Table::create()
                .table(Match::Table)
                .if_not_exists()
                .col(pk_auto(Match::Id))
                .col(unsigned(Match::JobID).not_null().default(0))
                .col(unsigned(Match::From).not_null().default(0))
                .col(unsigned(Match::To).not_null().default(0))
                .col(tiny_unsigned(Match::Status).not_null().default(0))
                .col(ColumnDef::new(Match::PriorityScore).decimal_len(10, 4).not_null().default(0.0))
                .col(unsigned(Match::BadgeCount).not_null().default(0))
                .col(tiny_unsigned(Match::MatchReason).not_null().default(0))
                .col(unsigned(Match::ApxID).not_null())
                .col(unsigned(Match::VdrID).not_null())
                .col(ColumnDef::new(Match::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Match::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("match_apxid_vdrid_jobid_idx")
                .table(Match::Table)
                .col(Match::ApxID)
                .col(Match::VdrID)
                .col(Match::JobID)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Match::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Match {
    #[sea_orm(iden = "matches")]
    Table,
    Id,
    /// 求人の JobID
    JobID,
    /// 求人を発行した法人の UsrID
    From,
    /// 求人のアプローチを受けた個人の UsrID
    To,
    /// 1:アプローチ, 2:面談設定, 3:面談実行, 4:採用成功
    Status,
    /// バッジ数による優先度スコア記録
    PriorityScore,
    /// マッチング時点での個人のバッジ数記録
    BadgeCount,
    /// マッチング理由記録（数値化した "high_badge", "random" etc.）
    MatchReason,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
