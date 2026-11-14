//! The aggregate admission outcome must describe the final gate decisions.
//! These use deterministic injected gates, not signature/authenticity fixtures.

use std::path::Path;

use macho_types::header::MachFlags;
use trust_policy::{
    CodeSignatureInfo, CodeSignatureValidator, EntitlementEvaluator, SecurityGate, SecurityReport,
    SecurityVerdict, TrustPolicy,
};

struct RevisableEntitlements {
    legacy_denied: bool,
}

impl SecurityGate for RevisableEntitlements {
    fn name(&self) -> &str {
        "test.entitlements"
    }

    fn is_enforcing(&self) -> bool {
        true
    }
}

impl EntitlementEvaluator for RevisableEntitlements {
    fn evaluate(&self, _path: &Path) -> SecurityVerdict {
        if self.legacy_denied {
            SecurityVerdict::Deny {
                reason: "legacy denial".to_string(),
            }
        } else {
            SecurityVerdict::Allow
        }
    }

    fn evaluate_with_keys(&self, _path: &Path, keys: &[(String, bool)]) -> SecurityVerdict {
        if keys.iter().any(|(key, _)| key == "deny") {
            SecurityVerdict::Deny {
                reason: "parsed-key denial".to_string(),
            }
        } else if keys.iter().any(|(key, _)| key == "warn") {
            SecurityVerdict::AllowWithWarning {
                warning: "parsed-key warning".to_string(),
            }
        } else {
            SecurityVerdict::Allow
        }
    }
}

struct DenySignature;

impl SecurityGate for DenySignature {
    fn name(&self) -> &str {
        "test.signature"
    }

    fn is_enforcing(&self) -> bool {
        true
    }
}

impl CodeSignatureValidator for DenySignature {
    fn validate_signature(&self, _path: &Path, _present: bool) -> SecurityVerdict {
        SecurityVerdict::Deny {
            reason: "signature denial".to_string(),
        }
    }
}

fn evaluate(legacy_denied: bool, signature_denied: bool, key_names: &[&str]) -> SecurityReport {
    let mut policy = TrustPolicy::permissive();
    policy.entitlements = Box::new(RevisableEntitlements { legacy_denied });
    if signature_denied {
        policy.code_signature = Box::new(DenySignature);
    }
    let keys: Vec<_> = key_names.iter().map(|key| (key.to_string(), true)).collect();
    let signature = CodeSignatureInfo {
        present: true,
        restrict: true,
        runtime: true,
        has_entitlements: !keys.is_empty(),
        ..CodeSignatureInfo::default()
    };
    policy.evaluate_binary_parsed_with_keys(
        Path::new("/test/report-probe"),
        MachFlags::empty(),
        &signature,
        &keys,
    )
}

fn assert_consistent(report: &SecurityReport) {
    let final_denials: Vec<_> = report
        .decisions
        .iter()
        .filter_map(|decision| match &decision.verdict {
            SecurityVerdict::Deny { reason } => Some(format!("{}: {reason}", decision.gate)),
            _ => None,
        })
        .collect();
    assert_eq!(report.denied_reasons, final_denials);
    assert_eq!(report.allowed, final_denials.is_empty());
}

#[test]
fn parsed_entitlement_denial_replaces_earlier_allow() {
    let report = evaluate(false, false, &["deny"]);
    assert!(!report.allowed);
    assert_eq!(report.denied_reasons, ["test.entitlements: parsed-key denial"]);
    assert_consistent(&report);
}

#[test]
fn parsed_allow_clears_obsolete_entitlement_denial() {
    let report = evaluate(true, false, &["allow"]);
    assert!(report.allowed);
    assert!(report.denied_reasons.is_empty());
    assert_consistent(&report);
}

#[test]
fn parsed_allow_does_not_clear_other_gate_denials() {
    let report = evaluate(true, true, &["allow"]);
    assert!(!report.allowed);
    assert_eq!(report.denied_reasons, ["test.signature: signature denial"]);
    assert_consistent(&report);
}

#[test]
fn multiple_final_denials_are_preserved_in_gate_order() {
    let report = evaluate(false, true, &["deny"]);
    assert!(!report.allowed);
    assert_eq!(
        report.denied_reasons,
        [
            "test.signature: signature denial",
            "test.entitlements: parsed-key denial",
        ]
    );
    assert_consistent(&report);
}

#[test]
fn empty_keys_retain_legacy_entitlement_verdict() {
    let report = evaluate(true, false, &[]);
    assert!(!report.allowed);
    assert_eq!(report.denied_reasons, ["test.entitlements: legacy denial"]);
    assert_consistent(&report);
}

#[test]
fn warnings_and_signature_flags_survive_aggregation() {
    let report = evaluate(true, false, &["warn"]);
    assert!(report.allowed);
    assert!(report.strip_dyld_env);
    assert!(report.hardened_runtime);
    assert!(report.cs_info.as_ref().unwrap().has_entitlements);
    assert!(report.decisions.iter().any(|decision| {
        decision.gate == "test.entitlements"
            && matches!(&decision.verdict, SecurityVerdict::AllowWithWarning { .. })
    }));
    assert_consistent(&report);
}
