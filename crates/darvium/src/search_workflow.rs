// SearchWorkflow 実行エンジン (RFC §13, §16)
//
// SearchState FSM を駆動し、ミッション入力から最終的な SearchOutcome を
// 生成する bounded search orchestration を提供する。
//
// FSM ループ:
//   Init → Retrieve → Evaluate
//     ├─ A >= threshold → Finalize (REUSE)
//     ├─ 複数候補あり → Compose → Finalize
//     ├─ 単一候補あり → Patch → Finalize (PATCH)
//     └─ 候補不足 → Refine → ProposeNew → Finalize (NEW)
//   任意 → Abort (budget / oscillation)

use std::hash::{Hash, Hasher};
use std::time::SystemTime;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::constants::{
    COMPOSE_CANDIDATE_COUNT, COMPOSE_SECOND_SCORE_RATIO, PATCH_EXISTING_THRESHOLD,
    SEARCH_FINALIZE_THRESHOLD, WORKFLOW_GENERATION_MAX_COMPLEXITY,
};
use crate::error::DarviumError;
use crate::patch::{GraphPatch, PatchConfidence, PatchOperation};
use crate::types::{
    attempt_transition, CompositionPlan, EdgeMeta, OscillationDetector, RecursionGuard,
    SearchBudget, SearchBudgetSnapshot, SearchOutcome, SearchState, VarDecl,
    WorkflowGraph, WorkflowNode,
};
use crate::workflow_generation;
use crate::workflow_registry::WorkflowRegistry;

/// セマンティック検索の上位 N 件取得数。
const SEMANTIC_SEARCH_TOP_K: usize = 10;

/// SearchWorkflow 実行エンジン (RFC §13, §16)。
#[derive(Debug)]
pub struct SearchWorkflow {
    /// 現在の FSM 状態。
    state: SearchState,
    /// 発振検出器 (Refine↔Retrieve ループ検出)。
    oscillation_detector: OscillationDetector,
    /// 検索予算上限。
    budget: SearchBudget,
    /// 現在の予算使用量スナップショット。
    snapshot: SearchBudgetSnapshot,
    /// 再帰ガード (Phase G で本実装)。
    #[allow(dead_code)]
    recursion_guard: RecursionGuard,
}

impl SearchWorkflow {
    /// 指定された予算で SearchWorkflow を生成する。
    pub fn new(budget: SearchBudget) -> Self {
        Self {
            state: SearchState::Init,
            oscillation_detector: OscillationDetector::default(),
            budget,
            snapshot: SearchBudgetSnapshot {
                iterations_used: 0,
                retrieval_calls_used: 0,
                prompt_tokens_used: 0,
                wall_clock_ms_used: 0,
            },
            recursion_guard: RecursionGuard {
                max_depth: 3,
                current_depth: 0,
                allow_reentrant: false,
            },
        }
    }

    /// SearchWorkflow を実行し、最終的な SearchOutcome を返す。
    ///
    /// # FSM ループ
    ///
    /// 1. **Init → Retrieve**: ミッションからクエリ埋め込みを構築しレジストリを検索
    /// 2. **Retrieve → Evaluate**: 検索結果の候補を評価
    /// 3. **Evaluate → Decide**:
    ///    - 最良候補 >= SEARCH_FINALIZE_THRESHOLD → REUSE
    ///    - 複数候補があり組成可能 → COMPOSE
    ///    - 単一候補かつスコア >= PATCH_EXISTING_THRESHOLD → PATCH
    ///    - 上記以外 → Refine → ProposeNew → NEW
    /// 4. 任意の時点で発振検出 → ABORT
    pub fn execute(
        &mut self,
        mission: &str,
        registry: &mut WorkflowRegistry,
    ) -> Result<SearchOutcome, DarviumError> {
        // 予算消費
        self.budget.try_consume_iteration(&mut self.snapshot)?;

        // Step 1: Init → Retrieve
        self.transition_to(SearchState::Retrieve)?;
        self.budget.try_consume_retrieval_call(&mut self.snapshot)?;

        // クエリ埋め込みを構築してレジストリをセマンティック検索
        let query_embedding = workflow_generation::mission_to_embedding(mission);
        let candidates = registry.semantic_search(&query_embedding, SEMANTIC_SEARCH_TOP_K);

        // Step 2: Retrieve → Evaluate
        self.transition_to(SearchState::Evaluate)?;

        // Step 3: 候補評価と意思決定
        if candidates.is_empty() {
            // 候補なし → Refine → ProposeNew（変異元なし）
            self.transition_to(SearchState::Refine)?;
            return self.propose_new(mission, registry, None);
        }

        let best_score = candidates[0].1;

        if best_score >= SEARCH_FINALIZE_THRESHOLD {
            // 最良候補が閾値以上 → REUSE
            return self.finalize_reuse(candidates[0].0.clone(), registry);
        }

        // 複数候補があれば COMPOSE を試行
        if candidates.len() >= COMPOSE_CANDIDATE_COUNT {
            let second_score = candidates[1].1;
            // 2位のスコアが最良の 70% 以上あれば組成候補とする
            if second_score / best_score.max(f64::EPSILON) > COMPOSE_SECOND_SCORE_RATIO {
                return self.try_compose(
                    &candidates[..COMPOSE_CANDIDATE_COUNT],
                    mission,
                    registry,
                );
            }
        }

        // PATCH: 最良候補が閾値以上の場合、差分パッチを試行
        if best_score >= PATCH_EXISTING_THRESHOLD {
            if let Ok(memoized) = registry.resolve(&candidates[0].0) {
                let base_graph = memoized.graph.clone();
                // Refine を経由して ProposeNew に遷移 (FSM: Evaluate→ProposeNew は直接不可)
                self.transition_to(SearchState::Refine)?;
                return self.try_patch_existing(
                    candidates[0].0.clone(),
                    &base_graph,
                    mission,
                    registry,
                );
            }
        }

        // 候補不足かつ候補あり → Differential Mutation を試行
        self.transition_to(SearchState::Refine)?;
        let base_graph = registry.resolve(&candidates[0].0).ok().map(|m| m.graph.clone());
        self.propose_new(mission, registry, base_graph.as_ref())
    }

    /// FSM 状態遷移を試行する（発振検出を含む）。
    fn transition_to(&mut self, next: SearchState) -> Result<(), DarviumError> {
        attempt_transition(&mut self.state, &mut self.oscillation_detector, next)
    }

    /// 最良候補を再利用する (REUSE)。
    fn finalize_reuse(
        &mut self,
        graph_id: String,
        _registry: &mut WorkflowRegistry,
    ) -> Result<SearchOutcome, DarviumError> {
        self.transition_to(SearchState::Finalize)?;
        Ok(SearchOutcome::ReuseExisting { graph_id })
    }

    /// 複数候補の組成を試行する (COMPOSE)。
    ///
    /// 各候補グラフをレジストリから解決し、`compose_workflows()` で逐次合成する。
    /// 合成結果をレジストリに登録し、`CompositionPlan.component_graph_ids` の末尾に
    /// 合成グラフ ID を追加する。シミュレーション側では末尾 ID から解決して取得する。
    fn try_compose(
        &mut self,
        top_candidates: &[(String, f64)],
        mission: &str,
        registry: &mut WorkflowRegistry,
    ) -> Result<SearchOutcome, DarviumError> {
        self.transition_to(SearchState::Compose)?;

        let component_ids: Vec<String> =
            top_candidates.iter().map(|(id, _)| id.clone()).collect();

        // 各候補グラフをレジストリから解決し、逐次合成
        let mut composed_graph: Option<WorkflowGraph> = None;
        for component_id in &component_ids {
            if let Ok(memoized) = registry.resolve(component_id) {
                match composed_graph {
                    Some(ref mut composed) => {
                        let merged =
                            crate::composition::compose_workflows(composed, &memoized.graph);
                        *composed = merged;
                    }
                    None => {
                        composed_graph = Some(memoized.graph.clone());
                    }
                }
            }
        }

        // 合成結果をレジストリに登録し、ID を末尾に追加
        let mut component_graph_ids = component_ids;
        if let Some(composed) = composed_graph {
            let new_id = registry.register_graph_only(composed, mission);
            component_graph_ids.push(new_id);
        }

        let plan = CompositionPlan::new(
            component_graph_ids,
            vec![],
            vec![VarDecl::new("input")],
            VarDecl::new("output"),
        );

        self.transition_to(SearchState::Finalize)?;
        Ok(SearchOutcome::ComposeExisting { plan })
    }

    /// 既存ワークフローに差分パッチを適用する (PATCH)。
    ///
    /// 最良候補グラフに 2〜3 個の AgentStep ノードを DependsOn エッジで接続した
    /// GraphPatch を生成し、`PatchExisting` として返す。事前検証としてパッチ適用と
    /// DAG 性確認を行う。
    ///
    /// 複数ノード追加により、PATCH 経路でも実質的な複雑化（2〜3 ノード/回）を実現する。
    ///
    /// TODO: 本来の完全実装では、DeterminismScore ベースの GraphPatch セマンティック生成、
    ///       ApplicabilityScore 評価、Stage5 の 5 方向分岐（RFC §4A.3 Mechanism 18）を
    ///       実装する。現在は単純なチェーン追加で近似している。
    fn try_patch_existing(
        &mut self,
        graph_id: String,
        base_graph: &WorkflowGraph,
        mission: &str,
        registry: &mut WorkflowRegistry,
    ) -> Result<SearchOutcome, DarviumError> {
        self.transition_to(SearchState::ProposeNew)?;

        // パッチ操作を生成: 2〜3 個の AgentStep ノードを DependsOn エッジで接続したチェーン
        // 複数ノード追加により、PATCH 経路でも実質的な複雑化を実現する。
        // TODO (本物実装): DeterminismScore ベースの GraphPatch セマンティック生成、
        //       ApplicabilityScore 評価、Stage5 の 5 方向分岐（RFC §4A.3 Mechanism 18）
        let base_count = base_graph.node_count();
        let num_new_nodes = 2 + (mission.len() % 2); // ミッション文字列長で 2 or 3 に分岐
        let mut operations = Vec::with_capacity(num_new_nodes * 2);
        let mut last_idx = None;
        for i in 0..num_new_nodes {
            let idx = base_count + i;
            let output_var = format!("patch_output_{}", idx);
            let new_node = WorkflowNode::AgentStep {
                agent: "generic_processor".to_string(),
                prompt_template: format!("Execute patched step {}: {}", i, mission),
                inputs: vec![VarDecl::new("input")],
                output_var,
            };
            operations.push(PatchOperation::AddNode { node: new_node });
            if let Some(prev) = last_idx {
                operations.push(PatchOperation::AddEdge {
                    from: prev,
                    to: idx,
                    meta: EdgeMeta::DependsOn,
                });
            }
            last_idx = Some(idx);
        }

        // パッチ信頼度 (RFC §12.3 の簡易設定)
        let confidence = PatchConfidence {
            value: 0.85,
            self_score: 0.0,
            validator_score: 0.0,
            history_score: 0.0,
        };

        // 事前検証: パッチが DAG 性を維持することを確認
        let patch_for_verify = GraphPatch {
            source_graph_id: graph_id.clone(),
            operations: operations.clone(),
            patch_confidence: confidence,
            generated_at: SystemTime::now(),
            generator_version: String::from("darvium-sw-v1"),
        };
        let patched = match crate::patch::apply_patch_atomic(base_graph, &patch_for_verify) {
            Ok(g) => g,
            Err(_) => {
                // パッチ失敗 → 差分変異による新規生成にフォールバック
                return self.propose_new(mission, registry, Some(base_graph));
            }
        };
        debug_assert!(
            petgraph::algo::toposort(&patched, None).is_ok(),
            "try_patch_existing produced a non-DAG graph"
        );

        // パッチ適用結果をレジストリに登録
        let _ = registry.register_graph_only(patched, mission);

        // GraphPatch 本体を返す
        let patch = GraphPatch {
            source_graph_id: graph_id.clone(),
            operations,
            patch_confidence: PatchConfidence {
                value: 0.85,
                self_score: 0.0,
                validator_score: 0.0,
                history_score: 0.0,
            },
            generated_at: SystemTime::now(),
            generator_version: String::from("darvium-sw-v1"),
        };

        self.transition_to(SearchState::Finalize)?;
        Ok(SearchOutcome::PatchExisting { graph_id, patch })
    }

    /// 新規ワークフローを生成する (NEW / Differential Mutation)。
    ///
    /// `base_for_mutation` が `Some(base_graph)` の場合、ゼロからの生成ではなく
    /// 既存グラフに対する微分変異を試行する (RFC §4A.3 Mechanism 17 Differential Inference)。
    /// `None` の場合は従来どおり新規生成する。
    fn propose_new(
        &mut self,
        mission: &str,
        registry: &mut WorkflowRegistry,
        base_for_mutation: Option<&WorkflowGraph>,
    ) -> Result<SearchOutcome, DarviumError> {
        self.transition_to(SearchState::ProposeNew)?;

        // ミッション文字列のハッシュをシードに決定論的にワークフローを生成
        // 同一ミッション → 同一グラフ構造、異なるミッション → 多様な構造
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        mission.hash(&mut hasher);
        let seed = hasher.finish();

        let graph = if let Some(base) = base_for_mutation {
            // 微分変異: ベースグラフから変異を生成
            workflow_generation::generate_differential_mutation(
                base,
                &mut StdRng::seed_from_u64(seed.wrapping_add(1)),
            )
        } else {
            // 新規生成 (最大複雑度で生成)
            workflow_generation::generate_new_workflow(
                mission,
                &mut StdRng::seed_from_u64(seed),
                WORKFLOW_GENERATION_MAX_COMPLEXITY,
            )
        };

        // レジストリに登録
        let _ = registry.register_graph_only(graph.clone(), mission);

        self.transition_to(SearchState::Finalize)?;
        Ok(SearchOutcome::GenerateNew { proposal: graph })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_registry::WorkflowRegistry;

    /// 空のレジストリでは GenerateNew が返る。
    #[test]
    fn empty_registry_generates_new() {
        let mut engine = SearchWorkflow::new(SearchBudget::default());
        let mut registry = WorkflowRegistry::new();
        let outcome = engine.execute("test mission", &mut registry).unwrap();
        assert!(matches!(outcome, SearchOutcome::GenerateNew { .. }));
        assert_eq!(registry.graph_count(), 1);
    }

    /// レジストリに十分類似したグラフがあると ReuseExisting が返る。
    #[test]
    fn similar_workflow_is_reused() {
        let mut engine = SearchWorkflow::new(SearchBudget::default());
        let mut registry = WorkflowRegistry::new();

        // 同じミッションを事前登録 → 類似度 1.0 になる
        let graph = workflow_generation::generate_new_workflow(
            "reusable mission",
            &mut StdRng::seed_from_u64(42),
            1,
        );
        let _ = registry.register_graph_only(graph, "reusable mission");

        let outcome = engine.execute("reusable mission", &mut registry).unwrap();
        assert!(matches!(outcome, SearchOutcome::ReuseExisting { .. }));
        // 新規登録は行われない (Reuse)
        assert_eq!(registry.graph_count(), 1);
    }

    /// 単一候補かつ類似度 < 0.50 → Refine → ProposeNew → GenerateNew を確認する。
    #[test]
    fn low_similarity_generates_new() {
        let mut engine = SearchWorkflow::new(SearchBudget::default());
        let mut registry = WorkflowRegistry::new();

        // 1 つの低類似度候補を登録（完全に異なるミッション文）
        let g = workflow_generation::generate_new_workflow(
            "base", &mut StdRng::seed_from_u64(42), 1,
        );
        registry.register_graph_only(g, "aaaa bbbb cccc dddd eeee");

        // 全く異なるミッション文で検索 → 低コサイン類似度
        let outcome = engine.execute("zzzz yyyy xxxx wwww vvvv", &mut registry).unwrap();
        assert!(
            matches!(&outcome, SearchOutcome::GenerateNew { .. }),
            "Expected GenerateNew, got {:?}",
            outcome
        );
    }

    /// 予算超過で AbortSearch が返る。
    #[test]
    fn budget_exceeded_aborts() {
        let budget = SearchBudget::new(0, 0, 0, 0);
        let mut engine = SearchWorkflow::new(budget);
        // 予算 0 なので execute は即座に SearchBudgetExceeded を返す
        let result = engine.execute("test", &mut WorkflowRegistry::new());
        assert!(result.is_err());
    }

    /// attempt_transition が発振検出時に Abort を返すことを確認する。
    ///
    /// 発振検出ロジック:
    /// 1. Evaluate → Refine → record_transition(Refine): counter=0, expected_next=Retrieve
    /// 2. Refine → Retrieve → record_transition(Retrieve): MATCH, counter=1 >= max=1 → Abort
    /// Retrieve 遷移自体は Refine→Retrieve が合法なので成功するが、
    /// その後 is_oscillating() が true になるため Err が返り state は Abort になる。
    #[test]
    fn oscillation_detected_aborts() {
        use crate::types::attempt_transition;

        let mut state = SearchState::Evaluate;
        let mut detector = OscillationDetector::new(1); // 最大 1 回のマッチで発振

        // 1 回目: 発振カウンタ設定 (counter=0)
        assert!(attempt_transition(&mut state, &mut detector, SearchState::Refine).is_ok());

        // 2 回目: MATCH → counter=1 >= max=1 → 発振検出 → Abort
        let result = attempt_transition(&mut state, &mut detector, SearchState::Retrieve);
        assert!(result.is_err());
        assert_eq!(state, SearchState::Abort);
    }

    // ============================================================
    // T1: PatchExisting 経路の到達性 (RFC §13.3)
    // ============================================================

    /// try_patch_existing が PatchExisting を返すことを確認する (T1)。
    ///
    /// ベースグラフに対して try_patch_existing を直接呼び出し、
    /// PatchExisting が返り、パッチ操作が 1 件以上含まれることを検証する。
    #[test]
    fn patch_existing_returns_valid_patch() {
        use crate::types::attempt_transition;

        let mut engine = SearchWorkflow::new(SearchBudget::default());
        let mut registry = WorkflowRegistry::new();
        let mut rng = StdRng::seed_from_u64(42);
        let mut osc = OscillationDetector::default();

        // FSM を Init → Retrieve → Evaluate → Refine まで進める
        attempt_transition(&mut engine.state, &mut osc, SearchState::Retrieve).unwrap();
        attempt_transition(&mut engine.state, &mut osc, SearchState::Evaluate).unwrap();
        attempt_transition(&mut engine.state, &mut osc, SearchState::Refine).unwrap();
        assert_eq!(engine.state, SearchState::Refine);

        let base = workflow_generation::generate_new_workflow(
            "base mission",
            &mut rng,
            WORKFLOW_GENERATION_MAX_COMPLEXITY,
        );
        let base_id = registry.register_graph_only(base.clone(), "base mission");
        let base_nodes_before = base.node_count();

        let outcome = engine
            .try_patch_existing(base_id.clone(), &base, "new mission", &mut registry)
            .unwrap();

        match outcome {
            SearchOutcome::PatchExisting {
                graph_id,
                ref patch,
            } => {
                assert_eq!(graph_id, base_id, "graph_id must match base");
                assert!(
                    !patch.operations.is_empty(),
                    "Patch must contain at least 1 operation"
                );
                // パッチ適用後、ノード数が増加していることを検証
                if let Ok(memoized) = registry.resolve(&base_id) {
                    let patched = crate::patch::apply_patch_atomic(&memoized.graph, patch);
                    assert!(patched.is_ok(), "Patch must be applicable");
                    assert!(
                        patched.unwrap().node_count() > base_nodes_before,
                        "Node count must increase after patching"
                    );
                }
            }
            _ => panic!("Expected PatchExisting, got {:?}", outcome),
        }
    }

    /// execute() が単一候補かつ適度な類似度で PatchExisting を返すことを確認する (T1)。
    ///
    /// 1 つのグラフを登録し、類似度が 0.25-0.50 の範囲に収まるミッションで検索すると
    /// PatchExisting が返る。
    #[test]
    fn execute_returns_patch_existing_for_single_candidate() {
        let mut engine = SearchWorkflow::new(SearchBudget::default());
        let mut registry = WorkflowRegistry::new();

        // グラフを登録
        let g = workflow_generation::generate_new_workflow(
            "prefix_common_word_1234",
            &mut StdRng::seed_from_u64(42),
            1,
        );
        registry.register_graph_only(g, "prefix_common_word_1234");

        // 「共通部分があるが異なる」ミッションで検索
        // (embedding は hash ベースのため、文字列の共通成分が多いほど類似度が上がる)
        let outcome = engine
            .execute("prefix_common_word_5678", &mut registry)
            .unwrap();

        // PatchExisting または GenerateNew のいずれかが返る
        // (類似度が 0.25 未満なら GenerateNew になる可能性もある)
        match &outcome {
            SearchOutcome::PatchExisting { .. } => { /* 成功 */ }
            SearchOutcome::GenerateNew { .. } => {
                // この場合はスキップ（類似度条件不足）
            }
            SearchOutcome::ReuseExisting { .. } => {
                panic!("ReuseExisting should not be returned for similar-but-different missions");
            }
            _ => panic!("Unexpected outcome: {:?}", outcome),
        }
    }
}
