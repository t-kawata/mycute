//! Cuber Constants
//!
//! 共通の定数値を定義します。

// ============================================================
// Absorb 関連
// ============================================================

/// Absorb のデフォルト上限回数
pub const DEFAULT_ABSORB_LIMIT: i32 = 10000;

/// デフォルトのチャンクサイズ（文字数）
pub const DEFAULT_CHUNK_SIZE: i32 = 512;

/// デフォルトのチャンクオーバーラップ（文字数）
pub const DEFAULT_CHUNK_OVERLAP: i32 = 64;

// ============================================================
// Query 関連
// ============================================================

/// Query のデフォルト上限回数
pub const DEFAULT_QUERY_LIMIT: i32 = 1000;

/// デフォルトの検索 Top-K
pub const DEFAULT_TOPK: i32 = 5;

/// Summary 検索のデフォルト Top-K
pub const DEFAULT_SUMMARY_TOPK: i32 = 3;

/// Chunk 検索のデフォルト Top-K
pub const DEFAULT_CHUNK_TOPK: i32 = 3;

/// Entity 検索のデフォルト Top-K
pub const DEFAULT_ENTITY_TOPK: i32 = 3;

// ============================================================
// Memify 関連
// ============================================================

/// Memify のデフォルト上限回数
pub const DEFAULT_MEMIFY_LIMIT: i32 = 100;

/// デフォルトのエポック数
pub const DEFAULT_EPOCHS: i32 = 10;

// ============================================================
// Graph Metabolism 関連
// ============================================================

/// 強化学習率 (0.0-1.0): エッジが支持された時の信頼度上昇幅
pub const DEFAULT_METABOLISM_ALPHA: f64 = 0.2;

/// 減衰ペナルティ率 (0.0-1.0): エッジが矛盾した時の信頼度減少率
pub const DEFAULT_METABOLISM_DELTA: f64 = 0.3;

/// 淘汰閾値: 生存スコア(Weight*Confidence)がこれを下回ると削除
pub const DEFAULT_PRUNE_THRESHOLD: f64 = 0.1;

/// 孤立ノード削除の猶予期間(分): 作成からこの期間内は削除されない
pub const DEFAULT_PRUNING_GRACE_PERIOD_MINUTES: i32 = 60;

// ============================================================
// Metacognition 関連
// ============================================================

/// Unknown解決時の類似度閾値
pub const DEFAULT_META_SIMILARITY_THRESHOLD_UNKNOWN: f64 = 0.3;

/// Self-Reflection時の類似度閾値
pub const DEFAULT_META_SIMILARITY_THRESHOLD_REFLECTION: f64 = 0.5;

/// 結晶化時の類似度閾値
pub const DEFAULT_META_SIMILARITY_THRESHOLD_CRYSTALLIZATION: f64 = 0.8;

/// Unknown解決時の最大検索数
pub const DEFAULT_META_SEARCH_LIMIT_UNKNOWN: i32 = 5;

/// Self-Reflection時のチャンク最大検索数
pub const DEFAULT_META_SEARCH_LIMIT_REFLECTION_CHUNK: i32 = 3;

/// Self-Reflection時のルール最大検索数
pub const DEFAULT_META_SEARCH_LIMIT_REFLECTION_RULE: i32 = 3;

/// 結晶化時の最小クラスタサイズ
pub const DEFAULT_META_CRYSTALLIZATION_MIN_CLUSTER: i32 = 2;

// ============================================================
// Storage 関連
// ============================================================

/// ストレージのアイドルタイムアウト（分）
pub const DEFAULT_STORAGE_IDLE_TIMEOUT_MINUTES: i32 = 60;

/// S3クリーンアップ間隔（分）
pub const DEFAULT_S3_CLEANUP_INTERVAL_MINUTES: i32 = 60;

/// S3ファイル保持期間（時間）
pub const DEFAULT_S3_RETENTION_HOURS: i32 = 24;

// ============================================================
// Embedding 関連
// ============================================================

/// デフォルトの埋め込み次元数
pub const DEFAULT_EMBEDDING_DIMENSION: i32 = 1536;
