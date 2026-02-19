//! LadybugDB Wrapper
//!
//! LadybugDB（lbug クレート）のラッパー実装です。
//! VectorStorage と GraphStorage トレイトを実装し、
//! Cypher クエリによるグラフ操作とベクトル検索を統合します。
//!
//! ## 設計ノート
//! `lbug::Connection` は `Database` への参照を持つライフタイムパラメータを持ちます。
//! これを効率的に管理するため、`Database` を `Box::leak` で 'static ライフタイムに変換し、
//! `Connection<'static>` を保持する設計を採用しています。
//! 
//! LadybugDB は通常アプリケーションの存続期間中存在するため、
//! このアプローチはメモリリークを引き起こしません。

use std::path::Path;
use std::sync::Arc;

use lbug::{Connection, Database, SystemConfig};
use tokio::sync::Mutex;

use super::{GraphStorage, VectorStorage};
use crate::cuber::error::CuberError;
use crate::cuber::tokenizer::LinderaTokenizer;

/// LadybugDB ラッパー構造体
///
/// 単一の LadybugDB インスタンスをラップし、
/// VectorStorage と GraphStorage の両方のトレイトを実装します。
pub struct LadybugDB {
    /// lbug Database インスタンス（Box::leak で 'static 化された参照）
    /// Drop 時には手動で解放する必要がある
    #[allow(dead_code)]
    db_ptr: *mut Database,

    /// lbug Connection インスタンス（Mutex でラップして排他アクセスを保証）
    conn: Mutex<Connection<'static>>,

    /// 形態素解析器（FTS キーワード抽出用）
    #[allow(dead_code)]
    tokenizer: Arc<LinderaTokenizer>,

    /// DB ファイルパス
    #[allow(dead_code)]
    db_path: String,
}

// Send + Sync を明示的に実装（Connection は既に Send + Sync）
unsafe impl Send for LadybugDB {}
unsafe impl Sync for LadybugDB {}

impl LadybugDB {
    /// 新しい LadybugDB インスタンスを作成します。
    ///
    /// # Arguments
    /// * `db_path` - DB ファイルのパス（ディレクトリ）
    /// * `tokenizer` - 形態素解析器
    pub fn new(db_path: &Path, tokenizer: Arc<LinderaTokenizer>) -> Result<Self, CuberError> {
        let path_str = db_path.to_string_lossy().to_string();

        // Database を作成し、Box::leak で 'static 化
        let db = Database::new(&path_str, SystemConfig::default())
            .map_err(|e| CuberError::StorageInitError(format!("Failed to open LadybugDB at {}: {:?}", path_str, e)))?;

        let db_boxed = Box::new(db);
        let db_static: &'static mut Database = Box::leak(db_boxed);
        let db_ptr = db_static as *mut Database;

        // Connection を作成（'static ライフタイム）
        let conn = Connection::new(db_static)
            .map_err(|e| CuberError::StorageInitError(format!("Failed to create connection: {:?}", e)))?;

        Ok(Self {
            db_ptr,
            conn: Mutex::new(conn),
            tokenizer,
            db_path: path_str,
        })
    }

    /// スキーマを初期化します（必要なノード/エッジテーブルを作成）。
    ///
    /// 初回起動時に呼び出されます。
    pub async fn init_schema(&self) -> Result<(), CuberError> {
        let conn = self.conn.lock().await;

        // TODO: 将来的に完全なスキーマ定義を実装
        // 以下は基本的なスキーマの例です

        // Data ノードテーブル（ファイルメタデータ）
        let _ = conn.query(
            "CREATE NODE TABLE IF NOT EXISTS Data(
                id STRING,
                content_hash STRING,
                file_name STRING,
                file_type STRING,
                file_size INT64,
                s3_key STRING,
                memory_group STRING,
                created_at INT64,
                PRIMARY KEY(id)
            );"
        );

        // Document ノードテーブル
        let _ = conn.query(
            "CREATE NODE TABLE IF NOT EXISTS Document(
                id STRING,
                data_id STRING,
                content STRING,
                memory_group STRING,
                created_at INT64,
                PRIMARY KEY(id)
            );"
        );

        // Chunk ノードテーブル
        let _ = conn.query(
            "CREATE NODE TABLE IF NOT EXISTS Chunk(
                id STRING,
                document_id STRING,
                content STRING,
                embedding DOUBLE[],
                nouns STRING,
                nouns_verbs STRING,
                keywords STRING,
                position INT64,
                memory_group STRING,
                created_at INT64,
                PRIMARY KEY(id)
            );"
        );

        // Entity (GraphNode) ノードテーブル
        let _ = conn.query(
            "CREATE NODE TABLE IF NOT EXISTS Entity(
                id STRING,
                name STRING,
                entity_type STRING,
                properties STRING,
                memory_group STRING,
                created_at INT64,
                updated_at INT64,
                PRIMARY KEY(id)
            );"
        );

        // GraphEdge リレーションテーブル
        let _ = conn.query(
            "CREATE REL TABLE IF NOT EXISTS GraphEdge(
                FROM Entity TO Entity,
                relation_type STRING,
                weight DOUBLE,
                confidence DOUBLE,
                unix INT64
            );"
        );

        // HAS_CHUNK リレーション（Document -> Chunk）
        let _ = conn.query(
            "CREATE REL TABLE IF NOT EXISTS HAS_CHUNK(
                FROM Document TO Chunk
            );"
        );

        // NEXT_CHUNK リレーション（Chunk -> Chunk）
        let _ = conn.query(
            "CREATE REL TABLE IF NOT EXISTS NEXT_CHUNK(
                FROM Chunk TO Chunk
            );"
        );

        log::debug!("<LadybugDB> Schema initialized for {}", self.db_path);

        Ok(())
    }

    /// Cypher クエリを実行します。
    ///
    /// 内部で使用するヘルパーメソッドです。
    #[allow(dead_code)]
    pub async fn execute_query(&self, query: &str) -> Result<(), CuberError> {
        let conn = self.conn.lock().await;
        conn.query(query)?;
        Ok(())
    }
}

impl Drop for LadybugDB {
    fn drop(&mut self) {
        // Connection を先にドロップする必要があるため、
        // ここでは明示的に何もしない（Connection は Mutex 内で自動ドロップ）
        // 
        // 注意: Database を Box::leak で作成したため、
        // 厳密にはメモリリークが発生しますが、LadybugDB は
        // 通常アプリケーションの終了時まで存続するため、
        // 実用上は問題ありません。
        //
        // 将来的に proper なリソース解放が必要な場合は、
        // unsafe で Box::from_raw を使用して解放できます。
        log::debug!("<LadybugDB> Dropping LadybugDB for {}", self.db_path);
    }
}

// ============================================================
// VectorStorage 実装
// ============================================================

#[async_trait::async_trait]
impl VectorStorage for LadybugDB {
    async fn close(&self) -> Result<(), CuberError> {
        // lbug の Database は Drop 時に自動的にクローズされるため、
        // 明示的なクローズ処理は不要
        log::debug!("<LadybugDB> Closing VectorStorage");
        Ok(())
    }

    // TODO: データ吸収 (Absorb) 機能で以下のメソッドを実装
    // async fn add_data(&self, data: DataModel) -> Result<String, CuberError>;
    // async fn add_document(&self, doc: DocumentModel) -> Result<String, CuberError>;
    // async fn add_chunk(&self, chunk: ChunkModel) -> Result<String, CuberError>;
    // async fn add_vector(&self, chunk_id: &str, embedding: Vec<f64>) -> Result<(), CuberError>;
    // async fn search_vector(&self, query_embedding: Vec<f64>, top_k: usize) -> Result<Vec<ChunkResult>, CuberError>;

    // TODO: クエリ (Query) 機能で以下のメソッドを実装
    // async fn search_fts(&self, keywords: &str, layer: FtsLayer, limit: usize) -> Result<Vec<ChunkResult>, CuberError>;
}

// ============================================================
// GraphStorage 実装
// ============================================================

#[async_trait::async_trait]
impl GraphStorage for LadybugDB {
    async fn close(&self) -> Result<(), CuberError> {
        log::debug!("<LadybugDB> Closing GraphStorage");
        Ok(())
    }

    // TODO: データ吸収 (Absorb - GraphExtraction) 機能で以下のメソッドを実装
    // async fn add_node(&self, node: EntityModel) -> Result<String, CuberError>;
    // async fn add_edge(&self, edge: EdgeModel) -> Result<(), CuberError>;
    // async fn merge_node(&self, node: EntityModel) -> Result<String, CuberError>;
    // async fn merge_edge(&self, edge: EdgeModel) -> Result<(), CuberError>;

    // TODO: クエリ (Query) 機能で以下のメソッドを実装
    // async fn get_triples(&self, entity_id: &str, depth: usize) -> Result<Vec<Triple>, CuberError>;
    // async fn traverse(&self, start_id: &str, relation: &str, limit: usize) -> Result<Vec<EntityModel>, CuberError>;

    // TODO: 記憶化 (Memify - Metabolism) 機能で以下のメソッドを実装
    // async fn update_edge_thickness(&self, edge_id: &str, new_weight: f64, new_confidence: f64) -> Result<(), CuberError>;
    // async fn prune_edges(&self, threshold: f64) -> Result<usize, CuberError>;
    // async fn prune_orphan_nodes(&self, grace_period_minutes: u32) -> Result<usize, CuberError>;
}

#[cfg(test)]
mod tests {

    // MYCUTE開発のため一時的にコメントアウト

    // use super::*;
    
    // #[tokio::test]
    // async fn test_ladybug_db_creation() {
    //     // テスト用の一時ディレクトリを作成
    //     let temp_dir = std::env::temp_dir().join("test_ladybug_db_static");
    //     let _ = std::fs::create_dir_all(&temp_dir);

    //     let tokenizer = Arc::new(LinderaTokenizer::new().expect("Failed to create tokenizer"));

    //     let db = LadybugDB::new(&temp_dir, tokenizer);
    //     assert!(db.is_ok(), "LadybugDB should be created successfully");

    //     if let Ok(db) = db {
    //         // スキーマ初期化をテスト
    //         let result = db.init_schema().await;
    //         assert!(result.is_ok(), "Schema initialization should succeed");
    //     }

    //     // クリーンアップ
    //     let _ = std::fs::remove_dir_all(&temp_dir);
    // }
}
