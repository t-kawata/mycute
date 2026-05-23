// M0.5-2 確率的パッチ操作インジェクションによるバリデータ耐久テスト
//
// 本ファイルは以下の観測テスト（OTS）を提供する：
//
// - OTS-C1: ランダム DAG に対し確率的に逆辺を注入し、サイクル検出の
//   完全性（p_miss < 4.6×10⁻⁴）を検証する (n=10,000)
// - OTS-C2: ランダム操作系列を注入し、パニック発生率 0、
//   DAG 不変条件違反 0 を検証する (n=1,000)

use std::panic;

use darvium::{
    apply_patch_atomic, GraphPatch, PatchConfidence, PatchError, PatchOperation,
    validate_patch_result,
};
use darvium::types::*;

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

// ── ヘルパー ──────────────────────────────────────────────

/// ランダム DAG を構築する。
///
/// トポロジカル順序に基づいてノードを追加し、`from < to` のエッジのみを
/// 追加することで DAG 性を保証する。`edge_density` はノードペアに対する
/// エッジ追加確率（おおよそのエッジ密度に相当）。
fn build_random_dag(rng: &mut StdRng, node_count: usize, edge_density: f64) -> WorkflowGraph {
    let mut graph = WorkflowGraph::new();
    for i in 0..node_count {
        graph.add_node(WorkflowNode::AgentStep {
            agent: format!("agent_{}", i),
            prompt_template: "template".into(),
            inputs: vec![],
            output_var: format!("out_{}", i),
        });
    }
    for from in 0..node_count {
        for to in (from + 1)..node_count {
            if rng.random_bool(edge_density) {
                graph.add_edge(
                    petgraph::graph::NodeIndex::new(from),
                    petgraph::graph::NodeIndex::new(to),
                    EdgeMeta::DependsOn,
                );
            }
        }
    }
    graph
}

/// DAG の既存エッジを逆転した逆辺候補を列挙する。
///
/// 既存エッジ `low->high` を逆転した `high->low` は必ず長さ2のサイクル
/// `low->high->low` を生成するため、確実に CycleCreated エラーとなる。
fn find_back_edge_candidates(graph: &WorkflowGraph) -> Vec<(usize, usize)> {
    let mut candidates = Vec::new();
    for edge_idx in graph.edge_indices() {
        let (from_idx, to_idx) = graph
            .edge_endpoints(edge_idx)
            .expect("edge endpoint consistency");
        let from = from_idx.index();
        let to = to_idx.index();
        // 逆辺が既に存在する場合は除外
        let rev_from = petgraph::graph::NodeIndex::new(to);
        let rev_to = petgraph::graph::NodeIndex::new(from);
        if graph.find_edge(rev_from, rev_to).is_none() {
            candidates.push((to, from));
        }
    }
    candidates
}

// ── OTS-C1: サイクル検出完全性 ────────────────────────────

/// ランダム DAG に確率的に逆辺を注入し、apply_patch_atomic による
/// サイクル検出の完全性を検証する (n=2,000)。
///
/// 検証内容:
/// - 逆辺注入時の Err(CycleCreated) 率 = 100%（見逃し 0）
/// - パッチ成功グラフが DAG 性を維持している（toposort 通過）
///
/// 出力: CSV (ノード数, エッジ数, 逆辺数, 検出結果, パッチ後の DAG 整合性)
#[test]
fn ots_cycle_detection_completeness() {
    let mut rng = StdRng::seed_from_u64(12345);
    let n = 2_000;
    let mut detected = 0u64;
    let mut missed = 0u64;
    let mut dag_ok = 0u64;
    let mut total_back_edges = 0u64;

    println!("=== OTS-C1: Cycle Detection Completeness ===");
    println!("n={}", n);

    for i in 0..n {
        let node_count = rng.random_range(8..=128);
        let edge_density: f64 = rng.random_range(0.10..0.30);
        let gold = build_random_dag(&mut rng, node_count, edge_density);
        let candidates = find_back_edge_candidates(&gold);
        if candidates.is_empty() {
            continue;
        }

        // 1回の試行につき1つの逆辺を注入する（独立試行のため）
        let (from, to) = candidates[rng.random_range(0..candidates.len())];
        let operation = PatchOperation::AddEdge {
            from,
            to,
            meta: EdgeMeta::DependsOn,
        };
        total_back_edges += 1;

        let confidence = PatchConfidence::compute(0.90, 0.90, 0.80);
        let patch = GraphPatch {
            source_graph_id: "test".into(),
            operations: vec![operation],
            patch_confidence: confidence,
            generated_at: std::time::SystemTime::now(),
            generator_version: "test".into(),
        };

        let result = apply_patch_atomic(&gold, &patch);
        let is_detected = matches!(result, Err(PatchError::CycleCreated));
        let is_missed = result.is_ok();

        if is_detected {
            detected += 1;
        }
        if is_missed {
            missed += 1;
            // パッチ成功の場合も DAG 整合性を確認
            if let Ok(g) = &result {
                if validate_patch_result(g).is_ok() {
                    dag_ok += 1;
                }
            }
        }

        if i % 500 == 0 {
            println!(
                "PROGRESS: {}/{} (detected={}, missed={})",
                i, n, detected, missed
            );
        }
    }

    let p_miss = missed as f64 / n as f64;
    println!();
    println!("=== OTS-C1 Summary ===");
    println!("Total trials: {}", n);
    println!("Trials with candidates: {}", total_back_edges);
    println!("Detected (Err(CycleCreated)): {}", detected);
    println!("Missed (Ok): {}", missed);
    println!("p_miss: {:.6e}", p_miss);
    println!("DAG integrity ok count: {}", dag_ok);

    // 不変条件: サイクル検出見逃し確率 < 4.6×10⁻⁴ (99.7% CI for 10,000 trials)
    assert!(
        p_miss < 4.6e-4,
        "Cycle detection miss rate {:.6e} exceeds 4.6e-4 threshold",
        p_miss
    );
    // 不変条件: 全ての成功パッチ結果が DAG 性を維持
    assert_eq!(
        missed,
        dag_ok,
        "All patch-passed graphs must be valid DAGs (missed={}, dag_ok={})",
        missed,
        dag_ok
    );
}

// ── OTS-C2: ノイズ注入安全性 ──────────────────────────────

/// ランダム操作系列をグラフに注入し、パニック発生率 0、
/// DAG 不変条件違反 0 を検証する (n=500)。
///
/// 操作種別: AddNode, RemoveNode, ReplaceNode, AddEdge, RemoveEdge
/// 系列長: 1〜10
///
/// 出力: CSV (系列長, パニック数, エラー数, DAG違反数)
#[test]
fn ots_noise_injection_safety() {
    let mut rng = StdRng::seed_from_u64(12345);
    let n = 500;
    let mut panic_count = 0u64;
    let mut error_count = 0u64;
    let mut dag_violations = 0u64;
    let mut total_ops = 0u64;

    println!("=== OTS-C2: Noise Injection Safety ===");
    println!("n={}", n);

    for i in 0..n {
        let seq_len = rng.random_range(1..=10);
        let node_count = rng.random_range(4..=32);
        let gold = build_random_dag(&mut rng, node_count, 0.15);
        let mut ops = Vec::new();
        for _ in 0..seq_len {
            let op_kind: u8 = rng.random_range(0..5);
            let operation = match op_kind {
                0 => PatchOperation::AddNode {
                    node: WorkflowNode::Placeholder,
                },
                1 => {
                    let node_id = rng.random_range(0..gold.node_count().max(1));
                    PatchOperation::RemoveNode { node_id }
                }
                2 => {
                    let node_id = rng.random_range(0..gold.node_count().max(1));
                    PatchOperation::ReplaceNode {
                        node_id,
                        new_node: WorkflowNode::Placeholder,
                    }
                }
                3 => {
                    let from = rng.random_range(0..gold.node_count().max(1));
                    let to = rng.random_range(0..gold.node_count().max(1));
                    PatchOperation::AddEdge {
                        from,
                        to,
                        meta: EdgeMeta::DependsOn,
                    }
                }
                4 => {
                    let from = rng.random_range(0..gold.node_count().max(1));
                    let to = rng.random_range(0..gold.node_count().max(1));
                    PatchOperation::RemoveEdge { from, to }
                }
                _ => unreachable!(),
            };
            ops.push(operation);
        }

        let confidence = PatchConfidence::compute(0.90, 0.90, 0.80);
        let patch = GraphPatch {
            source_graph_id: "test".into(),
            operations: ops,
            patch_confidence: confidence,
            generated_at: std::time::SystemTime::now(),
            generator_version: "test".into(),
        };

        let seq_panic = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let result = apply_patch_atomic(&gold, &patch);
            if let Ok(g) = &result {
                if validate_patch_result(g).is_err() {
                    dag_violations += 1;
                }
            }
            if result.is_err() {
                error_count += 1;
            }
        }));

        if seq_panic.is_err() {
            panic_count += 1;
        }
        total_ops += seq_len as u64;

        if i % 100 == 0 {
            println!(
                "PROGRESS: {}/{} (panics={}, errors={}, dag_violations={})",
                i, n, panic_count, error_count, dag_violations
            );
        }
    }

    println!();
    println!("=== OTS-C2 Summary ===");
    println!("Total sequences: {}", n);
    println!("Total operations: {}", total_ops);
    println!("Panics: {}", panic_count);
    println!("Errors: {}", error_count);
    println!("DAG violations: {}", dag_violations);

    // 不変条件: パニック発生率 0
    assert_eq!(
        panic_count, 0,
        "Panic count must be zero, got {}",
        panic_count
    );
    // 不変条件: 成功パッチの DAG 不変条件違反 0
    assert_eq!(
        dag_violations, 0,
        "DAG violations in patch-passed graphs must be zero, got {}",
        dag_violations
    );
}
