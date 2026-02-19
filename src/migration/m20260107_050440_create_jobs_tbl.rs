use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 求人情報
        manager.create_table(
            Table::create()
                .table(Job::Table)
                .if_not_exists()
                .col(pk_auto(Job::Id))
                .col(unsigned(Job::CorpID).not_null().default(0))
                .col(string_len(Job::Name, 100).not_null().default(""))
                .col(string_len(Job::Description, 1000).not_null().default(""))
                .col(unsigned(Job::Max).not_null().default(0))
                .col(unsigned(Job::Filled).not_null().default(0))
                .col(unsigned(Job::HourPrice).not_null().default(0))
                .col(string_len(Job::Requirements, 1000).not_null().default(""))
                .col(string_len(Job::Benefits, 1000).not_null().default(""))
                .col(string_len(Job::Location, 128).not_null().default(""))
                .col(string_len(Job::Phone, 15).not_null().default(""))
                .col(unsigned(Job::MaxBadges).not_null().default(0))
                .col(ColumnDef::new(Job::WorkBgnAt).date_time().null())
                .col(ColumnDef::new(Job::WorkEndAt).date_time().null())
                .col(ColumnDef::new(Job::OpenAt).date_time().null())
                .col(ColumnDef::new(Job::CloseAt).date_time().null())
                .col(unsigned(Job::ApxID).not_null())
                .col(unsigned(Job::VdrID).not_null())
                .col(ColumnDef::new(Job::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Job::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("job_apxid_vdrid_corpid_idx")
                .table(Job::Table)
                .col(Job::ApxID)
                .col(Job::VdrID)
                .col(Job::CorpID)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Job::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Job {
    #[sea_orm(iden = "jobs")]
    Table,
    Id,
    /// 求人を発行した法人の UsrID
    CorpID,
    Name,
    Description,
    /// 求人する人数
    Max,
    /// 充足した人数（当該 Job の Work の数に連動する）
    Filled,
    /// 時給
    HourPrice,
    /// 必要条件
    Requirements,
    /// 働くメリット
    Benefits,
    /// 働く場所
    Location,
    /// 働く場所の電話番号
    Phone,
    /// このお仕事でもらえる可能性のあるバッジの最大数
    MaxBadges,
    /// 就業開始日時
    WorkBgnAt,
    /// 就業終了日時
    WorkEndAt,
    /// 求人開始日時
    OpenAt,
    /// 求人終了日時
    CloseAt,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
