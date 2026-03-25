//! Tests for macho-resolve: resolution plans, transitive deps, and framework fixtures.
//!
//! Ported from:
//!   - LLVM lld: rpath resolution, two-level namespace ordering

mod common;

#[cfg(test)]
mod resolution_plan_tests {
    use macho_parse::parse;
    use macho_resolve::ResolutionPlan;

    // ── Resolution plan for simple_cli ──────────────────────────────

    #[test]
    fn simple_cli_resolution_plan() {
        let Some(data) = super::common::read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("simple_cli/app");
        let plan = ResolutionPlan::from_parsed(&parsed, exe_path);

        // Node 0 is always the main executable
        assert_eq!(plan.graph.nodes[0].index, 0);
        assert!(plan.graph.nodes[0].install_name.is_none());

        // Should have at least libSystem resolved or unresolved
        let total = plan.graph.edges.len() + plan.unresolved.len();
        assert!(total > 0, "should have at least one dependency");
    }

    #[test]
    fn simple_cli_libsystem_in_graph_or_unresolved() {
        let Some(data) = super::common::read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("simple_cli/app");
        let plan = ResolutionPlan::from_parsed(&parsed, exe_path);

        let in_edges = plan
            .graph
            .edges
            .iter()
            .any(|e| e.raw_name.contains("libSystem"));
        let in_unresolved = plan.unresolved.iter().any(|n| n.contains("libSystem"));
        assert!(
            in_edges || in_unresolved,
            "libSystem should be in resolved edges or unresolved list"
        );
    }

    // ── Resolution plan for dylib_chain ─────────────────────────────

    #[test]
    fn dylib_chain_has_rpath_dependency() {
        let Some(data) = super::common::read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("dylib_chain/app");
        let plan = ResolutionPlan::from_parsed(&parsed, exe_path);

        // The @rpath dependency should resolve because @executable_path
        // points to the fixtures/dylib_chain/ directory where libchaina.dylib lives
        let rpath_resolved = plan
            .graph
            .edges
            .iter()
            .any(|e| e.raw_name.contains("@rpath/libchaina"));
        let rpath_unresolved = plan
            .unresolved
            .iter()
            .any(|n| n.contains("@rpath/libchaina"));
        assert!(
            rpath_resolved || rpath_unresolved,
            "should have @rpath/libchaina dependency"
        );
    }

    #[test]
    fn dylib_chain_rpaths_collected() {
        let Some(data) = super::common::read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("dylib_chain/app");
        let plan = ResolutionPlan::from_parsed(&parsed, exe_path);

        assert!(!plan.rpaths.is_empty(), "should have rpaths from LC_RPATH");
    }

    // ── Edge from/to indices are valid ──────────────────────────────

    #[test]
    fn edge_indices_are_valid() {
        let Some(data) = super::common::read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("simple_cli/app");
        let plan = ResolutionPlan::from_parsed(&parsed, exe_path);

        for edge in &plan.graph.edges {
            assert!(
                edge.from < plan.graph.nodes.len(),
                "edge.from out of bounds"
            );
            assert!(edge.to < plan.graph.nodes.len(), "edge.to out of bounds");
        }
    }

    // ── JSON serialization ──────────────────────────────────────────

    #[test]
    fn resolution_plan_serializes_to_json() {
        let Some(data) = super::common::read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("simple_cli/app");
        let plan = ResolutionPlan::from_parsed(&parsed, exe_path);

        let json = serde_json::to_string_pretty(&plan).unwrap();
        assert!(json.contains("nodes"));
        assert!(json.contains("edges"));
    }
}

#[cfg(test)]
mod transitive_tests {
    use macho_parse::parse;
    use macho_resolve::ResolutionPlan;

    // ── Transitive resolution finds deeper dependencies ─────────────

    #[test]
    fn dylib_chain_transitive_finds_libb() {
        let Some(data) = super::common::read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("dylib_chain/app");
        let plan = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        // Transitive resolution:
        // app → libchaina.dylib → libchainb.dylib + libSystem.B.dylib
        // app → libSystem.B.dylib
        // Should find libchainb even though app doesn't directly depend on it
        let has_chainb = plan
            .graph
            .nodes
            .iter()
            .any(|n| n.path.to_string_lossy().contains("libchainb"));
        assert!(has_chainb, "transitive resolution should find libchainb");
    }

    #[test]
    fn dylib_chain_transitive_has_more_nodes_than_shallow() {
        let Some(data) = super::common::read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("dylib_chain/app");

        let shallow = ResolutionPlan::from_parsed(&parsed, exe_path.clone());
        let transitive = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        assert!(
            transitive.graph.nodes.len() >= shallow.graph.nodes.len(),
            "transitive should have at least as many nodes as shallow"
        );
    }

    #[test]
    fn transitive_plan_marks_itself() {
        let Some(data) = super::common::read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("simple_cli/app");

        let shallow = ResolutionPlan::from_parsed(&parsed, exe_path.clone());
        let transitive = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        assert!(!shallow.transitive);
        assert!(transitive.transitive);
    }

    #[test]
    fn transitive_nodes_have_depth() {
        let Some(data) = super::common::read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("dylib_chain/app");
        let plan = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        // Root node depth = 0
        assert_eq!(plan.graph.nodes[0].depth, 0);

        // Direct dependencies have depth 1
        for edge in &plan.graph.edges {
            if edge.from == 0 {
                let dep = &plan.graph.nodes[edge.to];
                assert_eq!(
                    dep.depth, 1,
                    "direct dependency should have depth 1: {:?}",
                    dep.path
                );
            }
        }
    }

    #[test]
    fn transitive_no_duplicate_nodes() {
        let Some(data) = super::common::read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("dylib_chain/app");
        let plan = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        // No two nodes should have the same canonical path
        let mut seen = std::collections::HashSet::new();
        for node in &plan.graph.nodes {
            let canonical = node
                .path
                .canonicalize()
                .unwrap_or_else(|_| node.path.clone());
            assert!(
                seen.insert(canonical.clone()),
                "duplicate node: {:?}",
                canonical
            );
        }
    }

    // ── Init order ──────────────────────────────────────────────────

    #[test]
    fn init_order_starts_with_leaves() {
        let Some(data) = super::common::read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("dylib_chain/app");
        let plan = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        let order = plan.init_order();
        assert!(!order.is_empty());

        // Last element should be node 0 (the main executable)
        assert_eq!(*order.last().unwrap(), 0, "main exe should init last");
    }

    #[test]
    fn init_order_covers_all_nodes() {
        let Some(data) = super::common::read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("simple_cli/app");
        let plan = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        let order = plan.init_order();
        assert_eq!(
            order.len(),
            plan.graph.nodes.len(),
            "init_order should include all nodes"
        );
    }
}

#[cfg(test)]
mod framework_fixture_tests {
    use macho_parse::parse;
    use macho_resolve::ResolutionPlan;

    #[test]
    fn framework_app_resolves_my_framework() {
        let Some(data) = super::common::read_fixture("framework_style/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("framework_style/app");
        let plan = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        let has_fw = plan
            .graph
            .nodes
            .iter()
            .any(|n| n.path.to_string_lossy().contains("MyFramework"));
        assert!(has_fw, "should resolve MyFramework.framework dependency");
    }

    #[test]
    fn framework_app_has_rpath_edge() {
        let Some(data) = super::common::read_fixture("framework_style/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("framework_style/app");
        let plan = ResolutionPlan::from_parsed(&parsed, exe_path);

        let has_rpath_edge = plan
            .graph
            .edges
            .iter()
            .any(|e| e.raw_name.contains("@rpath/MyFramework"));
        assert!(has_rpath_edge, "should have @rpath/MyFramework edge");
    }

    #[test]
    fn framework_dylib_has_install_name() {
        let Some(data) = super::common::read_fixture("framework_style/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("framework_style/app");
        let plan = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        let fw_node = plan
            .graph
            .nodes
            .iter()
            .find(|n| n.path.to_string_lossy().contains("MyFramework"));
        if let Some(node) = fw_node {
            assert!(
                node.install_name.is_some(),
                "framework dylib should have an install name"
            );
            let name = node.install_name.as_ref().unwrap();
            assert!(
                name.contains("MyFramework"),
                "install name should contain MyFramework: {name}"
            );
        }
    }

    #[test]
    fn framework_app_init_order_fw_before_app() {
        let Some(data) = super::common::read_fixture("framework_style/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = super::common::fixtures_dir().join("framework_style/app");
        let plan = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        let order = plan.init_order();
        let fw_pos = order.iter().position(|&idx| {
            plan.graph.nodes[idx]
                .path
                .to_string_lossy()
                .contains("MyFramework")
        });
        let app_pos = order.iter().position(|&idx| idx == 0);

        if let (Some(fw), Some(app)) = (fw_pos, app_pos) {
            assert!(fw < app, "framework should initialize before app");
        }
    }
}
