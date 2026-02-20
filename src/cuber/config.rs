//! Cuber Configuration
//!
//! Cuber の設定を管理する構造体を定義します。
//! Go 版 `config_types.go` に相当し、`serde` によるシリアライズ/デシリアライズに対応しています。

use serde::{Deserialize, Serialize};

use super::consts::*;

// ============================================================
// Default value functions for serde
// ============================================================

fn default_db_dir_path() -> String {
    "./disk/db".to_string()
}

fn default_storage_idle_timeout_minutes() -> u32 {
    DEFAULT_STORAGE_IDLE_TIMEOUT_MINUTES as u32
}

fn default_s3_cleanup_interval_minutes() -> u32 {
    DEFAULT_S3_CLEANUP_INTERVAL_MINUTES as u32
}

fn default_s3_retention_hours() -> u32 {
    DEFAULT_S3_RETENTION_HOURS as u32
}

fn default_chunk_size() -> u32 {
    DEFAULT_CHUNK_SIZE as u32
}

fn default_chunk_overlap() -> u32 {
    DEFAULT_CHUNK_OVERLAP as u32
}

fn default_meta_similarity_threshold_unknown() -> f64 {
    DEFAULT_META_SIMILARITY_THRESHOLD_UNKNOWN
}

fn default_meta_similarity_threshold_reflection() -> f64 {
    DEFAULT_META_SIMILARITY_THRESHOLD_REFLECTION
}

fn default_meta_similarity_threshold_crystallization() -> f64 {
    DEFAULT_META_SIMILARITY_THRESHOLD_CRYSTALLIZATION
}

fn default_meta_search_limit_unknown() -> u32 {
    DEFAULT_META_SEARCH_LIMIT_UNKNOWN as u32
}

fn default_meta_search_limit_reflection_chunk() -> u32 {
    DEFAULT_META_SEARCH_LIMIT_REFLECTION_CHUNK as u32
}

fn default_meta_search_limit_reflection_rule() -> u32 {
    DEFAULT_META_SEARCH_LIMIT_REFLECTION_RULE as u32
}

fn default_meta_crystallization_min_cluster() -> u32 {
    DEFAULT_META_CRYSTALLIZATION_MIN_CLUSTER as u32
}

fn default_metabolism_alpha() -> f64 {
    DEFAULT_METABOLISM_ALPHA
}

fn default_metabolism_delta() -> f64 {
    DEFAULT_METABOLISM_DELTA
}

fn default_prune_threshold() -> f64 {
    DEFAULT_PRUNE_THRESHOLD
}

fn default_pruning_grace_period_minutes() -> u32 {
    DEFAULT_PRUNING_GRACE_PERIOD_MINUTES as u32
}

fn default_embedding_dimension() -> u32 {
    DEFAULT_EMBEDDING_DIMENSION as u32
}

fn default_debug() -> bool {
    false
}

// ============================================================
// CuberConfig
// ============================================================

/// Cuber の設定構造体
///
/// Go 版 `CuberConfig` (config_types.go) に相当します。
/// 環境変数または設定ファイルからロードされ、`CuberService` の初期化時に使用されます。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuberConfig {
    // ============================================================
    // Storage 関連
    // ============================================================
    /// データベースファイルを保存するディレクトリパス
    #[serde(default = "default_db_dir_path")]
    pub db_dir_path: String,

    /// ストレージのアイドルタイムアウト（分）
    #[serde(default = "default_storage_idle_timeout_minutes")]
    pub storage_idle_timeout_minutes: u32,

    /// S3 クリーンアップ間隔（分）
    #[serde(default = "default_s3_cleanup_interval_minutes")]
    pub s3_cleanup_interval_minutes: u32,

    /// S3 ファイル保持期間（時間）
    #[serde(default = "default_s3_retention_hours")]
    pub s3_retention_hours: u32,

    // ============================================================
    // Absorb (Chunking) 関連
    // ============================================================
    /// デフォルトのチャンクサイズ（文字数）
    #[serde(default = "default_chunk_size")]
    pub chunk_size: u32,

    /// デフォルトのチャンクオーバーラップ（文字数）
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: u32,

    // ============================================================
    // Metacognition 関連
    // ============================================================
    /// Unknown 解決時の類似度閾値
    #[serde(default = "default_meta_similarity_threshold_unknown")]
    pub meta_similarity_threshold_unknown: f64,

    /// Self-Reflection 時の類似度閾値
    #[serde(default = "default_meta_similarity_threshold_reflection")]
    pub meta_similarity_threshold_reflection: f64,

    /// 結晶化時の類似度閾値
    #[serde(default = "default_meta_similarity_threshold_crystallization")]
    pub meta_similarity_threshold_crystallization: f64,

    /// Unknown 解決時の最大検索数
    #[serde(default = "default_meta_search_limit_unknown")]
    pub meta_search_limit_unknown: u32,

    /// Self-Reflection 時のチャンク最大検索数
    #[serde(default = "default_meta_search_limit_reflection_chunk")]
    pub meta_search_limit_reflection_chunk: u32,

    /// Self-Reflection 時のルール最大検索数
    #[serde(default = "default_meta_search_limit_reflection_rule")]
    pub meta_search_limit_reflection_rule: u32,

    /// 結晶化時の最小クラスタサイズ
    #[serde(default = "default_meta_crystallization_min_cluster")]
    pub meta_crystallization_min_cluster: u32,

    // ============================================================
    // Graph Metabolism 関連
    // ============================================================
    /// 強化学習率 (0.0-1.0): エッジが支持された時の信頼度上昇幅
    #[serde(default = "default_metabolism_alpha")]
    pub graph_metabolism_alpha: f64,

    /// 減衰ペナルティ率 (0.0-1.0): エッジが矛盾した時の信頼度減少率
    #[serde(default = "default_metabolism_delta")]
    pub graph_metabolism_delta: f64,

    /// 淘汰閾値: 生存スコアがこれを下回ると削除
    #[serde(default = "default_prune_threshold")]
    pub graph_metabolism_prune_threshold: f64,

    /// 孤立ノード削除の猶予期間（分）
    #[serde(default = "default_pruning_grace_period_minutes")]
    pub graph_pruning_grace_period_minutes: u32,

    // ============================================================
    // Embedding 関連
    // ============================================================
    /// 埋め込み次元数
    #[serde(default = "default_embedding_dimension")]
    pub embedding_dimension: u32,

    // ============================================================
    // Debug
    // ============================================================
    /// デバッグモード
    #[serde(default = "default_debug")]
    pub debug: bool,
    // TODO: 将来実装予定の LLM プロバイダー設定
    // TODO: 将来実装予定の Memify 関連設定
}

impl Default for CuberConfig {
    fn default() -> Self {
        Self {
            db_dir_path: default_db_dir_path(),
            storage_idle_timeout_minutes: default_storage_idle_timeout_minutes(),
            s3_cleanup_interval_minutes: default_s3_cleanup_interval_minutes(),
            s3_retention_hours: default_s3_retention_hours(),
            chunk_size: default_chunk_size(),
            chunk_overlap: default_chunk_overlap(),
            meta_similarity_threshold_unknown: default_meta_similarity_threshold_unknown(),
            meta_similarity_threshold_reflection: default_meta_similarity_threshold_reflection(),
            meta_similarity_threshold_crystallization:
                default_meta_similarity_threshold_crystallization(),
            meta_search_limit_unknown: default_meta_search_limit_unknown(),
            meta_search_limit_reflection_chunk: default_meta_search_limit_reflection_chunk(),
            meta_search_limit_reflection_rule: default_meta_search_limit_reflection_rule(),
            meta_crystallization_min_cluster: default_meta_crystallization_min_cluster(),
            graph_metabolism_alpha: default_metabolism_alpha(),
            graph_metabolism_delta: default_metabolism_delta(),
            graph_metabolism_prune_threshold: default_prune_threshold(),
            graph_pruning_grace_period_minutes: default_pruning_grace_period_minutes(),
            embedding_dimension: default_embedding_dimension(),
            debug: default_debug(),
        }
    }
}

impl CuberConfig {
    pub fn from_settings(
        c_settings: &crate::stt_config::CuberSettings,
        s_settings: &crate::stt_config::StorageSettings,
    ) -> Self {
        Self {
            db_dir_path: s_settings.db_dir_path.clone(),
            storage_idle_timeout_minutes: c_settings.storage_idle_timeout_minutes as u32,
            s3_cleanup_interval_minutes: c_settings.s3_cleanup_interval_minutes as u32,
            s3_retention_hours: c_settings.s3_retention_hours as u32,
            debug: c_settings.debug,
            // ... 他のフィールドは一旦デフォルトのままにするか、必要なら stt_config.rs に追加してマージ
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CuberConfig::default();
        assert_eq!(config.db_dir_path, "./disk/db");
        assert_eq!(config.storage_idle_timeout_minutes, 60);
        assert_eq!(config.chunk_size, 512);
        assert_eq!(config.chunk_overlap, 64);
        assert!((config.graph_metabolism_alpha - 0.2).abs() < 0.001);
    }
}
