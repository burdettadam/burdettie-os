//! Tests for ExecutionPlan integration, Apple fixture validation, and entitlement evaluation.

mod common;

// ── Integration with ExecutionPlan ──────────────────────────────────

#[cfg(test)]
mod plan_integration_tests {
    use macho_parse::parse;
    use macho_run_plan::ExecutionPlan;
    use trust_policy::TrustPolicy;

    use crate::common::{fixtures_dir, read_fixture};

    #[test]
    fn secured_plan_has_security_report() {
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
        assert!(
            plan.security.is_some(),
            "secured plan should have security report"
        );
        let sec = plan.security.as_ref().unwrap();
        assert!(sec.allowed);
    }

    #[test]
    fn secured_plan_also_has_bindings() {
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
        assert!(plan.symbol_bindings.is_some());
        assert!(plan.initializers.is_some());
        assert!(plan.objc_metadata.is_some());
    }

    #[test]
    fn secured_plan_display_shows_security() {
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
        let display = plan.display();
        assert!(display.contains("Security: all gates passed"));
    }

    #[test]
    fn full_plan_without_policy_has_no_security() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan =
            ExecutionPlan::from_parsed_full(&parsed, fixtures_dir().join("simple_cli/app"), &data);
        assert!(
            plan.security.is_none(),
            "full plan without policy should have no security report"
        );
    }

    #[test]
    fn secured_plan_serializes_with_security() {
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
        let json = serde_json::to_string_pretty(&plan).unwrap();
        assert!(json.contains("\"security\""));
        assert!(json.contains("\"allowed\""));
        assert!(json.contains("\"decisions\""));
    }

    #[test]
    fn constructor_test_secured_plan() {
        let Some(data) = read_fixture("constructor_test/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let policy = TrustPolicy::permissive();
        let plan = ExecutionPlan::from_parsed_secured(
            &parsed,
            fixtures_dir().join("constructor_test/app"),
            &data,
            &policy,
        );
        assert!(plan.security.as_ref().unwrap().allowed);
        assert_eq!(plan.initializers.as_ref().unwrap().constructors.len(), 2);
    }
}

// ── Apple policy + fixture integration ──────────────────────────────

#[cfg(test)]
mod apple_fixture_tests {
    use macho_parse::parse;
    use macho_run_plan::ExecutionPlan;
    use trust_policy::TrustPolicy;

    use crate::common::{fixtures_dir, read_fixture};

    #[test]
    fn simple_cli_passes_apple_policy() {
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
        let sec = plan.security.as_ref().unwrap();
        assert!(sec.allowed, "fixture binary should pass apple policy");
        // Fixture is not in SIP path, so no env stripping or hardened runtime
        assert!(!sec.strip_dyld_env);
        assert!(!sec.hardened_runtime);
    }

    #[test]
    fn dylib_chain_passes_apple_policy() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let policy = TrustPolicy::apple_default();
        let plan = ExecutionPlan::from_parsed_secured(
            &parsed,
            fixtures_dir().join("dylib_chain/app"),
            &data,
            &policy,
        );
        assert!(plan.security.as_ref().unwrap().allowed);
    }

    #[test]
    fn constructor_test_passes_apple_policy() {
        let Some(data) = read_fixture("constructor_test/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let policy = TrustPolicy::apple_default();
        let plan = ExecutionPlan::from_parsed_secured(
            &parsed,
            fixtures_dir().join("constructor_test/app"),
            &data,
            &policy,
        );
        assert!(plan.security.as_ref().unwrap().allowed);
    }

    #[test]
    fn apple_display_shows_security_passed() {
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
        assert!(display.contains("Security: all gates passed"));
    }
}

// ── Enriched EntitlementEvaluator ───────────────────────────────────

#[cfg(test)]
mod entitlement_evaluator_tests {
    use std::path::Path;
    use trust_policy::{AppleEntitlements, SecurityVerdict};
    use trust_policy::{EntitlementEvaluator, SecurityGate};

    #[test]
    fn apple_entitlements_evaluate_allows_by_default() {
        let gate = AppleEntitlements;
        let verdict = gate.evaluate(Path::new("/tmp/app"));
        assert!(verdict.is_allow());
    }

    #[test]
    fn apple_entitlements_with_empty_keys_allows() {
        let gate = AppleEntitlements;
        let verdict = gate.evaluate_with_keys(Path::new("/tmp/app"), &[]);
        assert!(verdict.is_allow());
    }

    #[test]
    fn apple_entitlements_with_safe_keys_allows() {
        let gate = AppleEntitlements;
        let keys = vec![
            ("com.apple.security.cs.allow-jit".to_string(), true),
            ("com.apple.security.get-task-allow".to_string(), true),
        ];
        let verdict = gate.evaluate_with_keys(Path::new("/tmp/app"), &keys);
        assert!(verdict.is_allow());
    }

    #[test]
    fn apple_entitlements_warns_disable_library_validation() {
        let gate = AppleEntitlements;
        let keys = vec![(
            "com.apple.security.cs.disable-library-validation".to_string(),
            true,
        )];
        let verdict = gate.evaluate_with_keys(Path::new("/tmp/app"), &keys);
        match &verdict {
            SecurityVerdict::AllowWithWarning { warning } => {
                assert!(
                    warning.contains("library validation"),
                    "warning should mention library validation: {warning}"
                );
            }
            _ => panic!("expected AllowWithWarning, got {verdict:?}"),
        }
    }

    #[test]
    fn apple_entitlements_warns_allow_dyld_env() {
        let gate = AppleEntitlements;
        let keys = vec![(
            "com.apple.security.cs.allow-dyld-environment-variables".to_string(),
            true,
        )];
        let verdict = gate.evaluate_with_keys(Path::new("/tmp/app"), &keys);
        match &verdict {
            SecurityVerdict::AllowWithWarning { warning } => {
                assert!(
                    warning.contains("DYLD_"),
                    "warning should mention DYLD_: {warning}"
                );
            }
            _ => panic!("expected AllowWithWarning, got {verdict:?}"),
        }
    }

    #[test]
    fn apple_entitlements_false_value_does_not_warn() {
        let gate = AppleEntitlements;
        let keys = vec![(
            "com.apple.security.cs.disable-library-validation".to_string(),
            false,
        )];
        let verdict = gate.evaluate_with_keys(Path::new("/tmp/app"), &keys);
        assert!(
            verdict.is_allow(),
            "false entitlement should not trigger warning"
        );
    }

    #[test]
    fn mock_entitlements_evaluate_with_keys_delegates() {
        let gate = trust_policy::MockEntitlements;
        let keys = vec![(
            "com.apple.security.cs.disable-library-validation".to_string(),
            true,
        )];
        // MockEntitlements delegates to evaluate() which always allows
        let verdict = gate.evaluate_with_keys(Path::new("/tmp/app"), &keys);
        assert!(verdict.is_allow());
    }

    #[test]
    fn entitlement_evaluator_is_enforcing() {
        let gate = AppleEntitlements;
        assert!(gate.is_enforcing());
    }

    #[test]
    fn entitlement_evaluator_name() {
        let gate = AppleEntitlements;
        assert_eq!(gate.name(), "Entitlements");
    }

    // ── evaluate_binary_parsed_with_keys ────────────────────────────

    #[test]
    fn evaluate_with_keys_updates_entitlement_decision() {
        let policy = trust_policy::TrustPolicy::apple_default();
        let cs_info = trust_policy::CodeSignatureInfo {
            present: true,
            ..Default::default()
        };
        let keys = vec![(
            "com.apple.security.cs.disable-library-validation".to_string(),
            true,
        )];
        let report = policy.evaluate_binary_parsed_with_keys(
            Path::new("/tmp/app"),
            macho_types::header::MachFlags::empty(),
            &cs_info,
            &keys,
        );
        let ent_decision = report.decisions.iter().find(|d| d.gate == "Entitlements");
        assert!(ent_decision.is_some());
        match &ent_decision.unwrap().verdict {
            SecurityVerdict::AllowWithWarning { .. } => {}
            other => panic!("expected AllowWithWarning, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_parsed_without_keys_still_works() {
        let policy = trust_policy::TrustPolicy::apple_default();
        let cs_info = trust_policy::CodeSignatureInfo {
            present: true,
            ..Default::default()
        };
        let report = policy.evaluate_binary_parsed(
            Path::new("/tmp/app"),
            macho_types::header::MachFlags::empty(),
            &cs_info,
        );
        assert!(report.allowed);
    }
}
