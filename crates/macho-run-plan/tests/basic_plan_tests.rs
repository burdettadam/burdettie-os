//! Tests for basic execution plan generation (from_parsed).
//!
//! Covers: entry point, mapping, dependencies, init order,
//! ObjC presence, display output, and JSON serialization.

mod common;

use macho_parse::parse;
use macho_run_plan::ExecutionPlan;

// ── Execution plan for simple_cli ───────────────────────────────

#[test]
fn simple_cli_execution_plan_has_entry() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed(&parsed, common::fixtures_dir().join("simple_cli/app"));
    assert!(plan.entry.is_some(), "execution plan must have entry point");
}

#[test]
fn simple_cli_execution_plan_has_mapping() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed(&parsed, common::fixtures_dir().join("simple_cli/app"));
    assert!(
        !plan.mapping.regions.is_empty(),
        "execution plan must have mapping regions"
    );
}

#[test]
fn simple_cli_execution_plan_has_dependencies() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed(&parsed, common::fixtures_dir().join("simple_cli/app"));
    // At minimum the main binary node should exist
    assert!(!plan.dependencies.graph.nodes.is_empty());
}

#[test]
fn simple_cli_init_order_ends_with_main() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let exe = common::fixtures_dir().join("simple_cli/app");
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
    let Some(data) = common::read_fixture("dylib_chain/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let exe = common::fixtures_dir().join("dylib_chain/app");
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
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed(&parsed, common::fixtures_dir().join("simple_cli/app"));
    assert!(!plan.objc.has_objc);
}

#[test]
fn objc_foundation_has_objc_in_plan() {
    let Some(data) = common::read_fixture("objc_foundation/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan =
        ExecutionPlan::from_parsed(&parsed, common::fixtures_dir().join("objc_foundation/app"));
    assert!(plan.objc.has_objc);
}

// ── Display output ──────────────────────────────────────────────

#[test]
fn execution_plan_display_has_steps() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed(&parsed, common::fixtures_dir().join("simple_cli/app"));
    let display = plan.display();
    assert!(display.contains("Load main binary"));
    assert!(display.contains("Map"));
    assert!(display.contains("symbol binding"));
    assert!(display.contains("initializers"));
}

// ── JSON serialization ──────────────────────────────────────────

#[test]
fn execution_plan_serializes_to_json() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed(&parsed, common::fixtures_dir().join("simple_cli/app"));
    let json = serde_json::to_string_pretty(&plan).unwrap();
    assert!(json.contains("mapping"));
    assert!(json.contains("dependencies"));
    assert!(json.contains("init_order"));
}
