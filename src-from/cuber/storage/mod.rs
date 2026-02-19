//! Storage Abstraction Layer
//!
//! LadybugDB を隠蔽するためのトレイトとストレージセット構造体を定義します。
//! Go 版 `storage/interfaces.go` に相当します。

pub mod ladybug;

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

use super::error::CuberError;

// ============================================================
// Storage Traits (Scaffolding)
// ============================================================

/// ベクトルストレージのトレイト
///
/// Go 版 `storage.VectorStorage` に相当します。
/// ドキュメント、チャンク、ベクトル、FTS などの操作を担当します。
#[async_trait::async_trait]
pub trait VectorStorage: Send + Sync {
    /// ストレージを閉じます。
    async fn close(&self) -> Result<(), CuberError>;

    // TODO: Phase 5 (Absorb) で追加するメソッド
    // - add_data: ファイルメタデータの保存
    // - add_document: ドキュメントの保存
    // - add_chunk: チャンクの保存
    // - add_vector: ベクトルの保存
    // - search_vector: ベクトル検索

    // TODO: Phase 6 (Query) で追加するメソッド
    // - search_fts: 全文検索
    // - get_chunk_by_id: チャンク取得

    // TODO: Phase 7 (Memify) で追加するメソッド
    // - add_summary: 要約の保存
}

/// グラフストレージのトレイト
///
/// Go 版 `storage.GraphStorage` に相当します。
/// 知識グラフ（Node, Edge, Triple）の操作を担当します。
#[async_trait::async_trait]
pub trait GraphStorage: Send + Sync {
    /// ストレージを閉じます。
    async fn close(&self) -> Result<(), CuberError>;

    // TODO: Phase 5 (Absorb - GraphExtraction) で追加するメソッド
    // - add_node: ノードの追加
    // - add_edge: エッジの追加
    // - merge_node: ノードの UPSERT
    // - merge_edge: エッジの UPSERT

    // TODO: Phase 6 (Query) で追加するメソッド
    // - get_triples: トリプル取得
    // - traverse: グラフトラバーサル

    // TODO: Phase 7 (Memify - Metabolism) で追加するメソッド
    // - update_edge_thickness: エッジの Thickness 更新
    // - prune_edges: エッジの剪定
    // - prune_orphan_nodes: 孤立ノードの削除
}

// ============================================================
// StorageSet
// ============================================================

/// ストレージセット
///
/// Go 版 `StorageSet` に相当します。
/// 単一の Cube（DB ファイル）に対応する Vector と Graph ストレージのペアを保持します。
pub struct StorageSet {
    /// ベクトルストレージ（LadybugDB の Vector 層）
    pub vector: Arc<dyn VectorStorage>,

    /// グラフストレージ（LadybugDB の Graph 層）
    pub graph: Arc<dyn GraphStorage>,

    /// 最後にアクセスされた時刻（アイドルタイムアウト用）
    pub last_used_at: RwLock<Instant>,
}

impl StorageSet {
    /// 新しい StorageSet を作成します。
    pub fn new(vector: Arc<dyn VectorStorage>, graph: Arc<dyn GraphStorage>) -> Self {
        Self {
            vector,
            graph,
            last_used_at: RwLock::new(Instant::now()),
        }
    }

    /// アクセス時刻を更新します。
    pub async fn touch(&self) {
        *self.last_used_at.write().await = Instant::now();
    }

    /// 最後のアクセスからの経過時間を取得します。
    pub async fn idle_duration(&self) -> std::time::Duration {
        Instant::now().duration_since(*self.last_used_at.read().await)
    }

    /// 両方のストレージを閉じます。
    pub async fn close(&self) -> Result<(), CuberError> {
        // Vector と Graph を順番に閉じる
        self.vector.close().await?;
        self.graph.close().await?;
        Ok(())
    }
}
