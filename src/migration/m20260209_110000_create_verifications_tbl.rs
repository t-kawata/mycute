use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Verifications::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Verifications::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    // 検証対象ノードの公開鍵
                    .col(
                        ColumnDef::new(Verifications::NodePubkey)
                            .string_len(128)
                            .not_null(),
                    )
                    // 検証を行ったCAの公開鍵
                    .col(
                        ColumnDef::new(Verifications::CaPubkey)
                            .string_len(128)
                            .not_null(),
                    )
                    // 検証を行ったCAのベースURL
                    .col(ColumnDef::new(Verifications::CaBaseUrl).string().not_null())
                    // CAの署名データ
                    .col(ColumnDef::new(Verifications::Signature).text().null())
                    // CAの任命証トークン
                    .col(ColumnDef::new(Verifications::CaToken).text().null())
                    // 申請中フラグ
                    .col(
                        ColumnDef::new(Verifications::IsCandidate)
                            .tiny_integer()
                            .not_null()
                            .default(0),
                    )
                    // 検証日時
                    .col(ColumnDef::new(Verifications::VerifiedAt).date_time().null())
                    // 有効期限
                    .col(ColumnDef::new(Verifications::ExpireAt).date_time().null())
                    // 申請された有効期限秒数
                    .col(
                        ColumnDef::new(Verifications::AppliedExpireSeconds)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Verifications::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Verifications::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // 同一ノード×CA公開鍵は1レコードのみ
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("verifications_nodepubkey_capubkey_unique")
                    .table(Verifications::Table)
                    .col(Verifications::NodePubkey)
                    .col(Verifications::CaPubkey)
                    .to_owned(),
            )
            .await?;

        // 同一ノード×CAベースURLは1レコードのみ (MyRem依存脱却のため)
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("verifications_nodepubkey_cabaseurl_unique")
                    .table(Verifications::Table)
                    .col(Verifications::NodePubkey)
                    .col(Verifications::CaBaseUrl)
                    .to_owned(),
            )
            .await?;

        // CA 公開鍵での検索を高速化するためインデックスを追加
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("verifications_capubkey_idx")
                    .table(Verifications::Table)
                    .col(Verifications::CaPubkey)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Verifications::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Verifications {
    Table,
    /// 内部的な自動インクリメントID
    Id,
    /// 検証対象ノードの公開鍵
    NodePubkey,
    /// 検証を行ったCAの公開鍵
    CaPubkey,
    /// 検証を行ったCAのベースURL
    CaBaseUrl,
    /// CAによる署名データ
    Signature,
    /// CAの任命証トークン
    CaToken,
    /// 申請中フラグ (1=申請中, 0=通常)
    IsCandidate,
    /// 検証日時
    VerifiedAt,
    /// 有効期限
    ExpireAt,
    /// 申請された有効期限秒数
    AppliedExpireSeconds,
    /// レコード作成日時
    CreatedAt,
    /// レコード更新日時
    UpdatedAt,
}
