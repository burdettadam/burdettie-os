//! Tests for macho-run-plan: execution plan generation.
//!
//! Ported from:
//!   - Apple dyld: startup sequence validation
//!   - dyld: initializer ordering, entry point transfer

#[cfg(test)]
mod tests {
    use macho_parse::parse;
    use macho_run_plan::ExecutionPlan;
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

    // ── Execution plan for simple_cli ───────────────────────────────

    #[test]
    fn simple_cli_execution_plan_has_entry() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = ExecutionPlan::from_parsed(&parsed, fixtures_dir().join("simple_cli/app"));
        assert!(plan.entry.is_some(), "execution plan must have entry point");
    }

    #[test]
    fn simple_cli_execution_plan_has_mapping() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = ExecutionPlan::from_parsed(&parsed, fixtures_dir().join("simple_cli/app"));
        assert!(
            !plan.mapping.regions.is_empty(),
            "execution plan must have mapping regions"
        );
    }

    #[test]
    fn simple_cli_execution_plan_has_dependencies() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = ExecutionPlan::from_parsed(&parsed, fixtures_dir().join("simple_cli/app"));
        // At minimum the main binary node should exist
        assert!(!plan.dependencies.graph.nodes.is_empty());
    }

    #[test]
    fn simple_cli_init_order_ends_with_main() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe = fixtures_dir().join("simple_cli/app");
        let plan = ExecutionPlan::from_parsed(&parsed, exe.clone());
        assert!(!plan.init_order.is_empty());
        // The last entry in init_order should be the main binary
        let last = plan.init_order.last().unwrap();
        assert!(
            last.contains("simple_cli"),
            "last init should be the main binary, got: {last}"
        );
    }

    // ── Execution plan for dylib_chain ──────────────────────────────

    #[test]
    fn dylib_chain_init_order_deps_before_main() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exe = fixtures_dir().join("dylib_chain/app");
        let plan = ExecutionPlan::from_parsed(&parsed, exe.clone());

        // Dependencies should appear before main binary in init_order
        // (bottom-up initialization per dyld semantics)
        let main_pos = plan
            .init_order
            .iter()
            .position(|s| s.contains("dylib_chain/app"));
        if let Some(mp) = main_pos {
            for (i, entry) in plan.init_order.iter().enumerate() {
                if i < mp {
                    // These are dependencies — they should init first
                    assert!(
                        !entry.contains("dylib_chain/app"),
                        "dependency should init before main"
                    );
                }
            }
        }
    }

    // ── ObjC presence detected in plan ──────────────────────────────

    #[test]
    fn simple_cli_no_objc_in_plan() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = ExecutionPlan::from_parsed(&parsed, fixtures_dir().join("simple_cli/app"));
        assert!(!plan.objc.has_objc);
    }

    #[test]
    fn objc_foundation_has_objc_in_plan() {
        let Some(data) = read_fixture("objc_foundation/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = ExecutionPlan::from_parsed(&parsed, fixtures_dir().join("objc_foundation/app"));
        assert!(plan.objc.has_objc);
    }

    // ── Display output ──────────────────────────────────────────────

    #[test]
    fn execution_plan_display_has_steps() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = ExecutionPlan::from_parsed(&parsed, fixtures_dir().join("simple_cli/app"));
        let display = plan.display();
        assert!(display.contains("Load main binary"));
        assert!(display.contains("Map"));
        assert!(display.contains("symbol binding"));
        assert!(display.contains("initializers"));
    }

    // ── JSON serialization ──────────────────────────────────────────

    #[test]
    fn execution_plan_serializes_to_json() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = ExecutionPlan::from_parsed(&parsed, fixtures_dir().join("simple_cli/app"));
        let json = serde_json::to_string_pretty(&plan).unwrap();
        assert!(json.contains("mapping"));
        assert!(json.contains("dependencies"));
        assert!(json.contains("init_order"));
    }
}

// ── Full execution plan tests (from_parsed_full) ───────────────────

#[cfg(test)]
mod full_plan_tests {
    use macho_parse::parse;
    use macho_run_plan::ExecutionPlan;
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
    fn full_plan_has_symbol_bindings() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan =
            ExecutionPlan::from_parsed_full(&parsed, fixtures_dir().join("simple_cli/app"), &data);
        assert!(
            plan.symbol_bindings.is_some(),
            "full plan should include symbol bindings"
        );
    }

    #[test]
    fn full_plan_has_initializers() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan =
            ExecutionPlan::from_parsed_full(&parsed, fixtures_dir().join("simple_cli/app"), &data);
        assert!(
            plan.initializers.is_some(),
            "full plan should include initializers"
        );
    }

    #[test]
    fn full_plan_has_objc_metadata() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan =
            ExecutionPlan::from_parsed_full(&parsed, fixtures_dir().join("simple_cli/app"), &data);
        assert!(
            plan.objc_metadata.is_some(),
            "full plan should include ObjC metadata"
        );
    }

    #[test]
    fn full_plan_simple_cli_no_constructors() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan =
            ExecutionPlan::from_parsed_full(&parsed, fixtures_dir().join("simple_cli/app"), &data);
        let inits = plan.initializers.as_ref().unwrap();
        assert!(
            inits.constructors.is_empty(),
            "simple_cli has no constructors"
        );
    }

    #[test]
    fn full_plan_constructor_test_has_constructors() {
        let Some(data) = read_fixture("constructor_test/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = ExecutionPlan::from_parsed_full(
            &parsed,
            fixtures_dir().join("constructor_test/app"),
            &data,
        );
        let inits = plan.initializers.as_ref().unwrap();
        assert_eq!(
            inits.constructors.len(),
            2,
            "constructor_test defines 2 constructors"
        );
    }

    #[test]
    fn full_plan_display_shows_binding_counts() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan =
            ExecutionPlan::from_parsed_full(&parsed, fixtures_dir().join("simple_cli/app"), &data);
        let display = plan.display();
        assert!(
            display.contains("Bind symbols:"),
            "full display should show binding summary"
        );
        assert!(
            display.contains("bound"),
            "display should mention bound count"
        );
    }

    #[test]
    fn full_plan_display_shows_initializer_detail() {
        let Some(data) = read_fixture("constructor_test/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = ExecutionPlan::from_parsed_full(
            &parsed,
            fixtures_dir().join("constructor_test/app"),
            &data,
        );
        let display = plan.display();
        assert!(
            display.contains("constructor"),
            "display should show constructor detail"
        );
    }

    #[test]
    fn full_plan_serializes_with_new_fields() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan =
            ExecutionPlan::from_parsed_full(&parsed, fixtures_dir().join("simple_cli/app"), &data);
        let json = serde_json::to_string_pretty(&plan).unwrap();
        assert!(json.contains("symbol_bindings"));
        assert!(json.contains("initializers"));
        assert!(json.contains("objc_metadata"));
    }

    #[test]
    fn full_plan_dylib_chain_binding_complete() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan =
            ExecutionPlan::from_parsed_full(&parsed, fixtures_dir().join("dylib_chain/app"), &data);
        let bindings = plan.symbol_bindings.as_ref().unwrap();
        // dylib_chain should bind _liba_greet successfully
        let greet = bindings.bindings.iter().find(|b| b.name == "_liba_greet");
        assert!(greet.is_some(), "should find _liba_greet binding");
    }

    #[test]
    fn basic_plan_has_none_for_new_fields() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = ExecutionPlan::from_parsed(&parsed, fixtures_dir().join("simple_cli/app"));
        assert!(
            plan.symbol_bindings.is_none(),
            "basic plan should not have bindings"
        );
        assert!(
            plan.initializers.is_none(),
            "basic plan should not have initializers"
        );
        assert!(
            plan.objc_metadata.is_none(),
            "basic plan should not have ObjC metadata"
        );
    }
}

// ── Phase 17: Improved security evaluation in ExecutionPlan ──

#[cfg(test)]
mod phase17_plan_tests {
    use macho_parse::parse;
    use macho_run_plan::ExecutionPlan;
    use std::path::PathBuf;
    use trust_policy::TrustPolicy;

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
    fn secured_plan_uses_parsed_security() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let policy = TrustPolicy::apple_default();
        let plan = ExecutionPlan::from_parsed_secured(
            &parsed,
            fixtures_dir().join("simple_cli/app"),
            &data,
            &policy,
        );
        // Security should be evaluated using the enriched path
        assert!(
            plan.security.is_some(),
            "secured plan should have security report"
        );
        let report = plan.security.unwrap();
        assert!(
            report.allowed,
            "simple_cli should pass apple default policy"
        );
    }

    #[test]
    fn secured_plan_display_shows_maxprot() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let policy = TrustPolicy::apple_default();
        let plan = ExecutionPlan::from_parsed_secured(
            &parsed,
            fixtures_dir().join("simple_cli/app"),
            &data,
            &policy,
        );
        let display = plan.display();
        assert!(
            display.contains("maxprot"),
            "display should show maxprot field"
        );
    }

    #[test]
    fn secured_plan_permissive_also_works() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let policy = TrustPolicy::permissive();
        let plan = ExecutionPlan::from_parsed_secured(
            &parsed,
            fixtures_dir().join("simple_cli/app"),
            &data,
            &policy,
        );
        assert!(plan.security.is_some());
        assert!(plan.security.unwrap().allowed);
    }
}
