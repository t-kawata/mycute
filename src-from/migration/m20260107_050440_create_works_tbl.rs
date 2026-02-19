use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Work::Table)
                .if_not_exists()
                .col(pk_auto(Work::Id))
                .col(unsigned(Work::JobID).not_null().default(0))
                .col(unsigned(Work::MatchID).not_null().default(0))
                .col(unsigned(Work::From).not_null().default(0))
                .col(unsigned(Work::To).not_null().default(0))
                .col(ColumnDef::new(Work::WorkBgnAt).date_time().null())
                .col(ColumnDef::new(Work::WorkEndAt).date_time().null())
                .col(ColumnDef::new(Work::RealWorkBgnAt).date_time().null())
                .col(ColumnDef::new(Work::RealWorkEndAt).date_time().null())
                .col(unsigned(Work::ApxID).not_null())
                .col(unsigned(Work::VdrID).not_null())
                .col(ColumnDef::new(Work::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Work::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("work_apxid_vdrid_from_idx")
                .table(Work::Table)
                .col(Work::ApxID)
                .col(Work::VdrID)
                .col(Work::From)
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("work_apxid_vdrid_to_idx")
                .table(Work::Table)
                .col(Work::ApxID)
                .col(Work::VdrID)
                .col(Work::To)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Work::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Work {
    #[sea_orm(iden = "works")]
    Table,
    Id,
    /// 求人の JobID
    JobID,
    /// 求人のアプローチ情報の MatchID
    MatchID,
    /// 求人を発行した法人の UsrID
    From,
    /// 求人のアプローチを受けた個人の UsrID（就業する人間）
    To,
    /// 就業開始日時予定
    WorkBgnAt,
    /// 就業終了日時予定
    WorkEndAt,
    /// 就業開始日時実績（タイムカードを兼ねる）
    RealWorkBgnAt,
    /// 就業終了日時実績（タイムカードを兼ねる）
    RealWorkEndAt,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
