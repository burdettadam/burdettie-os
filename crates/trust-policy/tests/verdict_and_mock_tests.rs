//! Tests for SecurityVerdict, GateDecision, mock gates, and permissive policy.

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
