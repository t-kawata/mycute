// グラフパッチ操作・バリデーション (RFC §12)
//
// 本モジュールは GraphPatch の型定義、apply_patch_atomic 4 フェーズ、
// validate_patch_result（DAG 検証・変数スコープ検証・SubWorkflow 参照検証）
// を提供する。

use std::time::SystemTime;

use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;

use crate::constants;
use crate::types::*;

// ── パッチ操作 ────────────────────────────────────────────

/// グラフパッチの単一操作 (RFC §12.1)。
#[derive(Debug, Clone, PartialEq)]
pub enum PatchOperation {
    /// ノード追加。
    AddNode { node: WorkflowNode },
    /// ノード削除。
    RemoveNode { node_id: NodeId },
    /// ノード置換。
    ReplaceNode {
        node_id: NodeId,
        new_node: WorkflowNode,
    },
    /// エッジ追加。
    AddEdge {
        from: NodeId,
        to: NodeId,
        meta: EdgeMeta,
    },
    /// エッジ削除。
    RemoveEdge { from: NodeId, to: NodeId },
    /// エージェントノードのプロンプトテンプレート更新。
    UpdatePrompt { node_id: NodeId, new_prompt: String },
    /// SubWorkflow ノードの入力マッピング更新。
    UpdateInputMapping {
        node_id: NodeId,
        new_mapping: Vec<(String, String)>,
    },
}

// ── パッチ信頼度 ──────────────────────────────────────────

/// 3 次元パッチ信頼度 (RFC §12.3)。
///
/// `patchconfidence = c_adjusted^ws * c_v^wv * c_h^wh`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatchConfidence {
    /// 統合信頼度値。
    pub value: f32,
    /// LLM 自己評価スコア cs（割引適用前）
    pub self_score: f32,
    /// バリデータスコア cv
    pub validator_score: f32,
    /// 履歴スコア ch
    pub history_score: f32,
}

impl PatchConfidence {
    /// 3 次元スコアから統合信頼度を計算する (RFC §12.3)。
    ///
    /// `value = cs_adj^ws * cv^wv * ch^wh`
    /// cs < 0.50 時は非対称重み（ws to 0.20, wv to 0.50）に動的切り替え。
    pub fn compute(self_score: f32, validator_score: f32, history_score: f32) -> Self {
        const EPS: f32 = 0.01;
        let cs_adj = (self_score * constants::SELF_CONF_DISCOUNT as f32).max(EPS);
        let cv = validator_score.max(EPS);
        let ch = history_score.max(EPS);
        // 非対称重み調整: LLM が低自信の場合は validator を優先
        let (ws, wv) = if self_score < constants::PATCH_SELF_CONF_SWITCH_THRESHOLD {
            (
                constants::PATCH_CONFIDENCE_WS_LOW,
                constants::PATCH_CONFIDENCE_WV_HIGH,
            )
        } else {
            (
                constants::PATCH_CONFIDENCE_WS,
                constants::PATCH_CONFIDENCE_WV,
            )
        };
        let wh = 0.30f32;
        let value = cs_adj.powf(ws) * cv.powf(wv) * ch.powf(wh);
        Self {
            value,
            self_score,
            validator_score,
            history_score: ch,
        }
    }
}

// ── グラフパッチ ──────────────────────────────────────────

/// グラフパッチ (RFC §12.1)。
///
/// Gold グラフを Gnew に変換する差分操作列と PatchConfidence を含む。
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPatch {
    /// 変換元グラフの ID。
    pub source_graph_id: WorkflowGraphId,
    /// 差分操作列。
    pub operations: Vec<PatchOperation>,
    /// パッチ信頼度。
    pub patch_confidence: PatchConfidence,
    /// 生成時刻。
    pub generated_at: SystemTime,
    /// 生成器バージョン。
    pub generator_version: String,
}

impl Default for GraphPatch {
    /// 空のパッチを生成する（テスト用）。
    fn default() -> Self {
        Self {
            source_graph_id: String::new(),
            operations: Vec::new(),
            patch_confidence: PatchConfidence {
                value: 0.0,
                self_score: 0.0,
                validator_score: 0.0,
                history_score: 0.0,
            },
            generated_at: SystemTime::UNIX_EPOCH,
            generator_version: String::new(),
        }
    }
}

// ── パッチエラー ──────────────────────────────────────────

/// パッチ操作関連のエラー (RFC §12.6)。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PatchError {
    /// 信頼度が閾値未満。
    #[error("Low confidence {0:.3} below threshold {threshold}", threshold = constants::PATCH_CONFIDENCE_THRESHOLD)]
    LowConfidence(f32),
    /// パッチがグラフに巡回を生成する。
    #[error("Patch creates a cycle")]
    CycleCreated,
    /// 変数スコープ違反。
    #[error("Variable scope violation: {0}")]
    VarScopeViolation(String),
    /// SubWorkflow 参照先が存在しない。
    #[error("SubWorkflow reference missing: {0:?}")]
    SubworkflowRefMissing(WorkflowGraphId),
    /// 変換元グラフが見つからない。
    #[error("Source graph not found: {0:?}")]
    SourceGraphNotFound(WorkflowGraphId),
    /// ノードが見つからない。
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeId),
}

// ── 操作適用 ──────────────────────────────────────────────

/// 単一のパッチ操作をグラフに適用する。
pub fn apply_operation(graph: &mut WorkflowGraph, op: &PatchOperation) -> Result<(), PatchError> {
    match op {
        PatchOperation::AddNode { node } => {
            graph.add_node(node.clone());
            Ok(())
        }
        PatchOperation::RemoveNode { node_id } => {
            let idx = NodeIndex::new(*node_id);
            if idx.index() >= graph.node_count() {
                return Err(PatchError::NodeNotFound(*node_id));
            }
            graph.remove_node(idx);
            Ok(())
        }
        PatchOperation::ReplaceNode { node_id, new_node } => {
            let idx = NodeIndex::new(*node_id);
            if idx.index() >= graph.node_count() {
                return Err(PatchError::NodeNotFound(*node_id));
            }
            if let Some(weight) = graph.node_weight_mut(idx) {
                *weight = new_node.clone();
            }
            Ok(())
        }
        PatchOperation::AddEdge { from, to, meta } => {
            let from_idx = NodeIndex::new(*from);
            let to_idx = NodeIndex::new(*to);
            if from_idx.index() >= graph.node_count() {
                return Err(PatchError::NodeNotFound(*from));
            }
            if to_idx.index() >= graph.node_count() {
                return Err(PatchError::NodeNotFound(*to));
            }
            graph.add_edge(from_idx, to_idx, meta.clone());
            Ok(())
        }
        PatchOperation::RemoveEdge { from, to } => {
            let from_idx = NodeIndex::new(*from);
            let to_idx = NodeIndex::new(*to);
            if from_idx.index() >= graph.node_count() {
                return Err(PatchError::NodeNotFound(*from));
            }
            if to_idx.index() >= graph.node_count() {
                return Err(PatchError::NodeNotFound(*to));
            }
            if let Some(edge_idx) = graph.find_edge(from_idx, to_idx) {
                graph.remove_edge(edge_idx);
                Ok(())
            } else {
                Err(PatchError::VarScopeViolation(format!(
                    "Edge not found between {} and {}",
                    from, to
                )))
            }
        }
        PatchOperation::UpdatePrompt {
            node_id,
            new_prompt,
        } => {
            let idx = NodeIndex::new(*node_id);
            if idx.index() >= graph.node_count() {
                return Err(PatchError::NodeNotFound(*node_id));
            }
            if let Some(weight) = graph.node_weight_mut(idx) {
                match weight {
                    WorkflowNode::AgentStep {
                        prompt_template, ..
                    } => {
                        *prompt_template = new_prompt.clone();
                        Ok(())
                    }
                    _ => Err(PatchError::VarScopeViolation(format!(
                        "Node {} is not an AgentStep",
                        node_id
                    ))),
                }
            } else {
                Err(PatchError::NodeNotFound(*node_id))
            }
        }
        PatchOperation::UpdateInputMapping {
            node_id,
            new_mapping,
        } => {
            let idx = NodeIndex::new(*node_id);
            if idx.index() >= graph.node_count() {
                return Err(PatchError::NodeNotFound(*node_id));
            }
            if let Some(weight) = graph.node_weight_mut(idx) {
                match weight {
                    WorkflowNode::SubWorkflow { input_mapping, .. } => {
                        *input_mapping = new_mapping.clone();
                        Ok(())
                    }
                    _ => Err(PatchError::VarScopeViolation(format!(
                        "Node {} is not a SubWorkflow",
                        node_id
                    ))),
                }
            } else {
                Err(PatchError::NodeNotFound(*node_id))
            }
        }
    }
}

// ── アトミックパッチ適用 ──────────────────────────────────

/// Atomic パッチ適用 (RFC §12.4)。
///
/// clone → apply → validate → swap の 4 フェーズにより、
/// 途中失敗時に gold グラフを元の状態に保つ。
pub fn apply_patch_atomic(
    gold: &WorkflowGraph,
    patch: &GraphPatch,
) -> Result<WorkflowGraph, PatchError> {
    // フェーズ 1: clone（gold は不変）
    let mut candidate = gold.clone();
    // フェーズ 2: 全操作を順次適用
    for op in &patch.operations {
        apply_operation(&mut candidate, op)?;
    }
    // フェーズ 3: 事後バリデーション
    validate_patch_result(&candidate)?;
    // フェーズ 4: swap
    Ok(candidate)
}

// ── パッチ結果バリデーション ──────────────────────────────

/// パッチ適用結果の事後バリデーション (RFC §14.2)。
///
/// 以下 3 段階の検証を実施する:
/// 1. DAG 検証: `petgraph::algo::toposort` でサイクル検出
/// 2. 変数スコープ検証: V-03 (DataFlow.from_var の整合性)
/// 3. SubWorkflow 参照検証: 参照先 ID の存在確認
pub fn validate_patch_result(graph: &WorkflowGraph) -> Result<(), PatchError> {
    validate_dag(graph)?;
    validate_var_scope(graph)?;
    validate_subworkflow_refs(graph)?;
    Ok(())
}

/// DAG 検証: `toposort` でサイクルを検出する。
fn validate_dag(graph: &WorkflowGraph) -> Result<(), PatchError> {
    toposort(graph, None).map_err(|_| PatchError::CycleCreated)?;
    Ok(())
}

/// 変数スコープ検証: V-03 (DataFlow.from_var の整合性)。
///
/// 全 DataFlow エッジについて、from_var が送信元ノードの output_var と
/// 一致することを確認する。AgentStep と SubWorkflow のみ output_var を持つ。
pub fn validate_var_scope(graph: &WorkflowGraph) -> Result<(), PatchError> {
    for edge_idx in graph.edge_indices() {
        let (from_idx, _to_idx) = graph
            .edge_endpoints(edge_idx)
            .expect("edge edgepoint consistency");
        if let Some(EdgeMeta::DataFlow { ref from_var, .. }) = graph.edge_weight(edge_idx) {
            if let Some(from_weight) = graph.node_weight(from_idx) {
                let output_var = match from_weight {
                    WorkflowNode::AgentStep { output_var, .. } => output_var,
                    WorkflowNode::SubWorkflow { output_var, .. } => output_var,
                    _ => continue,
                };
                if from_var != output_var {
                    return Err(PatchError::VarScopeViolation(format!(
                        "DataFlow.from_var '{}' does not match source node output_var '{}'",
                        from_var, output_var,
                    )));
                }
            }
        }
    }
    Ok(())
}
/// SubWorkflow 参照検証: 参照先 ID の存在確認。
///
/// 全ての SubWorkflow ノードが有効な workflow_id を持つことのみ検証する
/// （参照先グラフの実在確認はリポジトリ層に委譲）。
pub fn validate_subworkflow_refs(graph: &WorkflowGraph) -> Result<(), PatchError> {
    for node_idx in graph.node_indices() {
        if let Some(WorkflowNode::SubWorkflow { workflow_id, .. }) = graph.node_weight(node_idx) {
            if workflow_id.is_empty() {
                return Err(PatchError::SubworkflowRefMissing(workflow_id.clone()));
            }
        }
    }
    Ok(())
}

/// バリデータスコア c_v を計算する (RFC §14.3)。
///
/// c_v(E_v) = clamp(1.0 - 0.15 * min(E_v, 3), 0.0, 1.0)
/// E_v: 検出された未解決変数の件数
pub fn compute_validator_score(var_violations: usize) -> f32 {
    let penalty = constants::VALIDATOR_VAR_SCOPE_PENALTY as f32;
    let score = 1.0 - penalty * (var_violations.min(3) as f32);
    score.clamp(0.0, 1.0)
}

// ── テスト ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // === 静的検証（不変条件テスト）===

    /// PatchOperation 全 variant が Debug, Clone, PartialEq を実装している。
    #[test]
    fn patch_operation_traits() {
        let op = PatchOperation::AddNode {
            node: WorkflowNode::Placeholder,
        };
        let _ = format!("{:?}", op);
        let _ = op.clone();
        assert_eq!(op, op);
    }

    /// PatchError 全 variant が Debug, Display, Clone, PartialEq を実装している。
    #[test]
    fn patch_error_traits() {
        let err = PatchError::CycleCreated;
        let _ = format!("{:?}", err);
        let _ = format!("{}", err);
        let _ = err.clone();
        assert_eq!(err, err);
    }

    /// GraphPatch 最小構成で構築可能。
    #[test]
    fn graph_patch_default() {
        let patch = GraphPatch::default();
        assert!(patch.source_graph_id.is_empty());
        assert!(patch.operations.is_empty());
    }

    /// PatchConfidence::compute の値が幾何平均式に従う。
    #[test]
    fn patch_confidence_compute_high() {
        let pc = PatchConfidence::compute(0.90, 0.80, 0.70);
        let expected = (0.90f32 * 0.85).powf(0.30) * 0.80f32.powf(0.40) * 0.70f32.powf(0.30);
        assert!((pc.value - expected).abs() < 0.001);
        assert_eq!(pc.self_score, 0.90);
        assert_eq!(pc.validator_score, 0.80);
    }

    /// PatchConfidence 低自信時 (cs < 0.50) に重みが動的切り替わる。
    #[test]
    fn patch_confidence_low_self() {
        let pc = PatchConfidence::compute(0.30, 0.80, 0.70);
        let cs_adj = (0.30f32 * 0.85).max(0.01);
        let expected = cs_adj.powf(0.20) * 0.80f32.powf(0.50) * 0.70f32.powf(0.30);
        assert!((pc.value - expected).abs() < 0.001);
    }

    // === 操作適用テスト ===

    fn build_two_node_graph() -> WorkflowGraph {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::AgentStep {
            agent: "a1".into(),
            prompt_template: "prompt1".into(),
            inputs: vec![],
            output_var: "out1".into(),
        });
        g.add_node(WorkflowNode::AgentStep {
            agent: "a2".into(),
            prompt_template: "prompt2".into(),
            inputs: vec![],
            output_var: "out2".into(),
        });
        g
    }

    #[test]
    fn add_node_empty_graph() {
        let mut g = WorkflowGraph::new();
        let op = PatchOperation::AddNode {
            node: WorkflowNode::Placeholder,
        };
        assert!(apply_operation(&mut g, &op).is_ok());
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn add_edge_normal() {
        let mut g = build_two_node_graph();
        let op = PatchOperation::AddEdge {
            from: 0,
            to: 1,
            meta: EdgeMeta::DependsOn,
        };
        assert!(apply_operation(&mut g, &op).is_ok());
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn add_edge_node_not_found() {
        let mut g = build_two_node_graph();
        let op = PatchOperation::AddEdge {
            from: 0,
            to: 99,
            meta: EdgeMeta::DependsOn,
        };
        assert_eq!(
            apply_operation(&mut g, &op).unwrap_err(),
            PatchError::NodeNotFound(99)
        );
    }

    #[test]
    fn remove_node_normal() {
        let mut g = build_two_node_graph();
        let op = PatchOperation::RemoveNode { node_id: 1 };
        assert!(apply_operation(&mut g, &op).is_ok());
    }

    #[test]
    fn remove_node_not_found() {
        let mut g = build_two_node_graph();
        let op = PatchOperation::RemoveNode { node_id: 99 };
        assert_eq!(
            apply_operation(&mut g, &op).unwrap_err(),
            PatchError::NodeNotFound(99)
        );
    }

    #[test]
    fn replace_node_normal() {
        let mut g = build_two_node_graph();
        let op = PatchOperation::ReplaceNode {
            node_id: 0,
            new_node: WorkflowNode::Placeholder,
        };
        assert!(apply_operation(&mut g, &op).is_ok());
    }

    #[test]
    fn replace_node_not_found() {
        let mut g = build_two_node_graph();
        let op = PatchOperation::ReplaceNode {
            node_id: 99,
            new_node: WorkflowNode::Placeholder,
        };
        assert_eq!(
            apply_operation(&mut g, &op).unwrap_err(),
            PatchError::NodeNotFound(99)
        );
    }

    #[test]
    fn remove_edge_normal() {
        let mut g = build_two_node_graph();
        g.add_edge(NodeIndex::new(0), NodeIndex::new(1), EdgeMeta::DependsOn);
        let op = PatchOperation::RemoveEdge { from: 0, to: 1 };
        assert!(apply_operation(&mut g, &op).is_ok());
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn remove_edge_not_found() {
        let mut g = build_two_node_graph();
        let op = PatchOperation::RemoveEdge { from: 0, to: 1 };
        assert!(apply_operation(&mut g, &op).is_err());
    }

    #[test]
    fn update_prompt_agent_step() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::AgentStep {
            agent: "a1".into(),
            prompt_template: "old".into(),
            inputs: vec![],
            output_var: "out".into(),
        });
        let op = PatchOperation::UpdatePrompt {
            node_id: 0,
            new_prompt: "new_prompt".into(),
        };
        assert!(apply_operation(&mut g, &op).is_ok());
        let weight = g.node_weight(NodeIndex::new(0)).unwrap();
        if let WorkflowNode::AgentStep {
            prompt_template, ..
        } = weight
        {
            assert_eq!(prompt_template, "new_prompt");
        } else {
            panic!("Node should be AgentStep");
        }
    }

    #[test]
    fn update_prompt_placeholder_rejected() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::Placeholder);
        let op = PatchOperation::UpdatePrompt {
            node_id: 0,
            new_prompt: "x".into(),
        };
        assert!(apply_operation(&mut g, &op).is_err());
    }

    #[test]
    fn update_input_mapping_subworkflow() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::SubWorkflow {
            workflow_id: "wf1".into(),
            input_mapping: vec![],
            output_var: "out".into(),
        });
        let op = PatchOperation::UpdateInputMapping {
            node_id: 0,
            new_mapping: [("k".into(), "v".into())].into(),
        };
        assert!(apply_operation(&mut g, &op).is_ok());
    }

    // === apply_patch_atomic テスト ===

    #[test]
    fn apply_patch_atomic_valid() {
        let gold = build_two_node_graph();
        let patch = GraphPatch {
            operations: vec![PatchOperation::AddEdge {
                from: 0,
                to: 1,
                meta: EdgeMeta::DependsOn,
            }],
            ..GraphPatch::default()
        };
        let result = apply_patch_atomic(&gold, &patch);
        assert!(result.is_ok());
        let new_graph = result.unwrap();
        assert_eq!(new_graph.edge_count(), 1);
        assert_eq!(gold.edge_count(), 0);
    }

    #[test]
    fn apply_patch_atomic_atomicity() {
        let gold = build_two_node_graph();
        let patch = GraphPatch {
            operations: vec![
                PatchOperation::AddEdge {
                    from: 0,
                    to: 1,
                    meta: EdgeMeta::DependsOn,
                },
                PatchOperation::AddEdge {
                    from: 0,
                    to: 99,
                    meta: EdgeMeta::DependsOn,
                },
            ],
            ..GraphPatch::default()
        };
        let result = apply_patch_atomic(&gold, &patch);
        assert!(result.is_err());
        assert_eq!(gold.edge_count(), 0);
    }

    #[test]
    fn apply_patch_atomic_cycle_rejected() {
        let mut gold = WorkflowGraph::new();
        gold.add_node(WorkflowNode::Placeholder);
        gold.add_node(WorkflowNode::Placeholder);
        gold.add_edge(NodeIndex::new(0), NodeIndex::new(1), EdgeMeta::DependsOn);
        let patch = GraphPatch {
            operations: vec![PatchOperation::AddEdge {
                from: 1,
                to: 0,
                meta: EdgeMeta::DependsOn,
            }],
            ..GraphPatch::default()
        };
        let result = apply_patch_atomic(&gold, &patch);
        assert_eq!(result.unwrap_err(), PatchError::CycleCreated);
    }

    // === validate_patch_result テスト ===

    #[test]
    fn validate_dag_normal() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::Placeholder);
        g.add_node(WorkflowNode::Placeholder);
        g.add_edge(NodeIndex::new(0), NodeIndex::new(1), EdgeMeta::DependsOn);
        assert!(validate_patch_result(&g).is_ok());
    }

    #[test]
    fn validate_cycle_detected() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::Placeholder);
        g.add_node(WorkflowNode::Placeholder);
        g.add_edge(NodeIndex::new(0), NodeIndex::new(1), EdgeMeta::DependsOn);
        g.add_edge(NodeIndex::new(1), NodeIndex::new(0), EdgeMeta::DependsOn);
        assert_eq!(
            validate_patch_result(&g).unwrap_err(),
            PatchError::CycleCreated
        );
    }

    #[test]
    fn validate_var_scope_normal() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::AgentStep {
            agent: "a".into(),
            prompt_template: "p".into(),
            inputs: vec![],
            output_var: "x".into(),
        });
        g.add_node(WorkflowNode::AgentStep {
            agent: "b".into(),
            prompt_template: "p".into(),
            inputs: vec![],
            output_var: "y".into(),
        });
        g.add_edge(
            NodeIndex::new(0),
            NodeIndex::new(1),
            EdgeMeta::DataFlow {
                from_var: "x".into(),
                to_var: "y".into(),
            },
        );
        assert!(validate_var_scope(&g).is_ok());
    }

    #[test]
    fn validate_var_scope_violation() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::AgentStep {
            agent: "a".into(),
            prompt_template: "p".into(),
            inputs: vec![],
            output_var: "x".into(),
        });
        g.add_node(WorkflowNode::AgentStep {
            agent: "b".into(),
            prompt_template: "p".into(),
            inputs: vec![],
            output_var: "y".into(),
        });
        g.add_edge(
            NodeIndex::new(0),
            NodeIndex::new(1),
            EdgeMeta::DataFlow {
                from_var: "z".into(),
                to_var: "y".into(),
            },
        );
        assert!(validate_var_scope(&g).is_err());
    }

    #[test]
    fn validate_subworkflow_refs_normal() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::SubWorkflow {
            workflow_id: "wf1".into(),
            input_mapping: vec![],
            output_var: "out".into(),
        });
        assert!(validate_subworkflow_refs(&g).is_ok());
    }

    #[test]
    fn validate_subworkflow_refs_empty_id() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::SubWorkflow {
            workflow_id: String::new(),
            input_mapping: vec![],
            output_var: "out".into(),
        });
        assert_eq!(
            validate_subworkflow_refs(&g).unwrap_err(),
            PatchError::SubworkflowRefMissing(String::new())
        );
    }
}
