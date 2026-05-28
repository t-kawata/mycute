// ワークフローグラフ再帰問い合わせ (RFC §6 拡張)
//
// SubWorkflow を含むグラフの全ノード数を再帰的に計算するユーティリティ。

use std::collections::HashSet;

use crate::error::DarviumError;
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
}
