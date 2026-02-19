use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ChatModels::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChatModels::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ChatModels::Name).string_len(50).not_null().default(""))
                    .col(ColumnDef::new(ChatModels::Provider).string_len(50).not_null().default(""))
                    .col(ColumnDef::new(ChatModels::Model).string_len(100).not_null().default(""))
                    .col(ColumnDef::new(ChatModels::BaseUrl).string_len(255).not_null().default(""))
                    .col(ColumnDef::new(ChatModels::ApiKey).string_len(1024).not_null().default(""))
                    .col(ColumnDef::new(ChatModels::MaxTokens).integer().not_null().default(0))
                    .col(ColumnDef::new(ChatModels::Temperature).double().not_null().default(0.0))
                    .col(ColumnDef::new(ChatModels::ApxId).integer().not_null())
                    .col(ColumnDef::new(ChatModels::VdrId).integer().not_null())
                    .col(
                        ColumnDef::new(ChatModels::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ChatModels::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_chat_models_apx_vdr_id")
                    .table(ChatModels::Table)
                    .col(ChatModels::ApxId)
                    .col(ChatModels::VdrId)
                    .col(ChatModels::Id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ChatModels::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ChatModels {
    Table,
    Id,
    Name,
    Provider,
    Model,
    BaseUrl,
    ApiKey,
    MaxTokens,
    Temperature,
    ApxId,
    VdrId,
    CreatedAt,
    UpdatedAt,
}
