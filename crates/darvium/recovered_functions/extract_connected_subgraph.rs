// Recovered from line 13311 (Edit #13311, last occurrence)
// Target: fn extract_connected_subgraph

fn extract_connected_subgraph(
    graph: &WorkflowGraph,
    rng: &mut StdRng,
    det_score: f64,
) -> (Vec<NodeIndex>, Vec<(NodeIndex, NodeIndex, EdgeMeta)>) {
    // AgentStep ノードのみを収集
    let agent_step_nodes: Vec<NodeIndex> = graph
        .node_indices()
        .filter(|&ni| matches!(graph[ni], WorkflowNode::AgentStep { .. }))
        .collect();

    if agent_step_nodes.is_empty() {
        return (vec![], vec![]);
    }

    // DeterminismScore で抽出サイズを決定
    let max_size = if det_score > crate::constants::DETERMINISM_THRESHOLD {
        // 高 determinism: 2〜4 ノード（スコアが高いほど多ノード）
        let extra = (det_score * 2.0).floor() as usize;
        2 + extra.min(2)
    } else if det_score > crate::constants::DETERMINISM_THRESHOLD * 0.7
        && rng.random::<f64>() < crate::constants::GMR_DIFFUSION_PROBABILITY
    {
        // 低 determinism: 1 ノード（確率的）
        1
    } else {
        return (vec![], vec![]);
    };
    let target_size = max_size.min(agent_step_nodes.len());

    // ランダムな AgentStep 開始ノードを選択
    let start_idx = rng.random_range(0..agent_step_nodes.len());
    let start_node = agent_step_nodes[start_idx];

    // BFS で接続 AgentStep ノードを収集
    let mut bfs = Bfs::new(graph, start_node);
    let mut collected_nodes: Vec<NodeIndex> = Vec::new();
    let mut visited: HashSet<NodeIndex> = HashSet::new();

    while let Some(nx) = bfs.next(graph) {
        if collected_nodes.len() >= target_size {
            break;
        }
        if matches!(graph[nx], WorkflowNode::AgentStep { .. }) && visited.insert(nx) {
            collected_nodes.push(nx);
        }
    }

    // 収集ノード間のエッジを収集
    let node_set: HashSet<NodeIndex> = collected_nodes.iter().copied().collect();
    let mut collected_edges = Vec::new();
    for &from in &collected_nodes {
        for edge in graph.edges(from) {
            let to = edge.target();
            if node_set.contains(&to) {
                collected_edges.push((from, to, edge.weight().clone()));
            }
        }
    }

    // println!(
    //     "[GMR] extract: det_score={:.4} target={} actual_nodes={} actual_edges={}",
    //     det_score,
    //     target_size,
    //     collected_nodes.len(),
    //     collected_edges.len()
    // );

    (collected_nodes, collected_edges)
}

/// 抽出したサブグラフを GraphPatch に変換する。
///
/// ノードとエッジを PatchOperation::AddNode / AddEdge に変換する。
/// AddEdge のインデックスは helpee グラフのノード数からの相対位置に変換する。
/// この変換により、元グラフと独立した GraphPatch として適用可能になる。
///
/// # TODO (本物実装)
///
/// - patch_confidence の実質的な計算（現在は 0.0 固定）
/// - source_graph_id の設定（現在は空文字）
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
