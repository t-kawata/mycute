// Darvium ワークフローコンパイラ
//
// RFC §4A.4 に定義されたワークフロー実行機構のうち、
// compile_to_steps を提供する。petgraph のトポロジカルソートを使用して
// WorkflowGraph を実行可能なステップ順序に変換する。
//
// # 翻訳可能性
//
// 本モジュールの関数は動詞句（compile_to_steps）で命名され、
// 変数名はドメイン概念（sorted_nodes, step_indices）を使用する。
// エラーハンドリングは ? 演算子で伝播し、unwrap は使用しない。

use petgraph::algo::toposort;

use crate::error::DarviumError;
use crate::types::{NodeId, WorkflowGraph};

/// WorkflowGraph をトポロジカルソートし、実行可能な NodeId の順序付きリストを返す。
///
/// DAG に対して petgraph の toposort を実行し、依存関係を満たす実行順序を生成する。
/// 循環依存を検出した場合は DarviumError::CycleDetected を返す。
/// SubWorkflow ノードは展開せず、そのまま NodeId としてリストに含める。
/// 空グラフの場合は空の Vec を返す（エラーにはならない）。
///
/// # 引数
///
/// * `graph` - ソート対象の WorkflowGraph
///
/// # 戻り値
///
/// * `Ok(Vec<NodeId>)` — トポロジカル順序のノード ID リスト
/// * `Err(DarviumError::CycleDetected)` — 循環依存を検出した場合
pub fn compile_to_steps(graph: &WorkflowGraph) -> Result<Vec<NodeId>, DarviumError> {
    let sorted_nodes = toposort(graph, None).map_err(|cycle| {
        let cycle_node_id = cycle.node_id().index();
        DarviumError::CycleDetected(format!(
            "Workflow graph contains a cycle involving node {}",
            cycle_node_id
        ))
    })?;

    let step_indices: Vec<NodeId> = sorted_nodes.iter().map(|node| node.index()).collect();
    Ok(step_indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WorkflowNode;

    /// 与えられたノード数とエッジ数で直列 DAG を構築する。
    fn build_serial_dag(node_count: usize) -> WorkflowGraph {
        let mut graph = WorkflowGraph::new();
        let mut prev: Option<petgraph::graph::NodeIndex> = None;
        for _ in 0..node_count {
            let current = graph.add_node(WorkflowNode::AgentStep {
                agent: "test_agent".to_string(),
                prompt_template: "test_prompt".to_string(),
                inputs: vec![],
                output_var: "output".to_string(),
            });
            if let Some(prev_idx) = prev {
                graph.add_edge(prev_idx, current, crate::types::EdgeMeta::DependsOn);
            }
            prev = Some(current);
        }
        graph
    }

    // TC1: 単純 DAG のトポロジカルソート
    #[test]
    fn test_compile_to_steps_simple_dag() {
        let graph = build_serial_dag(3);
        let result = compile_to_steps(&graph).unwrap();
        assert_eq!(result.len(), 3);
        // 直列 A→B→C: 依存関係が維持されていること
        assert!(result[0] < result[1]);
        assert!(result[1] < result[2]);
    }

    // TC2: 空グラフ
    #[test]
    fn test_compile_to_steps_empty_graph() {
        let graph = WorkflowGraph::new();
        let result = compile_to_steps(&graph).unwrap();
        assert!(result.is_empty());
    }

    // TC3: 単一ノードグラフ
    #[test]
    fn test_compile_to_steps_single_node() {
        let mut graph = WorkflowGraph::new();
        let node = graph.add_node(WorkflowNode::AgentStep {
            agent: "test".to_string(),
            prompt_template: "test".to_string(),
            inputs: vec![],
            output_var: "out".to_string(),
        });
        let result = compile_to_steps(&graph).unwrap();
        assert_eq!(result, vec![node.index()]);
    }

    // TC4: 循環依存グラフ（異常系）
    #[test]
    fn test_compile_to_steps_cycle_detected() {
        let mut graph = WorkflowGraph::new();
        let a = graph.add_node(WorkflowNode::AgentStep {
            agent: "a".to_string(),
            prompt_template: "p".to_string(),
            inputs: vec![],
            output_var: "o".to_string(),
        });
        let b = graph.add_node(WorkflowNode::AgentStep {
            agent: "b".to_string(),
            prompt_template: "p".to_string(),
            inputs: vec![],
            output_var: "o".to_string(),
        });
        graph.add_edge(a, b, crate::types::EdgeMeta::DependsOn);
        graph.add_edge(b, a, crate::types::EdgeMeta::DependsOn);
        let result = compile_to_steps(&graph);
        assert!(result.is_err());
        assert!(matches!(result, Err(DarviumError::CycleDetected(_))));
    }

    // TC4b: 自己ループ（A→A）も検出されること
    #[test]
    fn test_compile_to_steps_self_loop() {
        let mut graph = WorkflowGraph::new();
        let a = graph.add_node(WorkflowNode::AgentStep {
            agent: "a".to_string(),
            prompt_template: "p".to_string(),
            inputs: vec![],
            output_var: "o".to_string(),
        });
        graph.add_edge(a, a, crate::types::EdgeMeta::DependsOn);
        let result = compile_to_steps(&graph);
        assert!(result.is_err());
        assert!(matches!(result, Err(DarviumError::CycleDetected(_))));
    }

    // TC5: 分岐・合流グラフ
    //      A
    //     / \
    //    B   C
    //     \ /
    //      D
    #[test]
    fn test_compile_to_steps_branch_merge_dag() {
        let mut graph = WorkflowGraph::new();
        let a = graph.add_node(WorkflowNode::AgentStep {
            agent: "a".to_string(),
            prompt_template: "p".to_string(),
            inputs: vec![],
            output_var: "o".to_string(),
        });
        let b = graph.add_node(WorkflowNode::AgentStep {
            agent: "b".to_string(),
            prompt_template: "p".to_string(),
            inputs: vec![],
            output_var: "o".to_string(),
        });
        let c = graph.add_node(WorkflowNode::AgentStep {
            agent: "c".to_string(),
            prompt_template: "p".to_string(),
            inputs: vec![],
            output_var: "o".to_string(),
        });
        let d = graph.add_node(WorkflowNode::AgentStep {
            agent: "d".to_string(),
            prompt_template: "p".to_string(),
            inputs: vec![],
            output_var: "o".to_string(),
        });
        graph.add_edge(a, b, crate::types::EdgeMeta::DependsOn);
        graph.add_edge(a, c, crate::types::EdgeMeta::DependsOn);
        graph.add_edge(b, d, crate::types::EdgeMeta::DependsOn);
        graph.add_edge(c, d, crate::types::EdgeMeta::DependsOn);

        let result = compile_to_steps(&graph).unwrap();
        assert_eq!(result.len(), 4);
        // A が最初で D が最後であることを確認
        assert_eq!(result[0], a.index());
        assert_eq!(result[3], d.index());
        // B と C が A より後、D より前
        for &node_id in &[b.index(), c.index()] {
            let pos = result.iter().position(|&id| id == node_id).unwrap();
            assert!(pos > 0);
            assert!(pos < 3);
        }
    }

    // TC6: SubWorkflow ノードを含むグラフ
    #[test]
    fn test_compile_to_steps_with_subworkflow() {
        let mut graph = WorkflowGraph::new();
        let agent = graph.add_node(WorkflowNode::AgentStep {
            agent: "a".to_string(),
            prompt_template: "p".to_string(),
            inputs: vec![],
            output_var: "o".to_string(),
        });
        let sub = graph.add_node(WorkflowNode::SubWorkflow {
            workflow_id: "sub_flow".to_string(),
            input_mapping: vec![],
            output_var: "sub_out".to_string(),
        });
        graph.add_edge(agent, sub, crate::types::EdgeMeta::DependsOn);

        let result = compile_to_steps(&graph).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], agent.index());
        assert_eq!(result[1], sub.index());
    }

    // TC7: ErrorMode の全バリアント構築
    #[test]
    fn test_error_mode_variants() {
        let fail_on_any = crate::types::ErrorMode::FailOnAny;
        let skip_on_error = crate::types::ErrorMode::SkipOnError;
        let _degrade = crate::types::ErrorMode::Degrade;
        let retry = crate::types::ErrorMode::RetryOnError(3);

        // Debug トレイト
        let _ = format!("{:?}", fail_on_any);
        // Clone トレイト
        let _ = fail_on_any.clone();
        // PartialEq の確認
        assert_eq!(fail_on_any, fail_on_any);
        assert_ne!(fail_on_any, skip_on_error);
        assert_eq!(retry, crate::types::ErrorMode::RetryOnError(3));
    }

    // TC8: StepExecutionResult の全フィールド構築・アクセス
    #[test]
    fn test_step_execution_result_fields() {
        let result = crate::types::StepExecutionResult {
            node_id: 42,
            status: crate::types::StepStatus::Success,
            output: Some("completed".to_string()),
            error: None,
            duration_ticks: 100,
        };

        assert_eq!(result.node_id, 42);
        assert_eq!(result.status, crate::types::StepStatus::Success);
        assert_eq!(result.output, Some("completed".to_string()));
        assert_eq!(result.error, None);
        assert_eq!(result.duration_ticks, 100);

        // Debug トレイト
        let _ = format!("{:?}", result);
        // Clone トレイト
        let _ = result.clone();
        // PartialEq
        assert_eq!(result, result);
    }

    // TC9: StepStatus の全バリアント構築
    #[test]
    fn test_step_status_variants() {
        let success = crate::types::StepStatus::Success;
        let failure = crate::types::StepStatus::Failure;
        let partial = crate::types::StepStatus::PartialSuccess;
        let skipped = crate::types::StepStatus::Skipped;

        // Debug トレイト
        let _ = format!("{:?}", success);
        // Clone トレイト
        let _ = success.clone();
        // PartialEq の確認
        assert_eq!(success, crate::types::StepStatus::Success);
        assert_ne!(success, failure);
        assert_eq!(failure, crate::types::StepStatus::Failure);
        assert_eq!(partial, crate::types::StepStatus::PartialSuccess);
        assert_eq!(skipped, crate::types::StepStatus::Skipped);
    }

    // TC10: 大規模グラフ（100 ノード直列 DAG）のパフォーマンス観測
    #[test]
    fn test_compile_to_steps_large_graph() {
        let graph = build_serial_dag(100);
        let start = std::time::Instant::now();
        let result = compile_to_steps(&graph).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result.len(), 100);
        // トポロジカル順序が維持されていること
        for i in 0..99 {
            assert!(result[i] < result[i + 1]);
        }

        // 観測出力（--nocapture 用）
        println!(
            "=== TC10: compile_to_steps 100-node DAG ===\n\
             elapsed_ns: {}\n\
             node_count: {}\n\
             === END TC10 ===",
            elapsed.as_nanos(),
            result.len(),
        );
    }
}
