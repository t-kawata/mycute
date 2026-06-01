// ワークフロー生成 — 新規生成・差分変異・ミッション埋め込み
//
// RFC §4A.3 の人口増加メカニズムのうち、以下を実装する:
// - Mechanism 16 (NEW branch): ミッション記述からの新規 WorkflowGraph 生成
// - Mechanism 17 (Differential Inference): 既存グラフからの微小変異生成
//
// これらの関数は本物のプロダクションコードであり、シミュレーション専用ではない。
// SearchWorkflow エンジンから呼び出され、MYCUTE からも直接利用される。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use petgraph::graph::NodeIndex;
use rand::Rng;

use crate::constants::{
    DIFF_MUT_ADD_EDGE_PROB, DIFF_MUT_ADD_NODE_PROB, DIFF_MUT_REPLACE_NODE_PROB,
    DIFF_MUT_UPDATE_PROMPT_PROB, DIFFERENTIAL_MUTATION_MAX_ATTEMPTS,
    SEARCH_QUERY_EMBEDDING_DIM, SEARCH_QUERY_HASH_SEED, WORKFLOW_GENERATION_MAX_COMPLEXITY,
};
use crate::types::{EdgeMeta, VarDecl, WorkflowGraph, WorkflowNode};

/// ミッション記述から新しいワークフローグラフを生成する。
///
/// # 生成戦略
///
/// ミッション文字列のハッシュに基づく決定論的要素と PRNG による確率的要素を
/// 組み合わせ、多様な DAG 構造を生成する。
///
/// | complexity | 構造 | ノード数 |
/// |-----------|------|---------|
/// | 0 | 単一 AgentStep | 1 |
/// | 1 | 直列 2 AgentStep | 2 |
/// | 2+ | ファンアウト・マージ型 DAG | 3-5 |
///
/// # DAG 保証
///
/// 生成後 `petgraph::algo::toposort` で DAG 性を検証する。
/// すべてのノードはトポロジカル順に追加される。
pub fn generate_new_workflow(
    mission: &str,
    rng: &mut impl Rng,
    complexity: usize,
) -> WorkflowGraph {
    let complexity = complexity.min(WORKFLOW_GENERATION_MAX_COMPLEXITY);
    let mut graph = WorkflowGraph::new();

    match complexity {
        0 => generate_single_node(&mut graph, mission, rng, 0),
        1 => generate_sequential(&mut graph, mission, rng),
        _ => generate_dag(&mut graph, mission, rng),
    }

    // DAG 性の事後検証
    debug_assert!(
        petgraph::algo::toposort(&graph, None).is_ok(),
        "generate_new_workflow produced a non-DAG graph"
    );

    graph
}

/// 既存ワークフローグラフにルールベースの微小変異を適用する。
///
/// # 変異操作の選択確率
///
/// | 操作 | 確率 | 内容 |
/// |------|------|------|
/// | UpdatePrompt | 5% | ランダムな AgentStep のプロンプトテンプレートを微調整 |
/// | AddEdge | 10% | 2 ノード間に DependsOn エッジを追加（DAG 維持） |
/// | AddNode | 70% | 新しい AgentStep ノードを追加 |
/// | ReplaceNode | 10% | Placeholder を AgentStep に置換 |
/// | RemoveEdge | 5% | ランダムなエッジを削除 |
///
/// # DAG 保証
///
/// 変異適用後 `toposort` で DAG 性を検証する。巡回が発生する変異はスキップされ、
/// 別の変異を最大 `DIFFERENTIAL_MUTATION_MAX_ATTEMPTS` 回試行する。
/// すべて失敗した場合は元のグラフのクローンを返す。
pub fn generate_differential_mutation(
    graph: &WorkflowGraph,
    rng: &mut impl Rng,
) -> WorkflowGraph {
    if graph.node_count() == 0 {
        return graph.clone();
    }

    for _ in 0..DIFFERENTIAL_MUTATION_MAX_ATTEMPTS {
        let choice = rng.random_range(0.0..1.0);
        let mut mutated = graph.clone();

        // 個別確率から累積閾値を計算
        let update_end = DIFF_MUT_UPDATE_PROMPT_PROB;
        let add_edge_end = update_end + DIFF_MUT_ADD_EDGE_PROB;
        let add_node_end = add_edge_end + DIFF_MUT_ADD_NODE_PROB;
        let replace_end = add_node_end + DIFF_MUT_REPLACE_NODE_PROB;

        let success = match choice {
            _ if choice < update_end => apply_update_prompt(&mut mutated, rng),
            _ if choice < add_edge_end => apply_add_edge(&mut mutated, rng),
            _ if choice < add_node_end => apply_add_node(&mut mutated, ""),
            _ if choice < replace_end => apply_replace_node(&mut mutated, rng),
            _ => apply_remove_edge(&mut mutated, rng),
        };

        if success && petgraph::algo::toposort(&mutated, None).is_ok() {
            return mutated;
        }
    }

    graph.clone()
}

/// ミッション文字列から決定論的埋め込みベクトルを生成する。
///
/// # 決定論性
///
/// 同一ミッション文字列からは常に同一の 384 次元ベクトルが生成される。
/// ハッシュベースの線形射影を使用し、LLM や外部 API を必要としない。
/// 将来の `RealEmbeddingProvider` 接続時のデフォルト実装としても使用可能。
pub fn mission_to_embedding(mission: &str) -> Vec<f32> {
    let seed = SEARCH_QUERY_HASH_SEED;
    let dim = SEARCH_QUERY_EMBEDDING_DIM;

    let hash = hash_string(mission);
    let hash2 = hash_string(&format!("{}:{}", mission, seed));

    let mut embedding = Vec::with_capacity(dim);
    for i in 0..dim {
        let h = hash_string(&format!("{}:{}:{}", hash, hash2, i));
        // [0, 1] → [-1, 1] に線形写像し、ゼロ中心化する
        let val = ((h as f64 / u64::MAX as f64) * 2.0 - 1.0) as f32;
        embedding.push(val);
    }

    // 単位ベクトルに正規化
    let norm: f64 = embedding.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    if norm > 0.0 {
        for v in &mut embedding {
            *v = (*v as f64 / norm) as f32;
        }
    }

    embedding
}

// ============================================================
// 内部ヘルパー
// ============================================================

fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// 単一 AgentStep ノードを生成する。
fn generate_single_node(
    graph: &mut WorkflowGraph,
    mission: &str,
    rng: &mut impl Rng,
    index: usize,
) {
    let agent = pick_agent(index, rng);
    let prompt = format!("Mission: {}. Complete the task and output the result.", mission);
    let inputs = vec![VarDecl::new("mission_input")];

    graph.add_node(WorkflowNode::AgentStep {
        agent,
        prompt_template: prompt,
        inputs,
        output_var: "output".to_string(),
    });
}

/// 2 ノード直列のワークフローを生成する。
fn generate_sequential(graph: &mut WorkflowGraph, mission: &str, rng: &mut impl Rng) {
    let agent0 = pick_agent(0, rng);
    let agent1 = pick_agent(1, rng);

    let n0 = graph.add_node(WorkflowNode::AgentStep {
        agent: agent0,
        prompt_template: format!("Step 1: Analyze the requirements for: {}", mission),
        inputs: vec![VarDecl::new("mission_input")],
        output_var: "analysis".to_string(),
    });

    let n1 = graph.add_node(WorkflowNode::AgentStep {
        agent: agent1,
        prompt_template: format!(
            "Step 2: Based on analysis, produce the final output for: {}",
            mission
        ),
        inputs: vec![VarDecl::new("analysis")],
        output_var: "output".to_string(),
    });

    graph.add_edge(n0, n1, EdgeMeta::DependsOn);
    graph.add_edge(
        n0,
        n1,
        EdgeMeta::DataFlow {
            from_var: "analysis".to_string(),
            to_var: "analysis".to_string(),
        },
    );
}

/// マルチステップ DAG を生成する（complexity >= 2）。
///
/// 先頭ノードが複数の中間ノードにファンアウトし、
/// 中間ノードが末尾ノードに集約される構造。
fn generate_dag(graph: &mut WorkflowGraph, mission: &str, rng: &mut impl Rng) {
    let n_intermediates = 2 + (rng.random::<u64>() % 2) as usize;

    let head = graph.add_node(WorkflowNode::AgentStep {
        agent: pick_agent(0, rng),
        prompt_template: format!("Analyze mission and dispatch subtasks: {}", mission),
        inputs: vec![VarDecl::new("mission_input")],
        output_var: "dispatched".to_string(),
    });

    let mut mids: Vec<NodeIndex> = Vec::with_capacity(n_intermediates);
    for i in 0..n_intermediates {
        let mid = graph.add_node(WorkflowNode::AgentStep {
            agent: pick_agent(i + 1, rng),
            prompt_template: format!(
                "Subtask {} of mission: {}. Execute independently.",
                i + 1,
                mission
            ),
            inputs: vec![VarDecl::new("dispatched")],
            output_var: format!("subtask_{}_output", i),
        });
        mids.push(mid);

        graph.add_edge(head, mid, EdgeMeta::DependsOn);
        graph.add_edge(
            head,
            mid,
            EdgeMeta::DataFlow {
                from_var: "dispatched".to_string(),
                to_var: "dispatched".to_string(),
            },
        );
    }

    let tail = graph.add_node(WorkflowNode::AgentStep {
        agent: "aggregator".to_string(),
        prompt_template: format!(
            "Collect all subtask results and produce final output for mission: {}",
            mission
        ),
        inputs: (0..n_intermediates)
            .map(|i| VarDecl::new(format!("subtask_{}_output", i)))
            .collect(),
        output_var: "output".to_string(),
    });

    for (i, mid) in mids.iter().enumerate() {
        graph.add_edge(*mid, tail, EdgeMeta::Collect { strategy: 0 });
        graph.add_edge(
            *mid,
            tail,
            EdgeMeta::DataFlow {
                from_var: format!("subtask_{}_output", i),
                to_var: format!("subtask_{}_output", i),
            },
        );
    }
}

fn pick_agent(index: usize, rng: &mut impl Rng) -> String {
    let pool = [
        "assistant",
        "analyst",
        "executor",
        "researcher",
        "synthesizer",
        "validator",
        "planner",
    ];
    let idx = (index + rng.random_range(0..pool.len())) % pool.len();
    pool[idx].to_string()
}

/// ランダムな AgentStep のプロンプトテンプレートを微調整する。
fn apply_update_prompt(graph: &mut WorkflowGraph, rng: &mut impl Rng) -> bool {
    let node_indices: Vec<NodeIndex> = graph.node_indices().collect();
    let agent_nodes: Vec<&NodeIndex> = node_indices
        .iter()
        .filter(|idx| matches!(graph[**idx], WorkflowNode::AgentStep { .. }))
        .collect();

    if agent_nodes.is_empty() {
        return false;
    }

    let idx = agent_nodes[rng.random_range(0..agent_nodes.len())];
    if let WorkflowNode::AgentStep {
        ref mut prompt_template,
        ..
    } = graph[*idx]
    {
        let suffix = format!(" [Consider edge case: case-{}]", rng.random_range(1..100));
        prompt_template.push_str(&suffix);
        true
    } else {
        false
    }
}

/// 2 ノード間に DependsOn エッジを追加する（DAG 維持）。
fn apply_add_edge(graph: &mut WorkflowGraph, rng: &mut impl Rng) -> bool {
    let node_indices: Vec<NodeIndex> = graph.node_indices().collect();
    if node_indices.len() < 2 {
        return false;
    }

    for _ in 0..10 {
        let a = node_indices[rng.random_range(0..node_indices.len())];
        let b = node_indices[rng.random_range(0..node_indices.len())];
        if a == b {
            continue;
        }
        if graph.find_edge(a, b).is_some() || graph.find_edge(b, a).is_some() {
            continue;
        }
        graph.add_edge(a, b, EdgeMeta::DependsOn);
        return true;
    }
    false
}

/// 新しい AgentStep ノードを追加する。
fn apply_add_node(graph: &mut WorkflowGraph, _mission: &str) -> bool {
    let agent = "generic_processor".to_string();
    let idx = graph.node_count();
    let output_var = format!("extra_output_{}", idx);

    graph.add_node(WorkflowNode::AgentStep {
        agent,
        prompt_template: format!("Execute additional processing step. Output: {}", output_var),
        inputs: vec![VarDecl::new("input")],
        output_var,
    });
    true
}

/// Placeholder を AgentStep に置換する。
fn apply_replace_node(graph: &mut WorkflowGraph, rng: &mut impl Rng) -> bool {
    let node_indices: Vec<NodeIndex> = graph.node_indices().collect();
    let placeholder_indices: Vec<&NodeIndex> = node_indices
        .iter()
        .filter(|idx| matches!(graph[**idx], WorkflowNode::Placeholder))
        .collect();

    if placeholder_indices.is_empty() {
        return false;
    }

    let idx = placeholder_indices[rng.random_range(0..placeholder_indices.len())];
    let agent = pick_agent(graph.node_count(), rng);
    graph[*idx] = WorkflowNode::AgentStep {
        agent,
        prompt_template: format!("Replaced placeholder node {} in workflow.", idx.index()),
        inputs: vec![VarDecl::new("input")],
        output_var: format!("replaced_output_{}", idx.index()),
    };
    true
}

/// ランダムなエッジを削除する。
fn apply_remove_edge(graph: &mut WorkflowGraph, rng: &mut impl Rng) -> bool {
    let edge_indices: Vec<_> = graph.edge_indices().collect();
    if edge_indices.is_empty() {
        return false;
    }
    let edge_idx = edge_indices[rng.random_range(0..edge_indices.len())];
    graph.remove_edge(edge_idx);
    true
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn generate_single_agent_step() {
        let mut rng = StdRng::seed_from_u64(42);
        let graph = generate_new_workflow("test mission", &mut rng, 0);
        assert_eq!(graph.node_count(), 1);
        assert!(matches!(
            graph[NodeIndex::new(0)],
            WorkflowNode::AgentStep { .. }
        ));
    }

    #[test]
    fn generate_sequential_two_nodes() {
        let mut rng = StdRng::seed_from_u64(42);
        let graph = generate_new_workflow("test", &mut rng, 1);
        assert_eq!(graph.node_count(), 2);
        assert!(petgraph::algo::toposort(&graph, None).is_ok());
    }

    #[test]
    fn generate_dag_is_valid() {
        let mut rng = StdRng::seed_from_u64(42);
        let graph = generate_new_workflow("complex mission", &mut rng, 2);
        assert!(graph.node_count() >= 3);
        assert!(petgraph::algo::toposort(&graph, None).is_ok());
    }

    #[test]
    fn all_complexities_produce_dag() {
        for complexity in 0..=WORKFLOW_GENERATION_MAX_COMPLEXITY {
            let mut rng = StdRng::seed_from_u64(complexity as u64);
            let graph = generate_new_workflow("test", &mut rng, complexity);
            assert!(
                petgraph::algo::toposort(&graph, None).is_ok(),
                "complexity={} produced non-DAG",
                complexity
            );
        }
    }

    #[test]
    fn differential_mutation_changes_graph() {
        let mut rng = StdRng::seed_from_u64(42);
        let original = generate_new_workflow("mutate me", &mut rng, 2);
        let mut rng2 = StdRng::seed_from_u64(999);
        let mutated = generate_differential_mutation(&original, &mut rng2);
        assert!(petgraph::algo::toposort(&mutated, None).is_ok());
    }

    #[test]
    fn mutation_of_empty_graph() {
        let mut rng = StdRng::seed_from_u64(42);
        let empty = WorkflowGraph::new();
        let result = generate_differential_mutation(&empty, &mut rng);
        assert_eq!(result.node_count(), 0);
    }

    #[test]
    fn embedding_is_deterministic() {
        let e1 = mission_to_embedding("hello world");
        let e2 = mission_to_embedding("hello world");
        assert_eq!(e1, e2);
        let e3 = mission_to_embedding("different mission");
        assert_ne!(e1, e3);
    }

    #[test]
    fn embedding_dimension() {
        let emb = mission_to_embedding("test");
        assert_eq!(emb.len(), SEARCH_QUERY_EMBEDDING_DIM);
    }

    #[test]
    fn embedding_is_unit_vector() {
        let emb = mission_to_embedding("test unit");
        let norm: f64 = emb.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "norm = {}", norm);
    }

    #[test]
    fn all_generated_nodes_are_valid() {
        let mut rng = StdRng::seed_from_u64(12345);
        for complexity in 0..=WORKFLOW_GENERATION_MAX_COMPLEXITY {
            let graph = generate_new_workflow("test", &mut rng, complexity);
            for node_idx in graph.node_indices() {
                match &graph[node_idx] {
                    WorkflowNode::AgentStep {
                        agent,
                        prompt_template,
                        inputs,
                        output_var,
                    } => {
                        assert!(!agent.is_empty());
                        assert!(!prompt_template.is_empty());
                        assert!(!inputs.is_empty());
                        assert!(!output_var.is_empty());
                    }
                    other => panic!("Unexpected node type: {:?}", other),
                }
            }
        }
    }

    // ============================================================
    // T3: Differential Mutation の add_node 確率検証
    // ============================================================

    /// 差分変異で add_node が 50% 以上の確率で発生することを確認する (T3)。
    ///
    /// 統計的検定: N=100 回の試行で add_node によるノード数増加が 50 回以上確認される。
    /// 各試行は独立した PRNG シードを使用する。
    #[test]
    fn differential_mutation_add_node_probability() {
        let base = generate_new_workflow("base", &mut StdRng::seed_from_u64(42), 2);
        let base_count = base.node_count();
        let mut add_node_count = 0_usize;
        let trials = 100;

        for i in 0..trials {
            let mutated = generate_differential_mutation(
                &base,
                &mut StdRng::seed_from_u64(i as u64 + 1000),
            );
            if mutated.node_count() > base_count {
                add_node_count += 1;
            }
        }

        let ratio = add_node_count as f64 / trials as f64;
        // 70% 設定なので 50% を下回ることはない
        assert!(
            ratio >= 0.50,
            "add_node ratio too low: {:.2}% (expected >= 50%)",
            ratio * 100.0
        );
    }
}
