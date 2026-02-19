use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Bd::Table)
                    .if_not_exists()
                    .col(pk_auto(Bd::Id))
                    .col(string_len(Bd::Hash, 60).not_null().default(""))
                    .col(ColumnDef::new(Bd::BgnAt).date_time().not_null())
                    .col(ColumnDef::new(Bd::EndAt).date_time().not_null())
                    .col(ColumnDef::new(Bd::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Bd::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Bd::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Bd {
    #[sea_orm(iden = "bds")]
    Table,
    Id,
    Hash,
    BgnAt,
    EndAt,
    CreatedAt,
    UpdatedAt,
}
