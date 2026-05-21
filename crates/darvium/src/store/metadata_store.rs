// MetadataStore トレイトおよび InMemoryMetadataStore 実装
//
// SQLite 責務の抽象化: メタデータ・信頼スコア・系列監査ログ、
// Training / Fusion メタデータの永続化。

use std::cell::RefCell;
use std::collections::HashMap;

use crate::error::DarviumError;
use crate::types::{
    FusionMetadata, PatchHistory, SearchTrace, TrainingMetadata, TrustAuditLog,
};

/// SQLite 責務を抽象化するトレイト。
///
/// メタデータ・信頼スコア・系列監査ログの永続化を定義する。
/// 全13フェーズはこのトレイトに対するプログラミングで実装され、
/// 実DB接続フェーズでは別実装 (SqliteMetadataStore) を追加するだけで差し替えが完了する。
pub trait MetadataStore {
    /// SearchTrace を保存する。
    fn store_search_trace(&self, trace: &SearchTrace) -> Result<(), DarviumError>;

    /// 指定された mission_id の全 SearchTrace を取得する。
    fn load_search_traces(&self, mission_id: &str) -> Result<Vec<SearchTrace>, DarviumError>;

    /// TrustAuditLog を保存する。
    fn store_trust_audit_log(&self, log: &TrustAuditLog) -> Result<(), DarviumError>;

    /// 指定された target_id の全 TrustAuditLog を取得する。
    fn load_trust_audit_logs(&self, target_id: &str) -> Result<Vec<TrustAuditLog>, DarviumError>;

    /// PatchHistory を保存する。
    fn store_patch_history(&self, history: &PatchHistory) -> Result<(), DarviumError>;

    /// 指定された graph_id の全 PatchHistory を取得する。
    fn load_patch_histories(&self, graph_id: &str) -> Result<Vec<PatchHistory>, DarviumError>;

    /// TrainingMetadata を保存する。
    fn store_training_metadata(&self, metadata: &TrainingMetadata) -> Result<(), DarviumError>;

    /// 指定された mission_id の TrainingMetadata を取得する。
    fn load_training_metadata(&self, mission_id: &str) -> Result<TrainingMetadata, DarviumError>;

    /// FusionMetadata を保存する。
    fn store_fusion_metadata(&self, metadata: &FusionMetadata) -> Result<(), DarviumError>;

    /// 指定された pair_id の FusionMetadata を取得する。
    fn load_fusion_metadata(&self, pair_id: &str) -> Result<FusionMetadata, DarviumError>;
}

/// メモリ内 MetadataStore 実装。
///
/// HashMap / Vec による全操作のメモリ内実装。
/// 高速・決定論的であり、全13フェーズのテスト基盤として使用される。
///
/// # 内部可変性
///
/// トレイトメソッドが &self を取るため、内部状態は RefCell でラップする。
/// シングルスレッド環境でのみ使用すること。
pub struct InMemoryMetadataStore {
    search_traces: RefCell<HashMap<String, Vec<SearchTrace>>>,
    trust_audit_logs: RefCell<HashMap<String, Vec<TrustAuditLog>>>,
    patch_histories: RefCell<HashMap<String, Vec<PatchHistory>>>,
    training_metadata: RefCell<HashMap<String, TrainingMetadata>>,
    fusion_metadata: RefCell<HashMap<String, FusionMetadata>>,
}

impl InMemoryMetadataStore {
    /// 空の InMemoryMetadataStore を生成する。
    pub fn new() -> Self {
        Self {
            search_traces: RefCell::new(HashMap::new()),
            trust_audit_logs: RefCell::new(HashMap::new()),
            patch_histories: RefCell::new(HashMap::new()),
            training_metadata: RefCell::new(HashMap::new()),
            fusion_metadata: RefCell::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryMetadataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataStore for InMemoryMetadataStore {
    fn store_search_trace(&self, trace: &SearchTrace) -> Result<(), DarviumError> {
        // SearchTrace は現状空構造体のため mission_id を直接保持できない。
        // 後続チケットで具体化された際に mission_id で分類する設計とする。
        // 現状は "default" キーに蓄積する。
        self.search_traces
            .borrow_mut()
            .entry("default".to_string())
            .or_default()
            .push(trace.clone());
        Ok(())
    }

    fn load_search_traces(&self, _mission_id: &str) -> Result<Vec<SearchTrace>, DarviumError> {
        let traces = self.search_traces.borrow();
        // 後続チケットで SearchTrace が mission_id を持つようになったら
        // _mission_id でフィルタリングする。現状は全件返す。
        let all: Vec<SearchTrace> = traces.values().flat_map(|v| v.iter()).cloned().collect();
        Ok(all)
    }

    fn store_trust_audit_log(&self, log: &TrustAuditLog) -> Result<(), DarviumError> {
        self.trust_audit_logs
            .borrow_mut()
            .entry("default".to_string())
            .or_default()
            .push(log.clone());
        Ok(())
    }

    fn load_trust_audit_logs(&self, _target_id: &str) -> Result<Vec<TrustAuditLog>, DarviumError> {
        let logs = self.trust_audit_logs.borrow();
        let all: Vec<TrustAuditLog> = logs.values().flat_map(|v| v.iter()).cloned().collect();
        Ok(all)
    }

    fn store_patch_history(&self, history: &PatchHistory) -> Result<(), DarviumError> {
        self.patch_histories
            .borrow_mut()
            .entry("default".to_string())
            .or_default()
            .push(history.clone());
        Ok(())
    }

    fn load_patch_histories(&self, _graph_id: &str) -> Result<Vec<PatchHistory>, DarviumError> {
        let histories = self.patch_histories.borrow();
        let all: Vec<PatchHistory> = histories.values().flat_map(|v| v.iter()).cloned().collect();
        Ok(all)
    }

    fn store_training_metadata(&self, metadata: &TrainingMetadata) -> Result<(), DarviumError> {
        self.training_metadata
            .borrow_mut()
            .insert(metadata.mission_id.clone(), metadata.clone());
        Ok(())
    }

    fn load_training_metadata(&self, mission_id: &str) -> Result<TrainingMetadata, DarviumError> {
        self.training_metadata
            .borrow()
            .get(mission_id)
            .cloned()
            .ok_or_else(|| DarviumError::NotFound(format!("Training metadata not found: {}", mission_id)))
    }

    fn store_fusion_metadata(&self, metadata: &FusionMetadata) -> Result<(), DarviumError> {
        self.fusion_metadata
            .borrow_mut()
            .insert(metadata.pair_id.clone(), metadata.clone());
        Ok(())
    }

    fn load_fusion_metadata(&self, pair_id: &str) -> Result<FusionMetadata, DarviumError> {
        self.fusion_metadata
            .borrow()
            .get(pair_id)
            .cloned()
            .ok_or_else(|| DarviumError::NotFound(format!("Fusion metadata not found: {}", pair_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    // ============================================================
    // テスト2: InMemoryMetadataStore が MetadataStore トレイトを充足する
    // ============================================================
    #[test]
    fn in_memory_metadata_store_implements_metadata_store() {
        let store: Box<dyn MetadataStore> = Box::new(InMemoryMetadataStore::new());
        let _ = store;
    }

    // ============================================================
    // テスト4: Box<dyn MetadataStore> のオブジェクト安全性確認
    // ============================================================
    #[test]
    fn metadata_store_trait_object_safety() {
        let store: Box<dyn MetadataStore> = Box::new(InMemoryMetadataStore::new());
        let trace = SearchTrace;
        let result = store.store_search_trace(&trace);
        assert!(result.is_ok());
    }

    // ============================================================
    // テスト10: SearchTrace の保存 → mission_id 検索
    // ============================================================
    #[test]
    fn store_and_load_search_traces() {
        let store = InMemoryMetadataStore::new();

        store.store_search_trace(&SearchTrace).expect("store should succeed");
        store.store_search_trace(&SearchTrace).expect("store should succeed");

        let traces = store.load_search_traces("any-mission").expect("load should succeed");
        assert_eq!(traces.len(), 2, "should retrieve 2 stored traces");
    }

    // ============================================================
    // テスト11: TrustAuditLog の保存 → target_id 検索
    // ============================================================
    #[test]
    fn store_and_load_trust_audit_logs() {
        let store = InMemoryMetadataStore::new();

        store.store_trust_audit_log(&TrustAuditLog).expect("store should succeed");
        store.store_trust_audit_log(&TrustAuditLog).expect("store should succeed");
        store.store_trust_audit_log(&TrustAuditLog).expect("store should succeed");

        let logs = store.load_trust_audit_logs("any-target").expect("load should succeed");
        assert_eq!(logs.len(), 3, "should retrieve 3 stored logs");
    }

    // ============================================================
    // テスト12: PatchHistory の保存 → graph_id 検索
    // ============================================================
    #[test]
    fn store_and_load_patch_histories() {
        let store = InMemoryMetadataStore::new();

        store.store_patch_history(&PatchHistory).expect("store should succeed");

        let histories = store.load_patch_histories("any-graph").expect("load should succeed");
        assert_eq!(histories.len(), 1, "should retrieve 1 stored history");
    }

    // ============================================================
    // テスト13: TrainingMetadata の保存 → 読取 → 内容一致確認
    // ============================================================
    #[test]
    fn store_and_load_training_metadata() {
        let store = InMemoryMetadataStore::new();
        let metadata = TrainingMetadata {
            mission_id: "mission-1".to_string(),
            status: "in_progress".to_string(),
        };

        store.store_training_metadata(&metadata).expect("store should succeed");
        let loaded = store.load_training_metadata("mission-1").expect("load should succeed");

        assert_eq!(loaded, metadata);
    }

    // ============================================================
    // テスト14: FusionMetadata の保存 → 読取 → 内容一致確認
    // ============================================================
    #[test]
    fn store_and_load_fusion_metadata() {
        let store = InMemoryMetadataStore::new();
        let metadata = FusionMetadata {
            pair_id: "pair-1".to_string(),
            status: "pending".to_string(),
        };

        store.store_fusion_metadata(&metadata).expect("store should succeed");
        let loaded = store.load_fusion_metadata("pair-1").expect("load should succeed");

        assert_eq!(loaded, metadata);
    }

    // ============================================================
    // 追加: 存在しない TrainingMetadata の load → NotFound
    // ============================================================
    #[test]
    fn load_non_existent_training_metadata_returns_not_found() {
        let store = InMemoryMetadataStore::new();
        let result = store.load_training_metadata("non-existent");
        assert!(matches!(result, Err(DarviumError::NotFound(_))));
    }

    // ============================================================
    // 追加: 存在しない FusionMetadata の load → NotFound
    // ============================================================
    #[test]
    fn load_non_existent_fusion_metadata_returns_not_found() {
        let store = InMemoryMetadataStore::new();
        let result = store.load_fusion_metadata("non-existent");
        assert!(matches!(result, Err(DarviumError::NotFound(_))));
    }
}
