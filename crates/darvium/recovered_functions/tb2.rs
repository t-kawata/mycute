// Recovered from line 19592 (Edit #19592, last occurrence)
// Target: fn tb2_

fn tb2_gmr_adds_new_individual() {
	        let mut ctx = SimulationContext::new(StdRng::seed_from_u64(12345));
	        let mut helper = make_population_entry("helper", 8);
	        helper.graph = make_connected_graph(8);
	        let mut helpee = make_population_entry("helpee", 2);
	        helpee.graph = make_connected_graph(2);
	        let helpee_graph_before = helpee.graph.clone();
	        ctx.population.push(helper);
	        ctx.population.push(helpee);

	        let added = try_gmr_diffusion(&mut ctx, 0_usize, 1_usize);

	        if added > 0 {
	            // 元の helpee のグラフは変更されていない
	            assert_eq!(
	                ctx.population[1].graph.node_count(),
	                helpee_graph_before.node_count(),
	                "T-B2: helpee graph unchanged"
	            );
	            // 新個体が追加されている
	            assert_eq!(
	                ctx.population.len(),
	                3,
	                "T-B2: population should have 3 entries"
	            );
	            // 新個体のグラフノード数が正しい
	            let new_pid = 2_usize;
	            assert!(
	                ctx.population[new_pid].graph.node_count() > 0,
	                "T-B2: new individual has positive node count"
	            );
	        }
	        println!("T-B2 PASS (added={})", added);
	    }

	    // ============================================================
	    // #146 T-B3: 自己抽象化後に SubWorkflow が新個体として追加される
	    // ============================================================
	    #[test]
	    fn tb3_self_abstraction_adds_new_individual() {
	        use crate::self_refinement::run_self_refinement_round;

	        let mut graph = WorkflowGraph::new();
	        let mut nodes = Vec::new();
	        // 51 AgentStep ノード（GED_GRAPH_SIZE_LIMIT=50 超過）を直列接続
	        for i in 0..GED_GRAPH_SIZE_LIMIT + 1 {
	            let idx = graph.add_node(WorkflowNode::AgentStep {
	                agent: format!("a{}", i),
	                prompt_template: "prompt".to_string(),
	                inputs: vec![VarDecl::new("input")],
	                output_var: format!("o{}", i),
	            });
	            nodes.push(idx);
	        }
	        for i in 1..nodes.len() {
	            graph.add_edge(nodes[i - 1], nodes[i], EdgeMeta::DependsOn);
	        }

	        let id = "tb3-graph".to_string();
	        let trust = TrustProfile {
	            operational: 0.0,
	            semantic: 0.0,
	            temporal: 0.0,
	            human: HumanTrustLogistic::default(),
	        };
	        let rep = ReputationProfile::default();
	        let mut registry = WorkflowRegistry::new();

	        let mut added_subgraphs: Vec<WorkflowGraph> = Vec::new();
	        {
	            let mut on_new_individual = |subgraph: WorkflowGraph, _new_id: &WorkflowGraphId| {
	                added_subgraphs.push(subgraph);
	            };
	            let count = run_self_refinement_round(
	                &mut graph,
	                &id,
	                &trust,
	                &rep,
	                &mut registry,
	                Some(&mut on_new_individual),
	            ).expect("run_self_refinement_round should succeed");
	            println!("T-B3: abstraction count={}", count);
	        }

	        if !added_subgraphs.is_empty() {
	            // 新個体として追加されたサブグラフがある
	            assert!(
	                added_subgraphs[0].node_count() >= MIN_ABSTRACTION_GROUP_SIZE,
	                "T-B3: subgraph should have >= {} nodes, got {}",
	                MIN_ABSTRACTION_GROUP_SIZE,
	                added_subgraphs[0].node_count()
	            );
	            // 親グラフが SubWorkflow に置換されている
	            let has_subworkflow = graph.node_indices().any(|ni| {
	                matches!(&graph[ni], WorkflowNode::SubWorkflow { .. })
	            });
	            assert!(
	                has_subworkflow,
	                "T-B3: parent graph should contain SubWorkflow node"
	            );
	            // 親グラフのノード数が減少している
	            assert!(
	                graph.node_count() < GED_GRAPH_SIZE_LIMIT + 1,
	                "T-B3: parent graph node count should decrease"
	            );
	        }
	        println!("T-B3 PASS (subgraphs={})", added_subgraphs.len());
	    }

	    // ============================================================
	    // #146 T-B4: HELP → 自己抽象化 → GMR の複合サイクルで個体数が正確
	    // ============================================================
	    #[test]
	    fn tb4_compound_cycle_accuracy() {
	        let mut ctx = SimulationContext::new(StdRng::seed_from_u64(12345));
	        ctx.use_gmr = true;

	        // 3 個体を設定
	        let mut p0 = make_connected_entry("p0", 5);
	        p0.reputation.benevolence_score = 1.0;
	        p0.trust.operational = 1.0;
	        p0.trust.semantic = 1.0;
	        p0.trust.temporal = 1.0;
	        let p1 = make_connected_entry("p1", 2);
	        let p2 = make_connected_entry("p2", 3);
	        ctx.population.push(p0);
	        ctx.population.push(p1);
	        ctx.population.push(p2);

	        let pop_before = ctx.population.len();

	        // サイクル 1: HELP
	        let help_added = propose_subgraph_and_accept(&mut ctx, 0, 1);
	        println!("T-B4: cycle1 HELP added={}", help_added);
	        let pop_after_help = ctx.population.len();

	        // サイクル 2: GMR
	        let gmr_added = try_gmr_diffusion(&mut ctx, 0, 2);
	        println!("T-B4: cycle2 GMR added={}", gmr_added);
	        let pop_after_gmr = ctx.population.len();

	        // サイクル 3: GMR with the new entity
	        if help_added > 0 && pop_after_help > pop_before {
	            let new_entity = pop_after_help - 1;
	            let gmr_added2 = try_gmr_diffusion(&mut ctx, 0, new_entity);
	            println!("T-B4: cycle3 GMR on new entity added={}", gmr_added2);
	        }

	        // 全部品の compute_all_node_count が 0 でない
	        let store = ctx.registry.store_ref();
	        for (pid, person) in ctx.population.iter().enumerate() {
	            let node_count = if let Some(s) = store {
	                compute_all_node_count(&person.graph, s).unwrap_or(0)
	            } else {
	                person.graph.node_count()
	            };
	            assert!(
	                node_count > 0,
	                "T-B4: person {} should have positive node count, got {}",
	                pid,
	                node_count
	            );
	        }

	        println!("T-B4 PASS: pop={}: help_added={}, gmr_added={}",
	            ctx.population.len(), help_added, gmr_added);
	    }

	    // ============================================================
	    // #146 T-B5: 元の個体が変更されない（HELP/GMR/自己抽象化）
	    // ============================================================
	    #[test]
	    fn tb5_original_unchanged() {
	        let mut ctx = SimulationContext::new(StdRng::seed_from_u64(12345));

	        // HELP の元個体不変性
	        let mut helper = make_connected_entry("helper", 5);
	        helper.reputation.benevolence_score = 1.0;
	        helper.trust.operational = 1.0;
	        helper.trust.semantic = 1.0;
	        helper.trust.temporal = 1.0;
	        let mut helpee = make_connected_entry("helpee", 2);
	        helpee.reputation.benevolence_score = 0.5;
	        let helpee_graph_snapshot = helpee.graph.clone();
	        let helper_graph_snapshot = helper.graph.clone();
	        ctx.population.push(helper);
	        ctx.population.push(helpee);

	        let _added = propose_subgraph_and_accept(&mut ctx, 0, 1);

	        // helpee のグラフは不変
	        assert_eq!(
	            ctx.population[1].graph.node_count(),
	            helpee_graph_snapshot.node_count(),
	            "T-B5: helpee graph unchanged after HELP"
	        );
	        // helper のグラフも不変
	        assert_eq!(
	            ctx.population[0].graph.node_count(),
	            helper_graph_snapshot.node_count(),
	            "T-B5: helper graph unchanged after HELP"
	        );

	        // GMR の元個体不変性
	        let mut gmr_helpee = make_population_entry("g-helpee", 2);
	        gmr_helpee.graph = make_connected_graph(2);
	        let gmr_helpee_snapshot = gmr_helpee.graph.clone();
	        ctx.population.push(gmr_helpee);
	        let gmr_helpee_id = ctx.population.len() - 1;

	        let _added = try_gmr_diffusion(&mut ctx, 0, gmr_helpee_id);

	        assert_eq!(
	            ctx.population[gmr_helpee_id].graph.node_count(),
	            gmr_helpee_snapshot.node_count(),
	            "T-B5: GMR helpee graph unchanged after GMR"
	        );

	        println!("T-B5 PASS: original individuals unchanged");
	    }

	    // ============================================================
	    // #146 T-B6: 新個体の属性が元の個体から適切に継承される
	    // ============================================================
	    #[test]
	    fn tb6_attribute_inheritance() {
	        let mut ctx = SimulationContext::new(StdRng::seed_from_u64(12345));

	        // HELP の属性継承
	        let mut helper = make_connected_entry("helper", 5);
	        helper.reputation.benevolence_score = 1.0;
	        helper.trust.operational = 1.0;
	        helper.trust.semantic = 1.0;
	        helper.trust.temporal = 1.0;
	        let mut helpee = make_connected_entry("helpee", 2);
	        helpee.reputation.benevolence_score = 0.5;
	        helpee.position = [0.3, 0.4, 0.5];
	        helpee.village_assignment = Some(7);
	        ctx.population.push(helper);
	        ctx.population.push(helpee);

	        let pop_before = ctx.population.len();
	        let _added = propose_subgraph_and_accept(&mut ctx, 0, 1);

	        if ctx.population.len() > pop_before {
	            let new_pid = ctx.population.len() - 1;
	            // position が helpee から継承されている
	            assert_eq!(
	                ctx.population[new_pid].position,
	                [0.3, 0.4, 0.5],
	                "T-B6: new individual inherits helpee's position"
	            );
	            // village_assignment が helpee から継承されている
	            assert_eq!(
	                ctx.population[new_pid].village_assignment,
	                Some(7),
	                "T-B6: new individual inherits helpee's village_assignment"
	            );
	        }

	        // GMR の属性継承
	        let mut gmr_helpee = make_population_entry("g-helpee", 3);
	        gmr_helpee.graph = make_connected_graph(3);
	        gmr_helpee.position = [0.9, 0.8, 0.7];
	        gmr_helpee.village_assignment = Some(3);
	        ctx.population.push(gmr_helpee);
	        let gmr_helpee_id = ctx.population.len() - 1;

	        let pop_before = ctx.population.len();
	        let _added = try_gmr_diffusion(&mut ctx, 0, gmr_helpee_id);

	        if ctx.population.len() > pop_before {
	            let new_pid = ctx.population.len() - 1;
	            assert_eq!(
	                ctx.population[new_pid].position,
	                [0.9, 0.8, 0.7],
	                "T-B6: GMR new individual inherits helpee's position"
	            );
	            assert_eq!(
	                ctx.population[new_pid].village_assignment,
	                Some(3),
	                "T-B6: GMR new individual inherits helpee's village_assignment"
	            );
	        }

	        println!("T-B6 PASS");
	    }
	}