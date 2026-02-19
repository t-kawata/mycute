use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Blacklists::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Blacklists::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    // 対象ノードの公開鍵 (Hex)
                    .col(
                        ColumnDef::new(Blacklists::TargetPubkey)
                            .string_len(128)
                            .not_null(),
                    )
                    // 不正の証拠 (JSON: CrimeEvidence)
                    // 署名、タイムスタンプ、生メッセージ等を含む
                    .col(ColumnDef::new(Blacklists::EvidenceJson).text().not_null())
                    // 犯罪種別 (Enum 値を整数で保存)
                    .col(ColumnDef::new(Blacklists::CrimeType).integer().not_null())
                    // 観測時刻 (ms単位タイムスタンプ)
                    .col(ColumnDef::new(Blacklists::ObservedAt).big_integer().not_null())
                    // 刑期 (時間単位)
                    .col(ColumnDef::new(Blacklists::PrisonTermHours).big_integer().not_null())
                    .col(
                        ColumnDef::new(Blacklists::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Blacklists::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // 同一ノードに対するレコードは1つのみ
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("blacklists_target_pubkey_unique")
                    .table(Blacklists::Table)
                    .col(Blacklists::TargetPubkey)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Blacklists::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Blacklists {
    Table,
    Id,
    /// 対象ノードの公開鍵
    TargetPubkey,
    /// 不正の証拠データ (JSON: CrimeEvidence)
    EvidenceJson,
    /// 犯罪種別
    CrimeType,
    /// 観測時刻
    ObservedAt,
    /// 刑期
    PrisonTermHours,
    CreatedAt,
    UpdatedAt,
}
