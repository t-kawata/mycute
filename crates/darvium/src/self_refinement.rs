// Self-Refinement パイプライン (RFC §8.3, §12.3A)
//
// ワークフローグラフが GED_GRAPH_SIZE_LIMIT を超えた際に自己抽象化を実行する。
// 部分グラフ抽出 → 独立 MemoizedGraph 登録 → 元グラフの SubWorkflow 置換。
//
// 部分グラフは新個体として population に追加され、独立した専門個体として振る舞う。

use std::collections::{HashMap, HashSet};

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

use crate::constants::{GED_GRAPH_SIZE_LIMIT, MIN_ABSTRACTION_GROUP_SIZE};
use crate::error::DarviumError;
use crate::event::ReputationProfile;
use crate::types::{AbstractableSubgraph, TrustProfile, WorkflowGraph, WorkflowGraphId, WorkflowNode};
use crate::workflow_registry::WorkflowRegistry;

/// グラフノード数が GED_GRAPH_SIZE_LIMIT を超えているか判定する。
pub fn check_needs_abstraction(graph: &WorkflowGraph) -> bool {
    graph.node_count() > GED_GRAPH_SIZE_LIMIT
}

/// 抽象化可能な部分グラフを抽出する。
///
/// トポロジカル順序での連続 AgentStep グループを抽出候補とする。
/// グループサイズが MIN_ABSTRACTION_GROUP_SIZE 未満のものは除外する。
pub fn extract_abstractable_subgraphs(
    graph: &WorkflowGraph,
    parent_id: &WorkflowGraphId,
) -> Vec<AbstractableSubgraph> {
    let topo = match petgraph::algo::toposort(graph, None) {
        Ok(order) => order,
        Err(_) => return Vec::new(), // cyclic graph — skip
    };

    // トポロジカル順に連続する AgentStep グループを収集
    let agent_steps: Vec<NodeIndex> = topo
        .into_iter()
        .filter(|idx| matches!(graph[*idx], WorkflowNode::AgentStep { .. }))
        .collect();

    if agent_steps.len() < MIN_ABSTRACTION_GROUP_SIZE {
        return Vec::new();
    }

    // 連続する AgentStep の最大グループを 1 つ返す
    // Phase F では複数グループの抽出も検討可能
    let mut best_start = 0;
    let mut best_len = 0;

    let mut current_len = 1;
    for i in 1..agent_steps.len() {
        if agent_steps[i].index() == agent_steps[i - 1].index() + 1 {
            current_len += 1;
        } else {
            if current_len > best_len {
                best_len = current_len;
                best_start = i - current_len;
            }
            current_len = 1;
        }
    }
    if current_len > best_len {
        best_len = current_len;
        best_start = agent_steps.len() - current_len;
    }

    if best_len < MIN_ABSTRACTION_GROUP_SIZE {
        return Vec::new();
    }

    let group: Vec<NodeIndex> = agent_steps[best_start..best_start + best_len].to_vec();
    let group_set: HashSet<NodeIndex> = group.iter().copied().collect();

    // 部分グラフを構築
    let mut subgraph = WorkflowGraph::new();
    let mut old_to_new: HashMap<NodeIndex, NodeIndex> = HashMap::new();
    for &idx in &group {
        let new_idx = subgraph.add_node(graph[idx].clone());
        old_to_new.insert(idx, new_idx);
    }

    // 内部エッジをコピー
    for &src in &group {
        for edge_idx in graph.edges(src).map(|e| e.id()) {
            let (a, b) = graph.edge_endpoints(edge_idx).unwrap();
            if group_set.contains(&a) && group_set.contains(&b) {
                let meta = graph[edge_idx].clone();
                subgraph.add_edge(old_to_new[&a], old_to_new[&b], meta);
            }
        }
    }

    // 外部入力変数を収集: グループ外→グループ内 への DataFlow
    let mut external_inputs: Vec<String> = Vec::new();
    for (src, tgt, meta) in graph.raw_edges().iter().map(|e| (e.source(), e.target(), &e.weight)) {
        if !group_set.contains(&src) && group_set.contains(&tgt) {
            if let crate::types::EdgeMeta::DataFlow { ref to_var, .. } = meta {
                if !external_inputs.contains(to_var) {
                    external_inputs.push(to_var.clone());
                }
            }
        }
    }

    // 出力変数: 最後の AgentStep の output_var
    let last_idx = group[best_len - 1];
    let output_var = match &graph[last_idx] {
        WorkflowNode::AgentStep { output_var, .. } => output_var.clone(),
        _ => "abstracted_output".to_string(),
    };

    let parent_size = graph.node_count().max(1);
    let compression_ratio = best_len as f64 / parent_size as f64;

    vec![AbstractableSubgraph {
        parent_id: parent_id.clone(),
        node_indices: group.iter().map(|n| n.index()).collect(),
        subgraph,
        external_inputs,
        output_var,
        compression_ratio,
    }]
}

/// 抽出された部分グラフを SubWorkflow として登録する (RFC §8.3)。
pub fn register_abstracted_subworkflow(
    subgraph: WorkflowGraph,
    parent_id: &WorkflowGraphId,
    _parent_trust: &TrustProfile,
    _parent_reputation: &ReputationProfile,
    _confidence: f32,
    registry: &mut WorkflowRegistry,
) -> Result<WorkflowGraphId, DarviumError> {
    // 部分グラフを説明するミッション文を生成
    let node_count = subgraph.node_count();
    let mission = format!(
        "Abstracted subgraph from {}: {} nodes",
        parent_id,
        node_count
    );
    let id = registry.register_graph_only(subgraph, &mission);
    Ok(id)
}

/// 抽出された部分グラフを SubWorkflow ノードで置換する。
pub fn replace_with_subworkflow(
    graph: &mut WorkflowGraph,
    subgraph_nodes: &[NodeIndex],
    new_workflow_id: &str,
) {
    if subgraph_nodes.is_empty() {
        return;
    }

    let removal_set: HashSet<NodeIndex> = subgraph_nodes.iter().copied().collect();
    let all_nodes: Vec<NodeIndex> = graph.node_indices().collect();
    let keep_nodes: Vec<NodeIndex> = all_nodes
        .iter()
        .copied()
        .filter(|n| !removal_set.contains(n))
        .collect();

    // 新しいグラフを構築
    let mut new_graph = WorkflowGraph::new();
    let mut old_to_new: HashMap<NodeIndex, NodeIndex> = HashMap::new();

    // 保持ノードをコピー
    for &idx in &keep_nodes {
        let new_idx = new_graph.add_node(graph[idx].clone());
        old_to_new.insert(idx, new_idx);
    }

    // SubWorkflow ノードを追加
    let sub_workflow_idx = new_graph.add_node(WorkflowNode::SubWorkflow {
        workflow_id: new_workflow_id.to_string(),
        input_mapping: Vec::new(),
        output_var: "abstracted_output".to_string(),
    });

    // エッジを再接続
    for edge in graph.raw_edges() {
        let src = edge.source();
        let tgt = edge.target();
        let meta = edge.weight.clone();

        let src_in_group = removal_set.contains(&src);
        let tgt_in_group = removal_set.contains(&tgt);

        match (src_in_group, tgt_in_group) {
            (false, false) => {
                // 両方とも保持 → そのままコピー
                new_graph.add_edge(old_to_new[&src], old_to_new[&tgt], meta);
            }
            (false, true) => {
                // 外部 → グループ内 → SubWorkflow への入力エッジ
                new_graph.add_edge(old_to_new[&src], sub_workflow_idx, meta);
            }
            (true, false) => {
                // グループ内 → 外部 → SubWorkflow からの出力エッジ
                new_graph.add_edge(sub_workflow_idx, old_to_new[&tgt], meta);
            }
            (true, true) => {
                // グループ内エッジ → スキップ
            }
        }
    }

    // 元グラフを置換
    *graph = new_graph;
}

/// グラフ全体に対して Self-Refinement を 1 ラウンド実行する。
pub fn run_self_refinement_round(
    graph: &mut WorkflowGraph,
    graph_id: &WorkflowGraphId,
    trust: &TrustProfile,
    reputation: &ReputationProfile,
    registry: &mut WorkflowRegistry,
    mut on_new_individual: Option<&mut dyn FnMut(WorkflowGraph, &WorkflowGraphId)>,
) -> Result<usize, DarviumError> {
    if !check_needs_abstraction(graph) {
        return Ok(0);
    }

    let subgraphs = extract_abstractable_subgraphs(graph, graph_id);
    if subgraphs.is_empty() {
        return Ok(0);
    }

    let mut count = 0;
    for sg in subgraphs {
        // NodeIndex に変換
        let node_indices: Vec<NodeIndex> = sg
            .node_indices
            .iter()
            .map(|&i| NodeIndex::new(i))
            .collect();

        // サブグラフをクローンして個体登録コールバックに渡せるようにする
        let subgraph_for_population = sg.subgraph.clone();

        let new_id = register_abstracted_subworkflow(
            sg.subgraph,
            graph_id,
            trust,
            reputation,
            0.8,
            registry,
        )?;

        // 新個体として population に追加するコールバックを呼び出す
        if let Some(ref mut callback) = on_new_individual {
            callback(subgraph_for_population, &new_id);
        }

        replace_with_subworkflow(graph, &node_indices, &new_id);
        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EdgeMeta, HumanTrustLogistic, VarDecl, WorkflowNode};

    #[test]
    fn empty_graph_no_abstraction() {
        let graph = WorkflowGraph::new();
        assert!(!check_needs_abstraction(&graph));
    }

    #[test]
    fn below_limit_no_abstraction() {
        let mut graph = WorkflowGraph::new();
        for _ in 0..GED_GRAPH_SIZE_LIMIT {
            graph.add_node(WorkflowNode::Placeholder);
        }
        assert!(!check_needs_abstraction(&graph));
    }

    #[test]
    fn above_limit_needs_abstraction() {
        let mut graph = WorkflowGraph::new();
        for _ in 0..=GED_GRAPH_SIZE_LIMIT {
            graph.add_node(WorkflowNode::Placeholder);
        }
        assert!(check_needs_abstraction(&graph));
    }

    #[test]
    fn empty_graph_no_subgraphs() {
        let graph = WorkflowGraph::new();
        let result = extract_abstractable_subgraphs(&graph, &"test".to_string());
        assert!(result.is_empty());
    }

    #[test]
    fn small_graph_no_subgraphs() {
        let mut graph = WorkflowGraph::new();
        for _ in 0..MIN_ABSTRACTION_GROUP_SIZE - 1 {
            graph.add_node(WorkflowNode::AgentStep {
                agent: "a".to_string(),
                prompt_template: "prompt".to_string(),
                inputs: vec![VarDecl::new("input")],
                output_var: "out".to_string(),
            });
        }
        let result = extract_abstractable_subgraphs(&graph, &"test".to_string());
        assert!(result.is_empty());
    }

    #[test]
    fn large_enough_group_extracts_subgraph() {
        let mut graph = WorkflowGraph::new();
        let mut nodes = Vec::new();
        for i in 0..MIN_ABSTRACTION_GROUP_SIZE + 1 {
            let idx = graph.add_node(WorkflowNode::AgentStep {
                agent: format!("agent_{}", i),
                prompt_template: "prompt".to_string(),
                inputs: vec![VarDecl::new("input")],
                output_var: format!("out_{}", i),
            });
            nodes.push(idx);
        }
        // 直列接続
        for i in 1..nodes.len() {
            graph.add_edge(nodes[i - 1], nodes[i], EdgeMeta::DependsOn);
        }

        let result = extract_abstractable_subgraphs(&graph, &"test".to_string());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].node_indices.len(), MIN_ABSTRACTION_GROUP_SIZE + 1);
    }

    #[test]
    fn replacement_reduces_node_count() {
        let mut graph = WorkflowGraph::new();
        let mut nodes = Vec::new();
        for i in 0..MIN_ABSTRACTION_GROUP_SIZE {
            let idx = graph.add_node(WorkflowNode::AgentStep {
                agent: format!("agent_{}", i),
                prompt_template: "prompt".to_string(),
                inputs: vec![VarDecl::new("input")],
                output_var: format!("out_{}", i),
            });
            nodes.push(idx);
        }
        // 直列接続
        for i in 1..nodes.len() {
            graph.add_edge(nodes[i - 1], nodes[i], EdgeMeta::DependsOn);
        }

        let subgraph_nodes: Vec<NodeIndex> = nodes.iter().copied().collect();
        replace_with_subworkflow(&mut graph, &subgraph_nodes, "abstracted-wf");
        assert_eq!(graph.node_count(), 1); // 1 SubWorkflow node
        assert!(matches!(graph[NodeIndex::new(0)], WorkflowNode::SubWorkflow { .. }));
    }

    #[test]
    fn replacement_preserves_external_edges() {
        let mut graph = WorkflowGraph::new();
        // 外部入力ノード
        let external = graph.add_node(WorkflowNode::AgentStep {
            agent: "pre".to_string(),
            prompt_template: "pre".to_string(),
            inputs: vec![VarDecl::new("input")],
            output_var: "pre_out".to_string(),
        });
        // 抽象化対象ノード
        let mut inner = Vec::new();
        for i in 0..MIN_ABSTRACTION_GROUP_SIZE {
            let idx = graph.add_node(WorkflowNode::AgentStep {
                agent: format!("inner_{}", i),
                prompt_template: "inner".to_string(),
                inputs: vec![VarDecl::new("pre_out")],
                output_var: format!("inner_out_{}", i),
            });
            inner.push(idx);
        }
        // 外部出力ノード
        let post = graph.add_node(WorkflowNode::AgentStep {
            agent: "post".to_string(),
            prompt_template: "post".to_string(),
            inputs: vec![VarDecl::new("inner_out_2")],
            output_var: "final".to_string(),
        });

        // 外部 → 抽象化対象
        graph.add_edge(external, inner[0], EdgeMeta::DependsOn);
        graph.add_edge(external, inner[0], EdgeMeta::DataFlow {
            from_var: "pre_out".to_string(),
            to_var: "pre_out".to_string(),
        });
        // 抽象化対象 → 外部
        graph.add_edge(inner[2], post, EdgeMeta::DependsOn);
        graph.add_edge(inner[2], post, EdgeMeta::DataFlow {
            from_var: "inner_out_2".to_string(),
            to_var: "inner_out_2".to_string(),
        });

        let old_count = graph.node_count();
        replace_with_subworkflow(&mut graph, &inner, "abstracted-wf");
        assert_eq!(graph.node_count(), old_count - MIN_ABSTRACTION_GROUP_SIZE + 1);

        // 外部ノードが保持されている
        let has_external = graph.node_indices().any(|n| matches!(&graph[n], WorkflowNode::AgentStep { agent, .. } if agent == "pre"));
        let has_post = graph.node_indices().any(|n| matches!(&graph[n], WorkflowNode::AgentStep { agent, .. } if agent == "post"));
        assert!(has_external);
        assert!(has_post);
    }

    #[test]
    fn refinement_round_empty_ok() {
        let mut graph = WorkflowGraph::new();
        let id = "test".to_string();
        let trust = TrustProfile {
            operational: 0.0,
            semantic: 0.0,
            temporal: 0.0,
            human: HumanTrustLogistic::default(),
        };
        let rep = ReputationProfile::default();
        let mut registry = WorkflowRegistry::new();
        let result = run_self_refinement_round(&mut graph, &id, &trust, &rep, &mut registry, None);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn refinement_round_extracts_and_registers() {
        let mut graph = WorkflowGraph::new();
        let mut nodes = Vec::new();
        // GED_GRAPH_SIZE_LIMIT + 1 ノードを作成（閾値超過を保証）
        for i in 0..GED_GRAPH_SIZE_LIMIT + 1 {
            let idx = graph.add_node(WorkflowNode::AgentStep {
                agent: format!("a{}", i),
                prompt_template: "prompt".to_string(),
                inputs: vec![VarDecl::new("input")],
                output_var: format!("o{}", i),
            });
            nodes.push(idx);
        }
        // 直列接続
        for i in 1..nodes.len() {
            graph.add_edge(nodes[i - 1], nodes[i], EdgeMeta::DependsOn);
        }

        let id = "test-graph".to_string();
        let trust = TrustProfile {
            operational: 0.0,
            semantic: 0.0,
            temporal: 0.0,
            human: HumanTrustLogistic::default(),
        };
        let rep = ReputationProfile::default();
        let mut registry = WorkflowRegistry::new();
        let count = run_self_refinement_round(&mut graph, &id, &trust, &rep, &mut registry, None)
            .unwrap();
        assert_eq!(count, 1);

        // レジストリに新しいワークフローが登録されている
        assert_eq!(registry.graph_count(), 1);

        // グラフノード数が減少している
        assert!(
            graph.node_count() < GED_GRAPH_SIZE_LIMIT + 1,
            "graph should be smaller after refinement: {} >= {}",
            graph.node_count(),
            GED_GRAPH_SIZE_LIMIT + 1
        );
    }

    #[test]
    fn check_needs_abstraction_uses_strict_greater() {
        let mut graph = WorkflowGraph::new();
        for _ in 0..=GED_GRAPH_SIZE_LIMIT {
            graph.add_node(WorkflowNode::Placeholder);
        }
        assert!(check_needs_abstraction(&graph));
        // GED_GRAPH_SIZE_LIMIT ぴったり → 不要
        let mut graph2 = WorkflowGraph::new();
        for _ in 0..GED_GRAPH_SIZE_LIMIT {
            graph2.add_node(WorkflowNode::Placeholder);
        }
        assert!(!check_needs_abstraction(&graph2));
    }
}
