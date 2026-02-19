use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Identities::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Identities::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Identities::ApxId).integer().not_null())
                    .col(ColumnDef::new(Identities::VdrId).integer().not_null())
                    .col(
                        ColumnDef::new(Identities::PublicKey)
                            .string_len(128)
                            .not_null()
                            .unique_key(), // ED448公開鍵はグローバルな一意識別子として機能する
                    )
                    .col(ColumnDef::new(Identities::Info).json().null())
                    .col(
                        ColumnDef::new(Identities::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Identities::UpdatedAt)
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
                    .name("identities_apxid_vdrid_idx")
                    .table(Identities::Table)
                    .col(Identities::ApxId)
                    .col(Identities::VdrId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("identities_public_key_idx")
                    .table(Identities::Table)
                    .col(Identities::PublicKey)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Identities::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Identities {
    Table,
    /// 内部的な自動インクリメントID
    Id,
    /// 所属する APX (Application Proxy) の ID
    ApxId,
    /// 所属する VDR (Vendor) の ID
    VdrId,
    /// 公開鍵 (Ed448)。グローバルで一意なアイデンティティ識別子として機能する。
    PublicKey,
    /// プロフィール情報等のメタデータ (JSON)
    Info,
    /// レコード作成日時
    CreatedAt,
    /// レコード更新日時
    UpdatedAt,
}
