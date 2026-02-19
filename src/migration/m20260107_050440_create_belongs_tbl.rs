use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Belong::Table)
                .if_not_exists()
                .col(pk_auto(Belong::Id))
                .col(unsigned(Belong::CorpID).not_null().default(0))
                .col(unsigned(Belong::UsrID).not_null().default(0))
                .col(ColumnDef::new(Belong::OpenAt).date_time().null())
                .col(ColumnDef::new(Belong::CloseAt).date_time().null())
                .col(unsigned(Belong::ApxID).not_null())
                .col(unsigned(Belong::VdrID).not_null())
                .col(ColumnDef::new(Belong::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Belong::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("belong_apxid_vdrid_corpid_idx")
                .table(Belong::Table)
                .col(Belong::ApxID)
                .col(Belong::VdrID)
                .col(Belong::CorpID)
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("belong_apxid_vdrid_usrid_idx")
                .table(Belong::Table)
                .col(Belong::ApxID)
                .col(Belong::VdrID)
                .col(Belong::UsrID)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Belong::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Belong {
    #[sea_orm(iden = "belongs")]
    Table,
    Id,
    /// 所属先法人の UsrID
    CorpID,
    /// 個人の UsrID
    UsrID,
    /// 所属開始
    OpenAt,
    /// 所属終了
    CloseAt,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
