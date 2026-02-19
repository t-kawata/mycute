use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ポイント履歴
        manager.create_table(
            Table::create()
                .table(Point::Table)
                .if_not_exists()
                .col(pk_auto(Point::Id))
                .col(unsigned(Point::BadgeID).not_null().default(0))
                .col(unsigned(Point::CorpID).not_null().default(0))
                .col(unsigned(Point::From).not_null().default(0))
                .col(unsigned(Point::To).not_null().default(0))
                .col(unsigned(Point::Point).not_null().default(0))
                .col(unsigned(Point::Extra).not_null().default(0))
                .col(unsigned(Point::ApxID).not_null())
                .col(unsigned(Point::VdrID).not_null())
                .col(ColumnDef::new(Point::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Point::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("point_apxid_vdrid_badgeid_idx")
                .table(Point::Table)
                .col(Point::ApxID)
                .col(Point::VdrID)
                .col(Point::BadgeID)
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("point_apxid_vdrid_corpid_idx")
                .table(Point::Table)
                .col(Point::ApxID)
                .col(Point::VdrID)
                .col(Point::CorpID)
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("point_apxid_vdrid_to_idx")
                .table(Point::Table)
                .col(Point::ApxID)
                .col(Point::VdrID)
                .col(Point::To)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Point::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Point {
    #[sea_orm(iden = "points")]
    Table,
    Id,
    BadgeID,
    /// Badgeを作った法人の UsrID
    CorpID,
    /// Badgeをあげた認定マン UsrID（個人）
    From,
    /// Badgeをもらったユーザー UsrID（個人）
    To,
    /// 付与された基本ポイント
    Point,
    /// 付与された割増分のポイント
    Extra,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
