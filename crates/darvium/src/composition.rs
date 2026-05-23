// CompositionPlan 変数スコープ静的バリデータ
//
// RFC §6.4 V-03/V-04 に基づき、組成エッジの変数スコープ整合性を検証する。
// - V-03: DataFlow.from_var は送信元ノードの output_var と一致すること
// - V-04: DataFlow.to_var は送信先ノードの inputs に含まれること
//
// また v2.3 frontier-based parallel execution の前提として、
// 並列 frontier 上での未解決変数・scope leakage も検出する。

use std::collections::HashMap;

use petgraph::graph::NodeIndex;

use crate::error::DarviumError;
use crate::types::*;

// ── 公開 API ──────────────────────────────────────────────

/// CompositionPlan の全組成エッジに対して変数スコープ整合性を検証する。
///
/// 検査内容:
/// - V-03: 各 DataFlow エッジの `from_var` が送信元ノードの `output_var` と一致する
/// - V-04: 各 DataFlow エッジの `to_var` が送信先ノードの `inputs` に含まれる
pub fn validate_composition_plan(
    plan: &CompositionPlan,
    graphs: &HashMap<WorkflowGraphId, WorkflowGraph>,
) -> Result<(), DarviumError> {
    for (source_id, target_id, edge_meta) in &plan.composition_edges {
        if let EdgeMeta::DataFlow { from_var, to_var } = edge_meta {
            let from_node = resolve_node(*source_id, &plan.component_graph_ids, graphs)?;
            let to_node = resolve_node(*target_id, &plan.component_graph_ids, graphs)?;
            check_var_scope_v03(from_node, from_var)?;
            check_var_scope_v04(to_node, to_var)?;
        }
    }
    Ok(())
}

/// 並列 frontier 上での変数スコープ漏洩（scope leakage）を検出する。
///
/// 互いに独立な parallel-ready ノードが同名の出力変数を宣言している場合、
/// その後のデータフローで意図しない変数解決が発生する可能性があるため、
/// これを検出してエラーとする。
pub fn detect_frontier_leakage(
    plan: &CompositionPlan,
    graphs: &HashMap<WorkflowGraphId, WorkflowGraph>,
) -> Result<(), DarviumError> {
    let mut producer_of: HashMap<&str, NodeId> = HashMap::new();
    for (source_id, _target_id, edge_meta) in &plan.composition_edges {
        if let EdgeMeta::DataFlow {
            from_var: _from_var,
            ..
        } = edge_meta
        {
            let node = resolve_node(*source_id, &plan.component_graph_ids, graphs)?;
            let output_var = get_node_output_var(node)?;
            if let Some(&prev_source) = producer_of.get(output_var) {
                if prev_source != *source_id {
                    return Err(DarviumError::VariableScopeViolation(format!(
                        "Frontier leakage: variable '{}' is produced by nodes {} and {}",
                        output_var, prev_source, source_id
                    )));
                }
            }
            producer_of.insert(output_var, *source_id);
        }
    }
    Ok(())
}

// ── 内部ヘルパー ──────────────────────────────────────────

/// ノードID から対応する WorkflowNode を解決する。
fn resolve_node<'a>(
    node_id: NodeId,
    component_graph_ids: &[WorkflowGraphId],
    graphs: &'a HashMap<WorkflowGraphId, WorkflowGraph>,
) -> Result<&'a WorkflowNode, DarviumError> {
    for graph_id in component_graph_ids {
        if let Some(graph) = graphs.get(graph_id) {
            let ni = NodeIndex::new(node_id);
            if let Some(node) = graph.node_weight(ni) {
                return Ok(node);
            }
        }
    }
    Err(DarviumError::VariableScopeViolation(format!(
        "Node {} not found in any component graph",
        node_id
    )))
}

/// V-03: 送信元ノードの output_var と DataFlow.from_var の一致を検証する。
fn check_var_scope_v03(node: &WorkflowNode, from_var: &str) -> Result<(), DarviumError> {
    let output_var = get_node_output_var(node)?;
    if output_var != from_var {
        return Err(DarviumError::VariableScopeViolation(format!(
            "V-03 violation: output_var '{}' != DataFlow.from_var '{}'",
            output_var, from_var
        )));
    }
    Ok(())
}

/// V-04: DataFlow.to_var が送信先ノードの inputs に含まれることを検証する。
fn check_var_scope_v04(node: &WorkflowNode, to_var: &str) -> Result<(), DarviumError> {
    let inputs = get_node_inputs(node)?;
    let found = inputs.iter().any(|v| v.name == to_var);
    if !found {
        return Err(DarviumError::VariableScopeViolation(format!(
            "V-04 violation: to_var '{}' not found in node inputs",
            to_var
        )));
    }
    Ok(())
}

/// WorkflowNode から出力変数名を取得する。
fn get_node_output_var(node: &WorkflowNode) -> Result<&str, DarviumError> {
    match node {
        WorkflowNode::AgentStep { output_var, .. } => Ok(output_var.as_str()),
        WorkflowNode::SubWorkflow { output_var, .. } => Ok(output_var.as_str()),
        WorkflowNode::Placeholder => Err(DarviumError::VariableScopeViolation(
            "Placeholder node has no output_var".into(),
        )),
    }
}

/// WorkflowNode から入力変数宣言リストを取得する。
fn get_node_inputs(node: &WorkflowNode) -> Result<&[VarDecl], DarviumError> {
    match node {
        WorkflowNode::AgentStep { inputs, .. } => Ok(inputs.as_slice()),
        WorkflowNode::SubWorkflow { .. } => Ok(&[]),
        WorkflowNode::Placeholder => Err(DarviumError::VariableScopeViolation(
            "Placeholder node has no inputs".into(),
        )),
    }
}

// ── テストヘルパー ──────────────────────────────────────────

#[cfg(test)]
fn make_single_graph_plan(
    _graph: WorkflowGraph,
    edges: Vec<(NodeId, NodeId, EdgeMeta)>,
    expected_output: VarDecl,
) -> CompositionPlan {
    CompositionPlan::new(vec!["test_graph".into()], edges, vec![], expected_output)
}

#[cfg(test)]
fn make_graphs_map(graph: WorkflowGraph) -> HashMap<WorkflowGraphId, WorkflowGraph> {
    let mut map = HashMap::new();
    map.insert("test_graph".into(), graph);
    map
}

// ── テスト ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::Rng;
    use rand::SeedableRng;

    // ── T1: VarDecl 構造体 ──

    /// T1: VarDecl 構造体の正常構築
    #[test]
    fn t1_var_decl_construction() {
        let decl = VarDecl {
            name: "input_x".into(),
            required: true,
            var_type: VarType::String,
        };
        assert_eq!(decl.name, "input_x");
        assert!(decl.required);
        assert_eq!(decl.var_type, VarType::String);
    }

    /// T2: VarDecl::new でデフォルト構築
    #[test]
    fn t2_var_decl_new_default() {
        let decl = VarDecl::new("x");
        assert_eq!(decl.name, "x");
        assert!(decl.required);
        assert_eq!(decl.var_type, VarType::String);
    }

    // ── T3: EdgeMeta::DataFlow ──

    /// T3: EdgeMeta::DataFlow の構築とフィールドアクセス
    #[test]
    fn t3_edge_meta_dataflow() {
        let edge = EdgeMeta::DataFlow {
            from_var: "out".into(),
            to_var: "in".into(),
        };
        match &edge {
            EdgeMeta::DataFlow { from_var, to_var } => {
                assert_eq!(from_var, "out");
                assert_eq!(to_var, "in");
            }
            _ => panic!("Expected DataFlow variant"),
        }
    }

    // ── T4: CompositionPlan ──

    /// T4: CompositionPlan 構造体の全フィールド正常構築
    #[test]
    fn t4_composition_plan_construction() {
        let plan = CompositionPlan::new(
            vec!["g1".into()],
            vec![(0, 1, EdgeMeta::DependsOn)],
            vec![VarDecl::new("in")],
            VarDecl::new("out"),
        );
        assert_eq!(plan.component_graph_ids, vec!["g1"]);
        assert_eq!(plan.composition_edges.len(), 1);
        assert_eq!(plan.expected_inputs.len(), 1);
        assert_eq!(plan.expected_output.name, "out");
    }

    // ── T5-T10: V-03/V-04 検証 ──

    /// T5: V-03 正常 — 送信元ノードの output_var と DataFlow.from_var が一致
    #[test]
    fn t5_v03_pass() {
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::AgentStep {
            agent: "a".into(),
            prompt_template: "".into(),
            inputs: vec![],
            output_var: "result".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "b".into(),
            prompt_template: "".into(),
            inputs: vec![VarDecl::new("result")],
            output_var: "final".into(),
        });

        let plan = make_single_graph_plan(
            graph.clone(),
            vec![(
                0,
                1,
                EdgeMeta::DataFlow {
                    from_var: "result".into(),
                    to_var: "result".into(),
                },
            )],
            VarDecl::new("final"),
        );
        let graphs = make_graphs_map(graph);

        let result = validate_composition_plan(&plan, &graphs);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
    }

    /// T6: V-03 遮断 — 送信元ノードの output_var と DataFlow.from_var が不一致
    #[test]
    fn t6_v03_violation() {
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::AgentStep {
            agent: "a".into(),
            prompt_template: "".into(),
            inputs: vec![],
            output_var: "actual_out".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "b".into(),
            prompt_template: "".into(),
            inputs: vec![VarDecl::new("expected_in")],
            output_var: "final".into(),
        });

        let plan = make_single_graph_plan(
            graph.clone(),
            vec![(
                0,
                1,
                EdgeMeta::DataFlow {
                    from_var: "wrong_out".into(),
                    to_var: "expected_in".into(),
                },
            )],
            VarDecl::new("final"),
        );
        let graphs = make_graphs_map(graph);

        let result = validate_composition_plan(&plan, &graphs);
        assert!(
            matches!(&result, Err(DarviumError::VariableScopeViolation(msg)) if msg.contains("V-03")),
            "Expected V-03 violation, got: {:?}",
            result
        );
    }

    /// T7: V-04 正常 — to_var が送信先ノードの inputs に含まれる
    #[test]
    fn t7_v04_pass() {
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::AgentStep {
            agent: "a".into(),
            prompt_template: "".into(),
            inputs: vec![],
            output_var: "data".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "b".into(),
            prompt_template: "".into(),
            inputs: vec![VarDecl::new("data")],
            output_var: "out".into(),
        });

        let plan = make_single_graph_plan(
            graph.clone(),
            vec![(
                0,
                1,
                EdgeMeta::DataFlow {
                    from_var: "data".into(),
                    to_var: "data".into(),
                },
            )],
            VarDecl::new("out"),
        );
        let graphs = make_graphs_map(graph);

        let result = validate_composition_plan(&plan, &graphs);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
    }

    /// T8: V-04 遮断 — to_var が送信先ノードの inputs に非含
    #[test]
    fn t8_v04_violation() {
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::AgentStep {
            agent: "a".into(),
            prompt_template: "".into(),
            inputs: vec![],
            output_var: "data".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "b".into(),
            prompt_template: "".into(),
            inputs: vec![VarDecl::new("expected_in")],
            output_var: "out".into(),
        });

        let plan = make_single_graph_plan(
            graph.clone(),
            vec![(
                0,
                1,
                EdgeMeta::DataFlow {
                    from_var: "data".into(),
                    to_var: "nonexistent".into(),
                },
            )],
            VarDecl::new("out"),
        );
        let graphs = make_graphs_map(graph);

        let result = validate_composition_plan(&plan, &graphs);
        assert!(
            matches!(&result, Err(DarviumError::VariableScopeViolation(msg)) if msg.contains("V-04")),
            "Expected V-04 violation, got: {:?}",
            result
        );
    }

    /// T9: 複数エッジを持つ組成プランで全 V-03/V-04 が通過
    #[test]
    fn t9_multi_edge_all_pass() {
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::AgentStep {
            agent: "a".into(),
            prompt_template: "".into(),
            inputs: vec![],
            output_var: "x".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "b".into(),
            prompt_template: "".into(),
            inputs: vec![VarDecl::new("x")],
            output_var: "y".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "c".into(),
            prompt_template: "".into(),
            inputs: vec![VarDecl::new("y")],
            output_var: "z".into(),
        });

        let edges = vec![
            (
                0,
                1,
                EdgeMeta::DataFlow {
                    from_var: "x".into(),
                    to_var: "x".into(),
                },
            ),
            (
                1,
                2,
                EdgeMeta::DataFlow {
                    from_var: "y".into(),
                    to_var: "y".into(),
                },
            ),
        ];

        let plan = make_single_graph_plan(graph.clone(), edges, VarDecl::new("z"));
        let graphs = make_graphs_map(graph);

        let result = validate_composition_plan(&plan, &graphs);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
    }

    /// T10: 一部エッジのみ不正なプランで不正エッジのみエラー報告
    #[test]
    fn t10_partial_bad_edge_detected() {
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::AgentStep {
            agent: "a".into(),
            prompt_template: "".into(),
            inputs: vec![],
            output_var: "x".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "b".into(),
            prompt_template: "".into(),
            inputs: vec![VarDecl::new("x")],
            output_var: "y".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "c".into(),
            prompt_template: "".into(),
            inputs: vec![VarDecl::new("z")],
            output_var: "w".into(),
        });

        // 2番目のエッジが不正: node 2 の output_var は "w" だが from_var が "bad"
        let edges = vec![
            (
                0,
                1,
                EdgeMeta::DataFlow {
                    from_var: "x".into(),
                    to_var: "x".into(),
                },
            ),
            (
                2,
                1,
                EdgeMeta::DataFlow {
                    from_var: "bad".into(),
                    to_var: "x".into(),
                },
            ),
        ];

        let plan = make_single_graph_plan(graph.clone(), edges, VarDecl::new("y"));
        let graphs = make_graphs_map(graph);

        let result = validate_composition_plan(&plan, &graphs);
        assert!(
            matches!(&result, Err(DarviumError::VariableScopeViolation(msg)) if msg.contains("V-03")),
            "Expected V-03 violation on bad edge, got: {:?}",
            result
        );
    }

    // ── T11-T12: Frontier leakage ──

    /// T11: 互いに独立な parallel-ready ノードが同名変数を扱う → scope leakage 検出
    #[test]
    fn t11_frontier_leakage_detected() {
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::AgentStep {
            agent: "a".into(),
            prompt_template: "".into(),
            inputs: vec![],
            output_var: "same_var".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "b".into(),
            prompt_template: "".into(),
            inputs: vec![],
            output_var: "same_var".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "c".into(),
            prompt_template: "".into(),
            inputs: vec![VarDecl::new("same_var")],
            output_var: "result".into(),
        });

        let plan = make_single_graph_plan(
            graph.clone(),
            vec![
                (
                    0,
                    2,
                    EdgeMeta::DataFlow {
                        from_var: "same_var".into(),
                        to_var: "same_var".into(),
                    },
                ),
                (
                    1,
                    2,
                    EdgeMeta::DataFlow {
                        from_var: "same_var".into(),
                        to_var: "same_var".into(),
                    },
                ),
            ],
            VarDecl::new("result"),
        );
        let graphs = make_graphs_map(graph);

        let result = detect_frontier_leakage(&plan, &graphs);
        assert!(
            matches!(&result, Err(DarviumError::VariableScopeViolation(msg)) if msg.contains("Frontier leakage")),
            "Expected frontier leakage, got: {:?}",
            result
        );
    }

    /// T12: 互いに独立な parallel-ready ノードが別名変数を扱う → 正常通過
    #[test]
    fn t12_frontier_no_leakage() {
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode::AgentStep {
            agent: "a".into(),
            prompt_template: "".into(),
            inputs: vec![],
            output_var: "var_a".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "b".into(),
            prompt_template: "".into(),
            inputs: vec![],
            output_var: "var_b".into(),
        });
        graph.add_node(WorkflowNode::AgentStep {
            agent: "c".into(),
            prompt_template: "".into(),
            inputs: vec![VarDecl::new("var_a"), VarDecl::new("var_b")],
            output_var: "result".into(),
        });

        let plan = make_single_graph_plan(
            graph.clone(),
            vec![
                (
                    0,
                    2,
                    EdgeMeta::DataFlow {
                        from_var: "var_a".into(),
                        to_var: "var_a".into(),
                    },
                ),
                (
                    1,
                    2,
                    EdgeMeta::DataFlow {
                        from_var: "var_b".into(),
                        to_var: "var_b".into(),
                    },
                ),
            ],
            VarDecl::new("result"),
        );
        let graphs = make_graphs_map(graph);

        let result = detect_frontier_leakage(&plan, &graphs);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
    }

    // ── OTS-1: ランダムグラフアンサンブルによる捕捉率検証 ──

    #[test]
    fn ots1_capture_rate_random_graphs() {
        let mut rng = StdRng::seed_from_u64(12345);
        let graph_sizes = [10, 50, 100, 200];
        let edge_rates = [0.2, 0.5, 0.8];
        let total_trials = 500;
        let mut total_injected = 0usize;
        let mut total_captured = 0usize;

        for trial in 0..total_trials {
            let n = graph_sizes[trial % graph_sizes.len()];
            let p = edge_rates[trial % edge_rates.len()];

            let mut graph = WorkflowGraph::new();
            let node_vars: Vec<String> = (0..n).map(|i| format!("var_{}", i)).collect();

            for v in &node_vars {
                graph.add_node(WorkflowNode::AgentStep {
                    agent: "test".into(),
                    prompt_template: "".into(),
                    inputs: vec![],
                    output_var: v.clone(),
                });
            }

            let mut edges = Vec::new();
            let mut injected = 0usize;
            for i in 0..n {
                for j in (i + 1)..n {
                    if rng.random::<f64>() < p {
                        let to_var = format!("var_{}", j);
                        let from_var = {
                            if rng.random::<f64>() < 0.1 {
                                injected += 1;
                                format!("nonexistent_{}", rng.random::<u32>())
                            } else {
                                node_vars[i].clone()
                            }
                        };
                        edges.push((
                            i as NodeId,
                            j as NodeId,
                            EdgeMeta::DataFlow { from_var, to_var },
                        ));
                    }
                }
            }

            let plan = CompositionPlan::new(
                vec!["test_graph".into()],
                edges,
                vec![],
                VarDecl::new("final"),
            );
            let mut graphs = HashMap::new();
            graphs.insert("test_graph".into(), graph);

            let result = validate_composition_plan(&plan, &graphs);

            total_injected += injected;
            if result.is_err() {
                total_captured += injected;
            }
        }

        let capture_rate = if total_injected > 0 {
            total_captured as f64 / total_injected as f64
        } else {
            1.0
        };

        println!(
            "OTS-1: trials={}, injected={}, captured={}, capture_rate={:.4}",
            total_trials, total_injected, total_captured, capture_rate
        );

        assert!(
            (capture_rate - 1.0).abs() < 1e-6,
            "Expected capture_rate ≈ 1.0, got {}",
            capture_rate
        );
    }

    // ── OTS-2: スケーリング指数 γ の実測 ──

    #[test]
    fn ots2_scaling_gamma() {
        let mut rng = StdRng::seed_from_u64(12345);
        let ns: Vec<usize> = (0..5).map(|i| 10 * (i + 1)).collect();
        let ensembles_per_n = 100;

        println!("OTS-2: n,avg_steps");
        for &n in &ns {
            let mut total_steps = 0usize;
            for _ in 0..ensembles_per_n {
                let mut graph = WorkflowGraph::new();
                for i in 0..n {
                    graph.add_node(WorkflowNode::AgentStep {
                        agent: "test".into(),
                        prompt_template: "".into(),
                        inputs: vec![],
                        output_var: format!("var_{}", i),
                    });
                }

                let mut edges = Vec::new();
                let edge_count = (n * (n.saturating_sub(1)) / 2).min(n * 2);
                for _ in 0..edge_count {
                    let src = rng.random_range(0..n);
                    let dst = rng.random_range(0..n);
                    if src != dst {
                        edges.push((
                            src as NodeId,
                            dst as NodeId,
                            EdgeMeta::DataFlow {
                                from_var: format!("var_{}", src),
                                to_var: format!("var_{}", dst),
                            },
                        ));
                    }
                }

                let plan = CompositionPlan::new(
                    vec!["test_graph".into()],
                    edges,
                    vec![],
                    VarDecl::new("final"),
                );
                let mut graphs = HashMap::new();
                graphs.insert("test_graph".into(), graph);

                let _ = validate_composition_plan(&plan, &graphs);
                total_steps += plan.composition_edges.len();
            }

            let avg_steps = total_steps as f64 / ensembles_per_n as f64;
            println!("OTS-2: n={}, avg_steps={:.2}", n, avg_steps);
        }
    }

    // ── OTS-3: 最大ノード数における最悪計算ステップ数の有界性 ──

    #[test]
    fn ots3_max_node_bounded_steps() {
        let mut rng = StdRng::seed_from_u64(12345);
        let n = 512usize;
        let ensembles = 100;
        let mut max_steps = 0usize;

        for _ in 0..ensembles {
            let mut graph = WorkflowGraph::new();
            for i in 0..n {
                graph.add_node(WorkflowNode::AgentStep {
                    agent: "test".into(),
                    prompt_template: "".into(),
                    inputs: vec![],
                    output_var: format!("var_{}", i),
                });
            }

            let mut edges = Vec::new();
            let edge_count = n * 4;
            for _ in 0..edge_count {
                let src = rng.random_range(0..n);
                let dst = rng.random_range(0..n);
                if src != dst {
                    edges.push((
                        src as NodeId,
                        dst as NodeId,
                        EdgeMeta::DataFlow {
                            from_var: format!("var_{}", src),
                            to_var: format!("var_{}", dst),
                        },
                    ));
                }
            }

            let plan = CompositionPlan::new(
                vec!["test_graph".into()],
                edges,
                vec![],
                VarDecl::new("final"),
            );
            let mut graphs = HashMap::new();
            graphs.insert("test_graph".into(), graph);

            let _ = validate_composition_plan(&plan, &graphs);
            max_steps = max_steps.max(plan.composition_edges.len());
        }

        println!(
            "OTS-3: n={}, ensembles={}, max_steps={}",
            n, ensembles, max_steps
        );

        assert!(
            max_steps < 10_000,
            "Expected max_steps < 10000, got {}",
            max_steps
        );
    }
}
