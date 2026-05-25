// WorkflowCache — Repository Pair 上の MemoizedGraph 群の runtime cache
//
// 本モジュールは RFC §8 (WorkflowCache と MemoizedGraph) および §8.4
// (GraphVersion による楽観的並行性制御) を実装する。
//
// WorkflowCache は source-of-truth ではなく、検索高速化・局所再利用・
// compile-time / retrieval-time 参照のための in-memory working set を提供する。
// MemoizedGraph の canonical persistence, consistency, repair, quarantine, availability
// は Repository Pair (= DualStoreCoordinator) により担保される。

use std::sync::{Arc, RwLock};

use crate::constants::HNSW_MOCK_DEFAULT_DIMENSION;
use crate::error::DarviumError;
use crate::store::coordinator::DualStoreCoordinator;
use crate::store::graph_store::InMemoryGraphStore;
use crate::store::metadata_store::InMemoryMetadataStore;
use crate::trust::MemoizedGraph;
use crate::vector_index::MockHnswIndex;

/// === RFC §8: Repository Pair の型エイリアス ===
///
/// RepositoryPair は SQLite + LadybugDB から構成される永続化ペアであり、
/// MemoizedGraph の canonical persistence, consistency, repair, quarantine, availability
/// を担保する。runtime の具象実装は DualStoreCoordinator が担う。
pub type RepositoryPair = DualStoreCoordinator;

/// WorkflowCache 層のエラー（インメモリ操作・CAS 競合）(RFC §8.4)。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CacheError {
    /// 楽観的並行性制御のバージョン競合 (P-09)
    #[error("Version conflict: expected {expected}, found {actual}")]
    CasConflict {
        /// 呼び出し側が期待したバージョン
        expected: u64,
        /// 実際の現在バージョン
        actual: u64,
    },

    /// キャッシュ不在
    #[error("Graph not found in cache: {0:?}")]
    NotFound(String),

    /// RepositoryPair からの lazy load 失敗
    #[error("Lazy load from Repository Pair failed: {0}")]
    LoadFailed(String),
}

/// Repository Pair 永続化層のエラー（デュアルストア一貫性）(RFC §8.4)。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PersistenceError {
    /// デュアルストア間の不整合
    #[error("Cross-store inconsistency detected: {0}")]
    CrossStoreInconsistency(String),

    /// SQLite 操作失敗
    #[error("SQLite operation failed: {0}")]
    SqliteError(String),

    /// LadybugDB 操作失敗
    #[error("LadybugDB operation failed: {0}")]
    LadybugError(String),

    /// Repository Pair 不在
    #[error("Repository Pair not found: {0}")]
    PairNotFound(String),
}

/// CacheError → DarviumError への変換。
impl From<CacheError> for DarviumError {
    fn from(err: CacheError) -> Self {
        match err {
            CacheError::CasConflict { expected, actual } => {
                DarviumError::GraphVersionConflict { expected, actual }
            }
            CacheError::NotFound(id) => DarviumError::NotFound(format!("cache miss: {}", id)),
            CacheError::LoadFailed(reason) => {
                DarviumError::Storage(format!("cache load failed: {}", reason))
            }
        }
    }
}

/// PersistenceError → DarviumError への変換。
impl From<PersistenceError> for DarviumError {
    fn from(err: PersistenceError) -> Self {
        match err {
            PersistenceError::CrossStoreInconsistency(reason) => {
                DarviumError::DualStoreInconsistency(reason)
            }
            PersistenceError::SqliteError(reason) => {
                DarviumError::Storage(format!("sqlite: {}", reason))
            }
            PersistenceError::LadybugError(reason) => {
                DarviumError::Storage(format!("ladybug: {}", reason))
            }
            PersistenceError::PairNotFound(id) => DarviumError::NotFound(id),
        }
    }
}

/// キャッシュポリシー (RFC §8)。
///
/// WorkflowCache がどの MemoizedGraph を保持するかの戦略を指定する。
#[derive(Debug, Clone, PartialEq)]
pub enum CachePolicy {
    /// デフォルト動作
    Default,
    /// 指定されたワークフロー ID を GC から保護
    Pinned {
        /// 保護対象のワークフロー ID 一覧
        workflow_ids: Vec<String>,
    },
    /// 起動時に指定されたワークフローをプリロード
    Preload {
        /// プリロード対象のワークフロー ID 一覧
        workflow_ids: Vec<String>,
    },
}

/// Repository Pair 上の AnnIndex の hot subset。
///
/// 最近の検索パターンに基づき WorkflowCache が保持する ANN ヒントであり、
/// 完全な AnnIndex は LadybugDB (Repository Pair) 上の HNSW インデックスである。
/// 現状は MockHnswIndex をエイリアスする。
pub type AnnHotIndex = MockHnswIndex;

/// Repository Pair 上に永続化された MemoizedGraph 群の runtime cache (RFC §8)。
///
/// # 責務
///
/// - 検索高速化: 頻繁にアクセスされる MemoizedGraph をインメモリで保持する
/// - 局所再利用: compile-time および retrieval-time の参照を高速化する
/// - 楽観的並行性制御: GraphVersion CAS による整合性を保証する (P-09)
///
/// # 非責務
///
/// - 永続化: MemoizedGraph の canonical persistence は Repository Pair が担う
/// - 整合性修復: consistency/repair は Repository Pair の責務である
/// - 完全性: source-of-truth ではない。cache miss は Repository Pair からの
///   lazy load で解決する
///
/// # 並行アクセス
///
/// 内部状態は `std::sync::RwLock` で保護される。
pub struct WorkflowCache {
    /// キャッシュされた MemoizedGraph の working set
    pub working_set: Arc<RwLock<Vec<MemoizedGraph>>>,
    /// 最近の検索パターンに最適化された ANN ヒント
    pub ann_hint: Arc<RwLock<AnnHotIndex>>,
    /// キャッシュポリシー
    pub policy: CachePolicy,
}

impl WorkflowCache {
    /// 新しい WorkflowCache を生成する。
    pub fn new(policy: CachePolicy, ann_hint: AnnHotIndex) -> Self {
        Self {
            working_set: Arc::new(RwLock::new(Vec::new())),
            ann_hint: Arc::new(RwLock::new(ann_hint)),
            policy,
        }
    }

    /// テスト用のインメモリ WorkflowCache を生成する。
    pub fn in_memory() -> Self {
        Self::new(
            CachePolicy::Default,
            MockHnswIndex::new(HNSW_MOCK_DEFAULT_DIMENSION),
        )
    }

    /// RepositoryPair のテスト用インメモリインスタンスを生成する。
    pub fn in_memory_pair() -> RepositoryPair {
        DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        )
    }

    /// RepositoryPair から MemoizedGraph を lazy load する (RFC §8.4)。
    ///
    /// cache hit → 即時返却
    /// cache miss → RepositoryPair から load して cache に昇格
    pub fn get_or_load(
        &self,
        graph_id: &str,
        pair: &RepositoryPair,
    ) -> Result<MemoizedGraph, CacheError> {
        // cache hit チェック
        {
            let store = self
                .working_set
                .read()
                .map_err(|e| CacheError::LoadFailed(format!("RwLock poisoned: {}", e)))?;
            if let Some(g) = store.iter().find(|g| g.id == graph_id) {
                return Ok(g.clone());
            }
        }
        // cache miss  → RepositoryPair から load → cache に昇格
        let memoized = pair.load_memoized_graph(graph_id).map_err(|e| {
            CacheError::LoadFailed(format!("Failed to load graph {}: {}", graph_id, e))
        })?;
        {
            let mut store = self
                .working_set
                .write()
                .map_err(|e| CacheError::LoadFailed(format!("RwLock poisoned: {}", e)))?;
            store.push(memoized.clone());
        }
        Ok(memoized)
    }

    /// 楽観的更新: expected_version が現在バージョンと一致する場合のみ更新 (RFC §8.4)。
    pub fn update_graph_cas(
        &self,
        graph_id: &str,
        new_graph: crate::types::WorkflowGraph,
        expected_version: u64,
    ) -> Result<u64, CacheError> {
        let mut store = self
            .working_set
            .write()
            .map_err(|e| CacheError::LoadFailed(format!("RwLock poisoned: {}", e)))?;
        let entry = store
            .iter_mut()
            .find(|g| g.id == graph_id)
            .ok_or_else(|| CacheError::NotFound(graph_id.to_string()))?;
        if entry.version != expected_version {
            return Err(CacheError::CasConflict {
                expected: expected_version,
                actual: entry.version,
            });
        }
        entry.graph = new_graph;
        entry.version += 1;
        Ok(entry.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WorkflowGraph;

    /// テスト用の MemoizedGraph を生成するヘルパー。
    fn make_memoized_graph(id: &str, human_score: f64, version: u64) -> MemoizedGraph {
        let mut graph = MemoizedGraph::new(id.to_string(), human_score);
        graph.version = version;
        graph.graph = WorkflowGraph::new();
        graph
    }

    // ================================================================
    // T1: cache hit
    // ================================================================
    #[test]
    fn t1_workflow_cache_get_or_load_hit() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();
        let graph = make_memoized_graph("wf-hit", 0.8, 1);

        cache.working_set.write().unwrap().push(graph);

        let result = cache.get_or_load("wf-hit", &pair);
        assert!(result.is_ok(), "T1: cache hit は Ok を返すこと");
        assert_eq!(result.unwrap().id, "wf-hit");
    }

    // ================================================================
    // T2: cache miss → LoadFailed
    // ================================================================
    #[test]
    fn t2_workflow_cache_get_or_load_miss() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let result = cache.get_or_load("wf-miss", &pair);
        assert!(result.is_err(), "T2: cache miss は Err を返すこと");
        assert!(
            matches!(result.unwrap_err(), CacheError::LoadFailed(_)),
            "T2: LoadFailed であること"
        );
    }

    // ================================================================
    // T3: CAS 更新成功
    // ================================================================
    #[test]
    fn t3_workflow_cache_update_graph_cas_ok() {
        let cache = WorkflowCache::in_memory();
        let mut graph = make_memoized_graph("wf-cas-ok", 0.8, 1);
        graph.graph = WorkflowGraph::new();

        let expected_version = graph.version;
        cache.working_set.write().unwrap().push(graph);

        let new_graph = WorkflowGraph::new();
        let result = cache.update_graph_cas("wf-cas-ok", new_graph, expected_version);

        assert!(result.is_ok(), "T3: CAS 成功は Ok");
        assert_eq!(
            result.unwrap(),
            expected_version + 1,
            "T3: バージョンが +1 されること"
        );
    }

    // ================================================================
    // T4: CAS 競合
    // ================================================================
    #[test]
    fn t4_workflow_cache_update_graph_cas_conflict() {
        let cache = WorkflowCache::in_memory();
        let graph = make_memoized_graph("wf-cas-conflict", 0.8, 5);

        cache.working_set.write().unwrap().push(graph);

        let new_graph = WorkflowGraph::new();
        let result = cache.update_graph_cas("wf-cas-conflict", new_graph, 3);

        assert!(result.is_err(), "T4: バージョン不一致は Err");
        match result.unwrap_err() {
            CacheError::CasConflict { expected, actual } => {
                assert_eq!(expected, 3, "T4: expected=3");
                assert_eq!(actual, 5, "T4: actual=5");
            }
            other => panic!("T4: CasConflict が返されること (got {:?})", other),
        }
    }

    // ================================================================
    // T5: 不在時の LoadFailed
    // ================================================================
    #[test]
    fn t5_workflow_cache_get_or_load_not_found() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let result = cache.get_or_load("wf-nonexistent", &pair);
        assert!(result.is_err(), "T5: 不在時は Err");
        assert!(
            matches!(result.unwrap_err(), CacheError::LoadFailed(_)),
            "T5: LoadFailed"
        );
    }

    // ================================================================
    // T6: CachePolicy::Default
    // ================================================================
    #[test]
    fn t6_cache_policy_default() {
        let policy = CachePolicy::Default;
        assert_eq!(format!("{:?}", policy), "Default");
    }

    // ================================================================
    // T7: CachePolicy::Pinned
    // ================================================================
    #[test]
    fn t7_cache_policy_pinned() {
        let ids = vec!["wf-a".to_string(), "wf-b".to_string()];
        let policy = CachePolicy::Pinned {
            workflow_ids: ids.clone(),
        };
        match &policy {
            CachePolicy::Pinned { workflow_ids } => {
                assert_eq!(workflow_ids.len(), 2);
                assert_eq!(workflow_ids[0], "wf-a");
            }
            _ => panic!("T7: Pinned variant"),
        }
    }

    // ================================================================
    // T8: CachePolicy::Preload
    // ================================================================
    #[test]
    fn t8_cache_policy_preload() {
        let ids = vec!["wf-pre-1".to_string()];
        let policy = CachePolicy::Preload {
            workflow_ids: ids.clone(),
        };
        match &policy {
            CachePolicy::Preload { workflow_ids } => {
                assert_eq!(workflow_ids.len(), 1);
                assert_eq!(workflow_ids[0], "wf-pre-1");
            }
            _ => panic!("T8: Preload variant"),
        }
    }

    // ================================================================
    // T9: CacheError Display
    // ================================================================
    #[test]
    fn t9_cache_error_display() {
        let cas = CacheError::CasConflict {
            expected: 1,
            actual: 3,
        };
        assert!(
            format!("{}", cas).contains("Version conflict"),
            "T9a: CasConflict"
        );

        let nf = CacheError::NotFound("wf-test".to_string());
        assert!(
            format!("{}", nf).contains("not found in cache"),
            "T9b: NotFound"
        );

        let lf = CacheError::LoadFailed("db error".to_string());
        assert!(
            format!("{}", lf).contains("Lazy load from Repository Pair failed"),
            "T9c: LoadFailed"
        );
    }

    // ================================================================
    // T10: PersistenceError Display
    // ================================================================
    #[test]
    fn t10_persistence_error_display() {
        let ci = PersistenceError::CrossStoreInconsistency("meta mismatch".to_string());
        assert!(
            format!("{}", ci).contains("Cross-store inconsistency"),
            "T10a"
        );

        let se = PersistenceError::SqliteError("lock timeout".to_string());
        assert!(
            format!("{}", se).contains("SQLite operation failed"),
            "T10b"
        );

        let le = PersistenceError::LadybugError("disk full".to_string());
        assert!(
            format!("{}", le).contains("LadybugDB operation failed"),
            "T10c"
        );

        let pnf = PersistenceError::PairNotFound("wf-pair".to_string());
        assert!(
            format!("{}", pnf).contains("Repository Pair not found"),
            "T10d"
        );
    }

    // ================================================================
    // T11: CacheError → DarviumError
    // ================================================================
    #[test]
    fn t11_cache_error_into_darvium_error() {
        let cas: DarviumError = CacheError::CasConflict {
            expected: 2,
            actual: 5,
        }
        .into();
        assert!(
            matches!(cas, DarviumError::GraphVersionConflict { .. }),
            "T11a"
        );

        let nf: DarviumError = CacheError::NotFound("wf-x".to_string()).into();
        assert!(matches!(nf, DarviumError::NotFound(_)), "T11b");

        let lf: DarviumError = CacheError::LoadFailed("timeout".to_string()).into();
        assert!(matches!(lf, DarviumError::Storage(_)), "T11c");
    }

    // ================================================================
    // T12: PersistenceError → DarviumError
    // ================================================================
    #[test]
    fn t12_persistence_error_into_darvium_error() {
        let ci: DarviumError =
            PersistenceError::CrossStoreInconsistency("mismatch".to_string()).into();
        assert!(
            matches!(ci, DarviumError::DualStoreInconsistency(_)),
            "T12a"
        );

        let se: DarviumError = PersistenceError::SqliteError("err".to_string()).into();
        assert!(matches!(se, DarviumError::Storage(_)), "T12b");

        let le: DarviumError = PersistenceError::LadybugError("err".to_string()).into();
        assert!(matches!(le, DarviumError::Storage(_)), "T12c");

        let pnf: DarviumError = PersistenceError::PairNotFound("id".to_string()).into();
        assert!(matches!(pnf, DarviumError::NotFound(_)), "T12d");
    }

    // ================================================================
    // T13: RepositoryPair::in_memory() Send
    // ================================================================
    #[test]
    fn t13_repository_pair_in_memory() {
        let pair = WorkflowCache::in_memory_pair();
        fn assert_send<T: Send>(_t: &T) {}
        assert_send(&pair);
    }

    // ================================================================
    // T14: WorkflowCache::in_memory()
    // ================================================================
    #[test]
    fn t14_workflow_cache_in_memory() {
        let cache = WorkflowCache::in_memory();
        assert_eq!(cache.policy, CachePolicy::Default);
    }

    // ================================================================
    // T15: clone 確認
    // ================================================================
    #[test]
    fn t15_workflow_cache_clone_graph() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();
        let original = make_memoized_graph("wf-clone", 0.9, 1);

        cache.working_set.write().unwrap().push(original.clone());

        let loaded = cache.get_or_load("wf-clone", &pair).unwrap();
        assert_eq!(loaded.id, original.id);
        assert_eq!(loaded.trust.human.score, original.trust.human.score);
    }

    // ================================================================
    // T16: 同一 id で2回目が cache hit
    // ================================================================
    #[test]
    fn t16_workflow_cache_get_or_load_miss_twice() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();
        let graph = make_memoized_graph("wf-twice", 0.7, 1);

        cache.working_set.write().unwrap().push(graph);

        let r1 = cache.get_or_load("wf-twice", &pair);
        assert!(r1.is_ok(), "T16a: 1回目 cache hit");
        let r2 = cache.get_or_load("wf-twice", &pair);
        assert!(r2.is_ok(), "T16b: 2回目 cache hit");
        assert_eq!(r1.unwrap().id, r2.unwrap().id);
    }

    // ================================================================
    // T17: 複数 Graph 管理
    // ================================================================
    #[test]
    fn t17_workflow_cache_multiple_graphs() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let g1 = make_memoized_graph("wf-ga", 0.9, 1);
        let g2 = make_memoized_graph("wf-gb", 0.8, 1);
        let g3 = make_memoized_graph("wf-gc", 0.7, 1);

        {
            let mut store = cache.working_set.write().unwrap();
            store.push(g1);
            store.push(g2);
            store.push(g3);
        }

        assert!(cache.get_or_load("wf-ga", &pair).is_ok());
        assert!(cache.get_or_load("wf-gb", &pair).is_ok());
        assert!(cache.get_or_load("wf-gc", &pair).is_ok());
        assert!(cache.get_or_load("wf-unknown", &pair).is_err());
    }
}
