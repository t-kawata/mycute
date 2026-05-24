// MetadataStore トレイトおよび InMemoryMetadataStore 実装
//
// SQLite 責務の抽象化: メタデータ・信頼スコア・系列監査ログ、
// Training / Fusion メタデータの永続化。

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::DarviumError;
use crate::types::{
    FusionMetadata, HumanOutcome, InteractionFilter, InteractionStatus, PatchHistory, SearchTrace,
    StoredInteraction, TrainingMetadata, TrustAuditLog,
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

    // === 汎用 Interaction API (v2.3-g, RFC §12C.7) ===

    /// インタラクションを保存する（新規作成または更新）。
    fn store_interaction(&self, record: &StoredInteraction) -> Result<(), DarviumError>;

    /// interaction_id でインタラクションを読み込む。
    /// 存在しない場合は Ok(None) を返す。
    fn load_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<Option<StoredInteraction>, DarviumError>;

    /// フィルタ条件に合致するインタラクション一覧を取得する。
    fn list_interactions(
        &self,
        filter: &InteractionFilter,
    ) -> Result<Vec<StoredInteraction>, DarviumError>;

    /// インタラクションを Resolved として解決する。
    fn resolve_interaction(
        &self,
        interaction_id: &str,
        outcome: &HumanOutcome,
    ) -> Result<(), DarviumError>;

    /// インタラクションを Aborted として中断する。
    fn abort_interaction(
        &self,
        interaction_id: &str,
        _reason: &str,
    ) -> Result<(), DarviumError>;

    /// インタラクションの再接続情報を更新する。
    /// 暫定実装: 現状 updated_at のみ更新。チャネル情報の更新は M1.5-R7 で拡充予定。
    fn reconnect_interaction(
        &self,
        interaction_id: &str,
        _new_channel_id: &str,
    ) -> Result<(), DarviumError>;

    // === HITL 特化ラッパー（後方互換: M-0.5-4） ===

    /// HITL インタラクションを保存する（新規作成時: status=Pending）。
    /// 汎用 `store_interaction` のラッパー。
    #[inline]
    fn store_human_interaction(&self, record: &StoredInteraction) -> Result<(), DarviumError> {
        self.store_interaction(record)
    }

    /// interaction_id で HITL インタラクションを取得する。
    /// 汎用 `load_interaction` のラッパー。NotFound 時は Err に変換。
    #[inline]
    fn load_human_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<StoredInteraction, DarviumError> {
        self.load_interaction(interaction_id)?
            .ok_or_else(|| {
                DarviumError::NotFound(format!("Human interaction not found: {}", interaction_id))
            })
    }

    /// status=Pending の全 HITL インタラクションを取得する。
    /// プロセス再起動時の回復ループで使用する。
    /// 汎用 `list_interactions` のラッパー。
    #[inline]
    fn list_pending_human_interactions(&self) -> Result<Vec<StoredInteraction>, DarviumError> {
        self.list_interactions(&InteractionFilter {
            status: Some(InteractionStatus::Pending),
            ..Default::default()
        })
    }

    /// インタラクションの outcome と status を更新する（Pending→Resolved）。
    /// 汎用 `resolve_interaction` のラッパー。
    #[inline]
    fn resolve_human_interaction(
        &self,
        interaction_id: &str,
        outcome: &HumanOutcome,
    ) -> Result<(), DarviumError> {
        self.resolve_interaction(interaction_id, outcome)
    }
}

/// メモリ内 MetadataStore 実装。
///
/// HashMap / Vec による全操作のメモリ内実装。
/// 高速・決定論的であり、全13フェーズのテスト基盤として使用される。
///
/// # 内部可変性
///
/// トレイトメソッドが &self を取るため、内部状態は Mutex でラップする。
/// スレッドセーフであり、Arc 経由でも使用可能。
pub struct InMemoryMetadataStore {
    search_traces: Mutex<HashMap<String, Vec<SearchTrace>>>,
    trust_audit_logs: Mutex<HashMap<String, Vec<TrustAuditLog>>>,
    patch_histories: Mutex<HashMap<String, Vec<PatchHistory>>>,
    training_metadata: Mutex<HashMap<String, TrainingMetadata>>,
    fusion_metadata: Mutex<HashMap<String, FusionMetadata>>,
    human_interactions: Mutex<HashMap<String, StoredInteraction>>,
}

impl InMemoryMetadataStore {
    /// 空の InMemoryMetadataStore を生成する。
    pub fn new() -> Self {
        Self {
            search_traces: Mutex::new(HashMap::new()),
            trust_audit_logs: Mutex::new(HashMap::new()),
            patch_histories: Mutex::new(HashMap::new()),
            training_metadata: Mutex::new(HashMap::new()),
            fusion_metadata: Mutex::new(HashMap::new()),
            human_interactions: Mutex::new(HashMap::new()),
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
            .lock().unwrap()
            .entry("default".to_string())
            .or_default()
            .push(trace.clone());
        Ok(())
    }

    fn load_search_traces(&self, _mission_id: &str) -> Result<Vec<SearchTrace>, DarviumError> {
        let traces = self.search_traces.lock().unwrap();
        // 後続チケットで SearchTrace が mission_id を持つようになったら
        // _mission_id でフィルタリングする。現状は全件返す。
        let all: Vec<SearchTrace> = traces.values().flat_map(|v| v.iter()).cloned().collect();
        Ok(all)
    }

    fn store_trust_audit_log(&self, log: &TrustAuditLog) -> Result<(), DarviumError> {
        self.trust_audit_logs
            .lock().unwrap()
            .entry("default".to_string())
            .or_default()
            .push(log.clone());
        Ok(())
    }

    fn load_trust_audit_logs(&self, _target_id: &str) -> Result<Vec<TrustAuditLog>, DarviumError> {
        let logs = self.trust_audit_logs.lock().unwrap();
        let all: Vec<TrustAuditLog> = logs.values().flat_map(|v| v.iter()).cloned().collect();
        Ok(all)
    }

    fn store_patch_history(&self, history: &PatchHistory) -> Result<(), DarviumError> {
        self.patch_histories
            .lock().unwrap()
            .entry("default".to_string())
            .or_default()
            .push(history.clone());
        Ok(())
    }

    fn load_patch_histories(&self, _graph_id: &str) -> Result<Vec<PatchHistory>, DarviumError> {
        let histories = self.patch_histories.lock().unwrap();
        let all: Vec<PatchHistory> = histories.values().flat_map(|v| v.iter()).cloned().collect();
        Ok(all)
    }

    fn store_training_metadata(&self, metadata: &TrainingMetadata) -> Result<(), DarviumError> {
        self.training_metadata
            .lock().unwrap()
            .insert(metadata.mission_id.clone(), metadata.clone());
        Ok(())
    }

    fn load_training_metadata(&self, mission_id: &str) -> Result<TrainingMetadata, DarviumError> {
        self.training_metadata
            .lock().unwrap()
            .get(mission_id)
            .cloned()
            .ok_or_else(|| {
                DarviumError::NotFound(format!("Training metadata not found: {}", mission_id))
            })
    }

    fn store_fusion_metadata(&self, metadata: &FusionMetadata) -> Result<(), DarviumError> {
        self.fusion_metadata
            .lock().unwrap()
            .insert(metadata.pair_id.clone(), metadata.clone());
        Ok(())
    }

    fn load_fusion_metadata(&self, pair_id: &str) -> Result<FusionMetadata, DarviumError> {
        self.fusion_metadata
            .lock().unwrap()
            .get(pair_id)
            .cloned()
            .ok_or_else(|| {
                DarviumError::NotFound(format!("Fusion metadata not found: {}", pair_id))
            })
    }

    // === 汎用 Interaction API (v2.3-g, RFC §12C.7) ===

    fn store_interaction(&self, record: &StoredInteraction) -> Result<(), DarviumError> {
        self.human_interactions
            .lock().unwrap()
            .insert(record.interaction_id.clone(), record.clone());
        Ok(())
    }

    fn load_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<Option<StoredInteraction>, DarviumError> {
        Ok(self
            .human_interactions
            .lock().unwrap()
            .get(interaction_id)
            .cloned())
    }

    fn list_interactions(
        &self,
        filter: &InteractionFilter,
    ) -> Result<Vec<StoredInteraction>, DarviumError> {
        let interactions = self.human_interactions.lock().unwrap();
        let mut results: Vec<StoredInteraction> = interactions
            .values()
            .filter(|r| {
                if let Some(ref status) = filter.status {
                    if r.status != *status {
                        return false;
                    }
                }
                if let Some(ref channel_id) = filter.channel_id {
                    // 現状ペイロードに channel_id がないため照合は保留。
                    // M1.5-R7 で HumanRequest 拡張時に具体化する。
                    let _ = channel_id;
                }
                if let Some(after) = filter.created_after {
                    if r.created_at < after {
                        return false;
                    }
                }
                if let Some(before) = filter.created_before {
                    if r.created_at >= before {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }
        Ok(results)
    }

    fn resolve_interaction(
        &self,
        interaction_id: &str,
        outcome: &HumanOutcome,
    ) -> Result<(), DarviumError> {
        let mut interactions = self.human_interactions.lock().unwrap();
        let record = interactions.get_mut(interaction_id).ok_or_else(|| {
            DarviumError::NotFound(format!("Interaction not found: {}", interaction_id))
        })?;
        record.outcome = Some(outcome.clone());
        record.status = InteractionStatus::Resolved;
        Ok(())
    }

    fn abort_interaction(
        &self,
        interaction_id: &str,
        _reason: &str,
    ) -> Result<(), DarviumError> {
        let mut interactions = self.human_interactions.lock().unwrap();
        let record = interactions.get_mut(interaction_id).ok_or_else(|| {
            DarviumError::NotFound(format!("Interaction not found: {}", interaction_id))
        })?;
        record.status = InteractionStatus::Aborted;
        Ok(())
    }

    fn reconnect_interaction(
        &self,
        interaction_id: &str,
        _new_channel_id: &str,
    ) -> Result<(), DarviumError> {
        let mut interactions = self.human_interactions.lock().unwrap();
        let record = interactions.get_mut(interaction_id).ok_or_else(|| {
            DarviumError::NotFound(format!("Interaction not found: {}", interaction_id))
        })?;
        // 暫定: 時刻のみ更新。M1.5-R7 でチャネル情報の更新を追加。
        record.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Ok(())
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
    /// テスト用の最小限 TrustAuditLog を生成する。
    fn make_test_audit_log() -> TrustAuditLog {
        use crate::types::TrustAuditEvent;
        TrustAuditLog {
            graph_id: "test-graph".into(),
            event_type: TrustAuditEvent::AdminFastTrack,
            actor_id: "test-admin".into(),
            old_value: 0.5,
            new_value: 0.8,
            timestamp: std::time::SystemTime::now(),
            reason: None,
        }
    }

    #[test]
    fn store_and_load_trust_audit_logs() {
        let store = InMemoryMetadataStore::new();

        store
            .store_trust_audit_log(&make_test_audit_log())
            .expect("store should succeed");
        store
            .store_trust_audit_log(&make_test_audit_log())
            .expect("store should succeed");
        store
            .store_trust_audit_log(&make_test_audit_log())
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
            payload: HitlPayload { request: HumanRequest {
                subject: "test".into(),
                body: "body".into(),
                context: serde_json::json!({}),
                timeout: None,
            },},
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
            payload: HitlPayload { request: HumanRequest {
                subject: id.into(),
                body: "".into(),
                context: serde_json::json!({}),
                timeout: None,
            },},
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
            payload: HitlPayload { request: HumanRequest {
                subject: "resolve".into(),
                body: "".into(),
                context: serde_json::json!({}),
                timeout: None,
            },},
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
            payload: HitlPayload { request: HumanRequest {
                subject: "v1".into(),
                body: "".into(),
                context: serde_json::json!({}),
                timeout: None,
            },},
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 0,
            updated_at: 0,
        };
        store.store_human_interaction(&record1).unwrap();
        let record2 = StoredInteraction {
            interaction_id: "dup".to_string(),
            payload: HitlPayload { request: HumanRequest {
                subject: "v2".into(),
                body: "".into(),
                context: serde_json::json!({}),
                timeout: None,
            },},
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 1,
            updated_at: 1,
        };
        store.store_human_interaction(&record2).unwrap();
        let loaded = store.load_human_interaction("dup").unwrap();
        assert_eq!(loaded.request().subject, "v2");
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
            payload: HitlPayload { request: HumanRequest {
                subject: "crash".into(),
                body: "recover".into(),
                context: serde_json::json!({"sim": "crash"}),
                timeout: None,
            },},
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
                pending_record.request(),
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
                stored.request(),
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

    // ================================================================
    // T1: store_interaction + load_interaction write-after-read 一貫性
    // ================================================================

    /// テスト用 StoredInteraction を生成するヘルパー。
    fn make_interaction(id: &str, status: InteractionStatus, created_at: u64) -> StoredInteraction {
        StoredInteraction {
            interaction_id: id.to_string(),
            payload: HitlPayload { request: HumanRequest {
                subject: format!("subject-{}", id),
                body: "body".into(),
                context: serde_json::json!({"key": id}),
                timeout: None,
            },},
            outcome: None,
            status,
            created_at,
            updated_at: created_at,
        }
    }

    #[test]
    fn t1_store_and_load_roundtrip_n100() {
        let store = InMemoryMetadataStore::new();
        let statuses = [
            InteractionStatus::Pending,
            InteractionStatus::AwaitingExternal,
            InteractionStatus::Resolved,
            InteractionStatus::TimedOut,
            InteractionStatus::Unreachable,
            InteractionStatus::ChannelClosed,
            InteractionStatus::Aborted,
        ];

        // 100 レコードを保存
        for i in 0..100 {
            let record = make_interaction(
                &format!("t1-id-{}", i),
                statuses[i % statuses.len()],
                i as u64 * 10,
            );
            store.store_interaction(&record).unwrap();
        }

        // 各レコードを読み出して完全一致確認
        for i in 0..100 {
            let loaded = store
                .load_interaction(&format!("t1-id-{}", i))
                .unwrap()
                .expect(&format!("record {} should exist", i));
            assert_eq!(loaded.interaction_id, format!("t1-id-{}", i));
            assert_eq!(loaded.status, statuses[i % statuses.len()]);
            assert_eq!(loaded.created_at, i as u64 * 10);
        }

        // 存在しない ID は Ok(None)
        let nonexistent = store.load_interaction("t1-id-nonexistent").unwrap();
        assert!(nonexistent.is_none());
    }

    // ================================================================
    // T2: list_interactions + InteractionFilter フィルタ精度
    // ================================================================

    #[test]
    fn t2_list_interactions_filter_accuracy() {
        let store = InMemoryMetadataStore::new();
        let statuses = [
            InteractionStatus::Pending,
            InteractionStatus::Pending,
            InteractionStatus::Resolved,
            InteractionStatus::Aborted,
        ];
        let times = [100, 200, 300, 400];

        for i in 0..4 {
            let record = make_interaction(&format!("t2-id-{}", i), statuses[i], times[i]);
            store.store_interaction(&record).unwrap();
        }

        // status フィルタ: Pending → 2件
        let pending = store
            .list_interactions(&InteractionFilter {
                status: Some(InteractionStatus::Pending),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending.iter().all(|r| r.status == InteractionStatus::Pending));

        // created_after フィルタ: >= 200 → 3件
        let after = store
            .list_interactions(&InteractionFilter {
                created_after: Some(200),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(after.len(), 3);

        // created_before フィルタ: < 300 → 2件
        let before = store
            .list_interactions(&InteractionFilter {
                created_before: Some(300),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(before.len(), 2);

        // created_after + created_before 複合: [200, 300) → 1件
        let range = store
            .list_interactions(&InteractionFilter {
                created_after: Some(200),
                created_before: Some(300),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(range.len(), 1);
        assert_eq!(range[0].interaction_id, "t2-id-1");

        // limit フィルタ: 2件に制限
        let limited = store
            .list_interactions(&InteractionFilter {
                limit: Some(2),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(limited.len(), 2);

        // 全フィールド None → 全4件
        let all = store
            .list_interactions(&InteractionFilter::default())
            .unwrap();
        assert_eq!(all.len(), 4);

        // status + limit 複合
        let filtered_limited = store
            .list_interactions(&InteractionFilter {
                status: Some(InteractionStatus::Pending),
                limit: Some(1),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered_limited.len(), 1);
        assert_eq!(filtered_limited[0].status, InteractionStatus::Pending);
    }

    // ================================================================
    // T3: resolve_interaction 状態遷移
    // ================================================================

    #[test]
    fn t3_resolve_interaction_transition() {
        let store = InMemoryMetadataStore::new();
        let record = make_interaction("t3-id", InteractionStatus::Pending, 100);
        store.store_interaction(&record).unwrap();

        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("resolved in T3".into()),
            revised_body: None,
        });
        store.resolve_interaction("t3-id", &outcome).unwrap();

        let loaded = store.load_interaction("t3-id").unwrap().unwrap();
        assert_eq!(loaded.status, InteractionStatus::Resolved);
        assert_eq!(loaded.outcome, Some(outcome.clone()));

        // 存在しない ID → Err(NotFound)
        let err = store.resolve_interaction("t3-nonexistent", &outcome);
        assert!(matches!(err, Err(DarviumError::NotFound(_))));
    }

    // ================================================================
    // T4: abort_interaction 状態遷移
    // ================================================================

    #[test]
    fn t4_abort_interaction_transition() {
        let store = InMemoryMetadataStore::new();

        // Pending から Abort
        let p_record = make_interaction("t4-pending", InteractionStatus::Pending, 100);
        store.store_interaction(&p_record).unwrap();
        store.abort_interaction("t4-pending", "timeout").unwrap();
        let loaded = store.load_interaction("t4-pending").unwrap().unwrap();
        assert_eq!(loaded.status, InteractionStatus::Aborted);

        // AwaitingExternal から Abort
        let a_record =
            make_interaction("t4-awaiting", InteractionStatus::AwaitingExternal, 200);
        store.store_interaction(&a_record).unwrap();
        store
            .abort_interaction("t4-awaiting", "channel closed")
            .unwrap();
        let loaded2 = store.load_interaction("t4-awaiting").unwrap().unwrap();
        assert_eq!(loaded2.status, InteractionStatus::Aborted);

        // 存在しない ID → Err(NotFound)
        let err = store.abort_interaction("t4-nonexistent", "reason");
        assert!(matches!(err, Err(DarviumError::NotFound(_))));
    }

    // ================================================================
    // T5: reconnect_interaction 再接続
    // ================================================================

    #[test]
    fn t5_reconnect_interaction_updates_timestamp() {
        let store = InMemoryMetadataStore::new();
        let record = make_interaction("t5-id", InteractionStatus::Pending, 100);
        store.store_interaction(&record).unwrap();

        // 再接続前の updated_at を記録
        let before = store.load_interaction("t5-id").unwrap().unwrap();
        let original_updated_at = before.updated_at;

        // 再接続実行
        store
            .reconnect_interaction("t5-id", "new-channel-xyz")
            .unwrap();

        let after = store.load_interaction("t5-id").unwrap().unwrap();
        assert!(
            after.updated_at > original_updated_at,
            "updated_at should increase after reconnect"
        );

        // 存在しない ID → Err(NotFound)
        let err = store.reconnect_interaction("t5-nonexistent", "channel");
        assert!(matches!(err, Err(DarviumError::NotFound(_))));
    }

    // ================================================================
    // T6: 既存 HITL 特化メソッドのラッパー検証
    // ================================================================

    #[test]
    fn t6_wrapper_delegation_consistency() {
        let store = InMemoryMetadataStore::new();
        let record = make_interaction("t6-id", InteractionStatus::Pending, 100);
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("wrapper test".into()),
            revised_body: None,
        });

        // store_human_interaction == store_interaction
        store.store_human_interaction(&record).unwrap();
        let via_generic = store.load_interaction("t6-id").unwrap().unwrap();
        assert_eq!(via_generic.interaction_id, "t6-id");

        // resolve_human_interaction == resolve_interaction
        store
            .resolve_human_interaction("t6-id", &outcome)
            .unwrap();
        let resolved = store.load_interaction("t6-id").unwrap().unwrap();
        assert_eq!(resolved.status, InteractionStatus::Resolved);

        // load_human_interaction: 存在しない ID → Err(NotFound)
        let not_found = store.load_human_interaction("t6-nonexistent");
        assert!(matches!(not_found, Err(DarviumError::NotFound(_))));

        // list_pending_human_interactions == list_interactions(status=Pending)
        let pending1 = store.list_pending_human_interactions().unwrap();
        let pending2 = store
            .list_interactions(&InteractionFilter {
                status: Some(InteractionStatus::Pending),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(pending1.len(), pending2.len());

        // load_human_interaction が Err->None 変換を正しく行う
        // Resolved は Pending としてカウントされない
        assert_eq!(pending1.len(), 0, "resolved record should not be pending");
    }

    // ================================================================
    // T7: 後方互換性
    // 既存の T10 テスト群は変更なしでパスする（同一ファイル内で定義済み）
    // ================================================================

    #[test]
    fn t7_generic_api_does_not_break_existing() {
        let store = InMemoryMetadataStore::new();
        let record = make_interaction("t7-id", InteractionStatus::Pending, 0);
        store.store_human_interaction(&record).unwrap();

        // 古い API で読み出し
        let loaded = store.load_human_interaction("t7-id").unwrap();
        assert_eq!(loaded.interaction_id, "t7-id");
        assert_eq!(loaded.status, InteractionStatus::Pending);

        // 新しい API でも同じ結果
        let loaded_generic = store.load_interaction("t7-id").unwrap().unwrap();
        assert_eq!(loaded, loaded_generic);
    }

    // ================================================================
    // T8: JsonMetadataStore 永続化検証（汎用 API + ファイル永続化）
    // ================================================================

    #[test]
    fn t8_json_store_generic_persistence() {
        let dir = std::env::temp_dir().join(format!("darvium-t8-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("store.json");

        {
            let store = crate::store::json_metadata_store::JsonMetadataStore::new(&path).unwrap();
            let record = make_interaction("t8-id", InteractionStatus::Pending, 100);
            store.store_interaction(&record).unwrap();

            // abort 後の状態も永続化
            store.abort_interaction("t8-id", "test").unwrap();
        }

        // 再読込: ファイルから状態が復元される
        let store2 = crate::store::json_metadata_store::JsonMetadataStore::new(&path).unwrap();
        let loaded = store2.load_interaction("t8-id").unwrap().unwrap();
        assert_eq!(loaded.interaction_id, "t8-id");
        assert_eq!(loaded.status, InteractionStatus::Aborted);
        assert_eq!(loaded.created_at, 100);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ================================================================
    // OTS-1: スループット計測
    // 6メソッド × 1000 呼び出しのスループットを計測
    // ================================================================

    #[test]
    fn ots1_throughput_measurement() {
        let store = InMemoryMetadataStore::new();
        let n = 1000u32;
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });

        // --- store_interaction ---
        let start = std::time::Instant::now();
        for i in 0..n {
            let record = make_interaction(
                &format!("ots1-store-{}", i),
                InteractionStatus::Pending,
                i as u64,
            );
            store.store_interaction(&record).unwrap();
        }
        let store_dur = start.elapsed();

        // --- load_interaction ---
        let start = std::time::Instant::now();
        for i in 0..n {
            let loaded = store
                .load_interaction(&format!("ots1-store-{}", i))
                .unwrap();
            assert!(loaded.is_some());
        }
        let load_dur = start.elapsed();

        // --- list_interactions ---
        let start = std::time::Instant::now();
        for _ in 0..n {
            let results = store
                .list_interactions(&InteractionFilter::default())
                .unwrap();
            assert_eq!(results.len() as u32, n);
        }
        let list_dur = start.elapsed();

        // --- resolve_interaction ---
        // 新規レコードを作成してから解決
        let start = std::time::Instant::now();
        for i in 0..n {
            let idx = i + n; // unique IDs
            let record = make_interaction(
                &format!("ots1-resolve-{}", idx),
                InteractionStatus::Pending,
                idx as u64,
            );
            store.store_interaction(&record).unwrap();
            store
                .resolve_interaction(&format!("ots1-resolve-{}", idx), &outcome)
                .unwrap();
        }
        let resolve_dur = start.elapsed();

        // --- abort_interaction ---
        let start = std::time::Instant::now();
        for i in 0..n {
            let idx = i + n * 2;
            let record = make_interaction(
                &format!("ots1-abort-{}", idx),
                InteractionStatus::Pending,
                idx as u64,
            );
            store.store_interaction(&record).unwrap();
            store
                .abort_interaction(&format!("ots1-abort-{}", idx), "test")
                .unwrap();
        }
        let abort_dur = start.elapsed();

        // --- reconnect_interaction ---
        // 事前にレコードを作成
        for i in 0..n {
            let idx = i + n * 3;
            let record = make_interaction(
                &format!("ots1-reconnect-{}", idx),
                InteractionStatus::Pending,
                idx as u64,
            );
            store.store_interaction(&record).unwrap();
        }
        let start = std::time::Instant::now();
        for i in 0..n {
            let idx = i + n * 3;
            store
                .reconnect_interaction(&format!("ots1-reconnect-{}", idx), "new-channel")
                .unwrap();
        }
        let reconnect_dur = start.elapsed();

        let total_dur = store_dur + load_dur + list_dur + resolve_dur + abort_dur + reconnect_dur;

        // --- 結果出力 ---
        println!("=== OTS-1: Throughput Measurement ===");
        println!("  n={}", n);
        println!("  store_interaction:    {:8?} (avg {:6.1} ns/call)", store_dur, store_dur.as_nanos() as f64 / n as f64);
        println!("  load_interaction:     {:8?} (avg {:6.1} ns/call)", load_dur, load_dur.as_nanos() as f64 / n as f64);
        println!("  list_interactions:    {:8?} (avg {:6.1} ns/call)", list_dur, list_dur.as_nanos() as f64 / n as f64);
        println!("  resolve_interaction:  {:8?} (avg {:6.1} ns/call)", resolve_dur, resolve_dur.as_nanos() as f64 / n as f64);
        println!("  abort_interaction:    {:8?} (avg {:6.1} ns/call)", abort_dur, abort_dur.as_nanos() as f64 / n as f64);
        println!("  reconnect_interaction:{:8?} (avg {:6.1} ns/call)", reconnect_dur, reconnect_dur.as_nanos() as f64 / n as f64);
        println!("  total (6 x 1000):     {:8?}", total_dur);
        println!("  throughput:           {:.0} calls/sec", 6.0 * n as f64 / total_dur.as_secs_f64());
        println!("=== 結果: PASS ===");

        // 全操作が O(1) で線形: 1000 回の呼び出しが 1秒以内に完了するはず
        assert!(
            total_dur.as_secs() < 5,
            "6x1000 operations should complete in <5s (took {:?})",
            total_dur
        );
    }
}
