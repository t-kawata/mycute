use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // バッジ認定情報
        manager.create_table(
            Table::create()
                .table(UsrBadge::Table)
                .if_not_exists()
                .col(pk_auto(UsrBadge::Id))
                .col(unsigned(UsrBadge::BadgeID).not_null().default(0))
                .col(unsigned(UsrBadge::CorpID).not_null().default(0))
                .col(unsigned(UsrBadge::From).not_null().default(0))
                .col(unsigned(UsrBadge::To).not_null().default(0))
                .col(string_len(UsrBadge::Title, 100).not_null().default(""))
                .col(string_len(UsrBadge::Message, 500).not_null().default(""))
                .col(tiny_unsigned(UsrBadge::Type).not_null().default(0))
                .col(unsigned(UsrBadge::ApxID).not_null())
                .col(unsigned(UsrBadge::VdrID).not_null())
                .col(ColumnDef::new(UsrBadge::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(UsrBadge::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("usrbadge_apxid_vdrid_corpid_idx")
                .table(UsrBadge::Table)
                .col(UsrBadge::ApxID)
                .col(UsrBadge::VdrID)
                .col(UsrBadge::CorpID)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(UsrBadge::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum UsrBadge {
    #[sea_orm(iden = "usr_badges")]
    Table,
    Id,
    BadgeID,
    /// Badgeを作った法人の UsrID
    CorpID,
    /// Badgeをあげた認定マン UsrID
    From,
    /// Badgeをもらったユーザー UsrID
    To,
    /// メッセージの件名
    Title,
    /// メッセージ本体
    Message,
    /// 1: 法人による授与, 2: 個人による授与 （バッジをあげた側の Usr のタイプ）
    Type,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
