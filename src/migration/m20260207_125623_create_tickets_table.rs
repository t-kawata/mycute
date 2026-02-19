use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tickets::Table)
                    .if_not_exists()
                    .col(pk_auto(Tickets::Id))
                    .col(string(Tickets::CaPubkey))
                    .col(string(Tickets::CaBaseUrl))
                    .col(uuid(Tickets::ForumId))
                    .col(string(Tickets::ForumName))
                    .col(string_null(Tickets::ForumDescription))
                    .col(json(Tickets::TicketData))
                    .col(string_null(Tickets::CaToken))
                    .col(date_time(Tickets::CreatedAt))
                    .col(date_time(Tickets::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        // 複合ユニーク制約 (ca_pubkey, ca_base_url, forum_id)
        manager
            .create_index(
                Index::create()
                    .name("idx_tickets_capubkey_cabaseurl_forumid_unique")
                    .table(Tickets::Table)
                    .col(Tickets::CaPubkey)
                    .col(Tickets::CaBaseUrl)
                    .col(Tickets::ForumId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // 検索用インデックス (CaPubkey + ForumId)
        manager
            .create_index(
                Index::create()
                    .name("idx_tickets_capubkey_forumid")
                    .table(Tickets::Table)
                    .col(Tickets::CaPubkey)
                    .col(Tickets::ForumId)
                    .to_owned(),
            )
            .await?;

        // 検索用インデックス (CaBaseUrl + ForumId)
        manager
            .create_index(
                Index::create()
                    .name("idx_tickets_cabaseurl_forumid")
                    .table(Tickets::Table)
                    .col(Tickets::CaBaseUrl)
                    .col(Tickets::ForumId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tickets::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Tickets {
    /// テーブル名
    Table,
    /// 内部的な自動インクリメントID
    Id,
    /// チケットを発行したCAの公開鍵
    CaPubkey,
    /// チケットを発行したCAのベースURL
    CaBaseUrl,
    /// チケットが属するフォーラムID (UUID)
    ForumId,
    /// フォーラム名 (視認用)
    ForumName,
    /// フォーラム説明 (視認用)
    ForumDescription,
    /// チケットの本体データ (JSON)
    TicketData,
    /// CAから発行された任命証トークン (Option)
    CaToken,
    /// レコード作成日時
    CreatedAt,
    /// レコード更新日時
    UpdatedAt,
}
