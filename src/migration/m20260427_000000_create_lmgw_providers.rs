use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop chat_models if it exists (legacy)
        let _ = manager
            .drop_table(
                Table::drop()
                    .table(ChatModels::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await;

        manager
            .create_table(
                Table::create()
                    .table(LmgwProviders::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LmgwProviders::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(LmgwProviders::ApxId).integer().not_null().default(0))
                    .col(ColumnDef::new(LmgwProviders::VdrId).integer().not_null().default(0))
                    .col(ColumnDef::new(LmgwProviders::ProviderName).string().not_null())
                    .col(ColumnDef::new(LmgwProviders::ConfigJson).text().not_null())
                    .col(
                        ColumnDef::new(LmgwProviders::CreatedAt)
                            .date_time()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(LmgwProviders::UpdatedAt)
                            .date_time()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        // 複合ユニークインデックスの作成
        manager
            .create_index(
                Index::create()
                    .name("idx_lmgw_providers_unique")
                    .table(LmgwProviders::Table)
                    .col(LmgwProviders::ApxId)
                    .col(LmgwProviders::VdrId)
                    .col(LmgwProviders::ProviderName)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LmgwProviders::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LmgwProviders {
    Table,
    Id,
    ApxId,
    VdrId,
    ProviderName,
    ConfigJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ChatModels {
    Table,
}
