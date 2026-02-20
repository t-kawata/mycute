use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CaVoteAllocatedSummaries::Table)
                    .if_not_exists()
                    .col(pk_uuid(CaVoteAllocatedSummaries::Id))
                    .col(string(CaVoteAllocatedSummaries::NodePubkey))
                    .col(uuid(CaVoteAllocatedSummaries::ForumId))
                    .col(integer(CaVoteAllocatedSummaries::VoteAllocated))
                    .col(big_integer(CaVoteAllocatedSummaries::NodeTimestamp))
                    .col(string(CaVoteAllocatedSummaries::NodeSignature))
                    .col(big_integer(CaVoteAllocatedSummaries::CaTimestamp))
                    .col(string(CaVoteAllocatedSummaries::CaSignature))
                    .col(date_time(CaVoteAllocatedSummaries::CreatedAt))
                    .col(date_time(CaVoteAllocatedSummaries::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        // Create Unique Index (node_pubkey, forum_id)
        manager
            .create_index(
                Index::create()
                    .name("idx_ca_vote_allocated_summaries_node_forum")
                    .table(CaVoteAllocatedSummaries::Table)
                    .col(CaVoteAllocatedSummaries::NodePubkey)
                    .col(CaVoteAllocatedSummaries::ForumId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(CaVoteAllocatedSummaries::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum CaVoteAllocatedSummaries {
    /// テーブル名
    Table,
    /// ID (UUID)
    Id,
    /// ノードの公開鍵
    NodePubkey,
    /// フォーラムID (UUID)
    ForumId,
    /// 投票消費量 (vote_allocated)
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
