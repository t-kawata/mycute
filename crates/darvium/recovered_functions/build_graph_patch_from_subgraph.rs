// Recovered from line 13311 (Edit #13311, last occurrence)
// Target: fn build_graph_patch_from_subgraph

fn build_graph_patch_from_subgraph(
    nodes: &[NodeIndex],
    edges: &[(NodeIndex, NodeIndex, EdgeMeta)],
    source_graph: &WorkflowGraph,
    helpee_base_count: usize,
) -> GraphPatch {
    let mut operations = Vec::new();

    // マッピング: 元の NodeIndex → 新しい AddNode 順の相対インデックス (helpee_base_count + i)
    // AddNode は helpee グラフの末尾に追加されるため、最初の追加ノードのインデックスは
    // helpee_base_count になる。
    let mut index_map: HashMap<usize, usize> = HashMap::new();
    for (i, &node_idx) in nodes.iter().enumerate() {
        let node = &source_graph[node_idx];
        operations.push(PatchOperation::AddNode { node: node.clone() });
        index_map.insert(node_idx.index(), helpee_base_count + i);
    }

    for (from, to, meta) in edges {
        let new_from = *index_map
            .get(&from.index())
            .expect("edge source must be in extracted nodes");
        let new_to = *index_map
            .get(&to.index())
            .expect("edge target must be in extracted nodes");
        operations.push(PatchOperation::AddEdge {
            from: new_from,
            to: new_to,
            meta: meta.clone(),
        });
    }

    GraphPatch {
        source_graph_id: String::new(),
        operations,
        patch_confidence: PatchConfidence {
            value: 0.0,
            self_score: 0.0,
            validator_score: 0.0,
            history_score: 0.0,
        },
        generated_at: SystemTime::now(),
        generator_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// GMR 機構を使用した追加能力拡散。
///
/// helper のグラフから接続 AgentStep サブグラフを抽出し、GraphPatch として
/// helpee のグラフに適用する。DeterminismScore が抽出サイズを制御する。
fn try_gmr_diffusion(
    ctx: &mut SimulationContext,
    helper_id: PersonId,
    helpee_id: PersonId,
) -> usize {
    // DeterminismScore の計算（TODO: 本物のグラフ構造に基づく計算に置き換え）
    let det_values: Vec<f64> = (0..5)
        .map(|_| ctx.rng.random::<f64>() * 0.5 + 0.5)
        .collect();
    let det_score =
        crate::gmr::DeterminismScore::compute(&det_values, crate::constants::SOFT_MIN_BETA);

    // サブグラフ抽出
    let helper_graph = &ctx.population[helper_id].graph;
    let helpee_graph = &ctx.population[helpee_id].graph;
    let (nodes, edges) = extract_connected_subgraph(helper_graph, &mut ctx.rng, det_score);

    if nodes.is_empty() {
        // println!("[GMR] try_gmr_diffusion: no nodes extracted -> skipped");
        return 0;
    }

    // GraphPatch 構築
    let helpee_base_count = helpee_graph.node_count();
    let patch = build_graph_patch_from_subgraph(&nodes, &edges, helper_graph, helpee_base_count);

    // Atomic 適用
    match apply_patch_atomic(helpee_graph, &patch) {
        Ok(new_graph) => {
            ctx.population[helpee_id].graph = new_graph;
            let added_count = nodes.len();
            // println!(
            //     "[GMR] try_gmr_diffusion: added {} nodes (det_score={:.4})",
            //     added_count, det_score
            // );
            added_count
        }
        Err(e) => {
            // println!(
            //     "[GMR] try_gmr_diffusion: apply_patch_atomic failed: {:?} -> 0 added",
            //     e
            // );
            0
        }
    }
}
