//! Tests for Apple-default restrictive gate implementations and apple_default policy.

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
