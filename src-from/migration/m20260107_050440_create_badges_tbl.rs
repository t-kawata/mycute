use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // バッジ情報（法人だけが発行できる）
        manager.create_table(
            Table::create()
                .table(Badge::Table)
                .if_not_exists()
                .col(pk_auto(Badge::Id))
                .col(unsigned(Badge::CorpID).not_null().default(0))
                .col(string_len(Badge::Name, 50).not_null().default(""))
                .col(string_len(Badge::ShortName, 20).not_null().default(""))
                .col(string_len(Badge::Description, 255).not_null().default(""))
                .col(unsigned(Badge::ApxID).not_null())
                .col(unsigned(Badge::VdrID).not_null())
                .col(ColumnDef::new(Badge::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Badge::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("badge_apxid_vdrid_corpid_idx")
                .table(Badge::Table)
                .col(Badge::ApxID)
                .col(Badge::VdrID)
                .col(Badge::CorpID)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Badge::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Badge {
    #[sea_orm(iden = "badges")]
    Table,
    Id,
    /// Badgeを作った法人の UsrID
    CorpID,
    Name,
    /// バッジを fe で表示する時の短い名前
    ShortName,
    Description,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
