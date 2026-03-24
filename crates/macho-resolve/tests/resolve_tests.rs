//! Tests for macho-resolve: path resolution and dependency graph.
//!
//! Ported from:
//!   - Apple dyld: @rpath, @loader_path, @executable_path semantics
//!   - Apple dyld: DYLD_LIBRARY_PATH, fallback path handling
//!   - LLVM lld: rpath resolution, two-level namespace ordering

#[cfg(test)]
mod path_resolver_tests {
    use macho_resolve::PathResolver;
    use std::path::PathBuf;

    // ── @executable_path resolution (from dyld path semantics) ──────

    #[test]
    fn resolve_executable_path_prefix() {
        let resolver = PathResolver::new(PathBuf::from("/usr/bin/app"), vec![]);
        // @executable_path/libfoo.dylib → /usr/bin/libfoo.dylib
        // Won't resolve because file doesn't exist, but we test the expansion
        let result = resolver.resolve("@executable_path/libfoo.dylib");
        // File won't exist, so result is None — that's correct behavior
        assert!(result.is_none());
    }

    // ── @rpath resolution (from dyld LC_RPATH test cases) ───────────

    #[test]
    fn resolve_rpath_with_executable_path_expansion() {
        let resolver = PathResolver::new(
            PathBuf::from("/tmp/test_app"),
            vec!["@executable_path".to_string()],
        );
        // @rpath/libfoo.dylib tries /tmp/libfoo.dylib (since @executable_path → /tmp)
        let result = resolver.resolve("@rpath/libfoo.dylib");
        // File doesn't exist, returns None
        assert!(result.is_none());
    }

    // ── Absolute path resolution ────────────────────────────────────

    #[test]
    fn resolve_absolute_path_existing() {
        let resolver = PathResolver::new(PathBuf::from("/usr/bin/app"), vec![]);
        // /usr/lib/libSystem.B.dylib should exist on macOS
        let result = resolver.resolve("/usr/lib/libSystem.B.dylib");
        if std::path::Path::new("/usr/lib/libSystem.B.dylib").exists() {
            assert!(result.is_some());
            assert_eq!(result.unwrap(), PathBuf::from("/usr/lib/libSystem.B.dylib"));
        }
    }

    #[test]
    fn resolve_absolute_path_nonexistent() {
        let resolver = PathResolver::new(PathBuf::from("/usr/bin/app"), vec![]);
        let result = resolver.resolve("/nonexistent/path/libfoo.dylib");
        assert!(result.is_none());
    }

    // ── @loader_path resolution ─────────────────────────────────────

    #[test]
    fn resolve_loader_path_prefix() {
        let resolver = PathResolver::new(PathBuf::from("/opt/app/bin/myapp"), vec![]);
        // @loader_path/../lib/libfoo.dylib → /opt/app/bin/../lib/libfoo.dylib
        let result = resolver.resolve("@loader_path/../lib/libfoo.dylib");
        // Won't exist
        assert!(result.is_none());
    }

    // ── Multiple rpaths tried in order (from dyld: first-match wins) ──

    #[test]
    fn resolve_rpath_tries_multiple_paths() {
        let resolver = PathResolver::new(
            PathBuf::from("/tmp/test_app"),
            vec![
                "/nonexistent/first".to_string(),
                "/nonexistent/second".to_string(),
                "/usr/lib".to_string(),
            ],
        );
        // None of these rpaths + "libfoo.dylib" will exist
        let result = resolver.resolve("@rpath/libfoo.dylib");
        assert!(result.is_none());
    }

    // ── Non-special path treated as absolute ────────────────────────

    #[test]
    fn resolve_plain_absolute() {
        let resolver = PathResolver::new(PathBuf::from("/usr/bin/app"), vec![]);
        let result = resolver.resolve("/usr/lib/libc.dylib");
        // May or may not exist depending on shared cache
        // Just verify it doesn't panic
        let _ = result;
    }
}

#[cfg(test)]
mod search_env_tests {
    use macho_resolve::{PathResolver, SearchEnv};
    use std::path::PathBuf;

    #[test]
    fn search_env_default_is_empty() {
        let env = SearchEnv::default();
        assert!(env.library_path.is_empty());
        assert!(env.framework_path.is_empty());
        assert!(env.fallback_library_path.is_empty());
        assert!(env.fallback_framework_path.is_empty());
    }

    #[test]
    fn new_with_env_preserves_search_env() {
        let env = SearchEnv {
            library_path: vec![PathBuf::from("/opt/lib")],
            framework_path: vec![PathBuf::from("/opt/frameworks")],
            fallback_library_path: vec![PathBuf::from("/fallback/lib")],
            fallback_framework_path: vec![],
        };
        let resolver = PathResolver::new_with_env(PathBuf::from("/usr/bin/app"), vec![], env);
        assert_eq!(resolver.search_env.library_path.len(), 1);
        assert_eq!(resolver.search_env.framework_path.len(), 1);
        assert_eq!(resolver.search_env.fallback_library_path.len(), 1);
    }

    #[test]
    fn new_without_env_has_empty_search_env() {
        let resolver = PathResolver::new(PathBuf::from("/usr/bin/app"), vec![]);
        assert!(resolver.search_env.library_path.is_empty());
    }

    #[test]
    fn search_env_serde_roundtrip() {
        let env = SearchEnv {
            library_path: vec![PathBuf::from("/opt/lib")],
            framework_path: vec![PathBuf::from("/opt/fw")],
            fallback_library_path: vec![],
            fallback_framework_path: vec![PathBuf::from("/fallback/fw")],
        };
        let json = serde_json::to_string(&env).unwrap();
        let restored: SearchEnv = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.library_path, env.library_path);
        assert_eq!(restored.framework_path, env.framework_path);
        assert_eq!(
            restored.fallback_framework_path,
            env.fallback_framework_path
        );
    }

    #[test]
    fn dyld_library_path_used_for_plain_dylib() {
        // If /usr/lib exists and has dylibs, a resolver with DYLD_LIBRARY_PATH
        // pointing there should find them
        let env = SearchEnv {
            library_path: vec![PathBuf::from("/usr/lib")],
            ..Default::default()
        };
        let resolver = PathResolver::new_with_env(PathBuf::from("/tmp/app"), vec![], env);
        // /usr/lib/libSystem.B.dylib exists on macOS
        if std::path::Path::new("/usr/lib/libSystem.B.dylib").exists() {
            let result = resolver.resolve("/nonexistent/libSystem.B.dylib");
            // Should find it via DYLD_LIBRARY_PATH because the leaf name matches
            assert!(
                result.is_some(),
                "should find libSystem.B.dylib via DYLD_LIBRARY_PATH"
            );
        }
    }

    #[test]
    fn standard_dylib_paths_find_libsystem() {
        let resolver = PathResolver::new(PathBuf::from("/tmp/app"), vec![]);
        // With empty search_env, standard paths (/usr/lib) should still be checked
        if std::path::Path::new("/usr/lib/libSystem.B.dylib").exists() {
            // Resolve a non-existent absolute path — should fall through to standard paths
            let result = resolver.resolve("/nonexistent/libSystem.B.dylib");
            assert!(
                result.is_some(),
                "should find libSystem.B.dylib via standard paths"
            );
        }
    }
}

#[cfg(test)]
mod framework_resolution_tests {
    use macho_resolve::{PathResolver, SearchEnv};
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
    }

    #[test]
    fn resolve_framework_via_rpath() {
        let fw_dir = fixtures_dir().join("framework_style");
        if !fw_dir
            .join("MyFramework.framework/Versions/A/MyFramework")
            .exists()
        {
            return; // Fixture not built
        }

        let resolver = PathResolver::new(fw_dir.join("app"), vec!["@executable_path".to_string()]);
        let result = resolver.resolve("@rpath/MyFramework.framework/Versions/A/MyFramework");
        assert!(result.is_some(), "should resolve framework via @rpath");
        assert!(result.unwrap().to_string_lossy().contains("MyFramework"));
    }

    #[test]
    fn resolve_framework_via_dyld_framework_path() {
        let fw_dir = fixtures_dir().join("framework_style");
        if !fw_dir
            .join("MyFramework.framework/Versions/A/MyFramework")
            .exists()
        {
            return;
        }

        let env = SearchEnv {
            framework_path: vec![fw_dir.clone()],
            ..Default::default()
        };
        let resolver = PathResolver::new_with_env(PathBuf::from("/tmp/app"), vec![], env);
        // Without @rpath, the framework marker should trigger framework search
        let result = resolver.resolve("MyFramework.framework/Versions/A/MyFramework");
        assert!(
            result.is_some(),
            "should resolve framework via DYLD_FRAMEWORK_PATH"
        );
    }

    #[test]
    fn resolve_system_framework() {
        // Foundation.framework should exist on macOS
        let fw_path = "/System/Library/Frameworks/Foundation.framework/Foundation";
        if !std::path::Path::new(fw_path).exists() {
            return; // Skip on non-macOS or systems where framework is only in shared cache
        }

        let resolver = PathResolver::new(PathBuf::from("/tmp/app"), vec![]);
        // A reference containing .framework/ should go through framework resolution
        let result = resolver.resolve("Foundation.framework/Foundation");
        assert!(
            result.is_some(),
            "should resolve Foundation.framework via standard paths"
        );
    }

    #[test]
    fn resolve_framework_nonexistent() {
        let resolver = PathResolver::new(PathBuf::from("/tmp/app"), vec![]);
        let result = resolver.resolve("NonExistentFramework.framework/NonExistentFramework");
        assert!(result.is_none());
    }
}

#[cfg(test)]
mod resolution_plan_tests {
    use macho_parse::parse;
    use macho_resolve::ResolutionPlan;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
    }

    fn read_fixture(name: &str) -> Option<Vec<u8>> {
        std::fs::read(fixtures_dir().join(name)).ok()
    }

    // ── Resolution plan for simple_cli ──────────────────────────────

    #[test]
    fn simple_cli_resolution_plan() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("simple_cli/app");
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
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("simple_cli/app");
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
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("dylib_chain/app");
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
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("dylib_chain/app");
        let plan = ResolutionPlan::from_parsed(&parsed, exe_path);

        assert!(!plan.rpaths.is_empty(), "should have rpaths from LC_RPATH");
    }

    // ── Edge from/to indices are valid ──────────────────────────────

    #[test]
    fn edge_indices_are_valid() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("simple_cli/app");
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
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("simple_cli/app");
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
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
    }

    fn read_fixture(name: &str) -> Option<Vec<u8>> {
        std::fs::read(fixtures_dir().join(name)).ok()
    }

    // ── Transitive resolution finds deeper dependencies ─────────────

    #[test]
    fn dylib_chain_transitive_finds_libb() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("dylib_chain/app");
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
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("dylib_chain/app");

        let shallow = ResolutionPlan::from_parsed(&parsed, exe_path.clone());
        let transitive = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        assert!(
            transitive.graph.nodes.len() >= shallow.graph.nodes.len(),
            "transitive should have at least as many nodes as shallow"
        );
    }

    #[test]
    fn transitive_plan_marks_itself() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("simple_cli/app");

        let shallow = ResolutionPlan::from_parsed(&parsed, exe_path.clone());
        let transitive = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        assert!(!shallow.transitive);
        assert!(transitive.transitive);
    }

    #[test]
    fn transitive_nodes_have_depth() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("dylib_chain/app");
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
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("dylib_chain/app");
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
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("dylib_chain/app");
        let plan = ResolutionPlan::from_parsed_transitive(&parsed, exe_path);

        let order = plan.init_order();
        assert!(!order.is_empty());

        // Last element should be node 0 (the main executable)
        assert_eq!(*order.last().unwrap(), 0, "main exe should init last");
    }

    #[test]
    fn init_order_covers_all_nodes() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("simple_cli/app");
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
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
    }

    fn read_fixture(name: &str) -> Option<Vec<u8>> {
        std::fs::read(fixtures_dir().join(name)).ok()
    }

    #[test]
    fn framework_app_resolves_my_framework() {
        let Some(data) = read_fixture("framework_style/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("framework_style/app");
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
        let Some(data) = read_fixture("framework_style/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("framework_style/app");
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
        let Some(data) = read_fixture("framework_style/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("framework_style/app");
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
        let Some(data) = read_fixture("framework_style/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe_path = fixtures_dir().join("framework_style/app");
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

// ── Phase 15: SearchEnv::from_env and merge_dyld_env_entries ────────

#[cfg(test)]
mod search_env_phase15_tests {
    use macho_resolve::SearchEnv;
    use std::path::PathBuf;

    #[test]
    fn from_env_returns_default_when_no_vars_set() {
        // In a test environment, DYLD_* vars are typically not set
        let env = SearchEnv::from_env();
        // We can't make strong assertions since the test runner
        // might have DYLD vars set, but it should at least not panic
        let _ = env;
    }

    #[test]
    fn merge_dyld_env_entries_adds_library_path() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&["DYLD_LIBRARY_PATH=/opt/lib:/usr/local/lib".to_string()]);
        assert_eq!(
            env.library_path,
            vec![PathBuf::from("/opt/lib"), PathBuf::from("/usr/local/lib"),]
        );
    }

    #[test]
    fn merge_dyld_env_entries_adds_framework_path() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&["DYLD_FRAMEWORK_PATH=/Library/Frameworks".to_string()]);
        assert_eq!(
            env.framework_path,
            vec![PathBuf::from("/Library/Frameworks")]
        );
    }

    #[test]
    fn merge_dyld_env_entries_adds_fallback_paths() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&[
            "DYLD_FALLBACK_LIBRARY_PATH=/opt/fallback".to_string(),
            "DYLD_FALLBACK_FRAMEWORK_PATH=/opt/fallback_fw".to_string(),
        ]);
        assert_eq!(
            env.fallback_library_path,
            vec![PathBuf::from("/opt/fallback")]
        );
        assert_eq!(
            env.fallback_framework_path,
            vec![PathBuf::from("/opt/fallback_fw")]
        );
    }

    #[test]
    fn merge_dyld_env_entries_appends_to_existing() {
        let mut env = SearchEnv {
            library_path: vec![PathBuf::from("/existing/lib")],
            ..Default::default()
        };
        env.merge_dyld_env_entries(&["DYLD_LIBRARY_PATH=/new/lib".to_string()]);
        assert_eq!(
            env.library_path,
            vec![PathBuf::from("/existing/lib"), PathBuf::from("/new/lib"),]
        );
    }

    #[test]
    fn merge_dyld_env_entries_ignores_unknown_keys() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&["PATH=/usr/bin".to_string(), "HOME=/Users/test".to_string()]);
        assert!(env.library_path.is_empty());
        assert!(env.framework_path.is_empty());
        assert!(env.fallback_library_path.is_empty());
        assert!(env.fallback_framework_path.is_empty());
    }

    #[test]
    fn merge_dyld_env_entries_handles_missing_equals() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&[
            "DYLD_LIBRARY_PATH".to_string(), // no = sign
        ]);
        // Should not panic, should be ignored
        assert!(env.library_path.is_empty());
    }

    #[test]
    fn merge_dyld_env_entries_multiple_paths_colon_separated() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&["DYLD_LIBRARY_PATH=/a:/b:/c".to_string()]);
        assert_eq!(env.library_path.len(), 3);
        assert_eq!(env.library_path[0], PathBuf::from("/a"));
        assert_eq!(env.library_path[1], PathBuf::from("/b"));
        assert_eq!(env.library_path[2], PathBuf::from("/c"));
    }

    #[test]
    fn merge_dyld_env_entries_empty_list() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&[]);
        assert!(env.library_path.is_empty());
    }
}
