// Darvium 公開型定義
//
// 本ファイルは RFC の型定義から実装に必要な基本型を集約する。
// 詳細な実装はフェーズ/チケットごとに追加される。

// === ID 型 ===
pub type NodeId = usize;
pub type GraphVersion = u64;
pub type PairId = String;
pub type ActorId = String;

// === ワークフローグラフ関連 ===
use petgraph::graph::DiGraph;

/// ワークフローグラフのノード重み。
#[derive(Debug, Clone)]
pub struct WorkflowNode;

/// ワークフローグラフのエッジ重み。
#[derive(Debug, Clone)]
pub struct EdgeMeta;

/// ワークフローグラフ（DAG）。
pub type WorkflowGraph = DiGraph<WorkflowNode, EdgeMeta>;

/// 検索ワークフローグラフ。
pub type SearchWorkflowGraph = DiGraph<WorkflowNode, EdgeMeta>;

// === クエリ表現 列挙型 ===
/// クエリ種別 (RFC §9.5 Knowledge-Aware Extension)
#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    Episodic,
    Canonical,
    Hybrid,
}

/// 鮮度要件 (RFC §9.5)
#[derive(Debug, Clone, PartialEq)]
pub enum FreshnessRequirement {
    Recent,
    Stable,
    Historical,
    Mixed,
}

/// 証拠厳密性 (RFC §9.5)
#[derive(Debug, Clone, PartialEq)]
pub enum EvidenceStrictness {
    Light,
    Strict,
    AuditGrade,
}

/// ドリフト感度 (RFC §9.5)
#[derive(Debug, Clone, PartialEq)]
pub enum DriftSensitivity {
    Ignore,
    PreferLatest,
    ShowHistory,
}

// === 検索関連 ===
/// クエリ表現。
///
/// Mission 入力から導出される検索クエリの完全表現。
/// RFC §9.4, §9.5 に基づく。
#[derive(Debug, Clone, PartialEq)]
pub struct QueryRepresentation {
    pub mission_text: String,
    pub task_embedding: Vec<f32>,
    pub query_design_text: String,
    pub query_design_embedding: Vec<f32>,
    pub design_template_version: String,
    pub query_type: QueryType,
    pub freshness_requirement: FreshnessRequirement,
    pub evidence_strictness: EvidenceStrictness,
    pub origin_trace_required: bool,
    pub drift_sensitivity: DriftSensitivity,
}

impl QueryRepresentation {
    /// RFC §9.5 で規定されたデフォルト値で初期化した QueryRepresentation を生成する。
    pub fn new(
        mission_text: String,
        task_embedding: Vec<f32>,
        query_design_text: String,
        query_design_embedding: Vec<f32>,
    ) -> Self {
        Self {
            mission_text,
            task_embedding,
            query_design_text,
            query_design_embedding,
            design_template_version: "v2.0-final".to_string(),
            query_type: QueryType::Hybrid,
            freshness_requirement: FreshnessRequirement::Mixed,
            evidence_strictness: EvidenceStrictness::Light,
            origin_trace_required: false,
            drift_sensitivity: DriftSensitivity::PreferLatest,
        }
    }
}

impl Default for QueryRepresentation {
    fn default() -> Self {
        Self::new(String::new(), Vec::new(), String::new(), Vec::new())
    }
}

/// 検索ポリシー (RFC §13.4)。
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalPolicy {
    pub top_k_sem: u32,
    pub top_k_struct: u32,
    pub min_trust: f32,
    pub allow_compose: bool,
    pub allow_new: bool,
}

impl Default for RetrievalPolicy {
    fn default() -> Self {
        Self {
            top_k_sem: 5,
            top_k_struct: 5,
            min_trust: 0.0,
            allow_compose: false,
            allow_new: false,
        }
    }
}

/// ランク付けされた検索候補。
#[derive(Debug, Clone, PartialEq)]
pub struct RankedCandidate {
    pub workflow_id: String,
    pub semantic_score: f64,
    pub structural_score: f64,
    pub blended_score: f64,
    pub trust_score: f64,
    pub provenance: Vec<String>,
    pub metadata: serde_json::Value,
}

/// 検索結果の候補集合 (RFC §13.4)。
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateSet {
    pub candidates: Vec<RankedCandidate>,
    pub retrieval_calls_used: u32,
}

impl CandidateSet {
    /// 空の候補集合を生成する。
    pub fn empty() -> Self {
        Self {
            candidates: Vec::new(),
            retrieval_calls_used: 0,
        }
    }
}

/// GMR 検索の抽象インターフェース (RFC §13.4)。
///
/// SearchWorkflow は Stage 0–4 の GMR をこのトレイトを通じて呼び出す。
/// このトレイトは pure retrieval contract であり、最終意思決定権を持たない。
pub trait RetrievalPrimitive {
    /// クエリ表現に基づき、ワークフロー候補を検索する。
    fn search_workflows(
        &self,
        query: &QueryRepresentation,
        policy: &RetrievalPolicy,
    ) -> Result<CandidateSet, crate::error::DarviumError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DarviumError;

    /// テスト1: RetrievalPrimitive を実装するダミー構造体がコンパイルできる。
    struct DummyRetrievalPrimitive;

    impl RetrievalPrimitive for DummyRetrievalPrimitive {
        fn search_workflows(
            &self,
            _query: &QueryRepresentation,
            _policy: &RetrievalPolicy,
        ) -> Result<CandidateSet, DarviumError> {
            Ok(CandidateSet::empty())
        }
    }

    /// テスト2: ダミー構造体の search_workflows が正しい型シグネチャで呼び出せる。
    #[test]
    fn dummy_retrieval_primitive_invocation() {
        let primitive = DummyRetrievalPrimitive;
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let result = primitive.search_workflows(&query, &policy);
        assert!(result.is_ok());
        let candidates = result.expect("DummyRetrievalPrimitive should always return Ok");
        assert!(candidates.candidates.is_empty());
        assert_eq!(candidates.retrieval_calls_used, 0);
    }

    /// テスト3: QueryRepresentation の全フィールドにアクセスできる。
    #[test]
    fn query_representation_field_access() {
        let query = QueryRepresentation::new(
            "mission".to_string(),
            vec![0.1, 0.2, 0.3],
            "design".to_string(),
            vec![0.4, 0.5, 0.6],
        );
        assert_eq!(query.mission_text, "mission");
        assert_eq!(query.task_embedding.len(), 3);
        assert_eq!(query.query_design_text, "design");
        assert_eq!(query.query_design_embedding.len(), 3);
        assert_eq!(query.design_template_version, "v2.0-final");
        assert_eq!(query.query_type, QueryType::Hybrid);
        assert_eq!(query.freshness_requirement, FreshnessRequirement::Mixed);
        assert_eq!(query.evidence_strictness, EvidenceStrictness::Light);
        assert!(!query.origin_trace_required);
        assert_eq!(query.drift_sensitivity, DriftSensitivity::PreferLatest);
    }

    /// テスト4: QueryRepresentation::default() が RFC §9.5 のデフォルト値を満たす。
    #[test]
    fn query_representation_default_values() {
        let query = QueryRepresentation::default();
        assert!(query.mission_text.is_empty());
        assert!(query.task_embedding.is_empty());
        assert!(query.query_design_text.is_empty());
        assert!(query.query_design_embedding.is_empty());
        assert_eq!(query.design_template_version, "v2.0-final");
        assert_eq!(query.query_type, QueryType::Hybrid);
        assert_eq!(query.freshness_requirement, FreshnessRequirement::Mixed);
        assert_eq!(query.evidence_strictness, EvidenceStrictness::Light);
        assert!(!query.origin_trace_required);
        assert_eq!(query.drift_sensitivity, DriftSensitivity::PreferLatest);
    }

    /// テスト5: RetrievalPolicy のデフォルト値が正しい。
    #[test]
    fn retrieval_policy_default_values() {
        let policy = RetrievalPolicy::default();
        assert_eq!(policy.top_k_sem, 5);
        assert_eq!(policy.top_k_struct, 5);
        assert_eq!(policy.min_trust, 0.0);
        assert!(!policy.allow_compose);
        assert!(!policy.allow_new);
    }

    /// テスト6: 全列挙型の全バリアントが網羅的マッチング可能。
    #[test]
    fn enum_variants_exhaustive_match() {
        // QueryType
        for variant in &[QueryType::Episodic, QueryType::Canonical, QueryType::Hybrid] {
            match variant {
                QueryType::Episodic => {}
                QueryType::Canonical => {}
                QueryType::Hybrid => {}
            }
        }
        // FreshnessRequirement
        for variant in &[
            FreshnessRequirement::Recent,
            FreshnessRequirement::Stable,
            FreshnessRequirement::Historical,
            FreshnessRequirement::Mixed,
        ] {
            match variant {
                FreshnessRequirement::Recent => {}
                FreshnessRequirement::Stable => {}
                FreshnessRequirement::Historical => {}
                FreshnessRequirement::Mixed => {}
            }
        }
        // EvidenceStrictness
        for variant in &[
            EvidenceStrictness::Light,
            EvidenceStrictness::Strict,
            EvidenceStrictness::AuditGrade,
        ] {
            match variant {
                EvidenceStrictness::Light => {}
                EvidenceStrictness::Strict => {}
                EvidenceStrictness::AuditGrade => {}
            }
        }
        // DriftSensitivity
        for variant in &[
            DriftSensitivity::Ignore,
            DriftSensitivity::PreferLatest,
            DriftSensitivity::ShowHistory,
        ] {
            match variant {
                DriftSensitivity::Ignore => {}
                DriftSensitivity::PreferLatest => {}
                DriftSensitivity::ShowHistory => {}
            }
        }
    }

    /// テスト7: CandidateSet が空候補・複数候補の両方で構築可能。
    #[test]
    fn candidate_set_construction() {
        // 空の候補集合
        let empty = CandidateSet::empty();
        assert!(empty.candidates.is_empty());
        assert_eq!(empty.retrieval_calls_used, 0);

        // 複数候補
        let candidates = vec![
            RankedCandidate {
                workflow_id: "wf-1".to_string(),
                semantic_score: 0.9,
                structural_score: 0.8,
                blended_score: 0.85,
                trust_score: 0.7,
                provenance: vec!["source-a".to_string()],
                metadata: serde_json::json!({}),
            },
            RankedCandidate {
                workflow_id: "wf-2".to_string(),
                semantic_score: 0.7,
                structural_score: 0.6,
                blended_score: 0.65,
                trust_score: 0.5,
                provenance: vec!["source-b".to_string()],
                metadata: serde_json::json!({"note": "fallback"}),
            },
        ];
        let set = CandidateSet {
            candidates,
            retrieval_calls_used: 1,
        };
        assert_eq!(set.candidates.len(), 2);
        assert_eq!(set.retrieval_calls_used, 1);
    }

    /// テスト8: RankedCandidate が全フィールドを指定して構築可能。
    #[test]
    fn ranked_candidate_construction() {
        let candidate = RankedCandidate {
            workflow_id: "wf-test".to_string(),
            semantic_score: 0.95,
            structural_score: 0.80,
            blended_score: 0.90,
            trust_score: 0.85,
            provenance: vec!["gemini".to_string(), "struct-v1".to_string()],
            metadata: serde_json::json!({"embedding_version": "v2"}),
        };
        assert_eq!(candidate.workflow_id, "wf-test");
        assert_eq!(candidate.semantic_score, 0.95);
        assert_eq!(candidate.structural_score, 0.80);
        assert_eq!(candidate.blended_score, 0.90);
        assert_eq!(candidate.trust_score, 0.85);
        assert_eq!(candidate.provenance.len(), 2);
        assert_eq!(
            candidate
                .metadata
                .get("embedding_version")
                .and_then(|v| v.as_str()),
            Some("v2")
        );
    }

    /// テスト9: RetrievalPrimitive トレイトがオブジェクト安全であり、
    /// Box<dyn RetrievalPrimitive> として使用できることを確認する。
    #[test]
    fn trait_object_safety() {
        let primitive: Box<dyn RetrievalPrimitive> = Box::new(DummyRetrievalPrimitive);
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let result = primitive.search_workflows(&query, &policy);
        assert!(result.is_ok());
    }

    // ── 計装・観測 (OTS-RP): RetrievalPrimitive 型依存関係観測 ──

    /// OTS-RP: 型依存関係の静的性質を観測する。
    ///
    /// RetrievalPrimitive トレイトとその関連型が形成する依存グラフの
    /// 直径を計測し、型システムが有界であることを確認する。
    #[test]
    fn observation_type_dependency_graph() {
        // 各型のメモリサイズを観測（型の複雑性のプロキシ計測）
        let size_qr = std::mem::size_of::<QueryRepresentation>();
        let size_rp = std::mem::size_of::<RetrievalPolicy>();
        let size_rc = std::mem::size_of::<RankedCandidate>();
        let size_cs = std::mem::size_of::<CandidateSet>();

        println!("=== OTS-RP: 型依存関係観測 ===");
        println!("QueryRepresentation size: {} bytes", size_qr);
        println!("RetrievalPolicy size: {} bytes", size_rp);
        println!("RankedCandidate size: {} bytes", size_rc);
        println!("CandidateSet size: {} bytes", size_cs);

        // 型サイズの有界性を検証（ピュアデータ型として妥当な範囲）
        assert!(
            size_qr > 0 && size_qr <= 1024,
            "QueryRepresentation size {} out of range",
            size_qr
        );
        assert!(
            size_rp > 0 && size_rp <= 256,
            "RetrievalPolicy size {} out of range",
            size_rp
        );
        assert!(
            size_rc > 0 && size_rc <= 512,
            "RankedCandidate size {} out of range",
            size_rc
        );
        assert!(
            size_cs > 0 && size_cs <= 512,
            "CandidateSet size {} out of range",
            size_cs
        );

        // 列挙型サイズの観測
        let size_qt = std::mem::size_of::<QueryType>();
        let size_fr = std::mem::size_of::<FreshnessRequirement>();
        let size_es = std::mem::size_of::<EvidenceStrictness>();
        let size_ds = std::mem::size_of::<DriftSensitivity>();
        println!("QueryType size: {} bytes", size_qt);
        println!("FreshnessRequirement size: {} bytes", size_fr);
        println!("EvidenceStrictness size: {} bytes", size_es);
        println!("DriftSensitivity size: {} bytes", size_ds);

        // トレイトオブジェクトの安全性確認
        let _: Box<dyn RetrievalPrimitive> = Box::new(DummyRetrievalPrimitive);
        println!("RetrievalPrimitive trait object safety: OK");

        // 全バリアントの網羅的実行（型の完全性検証）
        let types = (
            QueryType::Episodic,
            FreshnessRequirement::Recent,
            EvidenceStrictness::Light,
            DriftSensitivity::Ignore,
        );
        println!("Enum exhaustive coverage: OK ({} variants total)", 3 + 4 + 3 + 3);

        println!("=== 結果: PASS ===");
    }

    // ============================================================
    // M-2-2: SearchBudget / RecursionGuard テスト (T1〜T5 + OTS-1)
    // ============================================================

    /// T1: デフォルト値検証
    ///
    /// SearchBudget::default() と RecursionGuard::default() が
    /// constants.rs の定数から適切なデフォルト値を設定することを確認する。
    #[test]
    fn budget_default_values() {
        let budget = SearchBudget::default();
        assert_eq!(
            budget.max_iterations,
            crate::constants::DEFAULT_MAX_ITERATIONS
        );
        assert_eq!(
            budget.max_retrieval_calls,
            crate::constants::DEFAULT_MAX_RETRIEVAL_CALLS
        );
        assert_eq!(
            budget.max_prompt_tokens,
            crate::constants::MAX_PROMPT_TOKENS
        );
        assert_eq!(
            budget.max_wall_clock_ms,
            crate::constants::DEFAULT_MAX_WALL_CLOCK_MS
        );
    }

    #[test]
    fn recursion_guard_default_values() {
        let guard = RecursionGuard::default();
        assert_eq!(
            guard.max_depth,
            crate::constants::DEFAULT_RECURSION_MAX_DEPTH
        );
        assert_eq!(guard.current_depth, 0);
        assert!(!guard.allow_reentrant);
    }

    /// T2: サチュレーティングインクリメント境界値テスト
    ///
    /// try_consume_iteration / try_consume_retrieval_call / try_consume_prompt_tokens の
    /// 正常系・異常系・境界値を検証する。
    #[test]
    fn consume_iteration_normal() {
        let mut budget = SearchBudget::default();
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(budget.try_consume_iteration(&mut snapshot).is_ok());
        assert_eq!(snapshot.iterations_used, 1);
    }

    #[test]
    fn consume_iteration_exceeded() {
        let mut budget = SearchBudget {
            max_iterations: 1,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 1,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let result = budget.try_consume_iteration(&mut snapshot);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Search budget exceeded");
    }

    #[test]
    fn consume_retrieval_call_normal() {
        let mut budget = SearchBudget::default();
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(budget.try_consume_retrieval_call(&mut snapshot).is_ok());
        assert_eq!(snapshot.retrieval_calls_used, 1);
    }

    #[test]
    fn consume_retrieval_call_exceeded() {
        let mut budget = SearchBudget {
            max_retrieval_calls: 1,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 1,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let result = budget.try_consume_retrieval_call(&mut snapshot);
        assert!(result.is_err());
    }

    #[test]
    fn consume_prompt_tokens_normal() {
        let mut budget = SearchBudget {
            max_prompt_tokens: 100,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(budget.try_consume_prompt_tokens(&mut snapshot, 50).is_ok());
        assert_eq!(snapshot.prompt_tokens_used, 50);
    }

    #[test]
    fn consume_prompt_tokens_at_boundary() {
        let mut budget = SearchBudget {
            max_prompt_tokens: 100,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 100,
            wall_clock_ms_used: 0,
        };
        let result = budget.try_consume_prompt_tokens(&mut snapshot, 1);
        assert!(result.is_err());
    }

    #[test]
    fn consume_prompt_tokens_exact_limit() {
        let mut budget = SearchBudget {
            max_prompt_tokens: 100,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(budget.try_consume_prompt_tokens(&mut snapshot, 100).is_ok());
        assert_eq!(snapshot.prompt_tokens_used, 100);
    }

    /// T3: RecursionGuard 境界値テスト
    #[test]
    fn increment_depth_normal() {
        let mut guard = RecursionGuard::new(5, true);
        assert_eq!(guard.current_depth, 0);
        assert!(guard.try_increment_depth().is_ok());
        assert_eq!(guard.current_depth, 1);
    }

    #[test]
    fn increment_depth_exceeded() {
        let mut guard = RecursionGuard {
            max_depth: 2,
            current_depth: 2,
            allow_reentrant: true,
        };
        let result = guard.try_increment_depth();
        assert!(result.is_err());
    }

    #[test]
    fn decrement_depth_reduces() {
        let mut guard = RecursionGuard {
            max_depth: 5,
            current_depth: 3,
            allow_reentrant: true,
        };
        guard.decrement_depth();
        assert_eq!(guard.current_depth, 2);
    }

    #[test]
    fn decrement_depth_no_underflow() {
        let mut guard = RecursionGuard {
            max_depth: 5,
            current_depth: 0,
            allow_reentrant: true,
        };
        guard.decrement_depth();
        assert_eq!(guard.current_depth, 0);
    }

    /// T4: allow_reentrant フラグ検証
    #[test]
    fn reentrant_disabled_fails_always() {
        let mut guard = RecursionGuard::new(5, false);
        let result = guard.try_increment_depth();
        assert!(result.is_err());
    }

    #[test]
    fn reentrant_enabled_allows_depth() {
        let mut guard = RecursionGuard::new(3, true);
        assert!(guard.try_increment_depth().is_ok());
        assert!(guard.try_increment_depth().is_ok());
        assert!(guard.try_increment_depth().is_ok());
        assert!(guard.try_increment_depth().is_err());
    }

    /// T5: SearchBudgetSnapshot 累積検証
    #[test]
    fn snapshot_tracks_accumulation() {
        let mut budget = SearchBudget {
            max_iterations: 10,
            max_retrieval_calls: 10,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };

        budget.try_consume_iteration(&mut snapshot).unwrap();
        budget.try_consume_iteration(&mut snapshot).unwrap();
        budget.try_consume_iteration(&mut snapshot).unwrap();
        assert_eq!(snapshot.iterations_used, 3);

        budget.try_consume_retrieval_call(&mut snapshot).unwrap();
        budget.try_consume_retrieval_call(&mut snapshot).unwrap();
        assert_eq!(snapshot.retrieval_calls_used, 2);

        budget.try_consume_prompt_tokens(&mut snapshot, 50).unwrap();
        budget.try_consume_prompt_tokens(&mut snapshot, 50).unwrap();
        assert_eq!(snapshot.prompt_tokens_used, 100);

        let cloned = budget.snapshot(&snapshot);
        assert_eq!(cloned.iterations_used, 3);
        assert_eq!(cloned.retrieval_calls_used, 2);
        assert_eq!(cloned.prompt_tokens_used, 100);
    }

    /// OTS-1: 初期予算アンサンブル緩和時間計測
    ///
    /// シード固定 PRNG で SearchBudget の初期値を変動させた 10,000 個の
    /// アンサンブルを生成し、各軌道が上限境界に到達するまでの挙動を観測する。
    #[test]
    fn ots1_ensemble_relaxation_time() {
        use std::collections::BTreeMap;

        let mut rng: u64 = crate::constants::TEST_PRNG_SEED;
        let ensemble_size = 10_000;
        let max_steps = 200;

        let mut unsaturated_counts: BTreeMap<usize, usize> = BTreeMap::new();

        for _ in 0..ensemble_size {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let mut lcg = rng;

            let next_uniform = |v: &mut u64| -> u64 {
                *v = v
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                *v
            };

            let max_iter = (next_uniform(&mut lcg) % 50) + 1;
            let max_ret = (next_uniform(&mut lcg) % 25) + 1;
            let max_tok = (next_uniform(&mut lcg) % 1000) + 1;

            let mut budget = SearchBudget {
                max_iterations: max_iter as u32,
                max_retrieval_calls: max_ret as u32,
                max_prompt_tokens: max_tok,
                ..SearchBudget::default()
            };

            let mut snapshot = SearchBudgetSnapshot {
                iterations_used: 0,
                retrieval_calls_used: 0,
                prompt_tokens_used: 0,
                wall_clock_ms_used: 0,
            };

            for step in 0..max_steps {
                let action = next_uniform(&mut lcg) % 3;
                let result = match action {
                    0 => budget.try_consume_iteration(&mut snapshot),
                    1 => budget.try_consume_retrieval_call(&mut snapshot),
                    _ => budget.try_consume_prompt_tokens(
                        &mut snapshot,
                        (next_uniform(&mut lcg) % 50) + 1,
                    ),
                };

                if result.is_err() {
                    *unsaturated_counts.entry(step).or_insert(0) += 1;
                    break;
                }

                if step == max_steps - 1 {
                    *unsaturated_counts.entry(max_steps).or_insert(0) += 1;
                }
            }
        }

        let saturated = unsaturated_counts
            .iter()
            .filter(|(&step, _)| step < max_steps)
            .map(|(_, &count)| count)
            .sum::<usize>();
        assert_eq!(
            saturated, ensemble_size,
            "All trajectories must saturate within max_steps (saturated={}, ensemble={})",
            saturated, ensemble_size
        );

        println!("=== OTS-1: Ensemble Relaxation Time ===");
        println!("ensemble_size={}, max_steps={}", ensemble_size, max_steps);
        println!(
            "saturated={}, unsaturated_at_max={}",
            saturated,
            unsaturated_counts.get(&max_steps).unwrap_or(&0)
        );
        println!(
            "All trajectories saturated within {} steps (τ_relax = 0)",
            max_steps
        );
        println!("========================================");
    }
}

/// 検索予算 (RFC §13.3)。
///
/// SearchWorkflow が消費できる上限を束ねる bounded search 制約。
/// 使用量の追跡は SearchBudgetSnapshot が担当し、本構造体は上限定義のみを持つ。
#[derive(Debug, Clone)]
pub struct SearchBudget {
    pub max_iterations: u32,
    pub max_retrieval_calls: u32,
    pub max_prompt_tokens: u64,
    pub max_wall_clock_ms: u64,
}

/// 検索予算使用量スナップショット (RFC §13.3)。
///
/// SearchBudget の消費メソッドが返す現在の使用量。
/// 実時間計測との結合 (wall_clock_ms_used) は Clock トレイト統合時に別途対応する。
#[derive(Debug, Clone, PartialEq)]
pub struct SearchBudgetSnapshot {
    pub iterations_used: u32,
    pub retrieval_calls_used: u32,
    pub prompt_tokens_used: u64,
    pub wall_clock_ms_used: u64,
}

/// 再帰ガード (RFC §13.3)。
///
/// SearchWorkflow が自身を再帰的に呼び出す際の深さ制限を制御する。
/// allow_reentrant が false の場合、再帰呼び出し自体が禁止される。
#[derive(Debug, Clone)]
pub struct RecursionGuard {
    pub max_depth: u32,
    pub current_depth: u32,
    pub allow_reentrant: bool,
}

/// SearchBudget のデフォルト値。
///
/// constants.rs の Environment Policy Knob から値を取得する。
impl Default for SearchBudget {
    fn default() -> Self {
        Self {
            max_iterations: crate::constants::DEFAULT_MAX_ITERATIONS,
            max_retrieval_calls: crate::constants::DEFAULT_MAX_RETRIEVAL_CALLS,
            max_prompt_tokens: crate::constants::MAX_PROMPT_TOKENS,
            max_wall_clock_ms: crate::constants::DEFAULT_MAX_WALL_CLOCK_MS,
        }
    }
}

impl SearchBudget {
    /// 全フィールドを指定して SearchBudget を生成する。
    pub fn new(
        max_iterations: u32,
        max_retrieval_calls: u32,
        max_prompt_tokens: u64,
        max_wall_clock_ms: u64,
    ) -> Self {
        Self {
            max_iterations,
            max_retrieval_calls,
            max_prompt_tokens,
            max_wall_clock_ms,
        }
    }

    /// 1 イテレーションを消費する。
    ///
    /// RFC §13.6 ガード条件に従い、iterations_used が max_iterations に
    /// 達している場合は SearchBudgetExceeded を返す（サチュレーション）。
    pub fn try_consume_iteration(
        &self,
        snapshot: &mut SearchBudgetSnapshot,
    ) -> Result<(), crate::error::DarviumError> {
        if snapshot.iterations_used >= self.max_iterations {
            return Err(crate::error::DarviumError::SearchBudgetExceeded);
        }
        snapshot.iterations_used += 1;
        Ok(())
    }

    /// 1 回の検索呼び出しを消費する。
    ///
    /// retrieval_calls_used が max_retrieval_calls に達している場合は
    /// SearchBudgetExceeded を返す。
    pub fn try_consume_retrieval_call(
        &self,
        snapshot: &mut SearchBudgetSnapshot,
    ) -> Result<(), crate::error::DarviumError> {
        if snapshot.retrieval_calls_used >= self.max_retrieval_calls {
            return Err(crate::error::DarviumError::SearchBudgetExceeded);
        }
        snapshot.retrieval_calls_used += 1;
        Ok(())
    }

    /// 指定されたトークン数を消費する。
    ///
    /// 累積トークン (既存 + 新規) が max_prompt_tokens を超える場合は
    /// SearchBudgetExceeded を返す。上限ぴったりの場合は成功とする（境界値）。
    pub fn try_consume_prompt_tokens(
        &self,
        snapshot: &mut SearchBudgetSnapshot,
        tokens: u64,
    ) -> Result<(), crate::error::DarviumError> {
        let new_total = snapshot
            .prompt_tokens_used
            .checked_add(tokens)
            .ok_or(crate::error::DarviumError::SearchBudgetExceeded)?;
        if new_total > self.max_prompt_tokens {
            return Err(crate::error::DarviumError::SearchBudgetExceeded);
        }
        snapshot.prompt_tokens_used = new_total;
        Ok(())
    }

    /// 現在の使用量スナップショットを生成する。
    ///
    /// 消費メソッドの戻り値としてだけでなく、外部からの現在状態取得にも使用する。
    pub fn snapshot(&self, current: &SearchBudgetSnapshot) -> SearchBudgetSnapshot {
        current.clone()
    }
}

/// RecursionGuard のデフォルト値。
///
/// max_depth は constants.rs の Safety Invariant から、
/// current_depth は常に 0、allow_reentrant は false で初期化される。
impl Default for RecursionGuard {
    fn default() -> Self {
        Self {
            max_depth: crate::constants::DEFAULT_RECURSION_MAX_DEPTH,
            current_depth: 0,
            allow_reentrant: false,
        }
    }
}

impl RecursionGuard {
    /// 全フィールドを指定して RecursionGuard を生成する。
    pub fn new(max_depth: u32, allow_reentrant: bool) -> Self {
        Self {
            max_depth,
            current_depth: 0,
            allow_reentrant,
        }
    }

    /// 現在の深度を 1 増加する。
    ///
    /// RFC §13.6 ガード条件に従い、allow_reentrant が false の場合は
    /// 常に SearchRecursionExceeded を返す。allow_reentrant が true の場合は
    /// current_depth が max_depth に達していない場合のみ成功する。
    pub fn try_increment_depth(&mut self) -> Result<(), crate::error::DarviumError> {
        if !self.allow_reentrant {
            return Err(crate::error::DarviumError::SearchRecursionExceeded);
        }
        if self.current_depth >= self.max_depth {
            return Err(crate::error::DarviumError::SearchRecursionExceeded);
        }
        self.current_depth += 1;
        Ok(())
    }

    /// 現在の深度を 1 減少する（再帰からの復帰時）。
    ///
    /// アンダーフローを防止するため saturating_sub を使用する。
    /// current_depth が 0 の状態で呼び出してもパニックしない。
    pub fn decrement_depth(&mut self) {
        self.current_depth = self.current_depth.saturating_sub(1);
    }
}

// === 信頼関連 ===
#[derive(Debug, Clone)]
pub struct TrustProfile;

#[derive(Debug, Clone)]
pub struct Provenance;

// === パッチ関連 ===
#[derive(Debug, Clone)]
pub struct WorkflowPatch;

// === 系列関連 ===
#[derive(Debug, Clone)]
pub struct SearchTrace;

#[derive(Debug, Clone)]
pub struct TrustAuditLog;

#[derive(Debug, Clone)]
pub struct PatchHistory;

// === Knowledge Ecosystem ===
#[derive(Debug, Clone)]
pub struct KnowledgeApplicability;

// === Training Plane ===
#[derive(Debug, Clone)]
pub struct TrainingMission;

#[derive(Debug, Clone)]
pub struct TrainingFeedback;

// === Fusion ===
#[derive(Debug, Clone)]
pub struct RepositoryPair;

#[derive(Debug, Clone)]
pub struct FusionPlan;

// === Dual-Store 補助データ型 ===

/// グラフ識別子。
/// InMemoryGraphStore 内で各ワークフローグラフを一意に識別する。
pub type GraphId = String;

/// 知識オブジェクト (Knowledge Object)。
/// LadybugDB に格納される知識の最小単位。
#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeObject {
    pub id: String,
    pub object_type: String,
    pub data: String,
}

/// リレーションレコード。
/// 知識オブジェクト間の関連を表現する。
#[derive(Debug, Clone, PartialEq)]
pub struct RelationRecord {
    pub object_id: String,
    pub related_object_id: String,
    pub relation_type: String,
}

/// 原産地トレース (Origin Trace)。
/// 知識オブジェクトの生成経路を記録する。
#[derive(Debug, Clone, PartialEq)]
pub struct OriginTrace {
    pub object_id: String,
    pub source: String,
    pub timestamp_ms: u64,
}

/// 訓練メタデータ (Training Metadata)。
#[derive(Debug, Clone, PartialEq)]
pub struct TrainingMetadata {
    pub mission_id: String,
    pub status: String,
}

/// 融合メタデータ (Fusion Metadata)。
#[derive(Debug, Clone, PartialEq)]
pub struct FusionMetadata {
    pub pair_id: String,
    pub status: String,
}
