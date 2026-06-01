// Recovered from line 19592 (Edit #19592, last occurrence)
// Target: fn tb5_

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