use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Vdr単位の現金プール
        manager.create_table(
            Table::create()
                .table(Pool::Table)
                .if_not_exists()
                .col(pk_auto(Pool::Id))
                .col(unsigned(Pool::ApxID).not_null().default(0))
                .col(unsigned(Pool::VdrID).not_null().default(0))
                .col(unsigned(Pool::Remain).not_null().default(0))
                .col(unsigned(Pool::TotalIn).not_null().default(0))
                .col(unsigned(Pool::TotalOut).not_null().default(0))
                .col(ColumnDef::new(Pool::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Pool::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("pool_apxid_vdrid_idx")
                .table(Pool::Table)
                .col(Pool::ApxID)
                .col(Pool::VdrID)
                .unique()
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Pool::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Pool {
    #[sea_orm(iden = "pools")]
    Table,
    Id,
    ApxID,
    /// この現金プールの所属Vdr
    VdrID,
    /// 現在の現金プール残高
    Remain,
    /// 過去全期間の流入総額
    TotalIn,
    /// 過去全期間の分配総額
    TotalOut,
    CreatedAt,
    UpdatedAt,
}
