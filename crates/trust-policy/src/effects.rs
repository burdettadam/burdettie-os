//! Entitlement-driven cross-gate effects.
//!
//! Computes how entitlements from Gate 3 modify the behavior of other gates
//! (SIP env stripping, hardened runtime, library validation, sandbox activation).
//! This is computed once after entitlement parsing and applied to the SecurityReport.

use serde::{Deserialize, Serialize};

/// Cross-gate effects computed from entitlements.
///
/// These effects are applied after all individual gates have run,
/// modifying the final security report based on entitlement semantics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntitlementEffects {
    /// Entitlement disables library validation (Gate 7 becomes permissive).
    pub disable_library_validation: bool,
    /// Entitlement allows DYLD_* environment variables (Gate 5/6 don't strip).
    pub allow_dyld_env: bool,
    /// Entitlement enables app sandbox (Gate 8 should enforce).
    pub app_sandbox: bool,
    /// Entitlement allows JIT (MAP_JIT permitted under hardened runtime).
    pub allow_jit: bool,
    /// Entitlement allows unsigned executable memory.
    pub allow_unsigned_memory: bool,
    /// Entitlement has debugger capability.
    pub debugger: bool,
    /// get-task-allow set (debugging permitted, blocks notarization).
    pub get_task_allow: bool,
    /// Entitlement inherits parent entitlements.
    pub inherit: bool,
    /// How many security-relevant entitlements were found.
    pub security_relevant_count: usize,
}

impl EntitlementEffects {
    /// Compute effects from key/bool entitlement pairs.
    pub fn from_keys(keys: &[(String, bool)]) -> Self {
        let mut effects = Self::default();
        let mut count = 0;

        for (key, value) in keys {
            if !value {
                continue;
            }
            match key.as_str() {
                "com.apple.security.cs.disable-library-validation" => {
                    effects.disable_library_validation = true;
                    count += 1;
                }
                "com.apple.security.cs.allow-dyld-environment-variables" => {
                    effects.allow_dyld_env = true;
                    count += 1;
                }
                "com.apple.security.app-sandbox" => {
                    effects.app_sandbox = true;
                    count += 1;
                }
                "com.apple.security.cs.allow-jit" => {
                    effects.allow_jit = true;
                    count += 1;
                }
                "com.apple.security.cs.allow-unsigned-executable-memory" => {
                    effects.allow_unsigned_memory = true;
                    count += 1;
                }
                "com.apple.security.cs.debugger" => {
                    effects.debugger = true;
                    count += 1;
                }
                "com.apple.security.get-task-allow"
                | "com.apple.security.cs.get-task-allow" => {
                    effects.get_task_allow = true;
                    count += 1;
                }
                "com.apple.security.inherit" => {
                    effects.inherit = true;
                    count += 1;
                }
                _ => {}
            }
        }

        effects.security_relevant_count = count;
        effects
    }

    /// Apply these effects to a SecurityReport, modifying it in place.
    ///
    /// This is called after all gates have run to apply cross-gate effects:
    /// - `disable_library_validation` → relaxes Gate 7
    /// - `allow_dyld_env` → overrides SIP env stripping
    /// - CS_RUNTIME from cs_info → enforces hardened runtime
    pub fn apply_to_report(&self, report: &mut crate::SecurityReport) {
        // If entitlement allows DYLD env vars, don't strip them
        // (unless CS_RESTRICT is set — that takes precedence)
        if self.allow_dyld_env {
            let cs_restrict = report.cs_info.as_ref().is_some_and(|cs| cs.restrict);
            if !cs_restrict {
                report.strip_dyld_env = false;
            }
        }

        // Note: CS_RUNTIME override is already handled in evaluate_binary_parsed_with_keys.
        // Additional effects (sandbox activation, library validation relaxation)
        // are advisory — they don't directly modify report fields but are available
        // for downstream code to query.
    }

    /// Check if any security-relaxing entitlements are present.
    pub fn has_relaxations(&self) -> bool {
        self.disable_library_validation
            || self.allow_dyld_env
            || self.allow_jit
            || self.allow_unsigned_memory
            || self.debugger
    }

    /// Return a summary of which effects are active.
    pub fn summary(&self) -> Vec<&'static str> {
        let mut s = Vec::new();
        if self.disable_library_validation {
            s.push("disable-library-validation");
        }
        if self.allow_dyld_env {
            s.push("allow-dyld-env");
        }
        if self.app_sandbox {
            s.push("app-sandbox");
        }
        if self.allow_jit {
            s.push("allow-jit");
        }
        if self.allow_unsigned_memory {
            s.push("allow-unsigned-memory");
        }
        if self.debugger {
            s.push("debugger");
        }
        if self.get_task_allow {
            s.push("get-task-allow");
        }
        if self.inherit {
            s.push("inherit");
        }
        s
    }
}

/// Extended security report that includes entitlement effects and
/// hardened runtime effects.
///
/// This wraps the base SecurityReport with computed cross-gate effects,
/// giving downstream code a complete picture of the binary's security posture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedSecurityReport {
    /// The base security report from gate evaluation.
    pub base: crate::SecurityReport,
    /// Cross-gate effects from entitlements.
    pub entitlement_effects: EntitlementEffects,
}

impl EnrichedSecurityReport {
    /// Build from a base report and entitlement keys.
    pub fn from_report(
        mut base: crate::SecurityReport,
        entitlement_keys: &[(String, bool)],
    ) -> Self {
        let entitlement_effects = EntitlementEffects::from_keys(entitlement_keys);
        entitlement_effects.apply_to_report(&mut base);
        Self {
            base,
            entitlement_effects,
        }
    }

    /// Whether the binary is allowed to execute.
    pub fn allowed(&self) -> bool {
        self.base.allowed
    }

    /// Whether library validation is effectively disabled.
    pub fn library_validation_disabled(&self) -> bool {
        self.entitlement_effects.disable_library_validation
    }

    /// Whether sandbox should be enforced for this binary.
    pub fn sandbox_required(&self) -> bool {
        self.entitlement_effects.app_sandbox
    }

    /// Whether the binary is hardened (from base report).
    pub fn is_hardened(&self) -> bool {
        self.base.hardened_runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodeSignatureInfo;

    fn make_report() -> crate::SecurityReport {
        crate::SecurityReport {
            decisions: Vec::new(),
            allowed: true,
            denied_reasons: Vec::new(),
            strip_dyld_env: true,
            hardened_runtime: true,
            cs_info: Some(CodeSignatureInfo {
                restrict: false,
                ..Default::default()
            }),
        }
    }

    fn key(name: &str) -> (String, bool) {
        (name.to_string(), true)
    }

    #[test]
    fn empty_keys_no_effects() {
        let effects = EntitlementEffects::from_keys(&[]);
        assert!(!effects.disable_library_validation);
        assert!(!effects.allow_dyld_env);
        assert!(!effects.app_sandbox);
        assert!(!effects.allow_jit);
        assert!(!effects.allow_unsigned_memory);
        assert!(!effects.debugger);
        assert!(!effects.get_task_allow);
        assert!(!effects.inherit);
        assert_eq!(effects.security_relevant_count, 0);
    }

    #[test]
    fn disable_library_validation_effect() {
        let keys = [key("com.apple.security.cs.disable-library-validation")];
        let effects = EntitlementEffects::from_keys(&keys);
        assert!(effects.disable_library_validation);
        assert_eq!(effects.security_relevant_count, 1);
    }

    #[test]
    fn allow_dyld_env_effect() {
        let keys = [key(
            "com.apple.security.cs.allow-dyld-environment-variables",
        )];
        let effects = EntitlementEffects::from_keys(&keys);
        assert!(effects.allow_dyld_env);
        assert_eq!(effects.security_relevant_count, 1);
    }

    #[test]
    fn app_sandbox_effect() {
        let keys = [key("com.apple.security.app-sandbox")];
        let effects = EntitlementEffects::from_keys(&keys);
        assert!(effects.app_sandbox);
        assert_eq!(effects.security_relevant_count, 1);
    }

    #[test]
    fn multiple_effects() {
        let keys = [
            key("com.apple.security.cs.disable-library-validation"),
            key("com.apple.security.cs.allow-dyld-environment-variables"),
            key("com.apple.security.app-sandbox"),
        ];
        let effects = EntitlementEffects::from_keys(&keys);
        assert!(effects.disable_library_validation);
        assert!(effects.allow_dyld_env);
        assert!(effects.app_sandbox);
        assert_eq!(effects.security_relevant_count, 3);
    }

    #[test]
    fn has_relaxations_true() {
        let keys = [key("com.apple.security.cs.disable-library-validation")];
        let effects = EntitlementEffects::from_keys(&keys);
        assert!(effects.has_relaxations());
    }

    #[test]
    fn has_relaxations_false() {
        let keys = [key("com.apple.security.app-sandbox")];
        let effects = EntitlementEffects::from_keys(&keys);
        assert!(!effects.has_relaxations());
    }

    #[test]
    fn summary_lists_active() {
        let keys = [
            key("com.apple.security.cs.allow-jit"),
            key("com.apple.security.cs.debugger"),
        ];
        let effects = EntitlementEffects::from_keys(&keys);
        let s = effects.summary();
        assert_eq!(s.len(), 2);
        assert!(s.contains(&"allow-jit"));
        assert!(s.contains(&"debugger"));
    }

    #[test]
    fn apply_to_report_strips_dyld_override() {
        let effects = EntitlementEffects {
            allow_dyld_env: true,
            ..Default::default()
        };
        let mut report = make_report();
        assert!(report.strip_dyld_env);
        effects.apply_to_report(&mut report);
        assert!(!report.strip_dyld_env);
    }

    #[test]
    fn apply_to_report_cs_restrict_overrides() {
        let effects = EntitlementEffects {
            allow_dyld_env: true,
            ..Default::default()
        };
        let mut report = make_report();
        report.cs_info = Some(CodeSignatureInfo {
            restrict: true,
            ..Default::default()
        });
        effects.apply_to_report(&mut report);
        assert!(report.strip_dyld_env);
    }

    #[test]
    fn enriched_report_from_report() {
        let report = make_report();
        let keys = [key(
            "com.apple.security.cs.allow-dyld-environment-variables",
        )];
        let enriched = EnrichedSecurityReport::from_report(report, &keys);
        assert!(enriched.allowed());
        assert!(enriched.entitlement_effects.allow_dyld_env);
        assert!(!enriched.base.strip_dyld_env);
    }

    #[test]
    fn enriched_report_library_validation_disabled() {
        let report = make_report();
        let keys = [key("com.apple.security.cs.disable-library-validation")];
        let enriched = EnrichedSecurityReport::from_report(report, &keys);
        assert!(enriched.library_validation_disabled());
    }

    #[test]
    fn enriched_report_sandbox_required() {
        let report = make_report();
        let keys = [key("com.apple.security.app-sandbox")];
        let enriched = EnrichedSecurityReport::from_report(report, &keys);
        assert!(enriched.sandbox_required());
    }

    #[test]
    fn false_value_ignored() {
        let keys = [(
            "com.apple.security.cs.disable-library-validation".to_string(),
            false,
        )];
        let effects = EntitlementEffects::from_keys(&keys);
        assert!(!effects.disable_library_validation);
        assert_eq!(effects.security_relevant_count, 0);
    }
}
