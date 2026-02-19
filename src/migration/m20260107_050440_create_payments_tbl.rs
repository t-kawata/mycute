use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Payment::Table)
                .if_not_exists()
                .col(pk_auto(Payment::Id))
                .col(unsigned(Payment::CorpID).not_null().default(0))
                .col(tiny_unsigned(Payment::Type).not_null().default(0))
                .col(unsigned(Payment::Amount).not_null().default(0))
                .col(unsigned(Payment::Fee).not_null().default(0))
                .col(unsigned(Payment::Net).not_null().default(0))
                .col(string_len(Payment::Note, 255).not_null().default(""))
                .col(unsigned(Payment::ApxID).not_null())
                .col(unsigned(Payment::VdrID).not_null())
                .col(ColumnDef::new(Payment::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Payment::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("payment_apxid_vdrid_corpid_idx")
                .table(Payment::Table)
                .col(Payment::ApxID)
                .col(Payment::VdrID)
                .col(Payment::CorpID)
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Payment::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Payment {
    #[sea_orm(iden = "payments")]
    Table,
    Id,
    /// 支払った法人 UsrID
    CorpID,
    /// 1:面談フィー, 2:採用紹介料, etc.
    Type,
    /// 支払金額
    Amount,
    /// 運営費控除分
    Fee,
    /// プール流入額（Amount - Fee）
    Net,
    /// メモ
    Note,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
