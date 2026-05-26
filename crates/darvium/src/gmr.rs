// GMR（Goal-Mediated Reasoning）抽象化層
//
// RFC §4A.3 に定義された GMR 機構のうち、未実装の 7 機構を abstract 実装する。
// 既存の REAL コンポーネント（GraphPatch, apply_patch_atomic, AG-06, AG-07）を流用し、
// 不足機構は trait で抽象化することで将来の本実装への置き換えを可能にする。
//
// 参照: Darvium-RFC-0001-Unified-v2.3-final.md §4A.3, §10.2, §10.3, §10.4

use rand::rngs::StdRng;
use rand::Rng;

use crate::constants::*;
use crate::patch::GraphPatch;
use crate::search::pipeline::ApplicabilityOutcome;
use crate::types::*;

// ============================================================================
// 基本データ型
// ============================================================================

/// GMR 評価候補 (RFC §10.3)。
///
/// シミュレーション内で適用可能性を評価するための候補ワークフロー情報。
#[derive(Debug, Clone)]
pub struct ApplicabilityCandidate {
    /// 候補の類似度スコア [0, 1]。
    pub similarity: f64,
    /// 決定論スコア [0, 1]。
    pub determinism: f64,
    /// 信頼スコア [0, 1]。
    pub trust: f64,
    /// 履歴成功率 (AG-01 用) [0, 1]。
    pub historical_success_rate: f64,
    /// 期待効用 (AG-02 用) [0, 1]。
    pub expected_utility: f64,
    /// Embedding ベクトル (AG-03 用)。
    pub embedding: Option<Vec<f32>>,
    /// 利用可能 tick 数 (AG-04 用)。
    pub remaining_ticks: f64,
    /// リスクスコア (AG-05 用) [0, 1]。
    pub risk_score: f64,
}

impl ApplicabilityCandidate {
    /// デフォルト値で新しい候補を生成する。
    pub fn new(similarity: f64, determinism: f64, trust: f64) -> Self {
        Self {
            similarity,
            determinism,
            trust,
            historical_success_rate: AG01_DEFAULT_SUCCESS_RATE,
            expected_utility: AG02_DEFAULT_UTILITY,
            embedding: None,
            remaining_ticks: AG04_DEFAULT_DEADLINE,
            risk_score: 0.0,
        }
    }
}

/// Stage 5 分岐 (RFC §10.4, §12, §13)。
///
/// GMR 検索結果に対する 5 方向の適用判断。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage5Branch {
    /// そのまま再利用。
    Reuse,
    /// 修正して使用。
    Patch,
    /// 複数の知識を合成。
    Compose,
    /// 新規生成。
    New,
    /// 不適切として却下。
    Abort,
}

// ============================================================================
// ApplicabilityChannel トレイト + AG-01〜AG-05 実装
// ============================================================================

/// 適用可能性チャネル (RFC §10.3, AG-01〜AG-05)。
///
/// 各チャネルは異なる観点から候補の適用可能性を評価し、[0, 1] のスコアを返す。
pub trait ApplicabilityChannel {
    /// 候補の適用可能性スコアを計算する。
    fn score(&self, candidate: &ApplicabilityCandidate) -> f64;
}

/// AG-01 RewardSignalChannel: 履歴成功率で評価 (RFC §12)。
///
/// 過去の類似適用における成功率に基づいてスコアを決定する。
#[derive(Debug, Clone)]
pub struct RewardSignalChannel;

impl ApplicabilityChannel for RewardSignalChannel {
    fn score(&self, candidate: &ApplicabilityCandidate) -> f64 {
        candidate.historical_success_rate.clamp(0.0, 1.0)
    }
}

/// AG-02 UtilityChannel: 期待効用で評価 (RFC §12)。
///
/// 候補の適用から得られる期待効用に基づいてスコアを決定する。
#[derive(Debug, Clone)]
pub struct UtilityChannel;

impl ApplicabilityChannel for UtilityChannel {
    fn score(&self, candidate: &ApplicabilityCandidate) -> f64 {
        candidate.expected_utility.clamp(0.0, 1.0)
    }
}

/// AG-03 NoveltyChannel: Embedding 間距離で評価 (RFC §12)。
///
/// 既存知識との差異（新奇性）に基づいてスコアを決定する。
/// コサイン距離が大きいほど新奇性が高いとみなす。
#[derive(Debug, Clone)]
pub struct NoveltyChannel;

impl ApplicabilityChannel for NoveltyChannel {
    fn score(&self, candidate: &ApplicabilityCandidate) -> f64 {
        match &candidate.embedding {
            Some(emb) => {
                // 全ての要素が 0 に近い場合は既存知識とみなす
                let max_val = emb.iter().map(|x| x.abs()).fold(0.0f32, f32::max) as f64;
                if max_val < 0.01 {
                    return AG03_NOVELTY_THRESHOLD;
                }
                // コサイン距離 = 1 - コサイン類似度 (簡略化)
                // 埋め込みのノルムが大きいほど新奇性が高いとみなす
                let norm: f64 = emb.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
                let raw_novelty = (norm / emb.len() as f64).clamp(0.0, 1.0);
                if raw_novelty > AG03_NOVELTY_THRESHOLD {
                    raw_novelty
                } else {
                    AG03_NOVELTY_THRESHOLD * 0.5
                }
            }
            None => AG03_NOVELTY_THRESHOLD,
        }
    }
}

/// AG-04 UrgencyChannel: デッドライン残り時間で評価 (RFC §12)。
///
/// 残り tick 数が少ないほど緊急性が高いとみなす。
#[derive(Debug, Clone)]
pub struct UrgencyChannel;

impl ApplicabilityChannel for UrgencyChannel {
    fn score(&self, candidate: &ApplicabilityCandidate) -> f64 {
        let urgency = 1.0 - (candidate.remaining_ticks / AG04_DEFAULT_DEADLINE).clamp(0.0, 1.0);
        urgency.clamp(0.0, 1.0)
    }
}

/// AG-05 SafetyChannel: リスクスコアで評価 (RFC §12)。
///
/// リスクスコアが低いほど安全とみなす。
#[derive(Debug, Clone)]
pub struct SafetyChannel;

impl ApplicabilityChannel for SafetyChannel {
    fn score(&self, candidate: &ApplicabilityCandidate) -> f64 {
        let safety = 1.0 - (candidate.risk_score / AG05_MAX_RISK_SCORE).clamp(0.0, 1.0);
        safety.clamp(0.0, 1.0)
    }
}

// ============================================================================
// DeterminismScore (RFC §10.2)
// ============================================================================

/// 決定論スコア (RFC §10.2, SoftMin 合成)。
///
/// 各 AgentStep の determinism 値の SoftMin 合成。
/// シミュレーション内では determinism 値の配列を受け取り SoftMin を計算する。
///
/// D(G) = (-1/β) × ln(Σᵢ (wᵢ/W) × exp(-β × dᵢ))
#[derive(Debug, Clone)]
pub struct DeterminismScore;

impl DeterminismScore {
    /// determinism 値の配列から SoftMin 合成スコアを計算する。
    ///
    /// # 引数
    /// - `determinism_values`: 各 AgentStep の determinism 値 [0, 1]。
    /// - `beta`: SoftMin の鋭さパラメータ（デフォルト: SOFT_MIN_BETA = 5.0）。
    ///
    /// # 戻り値
    /// - [0, 1] 範囲の合成スコア。空配列の場合は 1.0（安全側に倒す）。
    pub fn compute(determinism_values: &[f64], beta: f64) -> f64 {
        if determinism_values.is_empty() {
            return 1.0;
        }

        // 各値の重み付き SoftMin を計算
        // シミュレーションでは均等重みを使用（本体実装では side_effect 重み付け）
        let num_values = determinism_values.len() as f64;
        let weight = 1.0 / num_values;

        let sum: f64 = determinism_values
            .iter()
            .map(|d| weight * (-beta * d.clamp(0.0, 1.0)).exp())
            .sum();

        if sum <= 0.0 {
            return 1.0;
        }

        (-sum.ln() / beta).clamp(0.0, 1.0)
    }
}

// ============================================================================
// ApplicabilityScore (RFC §10.3)
// ============================================================================

/// 適用可能性スコア (RFC §10.3, 幾何平均 + floor)。
///
/// A_workflow(G) = max(S_total, f_S)^αS × max(D_G, f_D)^αD × max(T_G, f_T)^αT
#[derive(Debug, Clone)]
pub struct ApplicabilityScore;

impl ApplicabilityScore {
    /// 3 成分から適用可能性スコアを計算する。
    ///
    /// # 引数
    /// - `similarity`: 総合類似度 S_total [0, 1]。
    /// - `determinism`: 決定論スコア D [0, 1]。
    /// - `trust`: 信頼スコア T [0, 1]。
    ///
    /// # 戻り値
    /// - [0, 1] 範囲の適用可能性スコア。
    pub fn compute(similarity: f64, determinism: f64, trust: f64) -> f64 {
        let vs = similarity.max(APPLICABILITY_FLOOR_S);
        let vd = determinism.max(APPLICABILITY_FLOOR_D);
        let vt = trust.max(APPLICABILITY_FLOOR_T);
        vs.powf(APPLICABILITY_ALPHA_S)
            * vd.powf(APPLICABILITY_ALPHA_D)
            * vt.powf(APPLICABILITY_ALPHA_T)
    }
}

// ============================================================================
// Stage5Decision (RFC §10.4)
// ============================================================================

/// Stage 5 分岐決定器 (RFC §10.4)。
///
/// ApplicabilityOutcome のスコアに基づいて 5 方向分岐を確率的に選択する。
#[derive(Debug, Clone)]
pub struct Stage5Decision;

impl Stage5Decision {
    /// ApplicabilityOutcome から分岐を決定する。
    ///
    /// 高スコア → REUSE/COMPOSE、低スコア → ABORT。
    /// 中間スコア → PATCH/NEW。
    pub fn decide(candidate: &ApplicabilityOutcome, rng: &mut StdRng) -> Stage5Branch {
        let total = candidate.total_score;

        if total >= STAGE5_REUSE_THRESHOLD {
            // 高スコア: REUSE または COMPOSE
            if rng.random::<f64>() < 0.7 {
                Stage5Branch::Reuse
            } else {
                Stage5Branch::Compose
            }
        } else if total < STAGE5_ABORT_THRESHOLD {
            // 低スコア: ほぼ ABORT
            if rng.random::<f64>() < 0.9 {
                Stage5Branch::Abort
            } else {
                Stage5Branch::Patch
            }
        } else {
            // 中間スコア: PATCH または NEW
            if rng.random::<f64>() < 0.6 {
                Stage5Branch::Patch
            } else {
                Stage5Branch::New
            }
        }
    }
}

// ============================================================================
// CapabilityGenerator トレイト (RFC §13)
// ============================================================================

/// 能力生成器 (RFC §13 NEW 機構)。
///
/// 既存 WorkflowGraph から新たな能力を生成する抽象化インターフェース。
pub trait CapabilityGenerator {
    /// シードグラフから新しい WorkflowGraph を生成する。
    fn generate(&self, seed: &WorkflowGraph, rng: &mut StdRng) -> WorkflowGraph;
}

/// 簡易 NEW 機構実装: シードグラフに微小変異を加えて新規生成。
///
/// シミュレーション用の簡略化実装。シードの AgentStep からランダムに選択し、
/// 入出力変数に微小変異を加えた新しい AgentStep を 1 つ含むグラフを生成する。
#[derive(Debug, Clone)]
pub struct SimpleNewGenerator;

impl CapabilityGenerator for SimpleNewGenerator {
    fn generate(&self, seed: &WorkflowGraph, rng: &mut StdRng) -> WorkflowGraph {
        let mut new_graph = WorkflowGraph::new();

        // シードから AgentStep を抽出
        let agent_steps: Vec<WorkflowNode> = seed
            .node_weights()
            .filter_map(|n| {
                if let WorkflowNode::AgentStep { .. } = n {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect();

        if agent_steps.is_empty() {
            // デフォルトの AgentStep を生成
            let node = WorkflowNode::AgentStep {
                agent: format!("new_agent_{}", rng.random::<u32>() % 1000),
                prompt_template: String::from("Generated capability template"),
                inputs: vec![VarDecl::new("input")],
                output_var: String::from("output"),
            };
            new_graph.add_node(node);
            return new_graph;
        }

        // ランダムに選択した AgentStep に変異を加える
        let idx = rng.random_range(0..agent_steps.len());
        let selected = &agent_steps[idx];

        match selected {
            WorkflowNode::AgentStep {
                agent,
                prompt_template,
                inputs,
                output_var,
            } => {
                let mutated_agent = format!("{}_{}", agent, rng.random::<u32>() % 100);
                let mutated_prompt = format!("{} (mutated)", prompt_template);
                let mutated_inputs = if rng.random::<f64>() < 0.3 {
                    let mut new_inputs = inputs.clone();
                    new_inputs.push(VarDecl::new(format!("extra_{}", rng.random::<u32>() % 100)));
                    new_inputs
                } else {
                    inputs.clone()
                };
                let mutated_output = if rng.random::<f64>() < 0.2 {
                    format!("{}_variant", output_var)
                } else {
                    output_var.clone()
                };

                let node = WorkflowNode::AgentStep {
                    agent: mutated_agent,
                    prompt_template: mutated_prompt,
                    inputs: mutated_inputs,
                    output_var: mutated_output,
                };
                new_graph.add_node(node);
            }
            _ => {
                // フォールバック: デフォルトノード
                let node = WorkflowNode::AgentStep {
                    agent: format!("fallback_{}", rng.random::<u32>() % 1000),
                    prompt_template: String::from("Fallback capability"),
                    inputs: vec![VarDecl::new("input")],
                    output_var: String::from("output"),
                };
                new_graph.add_node(node);
            }
        }

        new_graph
    }
}

// ============================================================================
// DifferentialInference (RFC §13, §15.3)
// ============================================================================

/// 差分推論器 (RFC §13, §15.3)。
///
/// 既存 WorkflowGraph からの不足 AgentStep を特定し、GraphPatch として差分生成する。
/// 人口増加メカニズムの 1 つ。
#[derive(Debug, Clone)]
pub struct DifferentialInference {
    /// 差分検出の感度閾値。
    pub sensitivity_threshold: f64,
}

impl DifferentialInference {
    /// 新しい差分推論器を生成する。
    pub fn new(sensitivity_threshold: f64) -> Self {
        Self {
            sensitivity_threshold,
        }
    }

    /// ソースグラフとターゲットグラフの差分から GraphPatch を生成する。
    ///
    /// # 引数
    /// - `source`: 比較元の WorkflowGraph。
    /// - `target`: 比較先の WorkflowGraph（可変）。
    /// - `rng`: 乱数生成器。
    ///
    /// # 戻り値
    /// - 適用可能な GraphPatch の Vec。
    pub fn infer(
        &self,
        source: &WorkflowGraph,
        target: &mut WorkflowGraph,
        rng: &mut StdRng,
    ) -> Vec<GraphPatch> {
        let source_steps: Vec<&WorkflowNode> = source.node_weights().collect();
        let target_steps: Vec<&WorkflowNode> = target.node_weights().collect();

        // ソースに存在してターゲットに存在しない AgentStep を特定
        let missing_steps: Vec<&WorkflowNode> = source_steps
            .iter()
            .filter(|src_node| {
                if let WorkflowNode::AgentStep {
                    agent: src_agent, ..
                } = src_node
                {
                    !target_steps.iter().any(|tgt_node| {
                        if let WorkflowNode::AgentStep {
                            agent: tgt_agent, ..
                        } = tgt_node
                        {
                            src_agent == tgt_agent
                        } else {
                            false
                        }
                    })
                } else {
                    false
                }
            })
            .copied()
            .collect();

        if missing_steps.is_empty() {
            return Vec::new();
        }

        // ランダムに選択した欠落ステップから GraphPatch を生成
        let idx = rng.random_range(0..missing_steps.len());
        let step_to_add = missing_steps[idx];

        // GraphPatch を生成
        match step_to_add {
            WorkflowNode::AgentStep { .. } => {
                let patch_ops = vec![crate::patch::PatchOperation::AddNode {
                    node: step_to_add.clone(),
                }];

                let patch = GraphPatch {
                    source_graph_id: String::from("differential_inference"),
                    operations: patch_ops,
                    patch_confidence: crate::patch::PatchConfidence {
                        value: 0.7,
                        self_score: 0.0,
                        validator_score: 0.0,
                        history_score: 0.0,
                    },
                    generated_at: std::time::SystemTime::now(),
                    generator_version: String::from("darvium-gmr-sim-v1"),
                };

                vec![patch]
            }
            _ => Vec::new(),
        }
    }
}

// ============================================================================
// ユーティリティ
// ============================================================================

/// 新しい WorkflowGraph をシードから生成する (NEW 機構)。
///
/// CapabilityGenerator トレイトのデフォルト実装として使用する。
/// 既存 WorkflowGraph に微小変異を加えて新規生成する。
pub fn new_workflow_from(seed: &WorkflowGraph, rng: &mut StdRng) -> WorkflowGraph {
    let generator = SimpleNewGenerator;
    generator.generate(seed, rng)
}

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use std::collections::HashMap;

    /// TC1: DeterminismScore::compute が全 determinism = 1.0 で 1.0、全 0.0 で 0.0 を返す。
    #[test]
    fn test_determinism_score_boundaries() {
        // 全 1.0 → 結果は 1.0
        let values = vec![1.0, 1.0, 1.0];
        let score = DeterminismScore::compute(&values, SOFT_MIN_BETA);
        assert!((score - 1.0).abs() < 0.01, "Expected ~1.0, got {}", score);

        // 全 0.0 → 結果は ~0.0（完全な 0 にはならないが限りなく近い）
        let values = vec![0.0, 0.0, 0.0];
        let score = DeterminismScore::compute(&values, SOFT_MIN_BETA);
        assert!(score < 0.01, "Expected ~0.0, got {}", score);

        // 空配列 → 1.0（安全側）
        let values: Vec<f64> = Vec::new();
        let score = DeterminismScore::compute(&values, SOFT_MIN_BETA);
        assert!((score - 1.0).abs() < 0.01, "Expected 1.0 for empty, got {}", score);

        // 単一値
        let values = vec![0.5];
        let score = DeterminismScore::compute(&values, SOFT_MIN_BETA);
        assert!(score > 0.0 && score < 1.0, "Expected in (0,1), got {}", score);
    }

    /// TC2: AG-01〜AG-05 の各チャネルが [0, 1] 範囲のスコアを返す。
    #[test]
    fn test_ag_channels_bounds() {
        let mut rng = StdRng::seed_from_u64(12345);

        let channels: Vec<Box<dyn ApplicabilityChannel>> = vec![
            Box::new(RewardSignalChannel),
            Box::new(UtilityChannel),
            Box::new(NoveltyChannel),
            Box::new(UrgencyChannel),
            Box::new(SafetyChannel),
        ];

        // ランダムな候補で各チャネルのスコアが [0, 1] 範囲内であることを確認
        for _ in 0..100 {
            let candidate = ApplicabilityCandidate {
                similarity: rng.random::<f64>(),
                determinism: rng.random::<f64>(),
                trust: rng.random::<f64>(),
                historical_success_rate: rng.random::<f64>(),
                expected_utility: rng.random::<f64>(),
                embedding: if rng.random::<f64>() < 0.5 {
                    Some(vec![rng.random::<f32>(), rng.random::<f32>(), rng.random::<f32>()])
                } else {
                    None
                },
                remaining_ticks: rng.random::<f64>() * 20.0,
                risk_score: rng.random::<f64>(),
            };

            for channel in &channels {
                let score = channel.score(&candidate);
                assert!(
                    (0.0..=1.0).contains(&score),
                    "Score {} out of [0, 1] range for {:?}",
                    score,
                    std::any::type_name_of_val(channel)
                );
            }
        }

        // 境界値テスト
        let edge_cases = vec![
            (0.0, 0.0, None, 0.0, 0.0),
            (1.0, 1.0, Some(vec![1.0, 1.0, 1.0]), 20.0, 1.0),
            (0.5, 0.5, Some(vec![0.0, 0.0, 0.0]), 10.0, 0.5),
        ];

        for (success_rate, utility, embedding, remaining_ticks, risk_score) in edge_cases {
            let candidate = ApplicabilityCandidate {
                similarity: 0.5,
                determinism: 0.5,
                trust: 0.5,
                historical_success_rate: success_rate,
                expected_utility: utility,
                embedding,
                remaining_ticks,
                risk_score,
            };

            for channel in &channels {
                let score = channel.score(&candidate);
                assert!(
                    (0.0..=1.0).contains(&score),
                    "Edge case score {} out of range for {:?}",
                    score,
                    std::any::type_name_of_val(channel)
                );
            }
        }
    }

    /// TC3: Stage5Decision::decide が高スコア候補に REUSE/COMPOSE を、
    /// 低スコアに ABORT を割り当てる。
    #[test]
    fn test_stage5_decision() {
        let mut rng = StdRng::seed_from_u64(12345);

        // 高スコア候補 → REUSE または COMPOSE
        let high_score_outcome = ApplicabilityOutcome {
            semantic_score: 0.9,
            structural_score: 0.9,
            total_score: 0.9,
            workflow_applicability: 0.9,
            final_applicability: 0.9,
            determinism_score: 0.9,
            trust_score: 0.9,
        };
        let high_decisions: Vec<Stage5Branch> = (0..100)
            .map(|_| Stage5Decision::decide(&high_score_outcome, &mut rng))
            .collect();
        let high_reuse_count = high_decisions.iter().filter(|d| **d == Stage5Branch::Reuse).count();
        let low_abort_count = high_decisions.iter().filter(|d| **d == Stage5Branch::Abort).count();
        assert!(
            high_reuse_count > low_abort_count,
            "High score should produce more REUSE than ABORT"
        );

        // 低スコア候補 → ABORT が支配的
        let low_score_outcome = ApplicabilityOutcome {
            semantic_score: 0.1,
            structural_score: 0.1,
            total_score: 0.1,
            workflow_applicability: 0.1,
            final_applicability: 0.1,
            determinism_score: 0.1,
            trust_score: 0.1,
        };
        let low_decisions: Vec<Stage5Branch> = (0..100)
            .map(|_| Stage5Decision::decide(&low_score_outcome, &mut rng))
            .collect();
        let low_abort_count = low_decisions.iter().filter(|d| **d == Stage5Branch::Abort).count();
        assert!(
            low_abort_count > 50,
            "Low score should favor ABORT (>=50% of decisions)"
        );

        // 中間スコア候補 → PATCH または NEW が出現
        let mid_score_outcome = ApplicabilityOutcome {
            semantic_score: 0.5,
            structural_score: 0.5,
            total_score: 0.5,
            workflow_applicability: 0.5,
            final_applicability: 0.5,
            determinism_score: 0.5,
            trust_score: 0.5,
        };
        let mid_decisions: Vec<Stage5Branch> = (0..100)
            .map(|_| Stage5Decision::decide(&mid_score_outcome, &mut rng))
            .collect();
        let has_patch = mid_decisions.iter().any(|d| *d == Stage5Branch::Patch);
        let has_new = mid_decisions.iter().any(|d| *d == Stage5Branch::New);
        assert!(
            has_patch || has_new,
            "Mid score should produce PATCH or NEW at least once"
        );
    }

    /// TC4: compose_workflows が 2 つの WorkflowGraph を正しく統合する（composition.rs で定義）。
    #[test]
    fn test_compose_workflows() {
        use crate::composition::compose_workflows;

        let mut graph_a = WorkflowGraph::new();
        graph_a.add_node(WorkflowNode::AgentStep {
            agent: String::from("agent_a"),
            prompt_template: String::from("Template A"),
            inputs: vec![VarDecl::new("input_a")],
            output_var: String::from("output_a"),
        });

        let mut graph_b = WorkflowGraph::new();
        graph_b.add_node(WorkflowNode::AgentStep {
            agent: String::from("agent_b"),
            prompt_template: String::from("Template B"),
            inputs: vec![VarDecl::new("input_b")],
            output_var: String::from("output_b"),
        });

        let merged = compose_workflows(&graph_a, &graph_b);
        assert_eq!(
            merged.node_count(),
            2,
            "compose_workflows should merge 2 nodes into 1 graph"
        );

        // 同一ノードを含むグラフの重複排除
        let mut graph_c = WorkflowGraph::new();
        graph_c.add_node(WorkflowNode::AgentStep {
            agent: String::from("agent_a"),
            prompt_template: String::from("Template A"),
            inputs: vec![VarDecl::new("input_a")],
            output_var: String::from("output_a"),
        });

        let merged_dedup = compose_workflows(&graph_a, &graph_c);
        assert_eq!(
            merged_dedup.node_count(),
            1,
            "compose_workflows should deduplicate identical nodes"
        );
    }

    /// TC5: NEW 機構で生成された WorkflowGraph が seed と同一構造ではない。
    #[test]
    fn test_new_workflow_from() {
        let mut rng = StdRng::seed_from_u64(12345);

        let mut seed = WorkflowGraph::new();
        seed.add_node(WorkflowNode::AgentStep {
            agent: String::from("original_agent"),
            prompt_template: String::from("Original template"),
            inputs: vec![VarDecl::new("input")],
            output_var: String::from("output"),
        });

        let generated = new_workflow_from(&seed, &mut rng);
        assert!(
            generated.node_count() > 0,
            "Generated graph should not be empty"
        );

        // 生成されたノードが seed と異なることを確認
        let seed_nodes: Vec<&WorkflowNode> = seed.node_weights().collect();
        let gen_nodes: Vec<&WorkflowNode> = generated.node_weights().collect();
        let is_different = seed_nodes.len() != gen_nodes.len()
            || seed_nodes.iter().zip(gen_nodes.iter()).any(|(s, g)| s != g);

        assert!(is_different, "Generated graph should differ from seed");
    }

    /// TC6: DifferentialInference::infer が生成する GraphPatch が
    /// apply_patch_atomic で適用可能であること。
    #[test]
    fn test_differential_inference_patch_applicable() {
        let mut rng = StdRng::seed_from_u64(12345);

        let mut source = WorkflowGraph::new();
        source.add_node(WorkflowNode::AgentStep {
            agent: String::from("source_agent"),
            prompt_template: String::from("Source template"),
            inputs: vec![VarDecl::new("input")],
            output_var: String::from("output"),
        });

        let mut target = WorkflowGraph::new();
        target.add_node(WorkflowNode::AgentStep {
            agent: String::from("target_agent"),
            prompt_template: String::from("Target template"),
            inputs: vec![VarDecl::new("input")],
            output_var: String::from("output"),
        });

        let inferencer = DifferentialInference::new(0.5);
        let patches = inferencer.infer(&source, &mut target, &mut rng);

        // 差分がない場合（同一構造）、空の Vec
        if !patches.is_empty() {
            // apply_patch_atomic が適用可能であることを確認
            let result = crate::patch::apply_patch_atomic(&target, &patches[0]);
            assert!(
                result.is_ok(),
                "Patch should be applicable: {:?}",
                result.err()
            );
        }

        // 同一グラフ間では差分がないことを確認
        let empty_patches = inferencer.infer(&source, &mut source.clone(), &mut rng);
        assert!(
            empty_patches.is_empty(),
            "Identical graphs should produce no patches"
        );
    }

    /// TC7: ApplicabilityScore::compute が [0, 1] 範囲を返す。
    #[test]
    fn test_applicability_score_bounds() {
        // 最大値
        let max_score = ApplicabilityScore::compute(1.0, 1.0, 1.0);
        assert!(
            (max_score - 1.0).abs() < 0.01,
            "Max score should be ~1.0, got {}",
            max_score
        );

        // 最小値（floor 未満は floor にクリップ）
        let _min_score = ApplicabilityScore::compute(0.0, 0.0, 0.0);

        // 中間値
        let mid_score = ApplicabilityScore::compute(0.5, 0.5, 0.5);
        assert!(
            (0.0..=1.0).contains(&mid_score),
            "Mid score should be in [0, 1], got {}",
            mid_score
        );
    }

    /// 観測テスト: AG チャネルスコア分布を JSON 出力 (--nocapture 用)。
    #[test]
    fn test_observe_ag_channel_distributions() {
        let mut rng = StdRng::seed_from_u64(12345);
        let channels: Vec<(&str, Box<dyn ApplicabilityChannel>)> = vec![
            ("AG01_RewardSignal", Box::new(RewardSignalChannel)),
            ("AG02_Utility", Box::new(UtilityChannel)),
            ("AG03_Novelty", Box::new(NoveltyChannel)),
            ("AG04_Urgency", Box::new(UrgencyChannel)),
            ("AG05_Safety", Box::new(SafetyChannel)),
        ];

        let n_samples = 1000;
        let mut all_scores: HashMap<String, Vec<f64>> = HashMap::new();

        for _ in 0..n_samples {
            let candidate = ApplicabilityCandidate {
                similarity: rng.random::<f64>(),
                determinism: rng.random::<f64>(),
                trust: rng.random::<f64>(),
                historical_success_rate: rng.random::<f64>(),
                expected_utility: rng.random::<f64>(),
                embedding: Some(vec![rng.random::<f32>(), rng.random::<f32>(), rng.random::<f32>()]),
                remaining_ticks: rng.random::<f64>() * 20.0,
                risk_score: rng.random::<f64>(),
            };

            for (name, channel) in &channels {
                let score = channel.score(&candidate);
                all_scores.entry(name.to_string()).or_default().push(score);
            }
        }

        println!("\n=== AG Channel Score Distributions (n={}) ===", n_samples);
        for (name, scores) in &all_scores {
            let mean = scores.iter().sum::<f64>() / scores.len() as f64;
            let min = scores.iter().cloned().fold(f64::MAX, f64::min);
            let max = scores.iter().cloned().fold(f64::MIN, f64::max);
            println!(
                "{{\"channel\":\"{}\",\"mean\":{:.4},\"min\":{:.4},\"max\":{:.4},\"n\":{}}}",
                name, mean, min, max, scores.len()
            );
        }
    }

    /// 観測テスト: Stage5 分岐確率を集計 (--nocapture 用)。
    #[test]
    fn test_observe_stage5_branch_probabilities() {
        let mut rng = StdRng::seed_from_u64(12345);
        let n_decisions = 1000;
        let mut branch_counts: HashMap<Stage5Branch, usize> = HashMap::new();

        for _ in 0..n_decisions {
            let outcome = ApplicabilityOutcome {
                semantic_score: rng.random::<f64>(),
                structural_score: rng.random::<f64>(),
                total_score: rng.random::<f64>(),
                workflow_applicability: rng.random::<f64>(),
                final_applicability: rng.random::<f64>(),
                determinism_score: rng.random::<f64>(),
                trust_score: rng.random::<f64>(),
            };
            let decision = Stage5Decision::decide(&outcome, &mut rng);
            *branch_counts.entry(decision).or_insert(0) += 1;
        }

        println!("\n=== Stage5 Branch Probabilities (n={}) ===", n_decisions);
        let total = n_decisions as f64;
        for (branch, count) in &branch_counts {
            let pct = *count as f64 / total * 100.0;
            println!(
                "{{\"branch\":\"{:?}\",\"count\":{},\"pct\":{:.2}}}",
                branch, count, pct
            );
        }
    }

    /// 観測テスト: GraphPatch サイズ分布 (--nocapture 用)。
    #[test]
    fn test_observe_patch_size_distribution() {
        let mut rng = StdRng::seed_from_u64(12345);
        let n_patches = 100;

        let mut source = WorkflowGraph::new();
        source.add_node(WorkflowNode::AgentStep {
            agent: String::from("source"),
            prompt_template: String::from("Source"),
            inputs: vec![VarDecl::new("input")],
            output_var: String::from("output"),
        });

        let inferencer = DifferentialInference::new(0.5);
        let mut patch_sizes = Vec::new();

        for _ in 0..n_patches {
            let mut target = WorkflowGraph::new();
            target.add_node(WorkflowNode::AgentStep {
                agent: format!("target_{}", rng.random::<u32>() % 1000),
                prompt_template: String::from("Target"),
                inputs: vec![VarDecl::new("input")],
                output_var: String::from("output"),
            });

            let patches = inferencer.infer(&source, &mut target, &mut rng);
            for patch in &patches {
                patch_sizes.push(patch.operations.len());
            }
        }

        println!("\n=== GraphPatch Size Distribution (n={}) ===", patch_sizes.len());
        if !patch_sizes.is_empty() {
            let mean = patch_sizes.iter().sum::<usize>() as f64 / patch_sizes.len() as f64;
            let min = patch_sizes.iter().min().copied().unwrap_or(0);
            let max = patch_sizes.iter().max().copied().unwrap_or(0);
            println!(
                "{{\"type\":\"patch_size\",\"mean\":{:.2},\"min\":{},\"max\":{},\"n\":{}}}",
                mean, min, max, patch_sizes.len()
            );
        } else {
            println!("{{\"type\":\"patch_size\",\"note\":\"no patches generated\"}}");
        }
    }
}
