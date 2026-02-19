//! Cuber Service
//!
//! Cuber システムの中心的なエントリポイントです。
//! Go 版 `cuber.go` の `CuberService` に相当します。
//!
//! ## 主な責務
//! - ストレージ（LadybugDB）接続のプーリングとライフサイクル管理
//! - S3 クライアント、Tokenizer、EventBus の共有
//! - バックグラウンドタスク（GC、S3 クリーンアップ）の管理
//! - Absorb / Query / Memify オペレーションのオーケストレーション

use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use super::config::CuberConfig;
use super::error::CuberError;
use super::event::EventBus;
use super::storage::ladybug::LadybugDB;
use super::storage::StorageSet;
use super::tokenizer::LinderaTokenizer;
use crate::utils::s3client::S3Client;

/// Cuber サービス
///
/// Go 版 `CuberService` に相当します。
/// 知識管理システムのコア機能を提供します。
pub struct CuberService {
    /// ストレージマップ（UUID -> StorageSet）
    ///
    /// DashMap を使用して、シャーデッドロックによる高い並行性を実現。
    storage_map: Arc<DashMap<String, Arc<StorageSet>>>,

    /// 設定
    pub config: CuberConfig,

    /// S3 クライアント（外部から注入）
    pub s3_client: Arc<S3Client>,

    /// 形態素解析器（Lindera）
    pub tokenizer: Arc<LinderaTokenizer>,

    /// イベントバス
    pub event_bus: Arc<EventBus>,

    /// キャンセルトークン（グレースフルシャットダウン用）
    cancel_token: CancellationToken,
}

impl CuberService {
    /// 新しい CuberService を作成します。
    ///
    /// # Arguments
    /// * `config` - Cuber 設定
    /// * `s3_client` - 共有 S3 クライアント
    ///
    /// # Returns
    /// 初期化された CuberService インスタンス
    pub async fn new(config: CuberConfig, s3_client: Arc<S3Client>) -> Result<Self, CuberError> {
        log::info!("<CuberService> Initializing CuberService...");

        // Tokenizer の初期化
        log::debug!("<CuberService> Initializing Lindera tokenizer...");
        let tokenizer = Arc::new(LinderaTokenizer::new()?);
        log::debug!("<CuberService> Lindera tokenizer initialized.");

        // EventBus の作成
        let event_bus = Arc::new(EventBus::default());

        // S3 接続/権限の確認
        log::debug!("<CuberService> Validating S3 connection...");
        s3_client.validate_connection().await.map_err(|e| CuberError::InternalError(e.to_string()))?;
        log::debug!("<CuberService> S3 connection validated.");

        // CancellationToken の作成
        let cancel_token = CancellationToken::new();

        let service = Self {
            storage_map: Arc::new(DashMap::new()),
            config,
            s3_client,
            tokenizer,
            event_bus,
            cancel_token,
        };

        // DB ディレクトリの存在確認・作成
        service.ensure_db_dir().await?;

        // バックグラウンドタスクの起動
        service.spawn_background_tasks();

        log::info!("<CuberService> CuberService initialized successfully.");
        Ok(service)
    }

    /// DB ディレクトリの存在を確認し、必要なら作成します。
    async fn ensure_db_dir(&self) -> Result<(), CuberError> {
        let db_path = PathBuf::from(&self.config.db_dir_path);
        if !db_path.exists() {
            log::debug!("<CuberService> Creating DB directory: {}", self.config.db_dir_path);
            tokio::fs::create_dir_all(&db_path).await?;
        }
        Ok(())
    }

    /// バックグラウンドタスクを起動します。
    ///
    /// - ストレージ GC タスク：アイドル状態のストレージ接続を閉じる
    fn spawn_background_tasks(&self) {
        let token = self.cancel_token.clone();
        let storage_map = Arc::clone(&self.storage_map);
        let retention = Duration::from_secs(self.config.storage_idle_timeout_minutes as u64 * 60);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(60)); // 1分ごとにチェック

            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        log::debug!("<CuberService> Background GC task cancelled.");
                        break;
                    },
                    _ = ticker.tick() => {
                        // アイドルストレージのクリーンアップ
                        let now = Instant::now();
                        let mut to_remove: Vec<String> = Vec::new();

                        for entry in storage_map.iter() {
                            let storage_set = entry.value();
                            let last_used = *storage_set.last_used_at.read().await;
                            if now.duration_since(last_used) > retention {
                                to_remove.push(entry.key().clone());
                            }
                        }

                        for uuid in to_remove {
                            log::debug!("<CuberService> Closing idle storage: {}", uuid);
                            if let Some((_, set)) = storage_map.remove(&uuid) {
                                if let Err(e) = set.close().await {
                                    log::error!("<CuberService> Error closing storage {}: {:?}", uuid, e);
                                }
                            }
                        }
                    }
                }
            }
        });

        log::debug!("<CuberService> Background tasks spawned.");
    }

    /// 指定された UUID に対応するストレージを取得または開きます。
    ///
    /// # Arguments
    /// * `uuid` - Cube の UUID
    ///
    /// # Returns
    /// 共有された StorageSet
    pub async fn get_or_open_storage(&self, uuid: &str) -> Result<Arc<StorageSet>, CuberError> {
        // 既存のストレージがあれば返す
        if let Some(entry) = self.storage_map.get(uuid) {
            let storage_set = Arc::clone(entry.value());
            storage_set.touch().await;
            log::debug!("<CuberService> Returning cached storage for UUID: {}", uuid);
            return Ok(storage_set);
        }

        // 新規にストレージを開く
        log::debug!("<CuberService> Opening new storage for UUID: {}", uuid);
        let db_path = PathBuf::from(&self.config.db_dir_path).join(uuid);

        // LadybugDB を初期化
        let ladybug = LadybugDB::new(&db_path, Arc::clone(&self.tokenizer))?;

        // スキーマの初期化
        ladybug.init_schema().await?;

        // StorageSet を作成（LadybugDB は VectorStorage と GraphStorage の両方を実装）
        let ladybug_arc = Arc::new(ladybug);
        let storage_set = Arc::new(StorageSet::new(
            Arc::clone(&ladybug_arc) as Arc<dyn super::storage::VectorStorage>,
            ladybug_arc as Arc<dyn super::storage::GraphStorage>,
        ));

        // マップに登録
        self.storage_map.insert(uuid.to_string(), Arc::clone(&storage_set));

        log::debug!("<CuberService> Storage opened and cached for UUID: {}", uuid);
        Ok(storage_set)
    }

    /// サービスを閉じます。
    ///
    /// バックグラウンドタスクを停止し、全てのストレージ接続を閉じます。
    pub async fn close(&self) -> Result<(), CuberError> {
        log::info!("<CuberService> Shutting down CuberService...");

        // バックグラウンドタスクをキャンセル
        self.cancel_token.cancel();

        // 全てのストレージを閉じる
        let uuids: Vec<String> = self.storage_map.iter().map(|e| e.key().clone()).collect();
        for uuid in uuids {
            if let Some((_, set)) = self.storage_map.remove(&uuid) {
                if let Err(e) = set.close().await {
                    log::error!("<CuberService> Error closing storage {}: {:?}", uuid, e);
                }
            }
        }

        log::info!("<CuberService> CuberService shutdown complete.");
        Ok(())
    }

    /// 現在開いているストレージの数を返します。
    pub fn storage_count(&self) -> usize {
        self.storage_map.len()
    }

    // ============================================================
    // TODO: 将来実装予定のメソッド
    // ============================================================

    // TODO: データ吸収 (Absorb) 機能の実装
    // pub async fn absorb(&self, cube_uuid: &str, files: Vec<AbsorbFile>, memory_group: &str) -> Result<TokenUsage, CuberError>;

    // TODO: クエリ (Query) 機能の実装
    // pub async fn query(&self, cube_uuid: &str, question: &str, memory_group: &str) -> Result<QueryResult, CuberError>;

    // TODO: 記憶化 (Memify) 機能の実装
    // pub async fn memify(&self, cube_uuid: &str, memory_group: &str) -> Result<MemifyResult, CuberError>;

    // TODO: エクスポート/インポート機能の実装
    // pub async fn export_cube(&self, cube_uuid: &str) -> Result<Vec<u8>, CuberError>;
    // pub async fn import_cube(&self, data: Vec<u8>) -> Result<String, CuberError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意: このテストは実際の S3 クライアントと LadybugDB を必要とするため、
    // 統合テストとして実行する必要があります。
    // ユニットテストとしては、モックを使用することを検討してください。

    #[tokio::test]
    #[ignore] // 統合テスト用
    async fn test_cuber_service_creation() {
        // テスト用の設定
        let _config = CuberConfig {
            db_dir_path: "/tmp/test_cuber_service".to_string(),
            ..Default::default()
        };

        // 注意: 実際のテストでは S3Client のモックを使用する必要があります
        // let s3_client = Arc::new(S3Client::new(...).await.unwrap());
        // let service = CuberService::new(config, s3_client).await;
        // assert!(service.is_ok());
    }
}
