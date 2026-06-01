// Recovered from line 15439 (Edit #15439, last occurrence)
// Target: fn propose_subgraph_and_accept

fn propose_subgraph_and_accept(
    ctx: &mut SimulationContext,
    helper_id: PersonId,
    helpee_id: PersonId,
) -> usize {
    let helper_graph = &ctx.population[helper_id].graph;

    // helper に AgentStep ノードがない場合は何もしない
    let has_agent_steps = helper_graph.node_indices().any(|ni| {
        matches!(helper_graph[ni], WorkflowNode::AgentStep { .. })
    });
    if !has_agent_steps {
        return 0;
    }

    // DeterminismScore を計算（try_gmr_diffusion と同じロジック）
    let det_values: Vec<f64> = helper_graph
        .node_indices()
        .filter_map(|ni| {
            if matches!(helper_graph[ni], WorkflowNode::AgentStep { .. }) {
                let out_degree = helper_graph.neighbors(ni).count() as f64;
                let determinism = (1.0 - out_degree * 0.15).clamp(0.3, 1.0);
                Some(determinism)
            } else {
                None
            }
        })
        .collect();
    let det_score = if det_values.is_empty() {
        0.5
    } else {
        crate::gmr::DeterminismScore::compute(&det_values, crate::constants::SOFT_MIN_BETA)
    };

    // サブグラフ抽出
    let (nodes, edges) = extract_connected_subgraph(helper_graph, &mut ctx.rng, det_score);

    if nodes.is_empty() {
        return 0;
    }

    // 受入確率を計算: helper の benevolence + 信頼に基づく
    // benevolence が高いほど、信頼が高いほど受入確率が上がる
    let helper_benevolence = ctx.population[helper_id].reputation.benevolence_score;
    let helper_trust = &ctx.population[helper_id].trust;
    let trust_factor = (helper_trust.operational
        + helper_trust.semantic
        + helper_trust.temporal)
        / 3.0;
    let accept_probability = crate::constants::HELP_PROPOSAL_ACCEPT_BASE
        + (helper_benevolence * 0.3) as f64
        + (trust_factor * 0.2);
    let accept_probability = accept_probability.clamp(0.1, 0.9);

    // 確率的受入
    if ctx.rng.random::<f64>() >= accept_probability {
        println!(
            "[PROPOSE] rejected (accept_prob={:.3}) helper={} helpee={} nodes={}",
            accept_probability,
            ctx.population[helper_id].graph.node_count(),
            ctx.population[helpee_id].graph.node_count(),
            nodes.len(),
        );
        return 0;
    }

    // GraphPatch 構築
    let helpee_base_count = ctx.population[helpee_id].graph.node_count();
    let patch = build_graph_patch_from_subgraph(
        &nodes,
        &edges,
        helper_graph,
        helpee_base_count,
    );

    // Atomic 適用
    match apply_patch_atomic(&ctx.population[helpee_id].graph, &patch) {
        Ok(new_graph) => {
            ctx.population[helpee_id].graph = new_graph;
            let added_count = nodes.len();
            println!(
                "[PROPOSE] accepted: added {} nodes (accept_prob={:.3}) helper={} helpee={}",
                added_count,
                accept_probability,
                ctx.population[helper_id].graph.node_count(),
                ctx.population[helpee_id].graph.node_count(),
            );
            added_count
        }
        Err(e) => {
            println!(
                "[PROPOSE] apply_patch_atomic failed: {:?} -> 0 added",
                e
            );
            0
        }
    }
}

/// helper のグラフから接続 AgentStep サブグラフを抽出する。