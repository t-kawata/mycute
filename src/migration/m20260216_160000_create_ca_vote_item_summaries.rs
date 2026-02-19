use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CaVoteItemSummaries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CaVoteItemSummaries::Id)
                            .binary_len(16)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CaVoteItemSummaries::NodePubkey).string().not_null())
                    .col(ColumnDef::new(CaVoteItemSummaries::ForumId).binary_len(16).not_null())
                    .col(ColumnDef::new(CaVoteItemSummaries::AppId).string().not_null())
                    .col(ColumnDef::new(CaVoteItemSummaries::VoteAllocated).integer().not_null()) // vote_allocated
                    .col(ColumnDef::new(CaVoteItemSummaries::NodeTimestamp).big_integer().not_null())
                    .col(ColumnDef::new(CaVoteItemSummaries::NodeSignature).string().not_null())
                    .col(ColumnDef::new(CaVoteItemSummaries::CaTimestamp).big_integer().not_null())
                    .col(ColumnDef::new(CaVoteItemSummaries::CaSignature).string().not_null())
                    .col(
                        ColumnDef::new(CaVoteItemSummaries::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CaVoteItemSummaries::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Composite Unique Index (NodePubkey, ForumId, AppId)
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_ca_vote_item_summaries_node_forum_app")
                    .table(CaVoteItemSummaries::Table)
                    .col(CaVoteItemSummaries::NodePubkey)
                    .col(CaVoteItemSummaries::ForumId)
                    .col(CaVoteItemSummaries::AppId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index on ForumId
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_ca_vote_item_summaries_forum_id")
                    .table(CaVoteItemSummaries::Table)
                    .col(CaVoteItemSummaries::ForumId)
                    .to_owned(),
            )
            .await?;

        // Index on AppId
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_ca_vote_item_summaries_app_id")
                    .table(CaVoteItemSummaries::Table)
                    .col(CaVoteItemSummaries::AppId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CaVoteItemSummaries::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum CaVoteItemSummaries {
    /// テーブル名
    Table,
    /// ID (UUID)
    Id,
    /// ノードの公開鍵
    NodePubkey,
    /// フォーラムID (UUID)
    ForumId,
    /// アプリID (UUID)
    AppId,
    /// このアプリへの投票配分量 (vote_allocated)
    VoteAllocated,
    /// ノードの申告タイムスタンプ
    NodeTimestamp,
    /// ノードの自白署名 (Hex)
    NodeSignature,
    /// CAの認定タイムスタンプ
    CaTimestamp,
    /// CAの認定署名 (Hex)
    CaSignature,
    /// レコード作成日時
    CreatedAt,
    /// レコード更新日時
    UpdatedAt,
}
