// MetadataStore トレイトおよび InMemoryMetadataStore 実装
//
// SQLite 責務の抽象化: メタデータ・信頼スコア・系列監査ログ、
// Training / Fusion メタデータの永続化。

use std::cell::RefCell;
use std::collections::HashMap;

use crate::error::DarviumError;
use crate::types::{
    FusionMetadata, HumanOutcome, InteractionStatus, PatchHistory, SearchTrace, StoredInteraction,
    TrainingMetadata, TrustAuditLog,
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

    // === HumanChannel インタラクション永続化（M-0.5-4） ===

    /// HITL インタラクションを保存する（新規作成時: status=Pending）。
    fn store_human_interaction(&self, record: &StoredInteraction) -> Result<(), DarviumError>;

    /// interaction_id で HITL インタラクションを取得する。
    fn load_human_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<StoredInteraction, DarviumError>;

    /// status=Pending の全 HITL インタラクションを取得する。
    /// プロセス再起動時の回復ループで使用する。
    fn list_pending_human_interactions(&self) -> Result<Vec<StoredInteraction>, DarviumError>;

    /// インタラクションの outcome と status を更新する（Pending→Resolved）。
    fn resolve_human_interaction(
        &self,
        interaction_id: &str,
        outcome: &HumanOutcome,
    ) -> Result<(), DarviumError>;
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
    human_interactions: RefCell<HashMap<String, StoredInteraction>>,
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
            human_interactions: RefCell::new(HashMap::new()),
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
            .ok_or_else(|| {
                DarviumError::NotFound(format!("Training metadata not found: {}", mission_id))
            })
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
            .ok_or_else(|| {
                DarviumError::NotFound(format!("Fusion metadata not found: {}", pair_id))
            })
    }

    fn store_human_interaction(&self, record: &StoredInteraction) -> Result<(), DarviumError> {
        self.human_interactions
            .borrow_mut()
            .insert(record.interaction_id.clone(), record.clone());
        Ok(())
    }

    fn load_human_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<StoredInteraction, DarviumError> {
        self.human_interactions
            .borrow()
            .get(interaction_id)
            .cloned()
            .ok_or_else(|| {
                DarviumError::NotFound(format!("Human interaction not found: {}", interaction_id))
            })
    }

    fn list_pending_human_interactions(&self) -> Result<Vec<StoredInteraction>, DarviumError> {
        let interactions = self.human_interactions.borrow();
        let pending: Vec<StoredInteraction> = interactions
            .values()
            .filter(|r| r.status == InteractionStatus::Pending)
            .cloned()
            .collect();
        Ok(pending)
    }

    fn resolve_human_interaction(
        &self,
        interaction_id: &str,
        outcome: &HumanOutcome,
    ) -> Result<(), DarviumError> {
        let mut interactions = self.human_interactions.borrow_mut();
        if let Some(record) = interactions.get_mut(interaction_id) {
            record.outcome = Some(outcome.clone());
            record.status = InteractionStatus::Resolved;
            Ok(())
        } else {
            Err(DarviumError::NotFound(format!(
                "Human interaction not found: {}",
                interaction_id
            )))
        }
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

        store
            .store_search_trace(&SearchTrace)
            .expect("store should succeed");
        store
            .store_search_trace(&SearchTrace)
            .expect("store should succeed");

        let traces = store
            .load_search_traces("any-mission")
            .expect("load should succeed");
        assert_eq!(traces.len(), 2, "should retrieve 2 stored traces");
    }

    // ============================================================
    // テスト11: TrustAuditLog の保存 → target_id 検索
    // ============================================================
    #[test]
    fn store_and_load_trust_audit_logs() {
        let store = InMemoryMetadataStore::new();

        store
            .store_trust_audit_log(&TrustAuditLog)
            .expect("store should succeed");
        store
            .store_trust_audit_log(&TrustAuditLog)
            .expect("store should succeed");
        store
            .store_trust_audit_log(&TrustAuditLog)
            .expect("store should succeed");

        let logs = store
            .load_trust_audit_logs("any-target")
            .expect("load should succeed");
        assert_eq!(logs.len(), 3, "should retrieve 3 stored logs");
    }

    // ============================================================
    // テスト12: PatchHistory の保存 → graph_id 検索
    // ============================================================
    #[test]
    fn store_and_load_patch_histories() {
        let store = InMemoryMetadataStore::new();

        store
            .store_patch_history(&PatchHistory)
            .expect("store should succeed");

        let histories = store
            .load_patch_histories("any-graph")
            .expect("load should succeed");
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

        store
            .store_training_metadata(&metadata)
            .expect("store should succeed");
        let loaded = store
            .load_training_metadata("mission-1")
            .expect("load should succeed");

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

        store
            .store_fusion_metadata(&metadata)
            .expect("store should succeed");
        let loaded = store
            .load_fusion_metadata("pair-1")
            .expect("load should succeed");

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

    // ============================================================
    // T10: MetadataStore HITL 永続化
    // ============================================================

    /// T10-1: store → load 一致
    #[test]
    fn t10_1_store_and_load_human_interaction() {
        let store = InMemoryMetadataStore::new();
        let record = StoredInteraction {
            interaction_id: "id-1".to_string(),
            request: HumanRequest {
                subject: "test".into(),
                body: "body".into(),
                context: serde_json::json!({}),
                timeout: None,
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 1000,
            updated_at: 1000,
        };
        store
            .store_human_interaction(&record)
            .expect("store should succeed");
        let loaded = store
            .load_human_interaction("id-1")
            .expect("load should succeed");
        assert_eq!(loaded, record);
    }

    /// T10-2: 存在しない ID → Err(NotFound)
    #[test]
    fn t10_2_load_nonexistent_human_interaction() {
        let store = InMemoryMetadataStore::new();
        let result = store.load_human_interaction("nonexistent");
        assert!(matches!(result, Err(DarviumError::NotFound(_))));
    }

    /// T10-3: Pending のみ抽出
    #[test]
    fn t10_3_list_pending_only() {
        let store = InMemoryMetadataStore::new();
        let make = |id: &str, status: InteractionStatus| StoredInteraction {
            interaction_id: id.to_string(),
            request: HumanRequest {
                subject: id.into(),
                body: "".into(),
                context: serde_json::json!({}),
                timeout: None,
            },
            outcome: None,
            status,
            created_at: 0,
            updated_at: 0,
        };
        store
            .store_human_interaction(&make("p1", InteractionStatus::Pending))
            .unwrap();
        store
            .store_human_interaction(&make("p2", InteractionStatus::Pending))
            .unwrap();
        store
            .store_human_interaction(&make("r1", InteractionStatus::Resolved))
            .unwrap();

        let pending = store.list_pending_human_interactions().unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending
            .iter()
            .all(|r| r.status == InteractionStatus::Pending));
    }

    /// T10-4: resolve で状態遷移
    #[test]
    fn t10_4_resolve_transition() {
        let store = InMemoryMetadataStore::new();
        let record = StoredInteraction {
            interaction_id: "id-r".to_string(),
            request: HumanRequest {
                subject: "resolve".into(),
                body: "".into(),
                context: serde_json::json!({}),
                timeout: None,
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 0,
            updated_at: 0,
        };
        store.store_human_interaction(&record).unwrap();
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: crate::types::HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        store.resolve_human_interaction("id-r", &outcome).unwrap();

        let loaded = store.load_human_interaction("id-r").unwrap();
        assert_eq!(loaded.status, InteractionStatus::Resolved);
        assert_eq!(loaded.outcome, Some(outcome));
    }

    /// T10-5: 重複 store 上書き
    #[test]
    fn t10_5_overwrite_duplicate() {
        let store = InMemoryMetadataStore::new();
        let record1 = StoredInteraction {
            interaction_id: "dup".to_string(),
            request: HumanRequest {
                subject: "v1".into(),
                body: "".into(),
                context: serde_json::json!({}),
                timeout: None,
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 0,
            updated_at: 0,
        };
        store.store_human_interaction(&record1).unwrap();
        let record2 = StoredInteraction {
            interaction_id: "dup".to_string(),
            request: HumanRequest {
                subject: "v2".into(),
                body: "".into(),
                context: serde_json::json!({}),
                timeout: None,
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 1,
            updated_at: 1,
        };
        store.store_human_interaction(&record2).unwrap();
        let loaded = store.load_human_interaction("dup").unwrap();
        assert_eq!(loaded.request.subject, "v2");
    }

    /// T10-6: FakeHumanChannel → MetadataStore 一貫性
    /// export_interactions() で取得したレコードを MetadataStore に保存できる。
    #[test]
    fn t10_6_export_to_metadata_store_consistency() {
        use crate::human_channel::{FakeHumanChannel, HumanChannel};
        use std::collections::VecDeque;

        let channel = FakeHumanChannel::new(VecDeque::from(vec![HumanOutcome::Responded(
            HumanResponse {
                decision: crate::types::HumanDecision::Approved,
                comment: None,
                revised_body: None,
            },
        )]));
        let request = HumanRequest {
            subject: "consistency".into(),
            body: "test".into(),
            context: serde_json::json!({"key": "value"}),
            timeout: None,
        };
        let handle = channel.communicate(&request).unwrap();
        let _outcome = handle.wait(None).unwrap();

        let interactions = channel.export_interactions();
        assert_eq!(interactions.len(), 1);

        let store = InMemoryMetadataStore::new();
        store.store_human_interaction(&interactions[0]).unwrap();
        let loaded = store
            .load_human_interaction(&interactions[0].interaction_id)
            .unwrap();
        assert_eq!(loaded.interaction_id, interactions[0].interaction_id);
        assert_eq!(loaded.status, InteractionStatus::Resolved);
        assert_eq!(loaded.outcome, interactions[0].outcome);
    }

    /// T10-7: 再起動シミュレーション — Pending 生存
    /// 手動構築した Pending レコードを store → 新インスタンスで reconnect。
    #[test]
    fn t10_7_crash_recovery_pending() {
        use crate::human_channel::{FakeHumanChannel, HumanChannel};
        use std::collections::VecDeque;

        let store = InMemoryMetadataStore::new();
        let pending_record = StoredInteraction {
            interaction_id: "crash-pending".to_string(),
            request: HumanRequest {
                subject: "crash".into(),
                body: "recover".into(),
                context: serde_json::json!({"sim": "crash"}),
                timeout: None,
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 100,
            updated_at: 100,
        };
        store.store_human_interaction(&pending_record).unwrap();
        let pending_list = store.list_pending_human_interactions().unwrap();
        assert_eq!(pending_list.len(), 1);
        assert_eq!(pending_list[0].interaction_id, "crash-pending");

        // 新インスタンス（プロセス再起動模擬）+ プリロードキューに応答設定
        let expected_outcome = HumanOutcome::Responded(HumanResponse {
            decision: crate::types::HumanDecision::Approved,
            comment: Some("recovered".into()),
            revised_body: None,
        });
        let new_channel = FakeHumanChannel::new(VecDeque::from(vec![expected_outcome.clone()]));
        let handle = new_channel
            .reconnect(
                uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap(),
                &pending_record.request,
            )
            .unwrap();
        let outcome = handle.wait(None).unwrap();
        assert_eq!(outcome, expected_outcome);

        // 解決を MetadataStore に反映
        store
            .resolve_human_interaction("crash-pending", &outcome)
            .unwrap();
        let loaded = store.load_human_interaction("crash-pending").unwrap();
        assert_eq!(loaded.status, InteractionStatus::Resolved);
        assert_eq!(loaded.outcome, Some(expected_outcome));
    }

    /// T10-8: 再起動シミュレーション — Resolved 生存
    /// export_interactions() → store → 新インスタンス reconnect。
    #[test]
    fn t10_8_crash_recovery_resolved() {
        use crate::human_channel::{FakeHumanChannel, HumanChannel};
        use std::collections::VecDeque;

        let store = InMemoryMetadataStore::new();
        let expected_outcome = HumanOutcome::Responded(HumanResponse {
            decision: crate::types::HumanDecision::Approved,
            comment: Some("done".into()),
            revised_body: None,
        });

        // 初回チャネルで communicate → wait → export
        let channel = FakeHumanChannel::new(VecDeque::from(vec![expected_outcome.clone()]));
        let request = HumanRequest {
            subject: "crash-resolved".into(),
            body: "test".into(),
            context: serde_json::json!({}),
            timeout: None,
        };
        let handle = channel.communicate(&request).unwrap();
        let original_outcome = handle.wait(None).unwrap();
        let interactions = channel.export_interactions();
        assert_eq!(interactions[0].status, InteractionStatus::Resolved);

        // MetadataStore に保存
        store.store_human_interaction(&interactions[0]).unwrap();

        // 新チャネルで reconnect
        let new_channel = FakeHumanChannel::new(VecDeque::from(vec![expected_outcome.clone()]));
        let stored = store
            .load_human_interaction(&interactions[0].interaction_id)
            .unwrap();
        let handle2 = new_channel
            .reconnect(
                uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap(),
                &stored.request,
            )
            .unwrap();
        let recovered_outcome = handle2.wait(None).unwrap();
        assert_eq!(recovered_outcome, original_outcome);

        // 冪等な再解決
        store
            .resolve_human_interaction(&interactions[0].interaction_id, &recovered_outcome)
            .unwrap();
        let final_state = store
            .load_human_interaction(&interactions[0].interaction_id)
            .unwrap();
        assert_eq!(final_state.status, InteractionStatus::Resolved);
    }
}
