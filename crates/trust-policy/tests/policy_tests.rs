//! Tests for trust-policy: security gate traits, mock policies, and integration.

#[cfg(test)]
mod verdict_tests {
    use trust_policy::{GateDecision, SecurityVerdict};

    #[test]
    fn allow_is_allow() {
        assert!(SecurityVerdict::Allow.is_allow());
        assert!(!SecurityVerdict::Allow.is_deny());
    }

    #[test]
    fn deny_is_deny() {
        let v = SecurityVerdict::Deny {
            reason: "test".into(),
        };
        assert!(v.is_deny());
        assert!(!v.is_allow());
    }

    #[test]
    fn allow_with_warning_is_allow() {
        let v = SecurityVerdict::AllowWithWarning {
            warning: "watch out".into(),
        };
        assert!(v.is_allow());
        assert!(!v.is_deny());
    }

    #[test]
    fn verdict_serializes_roundtrip() {
        let v = SecurityVerdict::Deny {
            reason: "unsigned".into(),
        };
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("unsigned"));
        let back: SecurityVerdict = serde_json::from_str(&json).unwrap();
        assert!(back.is_deny());
    }

    #[test]
    fn gate_decision_serializes() {
        let d = GateDecision {
            gate: "Code Signature".into(),
            verdict: SecurityVerdict::Allow,
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("Code Signature"));
        let back: GateDecision = serde_json::from_str(&json).unwrap();
        assert!(back.verdict.is_allow());
    }
}

#[cfg(test)]
mod mock_tests {
    use std::path::Path;
    use trust_policy::*;

    #[test]
    fn mock_quarantine_allows() {
        let q = MockQuarantine;
        assert_eq!(q.name(), "Quarantine (mock)");
        assert!(!q.is_enforcing());
        assert!(q.check_quarantine(Path::new("/tmp/app")).is_allow());
    }

    #[test]
    fn mock_code_signature_allows() {
        let cs = MockCodeSignature;
        assert!(!cs.is_enforcing());
        assert!(cs
            .validate_signature(Path::new("/tmp/app"), false)
            .is_allow());
        assert!(cs
            .validate_signature(Path::new("/tmp/app"), true)
            .is_allow());
    }

    #[test]
    fn mock_entitlements_allows() {
        let e = MockEntitlements;
        assert!(e.evaluate(Path::new("/tmp/app")).is_allow());
    }

    #[test]
    fn mock_amfi_allows() {
        let a = MockAmfi;
        assert!(a
            .check_code_integrity(Path::new("/tmp/app"), true)
            .is_allow());
    }

    #[test]
    fn mock_sip_never_strips() {
        let s = MockSip;
        assert!(!s.should_strip_dyld_env(
            Path::new("/usr/bin/ls"),
            macho_types::header::MachFlags::empty(),
        ));
    }

    #[test]
    fn mock_sip_nothing_protected() {
        let s = MockSip;
        assert!(!s.is_path_protected(Path::new("/usr/lib/libSystem.B.dylib")));
        assert!(!s.is_path_protected(Path::new(
            "/System/Library/Frameworks/Foundation.framework/Foundation"
        )));
    }

    #[test]
    fn mock_hardened_runtime_not_hardened() {
        let h = MockHardenedRuntime;
        assert!(!h.is_hardened(Path::new("/tmp/app"), macho_types::header::MachFlags::PIE));
    }

    #[test]
    fn mock_hardened_runtime_allows_insertion() {
        let h = MockHardenedRuntime;
        assert!(h.allows_dylib_insertion(
            Path::new("/tmp/app"),
            macho_types::header::MachFlags::empty()
        ));
    }

    #[test]
    fn mock_library_validator_allows() {
        let lv = MockLibraryValidator;
        assert!(lv
            .validate_dylib(
                Path::new("/usr/bin/app"),
                Path::new("/usr/lib/libfoo.dylib"),
            )
            .is_allow());
    }

    #[test]
    fn mock_sandbox_allows() {
        let sb = MockSandbox;
        assert!(sb.check_file_access(Path::new("/tmp/secret")).is_allow());
    }
}

#[cfg(test)]
mod policy_tests {
    use macho_types::header::MachFlags;
    use std::path::Path;
    use trust_policy::{SecurityReport, TrustPolicy};

    #[test]
    fn permissive_policy_allows_everything() {
        let policy = TrustPolicy::permissive();
        let report = policy.evaluate_binary(
            Path::new("/tmp/test_app"),
            MachFlags::PIE | MachFlags::TWO_LEVEL,
            true,
        );
        assert!(report.allowed, "permissive policy should allow everything");
        assert!(report.denied_reasons.is_empty());
        assert!(!report.strip_dyld_env, "mock SIP should not strip env");
        assert!(!report.hardened_runtime, "mock should not report hardened");
    }

    #[test]
    fn permissive_policy_has_all_gates() {
        let policy = TrustPolicy::permissive();
        let report = policy.evaluate_binary(Path::new("/tmp/test_app"), MachFlags::empty(), false);
        // Should have decisions for: Quarantine, Code Sig, Entitlements, AMFI, SIP, Hardened
        assert_eq!(
            report.decisions.len(),
            6,
            "should evaluate 6 pre-load gates"
        );
    }

    #[test]
    fn permissive_policy_gate_names() {
        let policy = TrustPolicy::permissive();
        let report = policy.evaluate_binary(Path::new("/tmp/test_app"), MachFlags::empty(), true);
        let names: Vec<&str> = report.decisions.iter().map(|d| d.gate.as_str()).collect();
        assert!(names.contains(&"Quarantine (mock)"));
        assert!(names.contains(&"Code Signature (mock)"));
        assert!(names.contains(&"Entitlements (mock)"));
        assert!(names.contains(&"AMFI (mock)"));
        assert!(names.contains(&"SIP (mock)"));
        assert!(names.contains(&"Hardened Runtime (mock)"));
    }

    #[test]
    fn validate_dylib_allows_in_permissive() {
        let policy = TrustPolicy::permissive();
        let verdict = policy.validate_dylib(
            Path::new("/usr/bin/app"),
            Path::new("/usr/lib/libSystem.B.dylib"),
        );
        assert!(verdict.is_allow());
    }

    #[test]
    fn security_report_serializes_roundtrip() {
        let policy = TrustPolicy::permissive();
        let report = policy.evaluate_binary(Path::new("/tmp/test_app"), MachFlags::PIE, true);
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("allowed"));
        assert!(json.contains("decisions"));
        let back: SecurityReport = serde_json::from_str(&json).unwrap();
        assert!(back.allowed);
        assert_eq!(back.decisions.len(), 6);
    }

    #[test]
    fn unsigned_binary_still_allowed_by_mock() {
        let policy = TrustPolicy::permissive();
        let report = policy.evaluate_binary(
            Path::new("/tmp/unsigned_app"),
            MachFlags::empty(),
            false, // no code signature
        );
        assert!(report.allowed, "mock policy should allow unsigned binaries");
    }
}

// ── Integration with ExecutionPlan ──────────────────────────────────

#[cfg(test)]
mod plan_integration_tests {
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

// ── Apple-default restrictive policy tests ──────────────────────────

#[cfg(test)]
mod apple_gate_tests {
    use macho_types::header::MachFlags;
    use std::path::Path;
    use trust_policy::*;

    // ── SIP path detection ──────────────────────────────────────────

    #[test]
    fn sip_protects_usr_bin() {
        let sip = AppleSip;
        assert!(sip.is_path_protected(Path::new("/usr/bin/ls")));
    }

    #[test]
    fn sip_protects_system() {
        let sip = AppleSip;
        assert!(sip.is_path_protected(Path::new(
            "/System/Library/Frameworks/Foundation.framework/Foundation"
        )));
    }

    #[test]
    fn sip_protects_bin() {
        let sip = AppleSip;
        assert!(sip.is_path_protected(Path::new("/bin/sh")));
    }

    #[test]
    fn sip_protects_sbin() {
        let sip = AppleSip;
        assert!(sip.is_path_protected(Path::new("/sbin/mount")));
    }

    #[test]
    fn sip_exempts_usr_local() {
        let sip = AppleSip;
        assert!(!sip.is_path_protected(Path::new("/usr/local/bin/brew")));
    }

    #[test]
    fn sip_does_not_protect_tmp() {
        let sip = AppleSip;
        assert!(!sip.is_path_protected(Path::new("/tmp/test_app")));
    }

    #[test]
    fn sip_does_not_protect_home() {
        let sip = AppleSip;
        assert!(!sip.is_path_protected(Path::new("/Users/dev/build/app")));
    }

    #[test]
    fn sip_strips_dyld_env_for_protected() {
        let sip = AppleSip;
        assert!(sip.should_strip_dyld_env(Path::new("/usr/bin/ls"), MachFlags::empty(),));
    }

    #[test]
    fn sip_no_strip_for_unprotected() {
        let sip = AppleSip;
        assert!(!sip.should_strip_dyld_env(Path::new("/tmp/my_app"), MachFlags::empty(),));
    }

    // ── Code signature enforcement ──────────────────────────────────

    #[test]
    fn code_sig_allows_signed_binary() {
        let cs = AppleCodeSignature;
        let v = cs.validate_signature(Path::new("/usr/bin/ls"), true);
        assert!(v.is_allow());
    }

    #[test]
    fn code_sig_denies_unsigned_sip_binary() {
        let cs = AppleCodeSignature;
        let v = cs.validate_signature(Path::new("/usr/bin/ls"), false);
        assert!(v.is_deny());
    }

    #[test]
    fn code_sig_warns_unsigned_non_sip() {
        let cs = AppleCodeSignature;
        let v = cs.validate_signature(Path::new("/tmp/test_app"), false);
        // Non-SIP unsigned binary: allowed with warning
        assert!(v.is_allow());
        assert!(matches!(v, SecurityVerdict::AllowWithWarning { .. }));
    }

    // ── AMFI ────────────────────────────────────────────────────────

    #[test]
    fn amfi_allows_signed() {
        let amfi = AppleAmfi;
        assert!(amfi
            .check_code_integrity(Path::new("/usr/bin/ls"), true)
            .is_allow());
    }

    #[test]
    fn amfi_denies_unsigned_sip() {
        let amfi = AppleAmfi;
        assert!(amfi
            .check_code_integrity(Path::new("/usr/bin/ls"), false)
            .is_deny());
    }

    #[test]
    fn amfi_warns_unsigned_non_sip() {
        let amfi = AppleAmfi;
        let v = amfi.check_code_integrity(Path::new("/tmp/app"), false);
        assert!(v.is_allow());
        assert!(matches!(v, SecurityVerdict::AllowWithWarning { .. }));
    }

    // ── Hardened runtime ────────────────────────────────────────────

    #[test]
    fn hardened_active_for_sip_path() {
        let hr = AppleHardenedRuntime;
        assert!(hr.is_hardened(Path::new("/usr/bin/ls"), MachFlags::empty()));
    }

    #[test]
    fn hardened_inactive_for_non_sip() {
        let hr = AppleHardenedRuntime;
        assert!(!hr.is_hardened(Path::new("/tmp/my_app"), MachFlags::empty()));
    }

    #[test]
    fn hardened_blocks_dylib_insertion_for_sip() {
        let hr = AppleHardenedRuntime;
        assert!(!hr.allows_dylib_insertion(Path::new("/usr/bin/ls"), MachFlags::empty()));
    }

    #[test]
    fn hardened_allows_dylib_insertion_for_non_sip() {
        let hr = AppleHardenedRuntime;
        assert!(hr.allows_dylib_insertion(Path::new("/tmp/my_app"), MachFlags::empty()));
    }

    // ── Library validator ───────────────────────────────────────────

    #[test]
    fn lib_validator_allows_sip_to_sip() {
        let lv = AppleLibraryValidator;
        let v = lv.validate_dylib(
            Path::new("/usr/bin/ls"),
            Path::new("/usr/lib/libSystem.B.dylib"),
        );
        assert!(v.is_allow());
    }

    #[test]
    fn lib_validator_denies_sip_to_non_sip() {
        let lv = AppleLibraryValidator;
        let v = lv.validate_dylib(Path::new("/usr/bin/ls"), Path::new("/tmp/evil.dylib"));
        assert!(v.is_deny());
    }

    #[test]
    fn lib_validator_allows_non_sip_to_non_sip() {
        let lv = AppleLibraryValidator;
        let v = lv.validate_dylib(Path::new("/tmp/my_app"), Path::new("/tmp/my_lib.dylib"));
        assert!(v.is_allow());
    }

    #[test]
    fn lib_validator_allows_non_sip_to_sip() {
        let lv = AppleLibraryValidator;
        let v = lv.validate_dylib(
            Path::new("/tmp/my_app"),
            Path::new("/usr/lib/libSystem.B.dylib"),
        );
        assert!(v.is_allow());
    }

    // ── Quarantine ──────────────────────────────────────────────────

    #[test]
    fn quarantine_is_enforcing() {
        let q = AppleQuarantine;
        assert!(q.is_enforcing());
    }

    #[test]
    fn quarantine_warns() {
        let q = AppleQuarantine;
        let v = q.check_quarantine(Path::new("/tmp/downloaded_app"));
        assert!(v.is_allow());
        assert!(matches!(v, SecurityVerdict::AllowWithWarning { .. }));
    }
}

#[cfg(test)]
mod apple_policy_tests {
    use macho_types::header::MachFlags;
    use std::path::Path;
    use trust_policy::TrustPolicy;

    #[test]
    fn apple_default_allows_signed_non_sip() {
        let policy = TrustPolicy::apple_default();
        let report = policy.evaluate_binary(
            Path::new("/tmp/test_app"),
            MachFlags::PIE | MachFlags::TWO_LEVEL,
            true,
        );
        assert!(report.allowed);
        assert!(!report.strip_dyld_env);
        assert!(!report.hardened_runtime);
    }

    #[test]
    fn apple_default_allows_signed_sip() {
        let policy = TrustPolicy::apple_default();
        let report = policy.evaluate_binary(
            Path::new("/usr/bin/ls"),
            MachFlags::PIE | MachFlags::TWO_LEVEL,
            true,
        );
        assert!(report.allowed);
        assert!(report.strip_dyld_env, "SIP binary should strip DYLD_* env");
        assert!(report.hardened_runtime, "SIP binary should be hardened");
    }

    #[test]
    fn apple_default_denies_unsigned_sip() {
        let policy = TrustPolicy::apple_default();
        let report = policy.evaluate_binary(
            Path::new("/usr/bin/malicious"),
            MachFlags::empty(),
            false, // no code signature
        );
        assert!(
            !report.allowed,
            "unsigned binary in SIP path should be denied"
        );
        assert!(!report.denied_reasons.is_empty());
    }

    #[test]
    fn apple_default_warns_unsigned_non_sip() {
        let policy = TrustPolicy::apple_default();
        let report = policy.evaluate_binary(Path::new("/tmp/dev_build"), MachFlags::empty(), false);
        // Non-SIP unsigned: allowed but with warnings
        assert!(report.allowed);
        // Should have warnings from quarantine + code sig + AMFI
        let warnings: Vec<_> = report
            .decisions
            .iter()
            .filter(|d| {
                matches!(
                    d.verdict,
                    trust_policy::SecurityVerdict::AllowWithWarning { .. }
                )
            })
            .collect();
        assert!(
            warnings.len() >= 2,
            "should have at least 2 warnings for unsigned non-SIP binary"
        );
    }

    #[test]
    fn apple_default_has_6_gate_decisions() {
        let policy = TrustPolicy::apple_default();
        let report = policy.evaluate_binary(Path::new("/tmp/app"), MachFlags::empty(), true);
        assert_eq!(report.decisions.len(), 6);
    }

    #[test]
    fn apple_default_gate_names_are_not_mock() {
        let policy = TrustPolicy::apple_default();
        let report = policy.evaluate_binary(Path::new("/tmp/app"), MachFlags::empty(), true);
        for d in &report.decisions {
            assert!(
                !d.gate.contains("mock"),
                "apple_default should not use mock gates, got: {}",
                d.gate
            );
        }
    }

    #[test]
    fn apple_default_dylib_sip_to_sip_allowed() {
        let policy = TrustPolicy::apple_default();
        let v = policy.validate_dylib(
            Path::new("/usr/bin/ls"),
            Path::new("/usr/lib/libSystem.B.dylib"),
        );
        assert!(v.is_allow());
    }

    #[test]
    fn apple_default_dylib_sip_to_non_sip_denied() {
        let policy = TrustPolicy::apple_default();
        let v = policy.validate_dylib(Path::new("/usr/bin/ls"), Path::new("/tmp/evil.dylib"));
        assert!(v.is_deny());
    }

    #[test]
    fn apple_serializes_report() {
        let policy = TrustPolicy::apple_default();
        let report = policy.evaluate_binary(Path::new("/usr/bin/ls"), MachFlags::PIE, true);
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("Quarantine"));
        assert!(json.contains("SIP"));
        assert!(json.contains("Hardened Runtime"));
        assert!(!json.contains("mock"));
    }
}

// ── Apple policy + fixture integration ──────────────────────────────

#[cfg(test)]
mod apple_fixture_tests {
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

// ── Phase 16: Enriched EntitlementEvaluator ─────────────────────────

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
