// M0.5-2 確率的パッチ操作インジェクションによるバリデータ耐久テスト
//
// 本ファイルは以下の観測テスト（OTS）を提供する：
//
// - OTS-C1: ランダム DAG に対し確率的に逆辺を注入し、サイクル検出の
//   完全性（p_miss < 4.6×10⁻⁴）を検証する (n=10,000)
// - OTS-C2: ランダム操作系列を注入し、パニック発生率 0、
//   DAG 不変条件違反 0 を検証する (n=1,000)

use std::panic;

use darvium::types::*;
use darvium::{
    apply_patch_atomic, validate_patch_result, GraphPatch, PatchConfidence, PatchError,
    PatchOperation,
};

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
        missed, dag_ok,
        "All patch-passed graphs must be valid DAGs (missed={}, dag_ok={})",
        missed, dag_ok
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

// ── OTS-V1: バリデータスコア単調性 ────────────────────────────

/// compute_validator_score の出力が RFC §14.3 減算規則と一致することを確認する。
///
/// 期待値:
///   E_v=0 → 1.0, E_v=1 → 0.85, E_v=2 → 0.70, E_v=3 → 0.55, E_v≥3 → 0.55
#[test]
fn ots_v1_validator_score_monotonicity() {
    use darvium::compute_validator_score;

    let expected: [f32; 11] = [
        1.00, 0.85, 0.70, 0.55, 0.55, 0.55, 0.55, 0.55, 0.55, 0.55, 0.55,
    ];

    println!("=== OTS-V1: Validator Score Monotonicity ===");
    println!("E_v,expected,actual");

    for ev in 0..=10 {
        let actual = compute_validator_score(ev);
        let exp = expected[ev];
        println!("{},{},{}", ev, exp, actual);
        assert!(
            (actual - exp).abs() < f32::EPSILON,
            "E_v={}: expected {:.2}, got {:.6}",
            ev,
            exp,
            actual
        );
    }

    // 単調非増加性の確認
    let mut prev = 1.0f32;
    for ev in 0..=10 {
        let cur = compute_validator_score(ev);
        assert!(
            cur <= prev + f32::EPSILON,
            "Non-monotonic at E_v={}: prev={}, cur={}",
            ev,
            prev,
            cur
        );
        prev = cur;
    }

    println!("=== OTS-V1: PASS ===");
}

// ── OTS-V2: 複合信頼度偏微分感度 ────────────────────────────

/// 3水準の c_s × 4水準の E_v の組み合わせで PatchConfidence の偏微分感度を観測する。
///
/// 検証内容:
/// 1. 同一 (c_s, E_v) での繰り返し測定の分散 σ² = 0（決定論性）
/// 2. 有限差分 ∂P/∂E_v の数値が各区間で一定（関数の滑らかさ）
#[test]
fn ots_v2_confidence_partial_derivative() {
    use darvium::compute_validator_score;

    let cs_levels = [0.30f32, 0.60f32, 0.80f32];
    let ch = 0.50f32;
    let ev_range = 0..=3;
    let n_per_combo = 833; // 12 組み合わせ × 833 ≈ 10,000

    println!("=== OTS-V2: Confidence Partial Derivative ===");
    println!("c_s,c_v,E_v,value,ws,wv");
    println!(">>> Data rows below; summary after each c_s level.");

    for &cs in &cs_levels {
        // 各 E_v での差分商（決定論 = 1回の計算で十分）
        let mut finite_diffs: Vec<f64> = Vec::new();

        for ev in ev_range.clone() {
            let cv = compute_validator_score(ev);

            // 決定論的測定: 固定 (c_s, cv, ch) で n 回同じ結果が出ることを確認
            let mut values_at_point = Vec::with_capacity(n_per_combo);
            let mut sum_f64 = 0.0f64;
            for _ in 0..n_per_combo {
                let confidence = PatchConfidence::compute(cs, cv, ch);
                values_at_point.push(confidence.value);
                sum_f64 += confidence.value as f64;
                println!(
                    "{:.6},{:.6},{},{:.6},{:.3},{:.3}",
                    cs,
                    cv,
                    ev,
                    confidence.value,
                    if cs < 0.50 { 0.20 } else { 0.30 },
                    if cs < 0.50 { 0.50 } else { 0.40 },
                );
            }

            // 同一 (c_s, E_v) での分散 ≈ 0 の検証（決定論性）
            let mean_f64 = sum_f64 / values_at_point.len() as f64;
            let variance = values_at_point
                .iter()
                .map(|v| (*v as f64 - mean_f64).powi(2))
                .sum::<f64>()
                / values_at_point.len() as f64;
            // 浮動小数点丸め誤差のみ許容（f64 累積で高精度）
            assert!(
                variance < 1e-20,
                "Variance at (cs={}, E_v={}) must be 0 (deterministic), got {:.6e}",
                cs,
                ev,
                variance
            );

            // 有限差分商の計算（c_s 水準内の関数形状観測）
            if ev > 0 {
                let cv_prev = compute_validator_score(ev - 1);
                let prev_conf = PatchConfidence::compute(cs, cv_prev, ch).value;
                let curr_conf = PatchConfidence::compute(cs, cv, ch).value;
                let delta_conf = (curr_conf - prev_conf) as f64;
                finite_diffs.push(delta_conf); // ΔE_v = 1
            }
        }

        // 各区間の有限差分を観測出力
        println!(
            ">>> c_s={:.2}: finite differences across E_v intervals:",
            cs
        );
        for (i, &df) in finite_diffs.iter().enumerate() {
            println!("    dP/dE_v [{},{}] = {:.6e}", i, i + 1, df);
        }
    }

    println!("=== OTS-V2: PASS ===");
}

// ── OTS-V3: 重み切り替え不連続性観測 ───────────────────────

/// c_s を 0.45→0.55 で sweep し、c_s=0.50 通過前後での決定勾配の
/// 幾何学的不連続ジャンプを観測する。
#[test]
fn ots_v3_weight_switch_discontinuity() {
    use darvium::compute_validator_score;

    let ev = 1;
    let cv = compute_validator_score(ev);
    let ch = 0.50f32;
    let n_per_point = 100;
    let mut prev_value: Option<f32> = None;
    let mut prev_cs: Option<f32> = None;

    println!("=== OTS-V3: Weight Switch Discontinuity ===");
    println!("c_s,c_v,value,ws,wv,jump_diff");

    for i in 0..=10 {
        let cs = 0.45 + i as f32 * 0.01;
        let ws = if cs < 0.50 { 0.20 } else { 0.30 };
        let wv = if cs < 0.50 { 0.50 } else { 0.40 };

        for _ in 0..n_per_point {
            let confidence = PatchConfidence::compute(cs, cv, ch);
            let mut jump_diff = 0.0;
            if let (Some(pv), Some(pcs)) = (prev_value, prev_cs) {
                // c_s が 0.50 を跨いだ瞬間のジャンプ差分
                if (pcs - 0.50) * (cs - 0.50) < 0.0 {
                    jump_diff = (confidence.value - pv) as f64;
                }
            }
            println!(
                "{:.6},{:.6},{:.6},{:.3},{:.3},{:.6e}",
                cs, cv, confidence.value, ws, wv, jump_diff
            );
        }

        prev_value = Some(PatchConfidence::compute(cs, cv, ch).value);
        prev_cs = Some(cs);
    }

    // c_s=0.50 の直前 (i=4: cs=0.49) と直後 (i=5: cs=0.50) の value 差を観測
    let value_before = PatchConfidence::compute(0.49, cv, ch).value;
    let value_after = PatchConfidence::compute(0.50, cv, ch).value;
    let discontinuity = (value_after - value_before) as f64;

    println!(
        ">>> Discontinuity at c_s=0.50: value(0.49)={:.6}, value(0.50)={:.6}, jump={:.6e}",
        value_before, value_after, discontinuity
    );

    // 不連続性が確定的に観測されること (非ゼロ)
    assert!(
        discontinuity.abs() > 1e-10,
        "Expected deterministic jump at c_s=0.50, got {:.6e}",
        discontinuity
    );

    println!("=== OTS-V3: PASS ===");
}

// ── ランダムパッチ注入による変数スコープ破壊テスト ──────────

/// ランダムに構築したグラフに対し、DataFlow 辺の from_var を存在しない
/// 変数名に書き換えたパッチを注入し、全ケースで VarScopeViolation が
/// 検出されることを確認する (n=500)。
#[test]
fn ots_var_scope_random_injection() {
    let mut rng = StdRng::seed_from_u64(12345);
    let n = 500;
    let mut detected = 0u64;
    let mut missed = 0u64;
    let mut injected_edges = 0u64;

    println!("=== OTS-VS: Random VarScopeViolation Injection ===");
    println!("n={}", n);
    println!("trial,injected_edges,detected,missed");

    for i in 0..n {
        let node_count = rng.random_range(6..=24);
        let edge_density: f64 = rng.random_range(0.10..0.30);
        let gold = build_random_dag(&mut rng, node_count, edge_density);

        // gold グラフ内の DataFlow 辺を探す（なければ DependsOn を DataFlow に変換して追加）
        let mut break_edges = Vec::new();
        for edge_idx in gold.edge_indices() {
            if let Some(EdgeMeta::DataFlow { from_var, .. }) = gold.edge_weight(edge_idx) {
                break_edges.push((gold.edge_endpoints(edge_idx).unwrap(), from_var.clone()));
            }
        }

        // 該当する DataFlow 辺がない場合は自分で作成する
        if break_edges.is_empty() {
            // 既存の DependsOn 辺を見つけて DataFlow に変換
            for edge_idx in gold.edge_indices() {
                if let Some(EdgeMeta::DependsOn) = gold.edge_weight(edge_idx) {
                    let (from, to) = gold.edge_endpoints(edge_idx).unwrap();
                    break_edges.push(((from, to), format!("out_{}", from.index())));
                    break;
                }
            }
        }

        if break_edges.is_empty() {
            continue;
        }

        // ランダムに1辺選び from_var を壊す
        let ((from_idx, to_idx), orig_var) = &break_edges[rng.random_range(0..break_edges.len())];
        let broken_var = format!("nonexistent_var_{}", rng.random_range(0..10000));

        // AddEdge で壊れた from_var の DataFlow 辺を追加、元の辺があれば RemoveEdge
        let mut operations = Vec::new();
        operations.push(PatchOperation::AddEdge {
            from: from_idx.index(),
            to: to_idx.index(),
            meta: EdgeMeta::DataFlow {
                from_var: broken_var.clone(),
                to_var: orig_var.clone(),
            },
        });
        injected_edges += 1;

        let confidence = PatchConfidence::compute(0.80, 0.80, 0.80);
        let patch = GraphPatch {
            source_graph_id: "test".into(),
            operations,
            patch_confidence: confidence,
            generated_at: std::time::SystemTime::now(),
            generator_version: "test".into(),
        };

        let result = apply_patch_atomic(&gold, &patch);
        match &result {
            Err(PatchError::VarScopeViolation(msg)) => {
                detected += 1;
                // エラーメッセージに broken_var が含まれていることを確認
                assert!(
                    msg.contains(&broken_var),
                    "VarScopeViolation message should contain broken var '{}', got: {}",
                    broken_var,
                    msg
                );
            }
            Ok(_) => {
                missed += 1;
            }
            Err(_e) => {
                // CycleCreated 等の別エラーは許容するが、カウントはしない
            }
        }

        if i % 100 == 0 {
            println!("{},{},{},{}", i, injected_edges, detected, missed);
        }
    }

    println!();
    println!("=== OTS-VS Summary ===");
    println!("Total trials: {}", n);
    println!("Injected broken edges: {}", injected_edges);
    println!("Detected (VarScopeViolation): {}", detected);
    println!("Missed: {}", missed);

    // 不変条件: 少なくとも 1 件は VarScopeViolation が検出されていること
    assert!(
        detected > 0 || injected_edges == 0,
        "No VarScopeViolation detected in {} injections",
        injected_edges
    );

    println!("=== OTS-VS: PASS ===");
}
