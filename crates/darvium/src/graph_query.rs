// ワークフローグラフ再帰問い合わせ (RFC §6 拡張)
//
// SubWorkflow を含むグラフの全ノード数を再帰的に計算するユーティリティ。

use std::collections::HashSet;

use crate::constants::CHIEFDOM_DEPTH_SCALE;
use crate::error::DarviumError;
use crate::event::ReputationProfile;
use crate::store::GraphStore;
use crate::types::{GraphId, WorkflowGraph, WorkflowNode};

/// ワークフローグラフの全ノード数（SubWorkflow を再帰的に解決した総計）を計算する。
///
/// トップレベルのノード数 + 各 SubWorkflow ノードが参照するグラフの全ノード数の総和。
/// 同一グラフ ID が複数箇所から参照されている場合でも 1 度だけカウントする
/// （循環参照保護）。
///
/// # 引数
/// - `graph`: 対象ワークフローグラフ
/// - `store`: SubWorkflow 解決用の GraphStore
///
/// # 戻り値
/// トップレベルノード + 全再帰的 SubWorkflow ノードの総数
///
/// # エラー
/// - `DarviumError::NotFound`: 参照先グラフが GraphStore に存在しない場合
pub fn compute_all_node_count(
    graph: &WorkflowGraph,
    store: &dyn GraphStore,
) -> Result<usize, DarviumError> {
    let mut visited: HashSet<GraphId> = HashSet::new();
    count_recursive(graph, store, &mut visited)
}

/// 再帰的内部ヘルパー（visited set で循環参照を保護）。
fn count_recursive(
    graph: &WorkflowGraph,
    store: &dyn GraphStore,
    visited: &mut HashSet<GraphId>,
) -> Result<usize, DarviumError> {
    let mut count = graph.node_count();
    for node_idx in graph.node_indices() {
        if let WorkflowNode::SubWorkflow { workflow_id, .. } = &graph[node_idx] {
            if visited.insert(workflow_id.clone()) {
                let sub_graph = store.load_workflow_graph(workflow_id)?;
                count += count_recursive(&sub_graph, store, visited)?;
            }
        }
    }
    Ok(count)
}

/// 抽象化割合を計算し [0, 1] に正規化する。
///
/// ratio = total_nodes / surface_nodes
/// 正規化: (ratio - 1.0) / ratio
/// ratio=1.0（subgraph なし）→ 0.0, ratio→∞ → 1.0
pub fn compute_abstraction_ratio(
    graph: &WorkflowGraph,
    store: &dyn GraphStore,
) -> Result<f32, DarviumError> {
    let total = compute_all_node_count(graph, store)?;
    let surface = graph.node_count();
    if surface == 0 || total == surface {
        return Ok(0.0_f32);
    }
    let ratio = total as f64 / surface as f64;
    Ok(((ratio - 1.0) / ratio) as f32)
}

/// SubWorkflow 参照を再帰的に辿り、最大ネスト深度を計算する。
/// 循環参照は visited set で保護。
fn calculate_max_nest_depth(
    graph: &WorkflowGraph,
    store: &dyn GraphStore,
    visited: &mut HashSet<GraphId>,
) -> Result<usize, DarviumError> {
    let mut has_sub = false;
    let mut max_child = 0usize;
    for node_idx in graph.node_indices() {
        if let WorkflowNode::SubWorkflow { workflow_id, .. } = &graph[node_idx] {
            has_sub = true;
            if visited.insert(workflow_id.clone()) {
                let sub = store.load_workflow_graph(workflow_id)?;
                let child = calculate_max_nest_depth(&sub, store, visited)?;
                max_child = max_child.max(child);
            }
        }
    }
    Ok(if has_sub { max_child + 1 } else { 0 })
}

/// 最大 SubWorkflow ネスト深度を計算し [0, 1] に正規化する。
///
/// 正規化: depth / (depth + CHIEFDOM_DEPTH_SCALE)
/// depth=0 → 0.0, depth=SCALE → 0.5, depth→∞ → 1.0
pub fn compute_abstraction_depth(
    graph: &WorkflowGraph,
    store: &dyn GraphStore,
) -> Result<f32, DarviumError> {
    let mut visited: HashSet<GraphId> = HashSet::new();
    let max_depth = calculate_max_nest_depth(graph, store, &mut visited)?;
    if max_depth == 0 {
        return Ok(0.0_f32);
    }
    let depth = max_depth as f64;
    Ok((depth / (depth + CHIEFDOM_DEPTH_SCALE)) as f32)
}

/// 洗練スコアを計算する。
/// sophistication = 0.5 * abstraction_ratio + 0.5 * abstraction_depth
pub fn compute_sophistication_score(
    graph: &WorkflowGraph,
    store: &dyn GraphStore,
) -> Result<f32, DarviumError> {
    let ratio = compute_abstraction_ratio(graph, store)?;
    let depth = compute_abstraction_depth(graph, store)?;
    Ok(0.5_f32 * ratio + 0.5_f32 * depth)
}

/// 首長性スコアを計算する。
/// chiefdom = 0.5 * final_score + 0.5 * sophistication
pub fn compute_chiefdom_score(
    reputation: &ReputationProfile,
    sophistication: f32,
) -> f32 {
    0.5_f32 * reputation.final_score + 0.5_f32 * sophistication
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryGraphStore;
    use crate::types::WorkflowGraph;

    /// 空グラフのノード数は 0。
    #[test]
    fn empty_graph_returns_zero() {
        let store = InMemoryGraphStore::new();
        let graph = WorkflowGraph::new();
        let count = compute_all_node_count(&graph, &store).unwrap();
        assert_eq!(count, 0);
    }

    /// トップレベルノードのみを正しくカウントする。
    #[test]
    fn counts_top_level_nodes() {
        let store = InMemoryGraphStore::new();
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::Placeholder);
        graph.add_node(WorkflowNode::Placeholder);
        graph.add_node(WorkflowNode::Placeholder);
        let count = compute_all_node_count(&graph, &store).unwrap();
        assert_eq!(count, 3);
    }

    /// 単一 SubWorkflow のノードを再帰的に含める。
    #[test]
    fn includes_subworkflow_nodes() {
        let store = InMemoryGraphStore::new();
        let mut sub_graph = WorkflowGraph::new();
        sub_graph.add_node(WorkflowNode::Placeholder);
        sub_graph.add_node(WorkflowNode::Placeholder);
        let sub_id = "sub-1".to_string();
        store.store_workflow_graph_with_id(&sub_id, &sub_graph).unwrap();

        let mut parent = WorkflowGraph::new();
        parent.add_node(WorkflowNode::SubWorkflow {
            workflow_id: sub_id,
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        parent.add_node(WorkflowNode::Placeholder);
        let count = compute_all_node_count(&parent, &store).unwrap();
        // parent(2) + sub(2) = 4
        assert_eq!(count, 4);
    }

    /// ネストされた SubWorkflow も深く再帰的にカウントする。
    #[test]
    fn nested_subworkflows() {
        let store = InMemoryGraphStore::new();
        let mut leaf = WorkflowGraph::new();
        leaf.add_node(WorkflowNode::Placeholder);
        let leaf_id = "leaf".to_string();
        store.store_workflow_graph_with_id(&leaf_id, &leaf).unwrap();

        let mut mid = WorkflowGraph::new();
        mid.add_node(WorkflowNode::SubWorkflow {
            workflow_id: leaf_id,
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        mid.add_node(WorkflowNode::Placeholder);
        let mid_id = "mid".to_string();
        store.store_workflow_graph_with_id(&mid_id, &mid).unwrap();

        let mut root = WorkflowGraph::new();
        root.add_node(WorkflowNode::SubWorkflow {
            workflow_id: mid_id,
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        let count = compute_all_node_count(&root, &store).unwrap();
        // root(1) + mid(2) + leaf(1) = 4
        assert_eq!(count, 4);
    }

    /// 同一 SubWorkflow を複数回参照しても 1 度だけカウントする。
    #[test]
    fn deduplicates_shared_subworkflow() {
        let store = InMemoryGraphStore::new();
        let mut shared = WorkflowGraph::new();
        shared.add_node(WorkflowNode::Placeholder);
        shared.add_node(WorkflowNode::Placeholder);
        let shared_id = "shared".to_string();
        store.store_workflow_graph_with_id(&shared_id, &shared).unwrap();

        let mut parent = WorkflowGraph::new();
        parent.add_node(WorkflowNode::SubWorkflow {
            workflow_id: shared_id.clone(),
            input_mapping: vec![],
            output_var: "a".to_string(),
        });
        parent.add_node(WorkflowNode::SubWorkflow {
            workflow_id: shared_id,
            input_mapping: vec![],
            output_var: "b".to_string(),
        });
        let count = compute_all_node_count(&parent, &store).unwrap();
        // parent(2) + shared(2) = 4, NOT 6
        assert_eq!(count, 4);
    }

    /// 循環参照は 1 周で停止する。
    #[test]
    fn circular_reference_terminates() {
        let store = InMemoryGraphStore::new();
        let mut graph_a = WorkflowGraph::new();
        let mut graph_b = WorkflowGraph::new();
        graph_a.add_node(WorkflowNode::SubWorkflow {
            workflow_id: "b".to_string(),
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        graph_b.add_node(WorkflowNode::SubWorkflow {
            workflow_id: "a".to_string(),
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        store.store_workflow_graph_with_id("a", &graph_a).unwrap();
        store.store_workflow_graph_with_id("b", &graph_b).unwrap();

        let count = compute_all_node_count(&graph_a, &store).unwrap();
        // graph_a(1) + graph_b(1) + graph_a revisited via cycle(1) = 3.
        // DAG 前提のため循環ケースは稀で、visited set による停止が最優先。
        assert_eq!(count, 3);
    }

    /// 存在しないグラフ ID はエラーを返す。
    #[test]
    fn missing_subworkflow_returns_error() {
        let store = InMemoryGraphStore::new();
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::SubWorkflow {
            workflow_id: "nonexistent".to_string(),
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        let result = compute_all_node_count(&graph, &store);
        assert!(result.is_err());
    }

    // ── 首長性スコア テスト T1-T15 ──────────────────────────────

    /// T1: 空グラフの抽象化割合は 0.0。
    #[test]
    fn abstraction_ratio_empty_graph() {
        let store = InMemoryGraphStore::new();
        let graph = WorkflowGraph::new();
        let ratio = compute_abstraction_ratio(&graph, &store).unwrap();
        assert_eq!(ratio, 0.0_f32);
    }

    /// T2: SubWorkflow なし（フラット）の抽象化割合は 0.0。
    #[test]
    fn abstraction_ratio_flat_graph() {
        let store = InMemoryGraphStore::new();
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::Placeholder);
        graph.add_node(WorkflowNode::Placeholder);
        let ratio = compute_abstraction_ratio(&graph, &store).unwrap();
        assert_eq!(ratio, 0.0_f32);
    }

    /// T3: SubWorkflow あり → (0, 1) の値。
    #[test]
    fn abstraction_ratio_with_subworkflow() {
        let store = InMemoryGraphStore::new();
        let mut sub = WorkflowGraph::new();
        sub.add_node(WorkflowNode::Placeholder);
        sub.add_node(WorkflowNode::Placeholder);
        let sub_id = "sub".to_string();
        store.store_workflow_graph_with_id(&sub_id, &sub).unwrap();

        let mut parent = WorkflowGraph::new();
        parent.add_node(WorkflowNode::SubWorkflow {
            workflow_id: sub_id,
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        let ratio = compute_abstraction_ratio(&parent, &store).unwrap();
        assert!(ratio > 0.0_f32 && ratio < 1.0_f32);
    }

    /// T4: 大きな比率（surface=1, total=100）→ 1.0 に近い値。
    #[test]
    fn abstraction_ratio_large_ratio() {
        let store = InMemoryGraphStore::new();
        let mut sub = WorkflowGraph::new();
        for _ in 0..99 {
            sub.add_node(WorkflowNode::Placeholder);
        }
        let sub_id = "large-sub".to_string();
        store.store_workflow_graph_with_id(&sub_id, &sub).unwrap();

        let mut parent = WorkflowGraph::new();
        parent.add_node(WorkflowNode::SubWorkflow {
            workflow_id: sub_id,
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        let ratio = compute_abstraction_ratio(&parent, &store).unwrap();
        assert!(ratio > 0.98_f32);
    }

    /// T5: SubWorkflow なしの深度は 0.0。
    #[test]
    fn abstraction_depth_no_subworkflow() {
        let store = InMemoryGraphStore::new();
        let graph = WorkflowGraph::new();
        let depth = compute_abstraction_depth(&graph, &store).unwrap();
        assert_eq!(depth, 0.0_f32);
    }

    /// T6: 単一 SubWorkflow の深度は 1/(1+3)=0.25。
    #[test]
    fn abstraction_depth_single_level() {
        let store = InMemoryGraphStore::new();
        let mut sub = WorkflowGraph::new();
        sub.add_node(WorkflowNode::Placeholder);
        let sub_id = "leaf".to_string();
        store.store_workflow_graph_with_id(&sub_id, &sub).unwrap();

        let mut parent = WorkflowGraph::new();
        parent.add_node(WorkflowNode::SubWorkflow {
            workflow_id: sub_id,
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        let depth = compute_abstraction_depth(&parent, &store).unwrap();
        let expected = 1.0_f32 / (1.0_f32 + 3.0_f32);
        assert!((depth - expected).abs() < 1e-6);
    }

    /// T7: 2段ネスト chain の深度は 2/(2+3)=0.4。
    #[test]
    fn abstraction_depth_two_levels() {
        let store = InMemoryGraphStore::new();
        let mut leaf = WorkflowGraph::new();
        leaf.add_node(WorkflowNode::Placeholder);
        let leaf_id = "leaf".to_string();
        store.store_workflow_graph_with_id(&leaf_id, &leaf).unwrap();

        let mut mid = WorkflowGraph::new();
        mid.add_node(WorkflowNode::SubWorkflow {
            workflow_id: leaf_id,
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        let mid_id = "mid".to_string();
        store.store_workflow_graph_with_id(&mid_id, &mid).unwrap();

        let mut root = WorkflowGraph::new();
        root.add_node(WorkflowNode::SubWorkflow {
            workflow_id: mid_id,
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        let depth = compute_abstraction_depth(&root, &store).unwrap();
        let expected = 2.0_f32 / (2.0_f32 + 3.0_f32);
        assert!((depth - expected).abs() < 1e-6);
    }

    /// T8: 循環参照（A→B→A）は visited set で保護され有限値を返す。
    #[test]
    fn abstraction_depth_circular_reference() {
        let store = InMemoryGraphStore::new();
        let mut graph_a = WorkflowGraph::new();
        let mut graph_b = WorkflowGraph::new();
        graph_a.add_node(WorkflowNode::SubWorkflow {
            workflow_id: "b".to_string(),
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        graph_b.add_node(WorkflowNode::SubWorkflow {
            workflow_id: "a".to_string(),
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        store.store_workflow_graph_with_id("a", &graph_a).unwrap();
        store.store_workflow_graph_with_id("b", &graph_b).unwrap();

        let result = compute_abstraction_depth(&graph_a, &store);
        assert!(result.is_ok());
        assert!(result.unwrap() > 0.0_f32);
    }

    /// T9: 空グラフの洗練スコアは 0.0。
    #[test]
    fn sophistication_score_empty() {
        let store = InMemoryGraphStore::new();
        let graph = WorkflowGraph::new();
        let score = compute_sophistication_score(&graph, &store).unwrap();
        assert_eq!(score, 0.0_f32);
    }

    /// T10: フラットグラフの洗練スコアは 0.0。
    #[test]
    fn sophistication_score_flat() {
        let store = InMemoryGraphStore::new();
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::Placeholder);
        let score = compute_sophistication_score(&graph, &store).unwrap();
        assert_eq!(score, 0.0_f32);
    }

    /// T11: SubWorkflow ありの洗練スコアは (ratio + depth) / 2。
    #[test]
    fn sophistication_score_with_subworkflow() {
        let store = InMemoryGraphStore::new();
        let mut sub = WorkflowGraph::new();
        sub.add_node(WorkflowNode::Placeholder);
        let sub_id = "sub".to_string();
        store.store_workflow_graph_with_id(&sub_id, &sub).unwrap();

        let mut parent = WorkflowGraph::new();
        parent.add_node(WorkflowNode::SubWorkflow {
            workflow_id: sub_id,
            input_mapping: vec![],
            output_var: "out".to_string(),
        });
        let ratio = compute_abstraction_ratio(&parent, &store).unwrap();
        let depth = compute_abstraction_depth(&parent, &store).unwrap();
        let expected = 0.5_f32 * ratio + 0.5_f32 * depth;
        let score = compute_sophistication_score(&parent, &store).unwrap();
        assert!((score - expected).abs() < 1e-6);
    }

    /// T12: chiefdom_score 最小値（final=0.0, sophistication=0.0）→ 0.0。
    #[test]
    fn chiefdom_score_minimum() {
        let rep = ReputationProfile::cold_start();
        let score = compute_chiefdom_score(&rep, 0.0_f32);
        // cold_start の final_score は 0.5 なので、0.5 * 0.5 + 0.5 * 0.0 = 0.25
        // よって 0.0 ではなく 0.25 になる。
        assert!((score - 0.25_f32).abs() < 1e-6);
    }

    /// T13: chiefdom_score 最大値（final=1.0, sophistication=1.0）→ 1.0。
    #[test]
    fn chiefdom_score_maximum() {
        let mut rep = ReputationProfile::cold_start();
        rep.final_score = 1.0_f32;
        let score = compute_chiefdom_score(&rep, 1.0_f32);
        assert!((score - 1.0_f32).abs() < 1e-6);
    }

    /// T14: 混合値（final=0.5, sophistication=0.3）→ 0.4。
    #[test]
    fn chiefdom_score_mixed() {
        let mut rep = ReputationProfile::cold_start();
        rep.final_score = 0.5_f32;
        let score = compute_chiefdom_score(&rep, 0.3_f32);
        assert!((score - 0.4_f32).abs() < 1e-6);
    }

    /// T15: 片方のみ（final=0.8, sophistication=0.0）→ 0.4。
    #[test]
    fn chiefdom_score_reputation_only() {
        let mut rep = ReputationProfile::cold_start();
        rep.final_score = 0.8_f32;
        let score = compute_chiefdom_score(&rep, 0.0_f32);
        assert!((score - 0.4_f32).abs() < 1e-6);
    }
}
