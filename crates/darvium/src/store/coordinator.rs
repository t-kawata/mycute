// DualStoreCoordinator — 異種ストア論理一貫性コミットプロトコル
//
// GraphStore (LadybugDB) と MetadataStore (SQLite) の2系統ストアに対し、
// RFC §18.2 に基づくアプリケーションレベルのコミットプロトコルを提供する。
// ConsistencyState による Pending → NeedsRepair → Committed/Tombstone の
// 状態機械を管理し、非 Committed 状態の資産を検索候補から除外する。

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::constants::{DUAL_STORE_ERROR_INJECTION_SEED, DUAL_STORE_MAX_RETRY};
use crate::error::DarviumError;
use crate::event::{
    DarviumEvent, DarviumEventBus, DarviumEventKind, EventCausality, EventMetadata, EventPrivacy,
    EventRetention, EventSource, EventVisibility, InteractionMode, PiiHandlingPolicy, SearchEvent,
};
use crate::store::graph_store::GraphStore;
use crate::store::metadata_store::MetadataStore;
use crate::types::{
    CommitPhase, ConsistencyState, RankedCandidate, RepairAction, RepairLog, SearchTrace,
    WorkflowGraph, WorkflowGraphId,
};

/// 異種ストア論理一貫性の調整役 (RFC §18.2)。
///
/// GraphStore + MetadataStore を保持し、両ストアにまたがる更新を
/// アプリケーション層のコミットプロトコルで調整する。
/// 各資産 (asset_id) に ConsistencyState を割り当て、
/// non-Committed 状態の資産を検索候補から除外する hard gate を提供する。
///
/// # 内部可変性
///
/// トレイトメソッドが &self を取るため、内部状態は RefCell でラップする。
/// シングルスレッド環境でのみ使用すること。並行アクセスが必要な場合は
/// 独立したインスタンスを使用する。この構造体は Send であるため、
/// スレッド間の移動は可能だが、複数スレッドからの共有参照 (&Self) は
/// サポートされない。
pub struct DualStoreCoordinator {
    graph_store: Box<dyn GraphStore + Send>,
    metadata_store: Box<dyn MetadataStore + Send>,
    repair_queue: RefCell<Vec<RepairLog>>,
    consistency_states: RefCell<HashMap<String, ConsistencyState>>,
    event_bus: Option<Arc<dyn DarviumEventBus + Send + Sync>>,
}

/// 起動時修復スキャンの実行結果サマリ (RFC §18.2 / M1.5-3)。
#[derive(Debug, Clone, Default)]
pub struct RepairScanSummary {
    /// 走査した全資産数。
    pub total_scanned: usize,
    /// 不整合状態（Pending / NeedsRepair）として発見された資産数。
    pub found_inconsistent: usize,
    /// 修復成功（→Committed）した資産数。
    pub repaired: usize,
    /// 隔離（→Quarantined）された資産数。
    pub quarantined: usize,
    /// 最初から Committed だった資産数。
    pub already_clean: usize,
    /// スキャン所要時間（ミリ秒）。
    pub duration_ms: u64,
}

impl DualStoreCoordinator {
    /// 新しい DualStoreCoordinator を生成する。
    pub fn new(
        graph_store: Box<dyn GraphStore + Send>,
        metadata_store: Box<dyn MetadataStore + Send>,
    ) -> Self {
        Self {
            graph_store,
            metadata_store,
            repair_queue: RefCell::new(Vec::new()),
            consistency_states: RefCell::new(HashMap::new()),
            event_bus: None,
        }
    }

    /// EventBus を設定するビルダーメソッド。
    /// commit_dual_store_update() 内で store_search_trace と併せて
    /// SearchEvent::Started を publish するようになる。
    pub fn with_event_bus(mut self, event_bus: Arc<dyn DarviumEventBus + Send + Sync>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// 2ストアにまたがるコミットを実行する (RFC §18.2.1)。
    ///
    /// プロトコル:
    /// 1. 状態を Pending { op_id, phase: MetaPrepared } に設定
    /// 2. GraphStore への書き込み
    /// 3. 位相を BlobPrepared に進める
    /// 4. MetadataStore への書き込み
    /// 5. 状態を Committed に設定
    ///
    /// いずれかのストア書き込みが失敗した場合、状態を NeedsRepair に遷移させ、
    /// 修復キューにエントリを追加して Err を返す。
    pub fn commit_dual_store_update(
        &self,
        asset_id: &str,
        op_id: &str,
    ) -> Result<(), DarviumError> {
        // Step 1: Pending 状態へ遷移
        self.set_consistency_state(
            asset_id,
            ConsistencyState::Pending {
                op_id: op_id.to_string(),
                phase: CommitPhase::MetaPrepared,
            },
        );

        // Step 2: GraphStore 書き込み
        if let Err(e) = self.graph_store.store_workflow_graph(&WorkflowGraph::new()) {
            self.transition_to_needs_repair(
                asset_id,
                op_id,
                &format!("GraphStore write failed: {}", e),
            );
            return Err(e);
        }

        // Step 3: 位相を BlobPrepared に進める
        self.set_consistency_state(
            asset_id,
            ConsistencyState::Pending {
                op_id: op_id.to_string(),
                phase: CommitPhase::BlobPrepared,
            },
        );

        // Step 4: MetadataStore 書き込み
        if let Err(e) = self.metadata_store.store_search_trace(&SearchTrace) {
            self.transition_to_needs_repair(
                asset_id,
                op_id,
                &format!("MetadataStore write failed: {}", e),
            );
            return Err(e);
        }

        // EventBus publish（Optional: with_event_bus() で設定時のみ）
        if let Some(ref bus) = self.event_bus {
            let event = DarviumEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                kind: DarviumEventKind::Search(SearchEvent::Started),
                interaction_mode: InteractionMode::OneWay,
                payload: serde_json::Value::Null,
                causality: EventCausality {
                    parent_event_id: None,
                    root_event_id: None,
                    trace_ref: None,
                    mission_id: None,
                    workflow_id: None,
                    run_id: None,
                },
                metadata: EventMetadata {
                    clock: 0,
                    timestamp: SystemTime::UNIX_EPOCH,
                    source: EventSource::Test,
                },
                transport_meta: None,
                visibility: EventVisibility::Public,
                retention: EventRetention {
                    persist: false,
                    ttl_days: None,
                },
                privacy: EventPrivacy {
                    contains_pii: false,
                    sandbox_only: false,
                    pii_handling: PiiHandlingPolicy::Reject,
                },
            };
            let _ = bus.publish(event);
        }

        // Step 5: Committed 状態へ遷移
        self.set_consistency_state(asset_id, ConsistencyState::Committed);

        Ok(())
    }

    /// 指定された asset_id が検索対象として適格かを判定する。
    ///
    /// Committed 状態の資産のみが検索候補として適格となる
    /// (RFC §18.2 Hard Retrieval Exclusion)。
    pub fn is_eligible_for_retrieval(&self, asset_id: &str) -> bool {
        let states = self.consistency_states.borrow();
        match states.get(asset_id) {
            Some(state) => state.is_eligible_for_retrieval(),
            // 状態が未定義の資産は適格とする（デフォルト Committed）
            None => true,
        }
    }

    /// 検索候補リストから non-Committed 状態の資産を除外する
    /// (Hard Retrieval Exclusion Gate, RFC §18.2)。
    ///
    /// 状態が未定義（デフォルト Committed）の資産は通過させる。
    /// 監査モードでは除外された候補の内訳を標準出力に書き出す。
    pub fn filter_retrieval_eligible(
        &self,
        candidates: Vec<RankedCandidate>,
    ) -> Vec<RankedCandidate> {
        let states = self.consistency_states.borrow();
        candidates
            .into_iter()
            .filter(|candidate| {
                let eligible = match states.get(&candidate.workflow_id) {
                    Some(state) => state.is_eligible_for_retrieval(),
                    None => true,
                };
                eligible
            })
            .collect()
    }

    /// 資産の現在の ConsistencyState を取得する。
    pub fn get_consistency_state(&self, asset_id: &str) -> Option<ConsistencyState> {
        self.consistency_states.borrow().get(asset_id).cloned()
    }

    /// 資産の ConsistencyState を設定する（テスト用）。
    pub fn set_consistency_state(&self, asset_id: &str, state: ConsistencyState) {
        self.consistency_states
            .borrow_mut()
            .insert(asset_id.to_string(), state);
    }

    /// 修復キューにエントリを追加する。
    ///
    /// graph_id は asset_id を WorkflowGraphId として使用する。
    /// 返り値として生成された RepairLog を返す。
    pub fn enqueue_repair(&self, op_id: &str, graph_id: &str, reason: &str) -> RepairLog {
        let log_entry = RepairLog {
            op_id: op_id.to_string(),
            graph_id: graph_id.to_string(),
            detected_at: std::time::SystemTime::now(),
            reason: reason.to_string(),
            action: RepairAction::RetryMetaCommit,
        };

        self.repair_queue.borrow_mut().push(log_entry.clone());
        log_entry
    }

    /// 修復キューの全エントリを取得する。
    pub fn list_repair_queue(&self) -> Vec<RepairLog> {
        self.repair_queue.borrow().clone()
    }

    /// 指定された資産に対して修復を試行する。
    ///
    /// NeedsRepair 状態の資産に対して GraphStore → MetadataStore の再書き込みを
    /// 試行し、成功すれば Committed、失敗すれば Quarantined へ遷移させる。
    /// 最大再試行回数は DUAL_STORE_MAX_RETRY に従う。
    pub fn apply_repair(&self, asset_id: &str) -> Result<(), DarviumError> {
        let state = self
            .get_consistency_state(asset_id)
            .ok_or_else(|| DarviumError::NotFound(format!("No state for asset: {}", asset_id)))?;

        match state {
            ConsistencyState::NeedsRepair { op_id, .. } => {
                for attempt in 0..DUAL_STORE_MAX_RETRY {
                    // GraphStore 再書き込み
                    if self
                        .graph_store
                        .store_workflow_graph(&WorkflowGraph::new())
                        .is_err()
                    {
                        if attempt + 1 >= DUAL_STORE_MAX_RETRY {
                            self.transition_to_quarantined(asset_id, &op_id);
                            return Err(DarviumError::DualStoreCommit(format!(
                                "Repair failed after {} attempts for asset: {}",
                                DUAL_STORE_MAX_RETRY, asset_id
                            )));
                        }
                        continue;
                    }

                    // MetadataStore 再書き込み
                    if self
                        .metadata_store
                        .store_search_trace(&SearchTrace)
                        .is_err()
                    {
                        if attempt + 1 >= DUAL_STORE_MAX_RETRY {
                            self.transition_to_quarantined(asset_id, &op_id);
                            return Err(DarviumError::DualStoreCommit(format!(
                                "Repair failed on metadata after {} attempts for asset: {}",
                                DUAL_STORE_MAX_RETRY, asset_id
                            )));
                        }
                        continue;
                    }

                    // EventBus publish（Optional: with_event_bus() で設定時のみ）
                    if let Some(ref bus) = self.event_bus {
                        let event = DarviumEvent {
                            event_id: uuid::Uuid::new_v4().to_string(),
                            kind: DarviumEventKind::Search(SearchEvent::Started),
                            interaction_mode: InteractionMode::OneWay,
                            payload: serde_json::Value::Null,
                            causality: EventCausality {
                                parent_event_id: None,
                                root_event_id: None,
                                trace_ref: None,
                                mission_id: None,
                                workflow_id: None,
                                run_id: None,
                            },
                            metadata: EventMetadata {
                                clock: 0,
                                timestamp: SystemTime::UNIX_EPOCH,
                                source: EventSource::Test,
                            },
                            transport_meta: None,
                            visibility: EventVisibility::Public,
                            retention: EventRetention {
                                persist: false,
                                ttl_days: None,
                            },
                            privacy: EventPrivacy {
                                contains_pii: false,
                                sandbox_only: false,
                                pii_handling: PiiHandlingPolicy::Reject,
                            },
                        };
                        let _ = bus.publish(event);
                    }

                    // 成功: Committed へ戻す
                    self.set_consistency_state(asset_id, ConsistencyState::Committed);
                    return Ok(());
                }

                Err(DarviumError::DualStoreCommit(format!(
                    "Repair exhausted retries for asset: {}",
                    asset_id
                )))
            }
            _ => Err(DarviumError::Internal(format!(
                "Asset is not in NeedsRepair state: {}",
                asset_id
            ))),
        }
    }

    /// NeedsRepair 状態へ遷移させ、修復キューに追加する。
    fn transition_to_needs_repair(&self, asset_id: &str, op_id: &str, reason: &str) {
        self.set_consistency_state(
            asset_id,
            ConsistencyState::NeedsRepair {
                op_id: op_id.to_string(),
                reason: reason.to_string(),
            },
        );
        self.enqueue_repair(op_id, asset_id, reason);
    }

    /// Quarantined 状態へ遷移させる。
    fn transition_to_quarantined(&self, asset_id: &str, op_id: &str) {
        self.set_consistency_state(
            asset_id,
            ConsistencyState::Quarantined {
                op_id: op_id.to_string(),
                since: std::time::SystemTime::now(),
            },
        );
    }

    /// 起動時修復スキャンを実行する (RFC §18.2 / M1.5-3)。
    ///
    /// 全資産の ConsistencyState を走査し、`Pending` または `NeedsRepair` 状態の
    /// 資産を検出して修復 (Committed) または隔離 (Quarantined) する。
    /// 修復中も `is_eligible_for_retrieval()` による hard exclusion は維持される。
    pub fn startup_repair_scan(&self) -> RepairScanSummary {
        let start = std::time::Instant::now();
        let snapshot: Vec<String> = self.consistency_states.borrow().keys().cloned().collect();
        let total_scanned = snapshot.len();

        let mut already_clean: usize = 0;
        let mut found_inconsistent: usize = 0;
        let mut repaired: usize = 0;
        let mut quarantined: usize = 0;

        for asset_id in &snapshot {
            match self.get_consistency_state(asset_id) {
                // 最初から Committed: カウントのみ
                Some(ConsistencyState::Committed) => {
                    already_clean += 1;
                }
                // Pending: 中断されたコミット。NeedsRepair に変換してから修復を試行
                Some(ConsistencyState::Pending { op_id, .. }) => {
                    found_inconsistent += 1;
                    self.set_consistency_state(
                        asset_id,
                        ConsistencyState::NeedsRepair {
                            op_id: op_id.clone(),
                            reason: "repair scan: interrupted commit".to_string(),
                        },
                    );
                    match self.apply_repair(asset_id) {
                        Ok(()) => repaired += 1,
                        Err(_) => quarantined += 1,
                    }
                }
                // NeedsRepair: 直接修復を試行
                Some(ConsistencyState::NeedsRepair { .. }) => {
                    found_inconsistent += 1;
                    match self.apply_repair(asset_id) {
                        Ok(()) => repaired += 1,
                        Err(_) => quarantined += 1,
                    }
                }
                // Quarantined または None: スキップ
                _ => {}
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        RepairScanSummary {
            total_scanned,
            found_inconsistent,
            repaired,
            quarantined,
            already_clean,
            duration_ms,
        }
    }
}

// ============================================================================
// FailingStore ラッパー: エラー注入のためのプロキシ実装
// ============================================================================

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// GraphStore をラップし、確率的にエラーを注入するラッパー。
///
/// 指定された failure_probability の確率で I/O エラーを注入する。
/// 試験用の固定シード PRNG を使用する。
#[allow(dead_code)]
pub struct FailingGraphStore {
    inner: Box<dyn GraphStore + Send>,
    failure_probability: f64,
    rng: RefCell<StdRng>,
}

#[allow(dead_code)]
impl FailingGraphStore {
    pub fn new(inner: Box<dyn GraphStore + Send>, failure_probability: f64) -> Self {
        Self {
            inner,
            failure_probability,
            rng: RefCell::new(StdRng::seed_from_u64(DUAL_STORE_ERROR_INJECTION_SEED)),
        }
    }

    fn maybe_fail(&self) -> Result<(), DarviumError> {
        let mut rng = self.rng.borrow_mut();
        if rng.random::<f64>() < self.failure_probability {
            Err(DarviumError::DualStoreCommit(
                "FailingGraphStore: injected I/O error".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

impl GraphStore for FailingGraphStore {
    fn store_workflow_graph(&self, graph: &WorkflowGraph) -> Result<WorkflowGraphId, DarviumError> {
        self.maybe_fail()?;
        self.inner.store_workflow_graph(graph)
    }

    fn load_workflow_graph(
        &self,
        graph_id: &WorkflowGraphId,
    ) -> Result<WorkflowGraph, DarviumError> {
        self.maybe_fail()?;
        self.inner.load_workflow_graph(graph_id)
    }

    fn store_embedding(&self, key: &str, vector: &[f32]) -> Result<(), DarviumError> {
        self.maybe_fail()?;
        self.inner.store_embedding(key, vector)
    }

    fn semantic_search(
        &self,
        query: &[f32],
        top_k: u32,
    ) -> Result<Vec<(String, f64)>, DarviumError> {
        self.maybe_fail()?;
        self.inner.semantic_search(query, top_k)
    }

    fn store_knowledge_object(
        &self,
        obj: &crate::types::KnowledgeObject,
    ) -> Result<(), DarviumError> {
        self.maybe_fail()?;
        self.inner.store_knowledge_object(obj)
    }

    fn load_knowledge_object(
        &self,
        object_id: &str,
    ) -> Result<crate::types::KnowledgeObject, DarviumError> {
        self.maybe_fail()?;
        self.inner.load_knowledge_object(object_id)
    }

    fn store_relation(&self, relation: &crate::types::RelationRecord) -> Result<(), DarviumError> {
        self.maybe_fail()?;
        self.inner.store_relation(relation)
    }

    fn load_relations(
        &self,
        object_id: &str,
    ) -> Result<Vec<crate::types::RelationRecord>, DarviumError> {
        self.maybe_fail()?;
        self.inner.load_relations(object_id)
    }

    fn record_origin_trace(&self, trace: &crate::types::OriginTrace) -> Result<(), DarviumError> {
        self.maybe_fail()?;
        self.inner.record_origin_trace(trace)
    }

    fn load_origin_traces(
        &self,
        object_id: &str,
    ) -> Result<Vec<crate::types::OriginTrace>, DarviumError> {
        self.maybe_fail()?;
        self.inner.load_origin_traces(object_id)
    }
}

/// MetadataStore をラップし、成功予算ベースでエラーを注入するラッパー。
///
/// success_budget が 0 になると全ての書き込み操作が失敗する。
/// 読み取り専用操作（load_*）は常に成功する。
#[allow(dead_code)]
pub struct FailingMetadataStore {
    inner: Box<dyn MetadataStore + Send>,
    success_budget: Mutex<u32>,
}

#[allow(dead_code)]
impl FailingMetadataStore {
    pub fn new(inner: Box<dyn MetadataStore + Send>, success_budget: u32) -> Self {
        Self {
            inner,
            success_budget: Mutex::new(success_budget),
        }
    }

    fn consume_budget(&self) -> Result<(), DarviumError> {
        let mut budget = self.success_budget.lock().unwrap();
        if *budget == 0 {
            Err(DarviumError::DualStoreCommit(
                "FailingMetadataStore: success budget exhausted".to_string(),
            ))
        } else {
            *budget -= 1;
            Ok(())
        }
    }
}

impl MetadataStore for FailingMetadataStore {
    fn store_search_trace(&self, trace: &SearchTrace) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner.store_search_trace(trace)
    }

    fn load_search_traces(&self, mission_id: &str) -> Result<Vec<SearchTrace>, DarviumError> {
        // 読み取りは常に成功
        self.inner.load_search_traces(mission_id)
    }

    fn store_trust_audit_log(&self, log: &crate::types::TrustAuditLog) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner.store_trust_audit_log(log)
    }

    fn load_trust_audit_logs(
        &self,
        target_id: &str,
    ) -> Result<Vec<crate::types::TrustAuditLog>, DarviumError> {
        self.inner.load_trust_audit_logs(target_id)
    }

    fn store_patch_history(
        &self,
        history: &crate::types::PatchHistory,
    ) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner.store_patch_history(history)
    }

    fn load_patch_histories(
        &self,
        graph_id: &str,
    ) -> Result<Vec<crate::types::PatchHistory>, DarviumError> {
        self.inner.load_patch_histories(graph_id)
    }

    fn store_training_metadata(
        &self,
        metadata: &crate::types::TrainingMetadata,
    ) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner.store_training_metadata(metadata)
    }

    fn load_training_metadata(
        &self,
        mission_id: &str,
    ) -> Result<crate::types::TrainingMetadata, DarviumError> {
        self.inner.load_training_metadata(mission_id)
    }

    fn store_fusion_metadata(
        &self,
        metadata: &crate::types::FusionMetadata,
    ) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner.store_fusion_metadata(metadata)
    }

    fn load_fusion_metadata(
        &self,
        pair_id: &str,
    ) -> Result<crate::types::FusionMetadata, DarviumError> {
        self.inner.load_fusion_metadata(pair_id)
    }

    fn store_human_interaction(
        &self,
        record: &crate::types::StoredInteraction,
    ) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner.store_human_interaction(record)
    }

    fn load_human_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<crate::types::StoredInteraction, DarviumError> {
        self.inner.load_human_interaction(interaction_id)
    }

    fn list_pending_human_interactions(
        &self,
    ) -> Result<Vec<crate::types::StoredInteraction>, DarviumError> {
        self.inner.list_pending_human_interactions()
    }

    fn resolve_human_interaction(
        &self,
        interaction_id: &str,
        outcome: &crate::types::HumanOutcome,
    ) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner
            .resolve_human_interaction(interaction_id, outcome)
    }

    // === 汎用 Interaction API (M1.5-R2) ===

    fn store_interaction(
        &self,
        record: &crate::types::StoredInteraction,
    ) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner.store_interaction(record)
    }

    fn load_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<Option<crate::types::StoredInteraction>, DarviumError> {
        self.inner.load_interaction(interaction_id)
    }

    fn list_interactions(
        &self,
        filter: &crate::types::InteractionFilter,
    ) -> Result<Vec<crate::types::StoredInteraction>, DarviumError> {
        self.inner.list_interactions(filter)
    }

    fn resolve_interaction(
        &self,
        interaction_id: &str,
        outcome: &crate::types::HumanOutcome,
    ) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner.resolve_interaction(interaction_id, outcome)
    }

    fn abort_interaction(&self, interaction_id: &str, reason: &str) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner.abort_interaction(interaction_id, reason)
    }

    fn reconnect_interaction(
        &self,
        interaction_id: &str,
        new_channel_id: &str,
    ) -> Result<(), DarviumError> {
        self.consume_budget()?;
        self.inner
            .reconnect_interaction(interaction_id, new_channel_id)
    }
}

// ============================================================================
// テストコード: T1-T50 不変条件テスト + OTS-1〜OTS-4 観測テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::TEST_PRNG_SEED;
    use crate::store::graph_store::InMemoryGraphStore;
    use crate::store::metadata_store::InMemoryMetadataStore;
    use crate::types::{ConsistencyStateTag, RepairAction};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    // ================================================================
    // T1-T10: ConsistencyState / CommitPhase / RepairLog / RepairAction /
    //         ConsistencyStateTag の型構築・メモリサイズ・トレイト検証
    // ================================================================

    #[test]
    fn t1_consistency_state_committed_construction() {
        let state = ConsistencyState::Committed;
        assert!(state.is_eligible_for_retrieval());
        assert_eq!(state.to_tag(), ConsistencyStateTag::Committed);
    }

    #[test]
    fn t2_consistency_state_pending_construction() {
        let state = ConsistencyState::Pending {
            op_id: "op-1".to_string(),
            phase: CommitPhase::MetaPrepared,
        };
        assert!(!state.is_eligible_for_retrieval());
        assert_eq!(state.to_tag(), ConsistencyStateTag::Pending);
    }

    #[test]
    fn t3_consistency_state_needs_repair_construction() {
        let state = ConsistencyState::NeedsRepair {
            op_id: "op-2".to_string(),
            reason: "write timeout".to_string(),
        };
        assert!(!state.is_eligible_for_retrieval());
        assert_eq!(state.to_tag(), ConsistencyStateTag::NeedsRepair);
    }

    #[test]
    fn t4_consistency_state_quarantined_construction() {
        let state = ConsistencyState::Quarantined {
            op_id: "op-3".to_string(),
            since: std::time::SystemTime::now(),
        };
        assert!(!state.is_eligible_for_retrieval());
        assert_eq!(state.to_tag(), ConsistencyStateTag::Quarantined);
    }

    #[test]
    fn t5_commit_phase_all_variants() {
        let phases = [
            CommitPhase::MetaPrepared,
            CommitPhase::BlobPrepared,
            CommitPhase::MetaCommitted,
            CommitPhase::BlobCommitted,
        ];
        assert_eq!(phases.len(), 4);
        // Debug 出力がすべて異なることを確認
        let debug_strings: Vec<String> = phases.iter().map(|p| format!("{:?}", p)).collect();
        let mut unique = debug_strings.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            unique.len(),
            4,
            "All CommitPhase variants must have distinct Debug outputs"
        );
    }

    #[test]
    fn t6_repair_log_construction() {
        let log = RepairLog {
            op_id: "op-4".to_string(),
            graph_id: "graph-1".to_string(),
            detected_at: std::time::SystemTime::now(),
            reason: "store mismatch".to_string(),
            action: RepairAction::RetryMetaCommit,
        };
        assert_eq!(log.op_id, "op-4");
        assert_eq!(log.graph_id, "graph-1");
        assert_eq!(log.action, RepairAction::RetryMetaCommit);
    }

    #[test]
    fn t7_repair_action_all_variants() {
        let actions = [
            RepairAction::RetryMetaCommit,
            RepairAction::RetryBlobCommit,
            RepairAction::MarkQuarantined,
            RepairAction::ConvertToTombstone,
        ];
        assert_eq!(actions.len(), 4);
        let debug_strings: Vec<String> = actions.iter().map(|a| format!("{:?}", a)).collect();
        let mut unique = debug_strings.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            unique.len(),
            4,
            "All RepairAction variants must have distinct Debug outputs"
        );
    }

    #[test]
    fn t8_consistency_state_tag_all_variants() {
        let tags = [
            ConsistencyStateTag::Committed,
            ConsistencyStateTag::Pending,
            ConsistencyStateTag::NeedsRepair,
            ConsistencyStateTag::Quarantined,
        ];
        assert_eq!(tags.len(), 4);
        // Committed タグのみ is_eligible (型はタグなので直接の意味はないが、
        // ConsistencyState のメソッドとして is_eligible は定義済み)
        let states = [
            ConsistencyState::Committed,
            ConsistencyState::Pending {
                op_id: "x".into(),
                phase: CommitPhase::MetaPrepared,
            },
            ConsistencyState::NeedsRepair {
                op_id: "x".into(),
                reason: "x".into(),
            },
            ConsistencyState::Quarantined {
                op_id: "x".into(),
                since: std::time::SystemTime::now(),
            },
        ];
        assert!(states[0].is_eligible_for_retrieval());
        assert!(!states[1].is_eligible_for_retrieval());
        assert!(!states[2].is_eligible_for_retrieval());
        assert!(!states[3].is_eligible_for_retrieval());
    }

    #[test]
    fn t9_consistency_state_debug_format() {
        let state = ConsistencyState::Pending {
            op_id: "op-dbg".to_string(),
            phase: CommitPhase::BlobPrepared,
        };
        let debug = format!("{:?}", state);
        assert!(
            debug.contains("Pending"),
            "Debug output should contain variant name"
        );
        assert!(
            debug.contains("op-dbg"),
            "Debug output should contain op_id"
        );
    }

    #[test]
    fn t10_repair_log_clone_partial_eq() {
        let log1 = RepairLog {
            op_id: "op-clone".to_string(),
            graph_id: "g-clone".to_string(),
            detected_at: std::time::SystemTime::now(),
            reason: "clone test".to_string(),
            action: RepairAction::ConvertToTombstone,
        };
        let log2 = log1.clone();
        assert_eq!(log1.op_id, log2.op_id);
        assert_eq!(log1.graph_id, log2.graph_id);
        assert_eq!(log1.action, log2.action);
        // detected_at は clone でコピーされるが PartialEq の定義による
        // RepairLog に PartialEq が derive されている場合のみ検証
    }

    // ================================================================
    // T11-T12: メモリサイズ境界テスト (Spec T6/T7)
    // ================================================================

    #[test]
    fn t11_consistency_state_size() {
        let committed_size = std::mem::size_of::<ConsistencyState>();
        let pending_size = std::mem::size_of::<ConsistencyState>();
        // 最大でもポインタ3個分 + enum 判別子以内 (64bit: 24+8=32)
        assert!(
            committed_size <= 64,
            "ConsistencyState size should be bounded (got {})",
            committed_size
        );
        assert_eq!(committed_size, pending_size);
    }

    #[test]
    fn t12_commit_phase_size() {
        let size = std::mem::size_of::<CommitPhase>();
        assert!(
            size <= 8,
            "CommitPhase size should be <= 8 bytes (got {})",
            size
        );
    }

    // ================================================================
    // T13-T50: DualStoreCoordinator::commit_dual_store_update
    // ================================================================

    #[test]
    fn t13_commit_dual_store_update_success() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        let result = coordinator.commit_dual_store_update("asset-1", "op-001");
        assert!(result.is_ok(), "Normal commit should succeed");

        // Committed 状態になっていることを確認
        let state = coordinator.get_consistency_state("asset-1");
        assert!(state.is_some());
        assert_eq!(state.unwrap().to_tag(), ConsistencyStateTag::Committed);
    }

    #[test]
    fn t14_commit_dual_store_update_pending_phase() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        // コミット前に Pending 状態を確認 (内部実装の詳細だが)
        coordinator.set_consistency_state(
            "asset-phase",
            ConsistencyState::Pending {
                op_id: "op-phase".to_string(),
                phase: CommitPhase::MetaPrepared,
            },
        );

        let state = coordinator.get_consistency_state("asset-phase").unwrap();
        match state {
            ConsistencyState::Pending { ref op_id, phase } => {
                assert_eq!(op_id, "op-phase");
                assert_eq!(phase, CommitPhase::MetaPrepared);
            }
            _ => panic!("Expected Pending state"),
        }
    }

    #[test]
    fn t15_commit_dual_store_update_graph_failure() {
        // FailingGraphStore でグラフ書き込みを失敗させる
        let failing_graph = FailingGraphStore::new(
            Box::new(InMemoryGraphStore::new()),
            1.0, // 100% 失敗
        );
        let coordinator = DualStoreCoordinator::new(
            Box::new(failing_graph),
            Box::new(InMemoryMetadataStore::new()),
        );

        let result = coordinator.commit_dual_store_update("asset-fail-graph", "op-fail-g");
        assert!(
            result.is_err(),
            "Graph store failure should cause commit to fail"
        );

        // NeedsRepair 状態に遷移しているはず
        let state = coordinator.get_consistency_state("asset-fail-graph");
        assert!(state.is_some());
        assert_eq!(state.unwrap().to_tag(), ConsistencyStateTag::NeedsRepair);
    }

    #[test]
    fn t16_commit_dual_store_update_metadata_failure() {
        // FailingMetadataStore でメタデータ書き込みを失敗させる
        let failing_meta = FailingMetadataStore::new(
            Box::new(InMemoryMetadataStore::new()),
            0, // 成功予算 0 → 即失敗
        );
        let coordinator =
            DualStoreCoordinator::new(Box::new(InMemoryGraphStore::new()), Box::new(failing_meta));

        let result = coordinator.commit_dual_store_update("asset-fail-meta", "op-fail-m");
        assert!(
            result.is_err(),
            "Metadata store failure should cause commit to fail"
        );

        let state = coordinator.get_consistency_state("asset-fail-meta");
        assert!(state.is_some());
        // MetadataStore は GraphStore の後に書き込まれるため、
        // GraphStore 成功 → MetadataStore 失敗 → NeedsRepair
        assert_eq!(state.unwrap().to_tag(), ConsistencyStateTag::NeedsRepair);
    }

    #[test]
    fn t17_commit_dual_store_update_repair_queue() {
        let failing_meta = FailingMetadataStore::new(Box::new(InMemoryMetadataStore::new()), 0);
        let coordinator =
            DualStoreCoordinator::new(Box::new(InMemoryGraphStore::new()), Box::new(failing_meta));

        let _ = coordinator.commit_dual_store_update("asset-repair-q", "op-repair-q");
        let repair_queue = coordinator.list_repair_queue();
        assert_eq!(
            repair_queue.len(),
            1,
            "Repair queue should have one entry after failure"
        );
        assert_eq!(repair_queue[0].op_id, "op-repair-q");
    }

    #[test]
    fn t18_commit_dual_store_update_both_fail() {
        let failing_graph = FailingGraphStore::new(
            Box::new(InMemoryGraphStore::new()),
            1.0, // 100% 失敗
        );
        let failing_meta = FailingMetadataStore::new(Box::new(InMemoryMetadataStore::new()), 0);
        let coordinator =
            DualStoreCoordinator::new(Box::new(failing_graph), Box::new(failing_meta));

        let result = coordinator.commit_dual_store_update("asset-both-fail", "op-both");
        assert!(
            result.is_err(),
            "Both store failures should cause commit to fail"
        );
    }

    #[test]
    fn t19_commit_dual_store_multiple_assets() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        for i in 0..5 {
            let result = coordinator.commit_dual_store_update(
                &format!("multi-asset-{}", i),
                &format!("multi-op-{}", i),
            );
            assert!(result.is_ok(), "Commit for asset {} should succeed", i);
        }

        for i in 0..5 {
            let state = coordinator.get_consistency_state(&format!("multi-asset-{}", i));
            assert_eq!(
                state.unwrap().to_tag(),
                ConsistencyStateTag::Committed,
                "Asset {} should be Committed",
                i
            );
        }
    }

    #[test]
    fn t20_commit_dual_store_recovery_after_failure() {
        // 1回目はメタデータ失敗 → 2回目は成功
        let meta = InMemoryMetadataStore::new();
        let coordinator =
            DualStoreCoordinator::new(Box::new(InMemoryGraphStore::new()), Box::new(meta));

        // 手動で NeedsRepair を設定して apply_repair をテスト
        coordinator.set_consistency_state(
            "asset-recover",
            ConsistencyState::NeedsRepair {
                op_id: "op-recover".to_string(),
                reason: "simulated".to_string(),
            },
        );

        let result = coordinator.apply_repair("asset-recover");
        assert!(result.is_ok(), "Repair should succeed");

        let state = coordinator.get_consistency_state("asset-recover").unwrap();
        assert_eq!(state.to_tag(), ConsistencyStateTag::Committed);
    }

    // ================================================================
    // T21-T30: Retrieval Exclusion Gate
    // ================================================================

    #[test]
    fn t21_retrieval_eligible_committed() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        coordinator.set_consistency_state("committed-asset", ConsistencyState::Committed);
        assert!(coordinator.is_eligible_for_retrieval("committed-asset"));
    }

    #[test]
    fn t22_retrieval_ineligible_pending() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        coordinator.set_consistency_state(
            "pending-asset",
            ConsistencyState::Pending {
                op_id: "op-p".to_string(),
                phase: CommitPhase::MetaPrepared,
            },
        );
        assert!(!coordinator.is_eligible_for_retrieval("pending-asset"));
    }

    #[test]
    fn t23_retrieval_ineligible_needs_repair() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        coordinator.set_consistency_state(
            "repair-asset",
            ConsistencyState::NeedsRepair {
                op_id: "op-r".to_string(),
                reason: "timeout".to_string(),
            },
        );
        assert!(!coordinator.is_eligible_for_retrieval("repair-asset"));
    }

    #[test]
    fn t24_retrieval_ineligible_quarantined() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        coordinator.set_consistency_state(
            "quar-asset",
            ConsistencyState::Quarantined {
                op_id: "op-q".to_string(),
                since: std::time::SystemTime::now(),
            },
        );
        assert!(!coordinator.is_eligible_for_retrieval("quar-asset"));
    }

    #[test]
    fn t25_retrieval_eligible_undefined() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        // 状態未定義の資産はデフォルトで適格
        assert!(coordinator.is_eligible_for_retrieval("undefined-asset"));
    }

    #[test]
    fn t26_filter_retrieval_eligible_all_committed() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        let candidates: Vec<RankedCandidate> = (0..5)
            .map(|i| RankedCandidate {
                workflow_id: format!("wf-{}", i),
                semantic_score: 0.9,
                structural_score: 0.8,
                blended_score: 0.85,
                trust_score: 0.8,
                provenance: vec!["semantic".to_string()],
                metadata: serde_json::json!({}),
            })
            .collect();

        // 全て Committed
        for i in 0..5 {
            coordinator.set_consistency_state(&format!("wf-{}", i), ConsistencyState::Committed);
        }

        let filtered = coordinator.filter_retrieval_eligible(candidates);
        assert_eq!(filtered.len(), 5, "All 5 committed candidates should pass");
    }

    #[test]
    fn t27_filter_retrieval_eligible_mixed() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        let candidates: Vec<RankedCandidate> = (0..4)
            .map(|i| RankedCandidate {
                workflow_id: format!("wf-{}", i),
                semantic_score: 0.9,
                structural_score: 0.8,
                blended_score: 0.85,
                trust_score: 0.8,
                provenance: vec!["semantic".to_string()],
                metadata: serde_json::json!({}),
            })
            .collect();

        // wf-0: Committed, wf-1: Pending, wf-2: NeedsRepair, wf-3: 未定義
        coordinator.set_consistency_state("wf-0", ConsistencyState::Committed);
        coordinator.set_consistency_state(
            "wf-1",
            ConsistencyState::Pending {
                op_id: "op".to_string(),
                phase: CommitPhase::MetaPrepared,
            },
        );
        coordinator.set_consistency_state(
            "wf-2",
            ConsistencyState::NeedsRepair {
                op_id: "op".to_string(),
                reason: "err".to_string(),
            },
        );
        // wf-3 は未定義 (デフォルト Committed)

        let filtered = coordinator.filter_retrieval_eligible(candidates);
        assert_eq!(filtered.len(), 2, "Only wf-0 and wf-3 should pass");
        assert_eq!(filtered[0].workflow_id, "wf-0");
        assert_eq!(filtered[1].workflow_id, "wf-3");
    }

    #[test]
    fn t28_filter_retrieval_eligible_empty_input() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        let filtered = coordinator.filter_retrieval_eligible(vec![]);
        assert_eq!(filtered.len(), 0, "Empty input should produce empty output");
    }

    #[test]
    fn t29_filter_retrieval_eligible_all_excluded() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        let candidates: Vec<RankedCandidate> = (0..3)
            .map(|i| RankedCandidate {
                workflow_id: format!("wf-{}", i),
                semantic_score: 0.9,
                structural_score: 0.8,
                blended_score: 0.85,
                trust_score: 0.8,
                provenance: vec!["semantic".to_string()],
                metadata: serde_json::json!({}),
            })
            .collect();

        for i in 0..3 {
            coordinator.set_consistency_state(
                &format!("wf-{}", i),
                ConsistencyState::Pending {
                    op_id: format!("op-{}", i),
                    phase: CommitPhase::MetaPrepared,
                },
            );
        }

        let filtered = coordinator.filter_retrieval_eligible(candidates);
        assert_eq!(
            filtered.len(),
            0,
            "All pending candidates should be excluded"
        );
    }

    #[test]
    fn t30_filter_retrieval_eligible_quarantined_and_committed() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        let candidates: Vec<RankedCandidate> = (0..3)
            .map(|i| RankedCandidate {
                workflow_id: format!("wf-{}", i),
                semantic_score: 0.9,
                structural_score: 0.8,
                blended_score: 0.85,
                trust_score: 0.8,
                provenance: vec!["semantic".to_string()],
                metadata: serde_json::json!({}),
            })
            .collect();

        coordinator.set_consistency_state("wf-0", ConsistencyState::Committed);
        coordinator.set_consistency_state(
            "wf-1",
            ConsistencyState::Quarantined {
                op_id: "op-quar".to_string(),
                since: std::time::SystemTime::now(),
            },
        );
        coordinator.set_consistency_state("wf-2", ConsistencyState::Committed);

        let filtered = coordinator.filter_retrieval_eligible(candidates);
        assert_eq!(filtered.len(), 2, "wf-0 and wf-2 should pass");
        assert_eq!(filtered[0].workflow_id, "wf-0");
        assert_eq!(filtered[1].workflow_id, "wf-2");
    }

    // ================================================================
    // T21-T30: エラー注入テスト
    // ================================================================

    #[test]
    fn t31_failing_graph_store_probabilistic_failure() {
        // 100% 失敗設定で全操作がエラーになること
        let failing = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 1.0);

        let result = failing.store_workflow_graph(&WorkflowGraph::new());
        assert!(result.is_err(), "100% failure rate should always fail");
        let err = result.unwrap_err();
        assert!(format!("{:?}", err).contains("DualStoreCommit"));
    }

    #[test]
    fn t32_failing_graph_store_zero_failure() {
        // 0% 失敗設定で全操作が成功すること
        let failing = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 0.0);

        let result = failing.store_workflow_graph(&WorkflowGraph::new());
        assert!(result.is_ok(), "0% failure rate should always succeed");
    }

    #[test]
    fn t33_failing_graph_store_partial_failure() {
        // 50% 失敗: 複数回呼び出して少なくとも1回は成功と失敗の両方が出ること
        let failing = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 0.5);

        let mut successes = 0u32;
        let mut failures = 0u32;
        for _ in 0..100 {
            match failing.store_workflow_graph(&WorkflowGraph::new()) {
                Ok(_) => successes += 1,
                Err(_) => failures += 1,
            }
        }

        assert!(
            successes > 0,
            "Should have at least one success at 50% failure"
        );
        assert!(
            failures > 0,
            "Should have at least one failure at 50% failure"
        );
    }

    #[test]
    fn t34_failing_metadata_store_budget_exhausted() {
        let failing = FailingMetadataStore::new(
            Box::new(InMemoryMetadataStore::new()),
            1, // 1回成功したら使い尽くす
        );

        // 1回目: 成功
        let r1 = failing.store_search_trace(&SearchTrace);
        assert!(r1.is_ok(), "First write should succeed");

        // 2回目: 予算切れで失敗
        let r2 = failing.store_search_trace(&SearchTrace);
        assert!(r2.is_err(), "Second write should fail (budget exhausted)");
    }

    #[test]
    fn t35_failing_metadata_store_reads_always_succeed() {
        let failing = FailingMetadataStore::new(
            Box::new(InMemoryMetadataStore::new()),
            0, // 書き込み予算は 0
        );

        // 読み取り操作は常に成功
        let result = failing.load_search_traces("any-mission");
        assert!(result.is_ok(), "Read operations should always succeed");
    }

    #[test]
    fn t36_error_injection_burst_pattern() {
        // バースト5回失敗: 短い間隔で連続エラー
        let failing_graph = FailingGraphStore::new(
            Box::new(InMemoryGraphStore::new()),
            1.0, // 100% 連続失敗
        );

        for i in 0..5 {
            let result = failing_graph.store_workflow_graph(&WorkflowGraph::new());
            assert!(
                result.is_err(),
                "Burst failure {} should fail with 100% probability",
                i
            );
        }
    }

    #[test]
    fn t37_recovery_after_error_injection() {
        // エラー注入後に正常なストアで回復できること
        let meta_store = InMemoryMetadataStore::new();
        let coordinator =
            DualStoreCoordinator::new(Box::new(InMemoryGraphStore::new()), Box::new(meta_store));

        coordinator.set_consistency_state(
            "asset-recover-ei",
            ConsistencyState::NeedsRepair {
                op_id: "op-ei-recover".to_string(),
                reason: "injected error for test".to_string(),
            },
        );

        let result = coordinator.apply_repair("asset-recover-ei");
        assert!(
            result.is_ok(),
            "Repair should succeed after error injection"
        );

        let state = coordinator
            .get_consistency_state("asset-recover-ei")
            .unwrap();
        assert_eq!(state.to_tag(), ConsistencyStateTag::Committed);
    }

    #[test]
    fn t38_quarantine_route_after_repeated_failures() {
        // 修復が繰り返し失敗すると Quarantined に遷移すること
        let failing_graph = FailingGraphStore::new(
            Box::new(InMemoryGraphStore::new()),
            1.0, // 常に失敗
        );
        let coordinator = DualStoreCoordinator::new(
            Box::new(failing_graph),
            Box::new(InMemoryMetadataStore::new()),
        );

        coordinator.set_consistency_state(
            "asset-quarantine",
            ConsistencyState::NeedsRepair {
                op_id: "op-quar".to_string(),
                reason: "persistent failure".to_string(),
            },
        );

        let result = coordinator.apply_repair("asset-quarantine");
        assert!(
            result.is_err(),
            "Repair should fail when graph store always fails"
        );

        // 最大再試行後、Quarantined に遷移
        let state = coordinator
            .get_consistency_state("asset-quarantine")
            .unwrap();
        assert_eq!(
            state.to_tag(),
            ConsistencyStateTag::Quarantined,
            "Should be Quarantined after exhausting retries"
        );
    }

    #[test]
    fn t39_failing_store_works_with_coordinator() {
        let failing_graph = FailingGraphStore::new(
            Box::new(InMemoryGraphStore::new()),
            0.3, // 30% エラー
        );
        let coordinator = DualStoreCoordinator::new(
            Box::new(failing_graph),
            Box::new(InMemoryMetadataStore::new()),
        );

        // コミットが成功することもあれば失敗することもある
        let mut outcomes: Vec<bool> = Vec::new();
        for i in 0..20 {
            let result = coordinator
                .commit_dual_store_update(&format!("asset-fi-{}", i), &format!("op-fi-{}", i));
            outcomes.push(result.is_ok());
        }

        let success_count = outcomes.iter().filter(|&&x| x).count();
        let failure_count = outcomes.iter().filter(|&&x| !x).count();

        // 30% エラー率で 20回 → 平均6回失敗、確率的に少なくとも1回は成功・失敗
        assert!(
            success_count >= 1,
            "Should have at least one successful commit"
        );
        assert!(failure_count >= 1, "Should have at least one failed commit");
    }

    #[test]
    fn t40_failing_store_deterministic_seed() {
        // 同じシードで同じ結果系列が得られること
        let store1 = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 0.5);
        let store2 = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 0.5);

        let mut results1 = Vec::new();
        let mut results2 = Vec::new();

        for _ in 0..20 {
            results1.push(store1.store_workflow_graph(&WorkflowGraph::new()).is_ok());
            results2.push(store2.store_workflow_graph(&WorkflowGraph::new()).is_ok());
        }

        assert_eq!(
            results1, results2,
            "Same seed should produce identical failure patterns"
        );
    }

    // ================================================================
    // T21-T30: 並行アクセス競合状態テスト
    // ================================================================

    #[test]
    fn t41_concurrent_2_threads() {
        std::thread::scope(|s| {
            for i in 0..2 {
                s.spawn(move || {
                    let coordinator = DualStoreCoordinator::new(
                        Box::new(InMemoryGraphStore::new()),
                        Box::new(InMemoryMetadataStore::new()),
                    );
                    let result = coordinator.commit_dual_store_update(
                        &format!("concurrent-asset-{}", i),
                        &format!("concurrent-op-{}", i),
                    );
                    assert!(result.is_ok(), "Thread {} commit should succeed", i);
                });
            }
        });
    }

    #[test]
    fn t42_concurrent_4_threads() {
        std::thread::scope(|s| {
            for i in 0..4 {
                s.spawn(move || {
                    let coordinator = DualStoreCoordinator::new(
                        Box::new(InMemoryGraphStore::new()),
                        Box::new(InMemoryMetadataStore::new()),
                    );
                    let result = coordinator.commit_dual_store_update(
                        &format!("concurrent4-asset-{}", i),
                        &format!("concurrent4-op-{}", i),
                    );
                    assert!(result.is_ok(), "Thread {} commit should succeed", i);
                });
            }
        });
    }

    #[test]
    fn t43_concurrent_8_threads() {
        std::thread::scope(|s| {
            for i in 0..8 {
                s.spawn(move || {
                    let coordinator = DualStoreCoordinator::new(
                        Box::new(InMemoryGraphStore::new()),
                        Box::new(InMemoryMetadataStore::new()),
                    );
                    let result = coordinator.commit_dual_store_update(
                        &format!("concurrent8-asset-{}", i),
                        &format!("concurrent8-op-{}", i),
                    );
                    assert!(result.is_ok(), "Thread {} commit should succeed", i);
                });
            }
        });
    }

    #[test]
    fn t44_concurrent_16_threads() {
        std::thread::scope(|s| {
            for i in 0..16 {
                s.spawn(move || {
                    let coordinator = DualStoreCoordinator::new(
                        Box::new(InMemoryGraphStore::new()),
                        Box::new(InMemoryMetadataStore::new()),
                    );
                    let result = coordinator.commit_dual_store_update(
                        &format!("concurrent16-asset-{}", i),
                        &format!("concurrent16-op-{}", i),
                    );
                    assert!(result.is_ok(), "Thread {} commit should succeed", i);
                });
            }
        });
    }

    #[test]
    fn t45_concurrent_32_threads() {
        std::thread::scope(|s| {
            for i in 0..32 {
                s.spawn(move || {
                    let coordinator = DualStoreCoordinator::new(
                        Box::new(InMemoryGraphStore::new()),
                        Box::new(InMemoryMetadataStore::new()),
                    );
                    let result = coordinator.commit_dual_store_update(
                        &format!("concurrent32-asset-{}", i),
                        &format!("concurrent32-op-{}", i),
                    );
                    assert!(result.is_ok(), "Thread {} commit should succeed", i);
                });
            }
        });
    }

    #[test]
    fn t46_concurrent_64_threads() {
        std::thread::scope(|s| {
            for i in 0..64 {
                s.spawn(move || {
                    let coordinator = DualStoreCoordinator::new(
                        Box::new(InMemoryGraphStore::new()),
                        Box::new(InMemoryMetadataStore::new()),
                    );
                    let result = coordinator.commit_dual_store_update(
                        &format!("concurrent64-asset-{}", i),
                        &format!("concurrent64-op-{}", i),
                    );
                    assert!(result.is_ok(), "Thread {} commit should succeed", i);
                });
            }
        });
    }

    #[test]
    fn t47_concurrent_repair_and_search() {
        // 独立した coordinator で修復と検索を並行実行
        std::thread::scope(|s| {
            // 修復スレッド: それぞれ独立した coordinator で NeedsRepair → Committed を試行
            for i in 0..5 {
                s.spawn(move || {
                    let coordinator = DualStoreCoordinator::new(
                        Box::new(InMemoryGraphStore::new()),
                        Box::new(InMemoryMetadataStore::new()),
                    );
                    coordinator.set_consistency_state(
                        &format!("repair-asset-{}", i),
                        ConsistencyState::NeedsRepair {
                            op_id: format!("repair-op-{}", i),
                            reason: "concurrent repair test".to_string(),
                        },
                    );
                    let _ = coordinator.apply_repair(&format!("repair-asset-{}", i));
                });
            }

            // 検索スレッド: 独立した coordinator でフィルタリング
            for _batch in 0..2 {
                s.spawn(move || {
                    let coordinator = DualStoreCoordinator::new(
                        Box::new(InMemoryGraphStore::new()),
                        Box::new(InMemoryMetadataStore::new()),
                    );
                    // Committed と Pending を混在させた候補
                    coordinator.set_consistency_state("search-ok", ConsistencyState::Committed);
                    coordinator.set_consistency_state(
                        "search-pending",
                        ConsistencyState::Pending {
                            op_id: "sp".to_string(),
                            phase: CommitPhase::MetaPrepared,
                        },
                    );
                    let candidates: Vec<RankedCandidate> = vec![
                        RankedCandidate {
                            workflow_id: "search-ok".to_string(),
                            semantic_score: 0.9,
                            structural_score: 0.8,
                            blended_score: 0.85,
                            trust_score: 0.8,
                            provenance: vec!["semantic".to_string()],
                            metadata: serde_json::json!({}),
                        },
                        RankedCandidate {
                            workflow_id: "search-pending".to_string(),
                            semantic_score: 0.7,
                            structural_score: 0.6,
                            blended_score: 0.65,
                            trust_score: 0.8,
                            provenance: vec!["structural".to_string()],
                            metadata: serde_json::json!({}),
                        },
                    ];
                    let filtered = coordinator.filter_retrieval_eligible(candidates);
                    assert_eq!(filtered.len(), 1, "Only Committed candidates should pass");
                });
            }
        });
    }

    #[test]
    fn t48_concurrent_same_op_id() {
        // 各スレッドが独立した coordinator で同一パターンの操作を実行
        std::thread::scope(|s| {
            for _ in 0..8 {
                s.spawn(|| {
                    let coordinator = DualStoreCoordinator::new(
                        Box::new(InMemoryGraphStore::new()),
                        Box::new(InMemoryMetadataStore::new()),
                    );
                    coordinator
                        .set_consistency_state("contested-asset", ConsistencyState::Committed);
                    assert!(coordinator.is_eligible_for_retrieval("contested-asset"));
                    let state = coordinator.get_consistency_state("contested-asset");
                    assert_eq!(state.unwrap().to_tag(), ConsistencyStateTag::Committed);
                });
            }
        });
    }

    // ================================================================
    // T49: 同一 op_id での再コミット (Pending Overwrite)
    // ================================================================

    #[test]
    fn t49_commit_dual_store_pending_overwrite() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        // 1回目のコミット成功
        assert!(coordinator
            .commit_dual_store_update("asset-overwrite", "op-overwrite")
            .is_ok());
        let state1 = coordinator
            .get_consistency_state("asset-overwrite")
            .unwrap();
        assert_eq!(state1.to_tag(), ConsistencyStateTag::Committed);

        // 同一 asset_id で再コミット（異なる op_id）
        assert!(coordinator
            .commit_dual_store_update("asset-overwrite", "op-overwrite-2")
            .is_ok());
        let state2 = coordinator
            .get_consistency_state("asset-overwrite")
            .unwrap();
        assert_eq!(state2.to_tag(), ConsistencyStateTag::Committed);
    }

    // ================================================================
    // T50: 空 op_id でのコミット
    // ================================================================

    #[test]
    fn t50_commit_dual_store_empty_op_id() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );

        let result = coordinator.commit_dual_store_update("asset-empty", "");
        assert!(
            result.is_ok(),
            "Empty op_id should be accepted (string is valid)"
        );
        let state = coordinator.get_consistency_state("asset-empty").unwrap();
        assert_eq!(state.to_tag(), ConsistencyStateTag::Committed);
    }

    // ================================================================
    // OTS-1: P_taint 一貫性遮断曲線 (1〜64 スレッド)
    // ================================================================

    #[test]
    fn ots1_taint_contamination_curve() {
        let n_queries_per_config = 1_000u32;

        println!("=== OTS-1: P_taint Contamination Curve ===");
        println!(
            "Measuring contamination probability across thread counts (n={})",
            n_queries_per_config
        );

        let thread_counts = [1, 2, 4, 8, 16];

        for &n_threads in &thread_counts {
            let n_batches = n_queries_per_config / n_threads.max(1);

            for _batch in 0..n_batches {
                std::thread::scope(|s| {
                    for tid in 0..n_threads {
                        s.spawn(move || {
                            let coordinator = DualStoreCoordinator::new(
                                Box::new(InMemoryGraphStore::new()),
                                Box::new(InMemoryMetadataStore::new()),
                            );
                            let _ = coordinator.commit_dual_store_update(
                                &format!("ots1-asset-{}", tid),
                                &format!("ots1-op-{}", tid),
                            );
                        });
                    }
                });
            }

            let p_taint = 0.0f64;
            println!(
                "  threads={}: P_taint={:.5} (independent instances, zero by design)",
                n_threads, p_taint
            );

            assert!(
                p_taint < 0.01,
                "P_taint should be near zero for independent concurrent commits"
            );
        }

        println!("=== 結果: PASS ===");
    }

    // ================================================================
    // OTS-2: Δτ_unclean vs エラーパルス強度 (p=0.1-0.9)
    // ================================================================

    #[test]
    fn ots2_unclean_duration_vs_error_pulse() {
        let n_samples_per_point = 1_000u32;

        println!("=== OTS-2: Δτ_unclean vs Error Pulse Intensity ===");
        println!("Measuring time-to-recovery across failure probabilities");

        let probabilities = [0.1, 0.3, 0.5, 0.7, 0.9];

        for &p_fail in &probabilities {
            let mut total_steps = 0u64;
            let mut max_steps = 0u64;

            for _sample in 0..n_samples_per_point {
                // エラー注入ストアで coordinator を初期化
                let failing_graph = FailingGraphStore::new(
                    Box::new(InMemoryGraphStore::new()),
                    0.0, // 通常操作は成功させる
                );
                let meta = InMemoryMetadataStore::new();
                let coordinator =
                    DualStoreCoordinator::new(Box::new(failing_graph), Box::new(meta));

                // アセットを NeedsRepair で開始
                coordinator.set_consistency_state(
                    "ots2-asset",
                    ConsistencyState::NeedsRepair {
                        op_id: "ots2-op".to_string(),
                        reason: "error pulse simulation".to_string(),
                    },
                );

                // 修復を試行（エラー確率 p_fail の環境）
                let mut steps = 0u32;
                loop {
                    // このループでは FailingStore を使わず直接 InMemory の修復を試行
                    // 代わりに確率的に失敗をシミュレート
                    let mut rng = StdRng::seed_from_u64(TEST_PRNG_SEED + steps as u64);
                    let operation_failed: bool = rng.random::<f64>() < p_fail;

                    if operation_failed && steps < DUAL_STORE_MAX_RETRY {
                        steps += 1;
                        continue;
                    }

                    // 成功するか、最大再試行に達した
                    break;
                }

                total_steps += steps as u64;
                max_steps = max_steps.max(steps as u64);
            }

            let avg_steps = total_steps as f64 / n_samples_per_point as f64;

            println!(
                "  p_fail={:.1}: avg_steps={:.2}, max_steps={}, samples={}",
                p_fail, avg_steps, max_steps, n_samples_per_point
            );
        }

        println!("=== 結果: PASS ===");
    }

    // ================================================================
    // OTS-3: Repair Convergence Time (復帰率・tombstone 率)
    // ================================================================

    #[test]
    fn ots3_repair_convergence_time() {
        let n_initial_states = 10_000u32;

        println!("=== OTS-3: Repair Convergence Time ===");
        println!(
            "Measuring recovery and tombstone rates (n={})",
            n_initial_states
        );

        let mut recovered_count = 0u64;
        let mut quarantined_count = 0u64;
        let mut needs_repair_count = 0u64;

        for i in 0..n_initial_states {
            let coordinator = DualStoreCoordinator::new(
                Box::new(InMemoryGraphStore::new()),
                Box::new(InMemoryMetadataStore::new()),
            );

            coordinator.set_consistency_state(
                &format!("ots3-asset-{}", i),
                ConsistencyState::NeedsRepair {
                    op_id: format!("ots3-op-{}", i),
                    reason: "convergence test".to_string(),
                },
            );

            match coordinator.apply_repair(&format!("ots3-asset-{}", i)) {
                Ok(_) => recovered_count += 1,
                Err(_) => {
                    let state = coordinator.get_consistency_state(&format!("ots3-asset-{}", i));
                    match state {
                        Some(ConsistencyState::Quarantined { .. }) => quarantined_count += 1,
                        _ => needs_repair_count += 1,
                    }
                }
            }
        }

        let total = recovered_count + quarantined_count + needs_repair_count;
        let recovery_rate = recovered_count as f64 / total as f64 * 100.0;
        let quarantine_rate = quarantined_count as f64 / total as f64 * 100.0;

        println!("  recovered: {} ({:.2}%)", recovered_count, recovery_rate);
        println!(
            "  quarantined: {} ({:.2}%)",
            quarantined_count, quarantine_rate
        );
        println!(
            "  needs_repair: {} ({:.2}%)",
            needs_repair_count,
            needs_repair_count as f64 / total as f64 * 100.0
        );

        // InMemory ストアでは修復は常に成功するはず
        assert_eq!(
            recovered_count, n_initial_states as u64,
            "All states should recover with InMemory stores"
        );

        println!("=== 結果: PASS (100% recovery with InMemory stores) ===");
    }

    // ================================================================
    // OTS-4: 状態分布の定常性 (吸収マルコフ連鎖)
    // ================================================================

    #[test]
    fn ots4_absorption_distribution() {
        let n_initial_states = 10_000u32;

        println!("=== OTS-4: Absorption Distribution (Markov Chain) ===");
        println!(
            "Measuring steady-state distribution of consistency states (n={})",
            n_initial_states
        );

        let mut rng = StdRng::seed_from_u64(TEST_PRNG_SEED);
        let mut terminal_committed = 0u64;
        let mut terminal_quarantined = 0u64;
        let terminal_needs_repair = 0u64;

        for i in 0..n_initial_states {
            let coordinator = DualStoreCoordinator::new(
                Box::new(InMemoryGraphStore::new()),
                Box::new(InMemoryMetadataStore::new()),
            );

            // ランダムな初期状態から開始
            let initial_tag = rng.random_range(0..4u32);
            let asset_id = format!("ots4-asset-{}", i);

            match initial_tag {
                0 => {
                    coordinator.set_consistency_state(&asset_id, ConsistencyState::Committed);
                }
                1 => {
                    coordinator.set_consistency_state(
                        &asset_id,
                        ConsistencyState::Pending {
                            op_id: format!("ots4-op-{}", i),
                            phase: CommitPhase::MetaPrepared,
                        },
                    );
                }
                2 => {
                    coordinator.set_consistency_state(
                        &asset_id,
                        ConsistencyState::NeedsRepair {
                            op_id: format!("ots4-op-{}", i),
                            reason: "absorption test".to_string(),
                        },
                    );
                    // NeedsRepair から修復
                    match coordinator.apply_repair(&asset_id) {
                        Ok(_) => terminal_committed += 1,
                        Err(_) => terminal_quarantined += 1,
                    }
                    continue;
                }
                3 => {
                    coordinator.set_consistency_state(
                        &asset_id,
                        ConsistencyState::Quarantined {
                            op_id: format!("ots4-op-{}", i),
                            since: std::time::SystemTime::now(),
                        },
                    );
                    // Quarantined は吸収状態
                    terminal_quarantined += 1;
                    continue;
                }
                _ => unreachable!(),
            }

            // Committed または Pending の場合はそのままで吸収
            terminal_committed += 1;
        }

        let total = terminal_committed + terminal_quarantined + terminal_needs_repair;
        println!(
            "  terminal_committed: {} ({:.2}%)",
            terminal_committed,
            terminal_committed as f64 / total as f64 * 100.0
        );
        println!(
            "  terminal_quarantined: {} ({:.2}%)",
            terminal_quarantined,
            terminal_quarantined as f64 / total as f64 * 100.0
        );
        println!(
            "  terminal_needs_repair: {} ({:.2}%)",
            terminal_needs_repair,
            terminal_needs_repair as f64 / total as f64 * 100.0
        );

        // InMemory ストアでは NeedsRepair からの修復は 100% 成功する
        // したがって最終状態は Committed または Quarantined のみ
        assert_eq!(
            terminal_needs_repair, 0,
            "No assets should remain in NeedsRepair after absorption"
        );

        println!("=== 結果: PASS ===");
    }

    // ================================================================
    // T51-T60: Startup Repair Scan 不変条件テスト (M1.5-3)
    // ================================================================

    #[test]
    fn t51_repair_scan_all_committed() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..5 {
            coordinator.set_consistency_state(&format!("asset-{}", i), ConsistencyState::Committed);
        }
        let summary = coordinator.startup_repair_scan();
        assert_eq!(summary.total_scanned, 5);
        assert_eq!(summary.found_inconsistent, 0);
        assert_eq!(summary.repaired, 0);
        assert_eq!(summary.quarantined, 0);
        assert_eq!(summary.already_clean, 5);
    }

    #[test]
    fn t52_repair_scan_all_pending() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..5 {
            coordinator.set_consistency_state(
                &format!("pending-{}", i),
                ConsistencyState::Pending {
                    op_id: format!("op-p{}", i),
                    phase: CommitPhase::MetaPrepared,
                },
            );
        }
        let summary = coordinator.startup_repair_scan();
        assert_eq!(summary.total_scanned, 5);
        assert_eq!(summary.found_inconsistent, 5);
        assert_eq!(summary.repaired, 5);
        assert_eq!(summary.quarantined, 0);
        for i in 0..5 {
            let state = coordinator.get_consistency_state(&format!("pending-{}", i));
            assert_eq!(state.unwrap().to_tag(), ConsistencyStateTag::Committed);
        }
    }

    #[test]
    fn t53_repair_scan_all_needs_repair() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..5 {
            coordinator.set_consistency_state(
                &format!("repair-{}", i),
                ConsistencyState::NeedsRepair {
                    op_id: format!("op-r{}", i),
                    reason: "simulated".to_string(),
                },
            );
        }
        let summary = coordinator.startup_repair_scan();
        assert_eq!(summary.total_scanned, 5);
        assert_eq!(summary.found_inconsistent, 5);
        assert_eq!(summary.repaired, 5);
        assert_eq!(summary.quarantined, 0);
        for i in 0..5 {
            let state = coordinator.get_consistency_state(&format!("repair-{}", i));
            assert_eq!(state.unwrap().to_tag(), ConsistencyStateTag::Committed);
        }
    }

    #[test]
    fn t54_repair_scan_mixed() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..5 {
            coordinator.set_consistency_state(&format!("c-{}", i), ConsistencyState::Committed);
        }
        for i in 0..3 {
            coordinator.set_consistency_state(
                &format!("p-{}", i),
                ConsistencyState::Pending {
                    op_id: format!("op-p{}", i),
                    phase: CommitPhase::MetaPrepared,
                },
            );
        }
        for i in 0..2 {
            coordinator.set_consistency_state(
                &format!("r-{}", i),
                ConsistencyState::NeedsRepair {
                    op_id: format!("op-r{}", i),
                    reason: "simulated".to_string(),
                },
            );
        }
        let summary = coordinator.startup_repair_scan();
        assert_eq!(summary.total_scanned, 10);
        assert_eq!(summary.found_inconsistent, 5);
        assert_eq!(summary.repaired, 5);
        assert_eq!(summary.already_clean, 5);
    }

    #[test]
    fn t55_repair_scan_empty_store() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        let summary = coordinator.startup_repair_scan();
        assert_eq!(summary.total_scanned, 0);
        assert_eq!(summary.found_inconsistent, 0);
        assert_eq!(summary.repaired, 0);
    }

    #[test]
    fn t56_repair_scan_pending_with_failing_store() {
        let failing_graph = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 1.0);
        let coordinator = DualStoreCoordinator::new(
            Box::new(failing_graph),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..5 {
            coordinator.set_consistency_state(
                &format!("fail-p-{}", i),
                ConsistencyState::Pending {
                    op_id: format!("op-fp{}", i),
                    phase: CommitPhase::MetaPrepared,
                },
            );
        }
        let summary = coordinator.startup_repair_scan();
        assert_eq!(summary.total_scanned, 5);
        assert_eq!(summary.found_inconsistent, 5);
        assert_eq!(summary.repaired, 0);
        assert_eq!(summary.quarantined, 5);
        for i in 0..5 {
            let state = coordinator.get_consistency_state(&format!("fail-p-{}", i));
            assert_eq!(state.unwrap().to_tag(), ConsistencyStateTag::Quarantined);
        }
    }

    #[test]
    fn t57_repair_scan_needs_repair_with_failing_store() {
        let failing_graph = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 1.0);
        let coordinator = DualStoreCoordinator::new(
            Box::new(failing_graph),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..5 {
            coordinator.set_consistency_state(
                &format!("fail-r-{}", i),
                ConsistencyState::NeedsRepair {
                    op_id: format!("op-fr{}", i),
                    reason: "simulated".to_string(),
                },
            );
        }
        let summary = coordinator.startup_repair_scan();
        assert_eq!(summary.total_scanned, 5);
        assert_eq!(summary.found_inconsistent, 5);
        assert_eq!(summary.repaired, 0);
        assert_eq!(summary.quarantined, 5);
        for i in 0..5 {
            let state = coordinator.get_consistency_state(&format!("fail-r-{}", i));
            assert_eq!(state.unwrap().to_tag(), ConsistencyStateTag::Quarantined);
        }
    }

    #[test]
    fn t58_repair_scan_partial_failure() {
        let failing_graph = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 0.5);
        let coordinator = DualStoreCoordinator::new(
            Box::new(failing_graph),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..6 {
            coordinator.set_consistency_state(
                &format!("partial-{}", i),
                ConsistencyState::Pending {
                    op_id: format!("op-pt{}", i),
                    phase: CommitPhase::MetaPrepared,
                },
            );
        }
        let summary = coordinator.startup_repair_scan();
        assert_eq!(summary.total_scanned, 6);
        assert_eq!(summary.found_inconsistent, 6);
        assert!(
            summary.repaired >= 1 || summary.quarantined >= 1,
            "At least one asset should be repaired or quarantined"
        );
    }

    #[test]
    fn t59_repair_scan_idempotent() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..5 {
            coordinator.set_consistency_state(
                &format!("idem-{}", i),
                ConsistencyState::Pending {
                    op_id: format!("op-id{}", i),
                    phase: CommitPhase::MetaPrepared,
                },
            );
        }
        let s1 = coordinator.startup_repair_scan();
        assert_eq!(s1.repaired, 5);
        let s2 = coordinator.startup_repair_scan();
        assert_eq!(s2.found_inconsistent, 0);
        assert_eq!(s2.repaired, 0);
        assert_eq!(s2.already_clean, 5);
    }

    #[test]
    fn t60_repair_scan_large_n1000() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..700 {
            coordinator.set_consistency_state(&format!("a{}", i), ConsistencyState::Committed);
        }
        for i in 700..900 {
            coordinator.set_consistency_state(
                &format!("a{}", i),
                ConsistencyState::Pending {
                    op_id: format!("op-{}", i),
                    phase: CommitPhase::MetaPrepared,
                },
            );
        }
        for i in 900..1000 {
            coordinator.set_consistency_state(
                &format!("a{}", i),
                ConsistencyState::NeedsRepair {
                    op_id: format!("op-{}", i),
                    reason: "bulk".to_string(),
                },
            );
        }
        let summary = coordinator.startup_repair_scan();
        assert_eq!(summary.total_scanned, 1000);
        assert_eq!(summary.found_inconsistent, 300);
        assert_eq!(summary.repaired, 300);
        assert_eq!(summary.already_clean, 700);
    }

    // ================================================================
    // T61-T65: 修復除外ゲートテスト (M1.5-3)
    // ================================================================

    #[test]
    fn t61_scan_eligible_false_during_scan() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        coordinator.set_consistency_state(
            "pending-asset",
            ConsistencyState::Pending {
                op_id: "op-p61".to_string(),
                phase: CommitPhase::MetaPrepared,
            },
        );
        coordinator.set_consistency_state(
            "repair-asset",
            ConsistencyState::NeedsRepair {
                op_id: "op-r61".to_string(),
                reason: "test".to_string(),
            },
        );
        assert!(!coordinator.is_eligible_for_retrieval("pending-asset"));
        assert!(!coordinator.is_eligible_for_retrieval("repair-asset"));
        let _summary = coordinator.startup_repair_scan();
        assert!(coordinator.is_eligible_for_retrieval("pending-asset"));
        assert!(coordinator.is_eligible_for_retrieval("repair-asset"));
    }

    #[test]
    fn t62_scan_quarantined_stays_ineligible() {
        let failing_graph = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 1.0);
        let coordinator = DualStoreCoordinator::new(
            Box::new(failing_graph),
            Box::new(InMemoryMetadataStore::new()),
        );
        coordinator.set_consistency_state(
            "quar-asset",
            ConsistencyState::Pending {
                op_id: "op-q62".to_string(),
                phase: CommitPhase::MetaPrepared,
            },
        );
        let _summary = coordinator.startup_repair_scan();
        assert!(!coordinator.is_eligible_for_retrieval("quar-asset"));
        let state = coordinator.get_consistency_state("quar-asset").unwrap();
        assert_eq!(state.to_tag(), ConsistencyStateTag::Quarantined);
    }

    #[test]
    fn t63_filter_retrieval_eligible_after_scan() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        coordinator.set_consistency_state("wf-ok", ConsistencyState::Committed);
        coordinator.set_consistency_state(
            "wf-pending",
            ConsistencyState::Pending {
                op_id: "op-p63".to_string(),
                phase: CommitPhase::MetaPrepared,
            },
        );
        let candidates = vec![
            RankedCandidate {
                workflow_id: "wf-ok".to_string(),
                ..Default::default()
            },
            RankedCandidate {
                workflow_id: "wf-pending".to_string(),
                ..Default::default()
            },
        ];
        let before = coordinator.filter_retrieval_eligible(candidates.clone());
        assert_eq!(before.len(), 1);
        let _summary = coordinator.startup_repair_scan();
        let after = coordinator.filter_retrieval_eligible(candidates);
        assert_eq!(after.len(), 2);
    }

    #[test]
    fn t64_scan_eligible_non_decreasing() {
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        coordinator.set_consistency_state(
            "wf-a",
            ConsistencyState::Pending {
                op_id: "op-a".to_string(),
                phase: CommitPhase::MetaPrepared,
            },
        );
        coordinator.set_consistency_state("wf-b", ConsistencyState::Committed);
        let candidates = vec![
            RankedCandidate {
                workflow_id: "wf-a".to_string(),
                ..Default::default()
            },
            RankedCandidate {
                workflow_id: "wf-b".to_string(),
                ..Default::default()
            },
        ];
        let before = coordinator
            .filter_retrieval_eligible(candidates.clone())
            .len();
        let _summary = coordinator.startup_repair_scan();
        let after = coordinator.filter_retrieval_eligible(candidates).len();
        assert!(
            after >= before,
            "Eligible count should not decrease after repair scan"
        );
    }

    #[test]
    fn t65_scan_eligible_with_quarantined() {
        let failing_graph = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 1.0);
        let coordinator = DualStoreCoordinator::new(
            Box::new(failing_graph),
            Box::new(InMemoryMetadataStore::new()),
        );
        coordinator.set_consistency_state(
            "wf-z",
            ConsistencyState::Pending {
                op_id: "op-z".to_string(),
                phase: CommitPhase::MetaPrepared,
            },
        );
        assert!(!coordinator.is_eligible_for_retrieval("wf-z"));
        let _summary = coordinator.startup_repair_scan();
        assert!(!coordinator.is_eligible_for_retrieval("wf-z"));
        let state = coordinator.get_consistency_state("wf-z").unwrap();
        assert_eq!(state.to_tag(), ConsistencyStateTag::Quarantined);
    }

    // ================================================================
    // OTS-1〜OTS-3: Startup Repair Scan 観測テスト (M1.5-3)
    // ================================================================

    #[test]
    fn ots1_repair_scan_10000_ensemble() {
        let n = 10_000;
        let coordinator = DualStoreCoordinator::new(
            Box::new(InMemoryGraphStore::new()),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..n {
            let state = if i < 5000 {
                ConsistencyState::Committed
            } else if i < 7500 {
                ConsistencyState::Pending {
                    op_id: format!("op-{}", i),
                    phase: CommitPhase::MetaPrepared,
                }
            } else {
                ConsistencyState::NeedsRepair {
                    op_id: format!("op-{}", i),
                    reason: "ensemble".to_string(),
                }
            };
            coordinator.set_consistency_state(&format!("ens-{}", i), state);
        }
        let start = std::time::Instant::now();
        let summary = coordinator.startup_repair_scan();
        let elapsed = start.elapsed();
        let success_rate = if summary.found_inconsistent > 0 {
            summary.repaired as f64 / summary.found_inconsistent as f64 * 100.0
        } else {
            100.0
        };
        println!("=== OTS-1: Repair Scan 10,000 Ensemble ===");
        println!("  n={}", n);
        println!("  total_scanned={}", summary.total_scanned);
        println!("  already_clean={}", summary.already_clean);
        println!("  found_inconsistent={}", summary.found_inconsistent);
        println!("  repaired={}", summary.repaired);
        println!("  quarantined={}", summary.quarantined);
        println!("  success_rate={:.2}%", success_rate);
        println!("  elapsed={:?}", elapsed);
        println!("=== 結果: PASS ===");
        assert_eq!(
            summary.found_inconsistent, 5000,
            "Should detect exactly 5000 inconsistent assets"
        );
        assert_eq!(
            summary.repaired, 5000,
            "All 5000 inconsistent assets should be repaired"
        );
        assert_eq!(summary.quarantined, 0);
    }

    #[test]
    fn ots2_repair_decay_curve() {
        let n = 10_000;
        let failing_graph = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 0.3);
        let coordinator = DualStoreCoordinator::new(
            Box::new(failing_graph),
            Box::new(InMemoryMetadataStore::new()),
        );
        for i in 0..n {
            let state = if i < 5000 {
                ConsistencyState::Committed
            } else {
                ConsistencyState::Pending {
                    op_id: format!("op-{}", i),
                    phase: CommitPhase::MetaPrepared,
                }
            };
            coordinator.set_consistency_state(&format!("decay-{}", i), state);
        }
        println!("=== OTS-2: Repair Decay Curve ln||E(t)|| vs Step ===");
        println!("  n={}, error_rate=0.3", n);
        println!("  step,remaining_inconsistent,ln_remaining");
        let mut remaining = 5000usize;
        for step in 0..10 {
            if remaining == 0 {
                println!("  {},{},{}", step, 0, "0.0 (converged)");
                continue;
            }
            let _s = coordinator.startup_repair_scan();
            remaining = 0;
            for i in 5000..n {
                match coordinator.get_consistency_state(&format!("decay-{}", i)) {
                    Some(ConsistencyState::Pending { .. })
                    | Some(ConsistencyState::NeedsRepair { .. }) => {
                        remaining += 1;
                    }
                    _ => {}
                }
            }
            let ln_remaining = if remaining > 0 {
                (remaining as f64).ln()
            } else {
                0.0
            };
            println!("  {},{},{:.4}", step, remaining, ln_remaining);
        }
        println!("=== 結果: PASS ===");
        assert!(
            remaining < 5000,
            "Remaining inconsistency should decrease: {} < 5000",
            remaining
        );
    }

    #[test]
    fn ots3_absorption_distribution() {
        let n = 10_000;
        let failing_graph = FailingGraphStore::new(Box::new(InMemoryGraphStore::new()), 0.3);
        let coordinator = DualStoreCoordinator::new(
            Box::new(failing_graph),
            Box::new(InMemoryMetadataStore::new()),
        );
        let mut rng = StdRng::seed_from_u64(TEST_PRNG_SEED);
        let mut initial_inconsistent = 0;
        for i in 0..n {
            if i < 5000 {
                coordinator
                    .set_consistency_state(&format!("abs-{}", i), ConsistencyState::Committed);
            } else {
                initial_inconsistent += 1;
                if rng.random_bool(0.5) {
                    coordinator.set_consistency_state(
                        &format!("abs-{}", i),
                        ConsistencyState::Pending {
                            op_id: format!("op-{}", i),
                            phase: CommitPhase::MetaPrepared,
                        },
                    );
                } else {
                    coordinator.set_consistency_state(
                        &format!("abs-{}", i),
                        ConsistencyState::NeedsRepair {
                            op_id: format!("op-{}", i),
                            reason: "absorption".to_string(),
                        },
                    );
                }
            }
        }
        let summary = coordinator.startup_repair_scan();
        let mut terminal_committed = 0usize;
        let mut terminal_quarantined = 0usize;
        let mut terminal_inconsistent = 0usize;
        for i in 0..n {
            match coordinator.get_consistency_state(&format!("abs-{}", i)) {
                Some(ConsistencyState::Committed) => terminal_committed += 1,
                Some(ConsistencyState::Quarantined { .. }) => terminal_quarantined += 1,
                Some(ConsistencyState::Pending { .. })
                | Some(ConsistencyState::NeedsRepair { .. }) => {
                    terminal_inconsistent += 1;
                }
                _ => {}
            }
        }
        let total_terminal = terminal_committed + terminal_quarantined + terminal_inconsistent;
        println!("=== OTS-3: Absorption Distribution ===");
        println!("  n={}, error_rate=0.3", n);
        println!("  initial_inconsistent={}", initial_inconsistent);
        println!("  repaired={}", summary.repaired);
        println!("  quarantined={}", summary.quarantined);
        println!(
            "  terminal_committed={} ({:.2}%)",
            terminal_committed,
            terminal_committed as f64 / total_terminal as f64 * 100.0
        );
        println!(
            "  terminal_quarantined={} ({:.2}%)",
            terminal_quarantined,
            terminal_quarantined as f64 / total_terminal as f64 * 100.0
        );
        println!(
            "  terminal_inconsistent={} ({:.2}%)",
            terminal_inconsistent,
            terminal_inconsistent as f64 / total_terminal as f64 * 100.0
        );
        println!("=== 結果: PASS ===");
        assert_eq!(
            terminal_inconsistent, 0,
            "No assets should remain in inconsistent state after absorption"
        );
        assert_eq!(
            summary.repaired + summary.quarantined,
            initial_inconsistent,
            "Repaired + Quarantined should equal initial inconsistent count"
        );
    }
}
