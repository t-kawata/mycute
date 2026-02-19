use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Crypto::Table)
                .if_not_exists()
                .col(pk_auto(Crypto::Id))
                .col(string_len(Crypto::Key, 50).not_null().default("").unique_key())
                .col(string_len(Crypto::Value, 1024).not_null().default(""))
                .col(unsigned(Crypto::ApxID).null())
                .col(unsigned(Crypto::VdrID).null())
                .col(ColumnDef::new(Crypto::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Crypto::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("crypto_key_dx")
                .table(Crypto::Table)
                .col(Crypto::Key)
                .unique()
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Crypto::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Crypto {
    #[sea_orm(iden = "cryptos")]
    Table,
    Id,
    Key,
    Value,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
