//! Tests for full execution plans (from_parsed_full) and secured plans (from_parsed_secured).
//!
//! Covers: symbol bindings, initializers, ObjC metadata in full plans,
//! display output, serialization, and security evaluation via TrustPolicy.

mod common;

use macho_parse::parse;
use macho_run_plan::ExecutionPlan;

// ── Full execution plan tests (from_parsed_full) ───────────────────

#[test]
fn full_plan_has_symbol_bindings() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed_full(
        &parsed,
        common::fixtures_dir().join("simple_cli/app"),
        &data,
    );
    assert!(
        plan.symbol_bindings.is_some(),
        "full plan should include symbol bindings"
    );
}

#[test]
fn full_plan_has_initializers() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed_full(
        &parsed,
        common::fixtures_dir().join("simple_cli/app"),
        &data,
    );
    assert!(
        plan.initializers.is_some(),
        "full plan should include initializers"
    );
}

#[test]
fn full_plan_has_objc_metadata() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed_full(
        &parsed,
        common::fixtures_dir().join("simple_cli/app"),
        &data,
    );
    assert!(
        plan.objc_metadata.is_some(),
        "full plan should include ObjC metadata"
    );
}

#[test]
fn full_plan_simple_cli_no_constructors() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed_full(
        &parsed,
        common::fixtures_dir().join("simple_cli/app"),
        &data,
    );
    let inits = plan.initializers.as_ref().unwrap();
    assert!(
        inits.constructors.is_empty(),
        "simple_cli has no constructors"
    );
}

#[test]
fn full_plan_constructor_test_has_constructors() {
    let Some(data) = common::read_fixture("constructor_test/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed_full(
        &parsed,
        common::fixtures_dir().join("constructor_test/app"),
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
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed_full(
        &parsed,
        common::fixtures_dir().join("simple_cli/app"),
        &data,
    );
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
    let Some(data) = common::read_fixture("constructor_test/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed_full(
        &parsed,
        common::fixtures_dir().join("constructor_test/app"),
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
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed_full(
        &parsed,
        common::fixtures_dir().join("simple_cli/app"),
        &data,
    );
    let json = serde_json::to_string_pretty(&plan).unwrap();
    assert!(json.contains("symbol_bindings"));
    assert!(json.contains("initializers"));
    assert!(json.contains("objc_metadata"));
}

#[test]
fn full_plan_dylib_chain_binding_complete() {
    let Some(data) = common::read_fixture("dylib_chain/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = ExecutionPlan::from_parsed_full(
        &parsed,
        common::fixtures_dir().join("dylib_chain/app"),
        &data,
    );
    let bindings = plan.symbol_bindings.as_ref().unwrap();
    // dylib_chain should bind _liba_greet successfully
    let greet = bindings.bindings.iter().find(|b| b.name == "_liba_greet");
    assert!(greet.is_some(), "should find _liba_greet binding");
}

#[test]
fn basic_plan_has_none_for_new_fields() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan =
        ExecutionPlan::from_parsed(&parsed, common::fixtures_dir().join("simple_cli/app"));
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

// ── Phase 17: Improved security evaluation in ExecutionPlan ──

use trust_policy::TrustPolicy;

#[test]
fn secured_plan_uses_parsed_security() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let policy = TrustPolicy::apple_default();
    let plan = ExecutionPlan::from_parsed_secured(
        &parsed,
        common::fixtures_dir().join("simple_cli/app"),
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
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let policy = TrustPolicy::apple_default();
    let plan = ExecutionPlan::from_parsed_secured(
        &parsed,
        common::fixtures_dir().join("simple_cli/app"),
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
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let policy = TrustPolicy::permissive();
    let plan = ExecutionPlan::from_parsed_secured(
        &parsed,
        common::fixtures_dir().join("simple_cli/app"),
        &data,
        &policy,
    );
    assert!(plan.security.is_some());
    assert!(plan.security.unwrap().allowed);
}
