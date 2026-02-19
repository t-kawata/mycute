use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Apps::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Apps::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Apps::ApxId).integer().not_null())
                    .col(ColumnDef::new(Apps::VdrId).integer().not_null())
                    .col(ColumnDef::new(Apps::IdentityId).integer().not_null()) // Identitiesテーブルへの外部キー
                    
                    // P2P 3-Key システム (Identity/App/Versionの整合性検証用)
                    .col(ColumnDef::new(Apps::GlobalAppId).uuid().not_null())
                    .col(ColumnDef::new(Apps::GlobalAppVersion).string().not_null()) // バージョン表記 (例: "000.00.00")
                    .col(ColumnDef::new(Apps::GlobalAppHash).string().not_null())
                    
                    .col(ColumnDef::new(Apps::Name).string().not_null())
                    .col(ColumnDef::new(Apps::Layer).string().not_null()) // 配置レイヤー: Preinstall, Local, Remote
                    .col(ColumnDef::new(Apps::InstallPath).string().null())
                    .col(ColumnDef::new(Apps::RemoteUrl).string().null())
                    
                    .col(ColumnDef::new(Apps::Properties).json().null())
                    .col(
                        ColumnDef::new(Apps::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Apps::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )

                    // ===================================
                    // 検証用カラム (Phase 5.5: 署名とチェーン)
                    // ===================================
                    .col(ColumnDef::new(Apps::DevPublicKey).string().null())
                    .col(ColumnDef::new(Apps::ManifestData).json().null())
                    .col(ColumnDef::new(Apps::Verifications).json().null())
                    .col(ColumnDef::new(Apps::VerificationResultsCache).json().null())

                    // ===================================
                    // 信頼および投票用カラム
                    // ===================================
                    .col(ColumnDef::new(Apps::Author).string().null())
                    // Foreign Key (SQLite requires this inside Table::create)
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_apps_identity_id")
                            .from(Apps::Table, Apps::IdentityId)
                            .to(Identities::Table, Identities::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("apps_apxid_vdrid_idx")
                    .table(Apps::Table)
                    .col(Apps::ApxId)
                    .col(Apps::VdrId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("apps_global_app_id_idx")
                    .table(Apps::Table)
                    .col(Apps::GlobalAppId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Apps::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Apps {
    Table,
    /// 内部的な自動インクリメントID
    Id,
    /// 所属する APX ID (パーティショニング用)
    ApxId,
    /// 所属する VDR ID (パーティショニング用)
    VdrId,
    /// 所有者のアイデンティティID (identities.id への外部参照)
    IdentityId,
    /// アプリケーションの不変なグローバルID (UUID v4)
    GlobalAppId,
    /// アプリケーションのバージョン記法 (例: "000.00.00")
    GlobalAppVersion,
    /// 署名対象となるパッケージのハッシュ値。改ざん検知に使用。
    GlobalAppHash,
    /// アプリケーション名
    Name,
    /// アプリの配置レイヤー (Preinstall, Local, Remote)
    Layer,
    /// ローカル環境でのインストールパス
    InstallPath,
    /// リモート環境でのベースURL
    RemoteUrl,
    /// アプリ特有のプロパティ設定 (JSON)
    Properties,
    /// レコード作成日時
    CreatedAt,
    /// レコード更新日時
    UpdatedAt,

    // ===================================
    // 検証用カラム (Phase 5.5: 署名とチェーン)
    // ===================================
    /// 開発者公開鍵 (Ed448 Hex 128 chars)
    DevPublicKey,
    /// マニフェスト全量 (JSON)
    ManifestData,
    /// [証拠]: 開発者が提供した生の検証情報のリスト
    Verifications,
    /// [検証結果キャッシュ]: ノードが検証した結果の詳細リスト
    VerificationResultsCache,

    // ===================================
    // 信頼スコアおよび投票用カラム (Phase 5.5)
    // ===================================
    /// [著者]: マニフェストから抽出された表示用の著者名。
    Author,
}

#[derive(DeriveIden)]
enum Identities {
    Table,
    Id,
}
