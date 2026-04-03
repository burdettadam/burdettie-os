//! Security gate traits and mock policies for the Mach-O loader pipeline.
//!
//! Models Apple's 8-stage security enforcement chain from Gatekeeper quarantine
//! through sandbox evaluation. Each gate is a trait that returns a [`SecurityVerdict`].
//! The [`TrustPolicy`] struct composes all gates into a single injectable policy.
//!
//! All initial implementations are permissive mocks. Restrictive (Apple-default)
//! implementations can be added incrementally without changing the trait API.
//!
//! Phase 8 adds [`CodeSignatureInfo`] for enriched security evaluation using
//! parsed code signature data (SuperBlob, CodeDirectory, CS flags).

pub mod anchor;
mod apple;
mod builder;
pub mod cert;
pub mod config;
pub mod effects;
pub mod entitlements;
pub mod enforcement;
mod gates;
pub mod hardened;
mod mock;
pub mod quarantine;
pub mod real_amfi;
pub mod sandbox;
mod verdict;
pub mod verify_gate;

pub use enforcement::EnforcementMode;
pub use real_amfi::{AmfiEvaluation, CdHashResult, RealAmfi, RealQuarantine, RealSip};

pub use apple::{
    AppleAmfi, AppleCodeSignature, AppleEntitlements, AppleHardenedRuntime, AppleLibraryValidator,
    AppleQuarantine, AppleSandbox, AppleSip,
};
pub use builder::{PolicyPreset, TrustPolicyBuilder};
pub use gates::{
    AmfiPolicy, CodeSignatureValidator, EntitlementEvaluator, HardenedRuntimePolicy,
    LibraryValidator, QuarantinePolicy, SandboxPolicy, SecurityGate, SipPolicy,
};
pub use mock::{
    MockAmfi, MockCodeSignature, MockEntitlements, MockHardenedRuntime, MockLibraryValidator,
    MockQuarantine, MockSandbox, MockSip,
};
pub use verdict::{GateDecision, SecurityVerdict};
pub use effects::{EntitlementEffects, EnrichedSecurityReport};

use macho_types::codesig::CsFlags;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Security-relevant information extracted from a parsed code signature.
///
/// This is a summary of the fields from `CodeSignature` / `CodeDirectory`
/// that are relevant to security gate decisions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeSignatureInfo {
    /// Whether LC_CODE_SIGNATURE was present and successfully parsed.
    pub present: bool,
    /// Whether the signature is ad-hoc (no identity, linker-signed).
    pub adhoc: bool,
    /// Whether CS_RUNTIME (hardened runtime) is set.
    pub runtime: bool,
    /// Whether CS_RESTRICT is set (env var stripping).
    pub restrict: bool,
    /// Whether CS_REQUIRE_LV (library validation) is set.
    pub library_validation: bool,
    /// Whether CS_GET_TASK_ALLOW is set (debugging permitted).
    pub get_task_allow: bool,
    /// Signing identity (e.g., bundle ID or "app").
    pub identity: String,
    /// Team identifier (empty for ad-hoc).
    pub team_id: String,
    /// CDHash hex string.
    pub cd_hash: String,
    /// Whether a CMS (PKCS#7) signature is present (not just ad-hoc).
    pub has_cms_signature: bool,
    /// Whether entitlements plist was found.
    pub has_entitlements: bool,
}

impl CodeSignatureInfo {
    /// Build from a parsed `CodeSignature`.
    pub fn from_parsed(cs: &macho_types::CodeSignature) -> Self {
        let (flags, identity, team_id, cd_hash) = cs
            .code_directory
            .as_ref()
            .map(|cd| {
                let hash_hex = cd
                    .cd_hash
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                (cd.flags, cd.identity.clone(), cd.team_id.clone(), hash_hex)
            })
            .unwrap_or_else(|| {
                (
                    CsFlags::empty(),
                    String::new(),
                    String::new(),
                    String::new(),
                )
            });

        Self {
            present: true,
            adhoc: flags.contains(CsFlags::ADHOC),
            runtime: flags.contains(CsFlags::RUNTIME),
            restrict: flags.contains(CsFlags::RESTRICT),
            library_validation: flags.contains(CsFlags::REQUIRE_LV),
            get_task_allow: flags.contains(CsFlags::GET_TASK_ALLOW),
            identity,
            team_id,
            cd_hash,
            has_cms_signature: cs.has_cms_signature,
            has_entitlements: cs.entitlements_plist.is_some(),
        }
    }
}

/// All security gates composed into a single policy object.
///
/// Injected into the loader pipeline at construction time.
/// Each field holds a boxed trait object for one security gate.
pub struct TrustPolicy {
    pub quarantine: Box<dyn QuarantinePolicy>,
    pub code_signature: Box<dyn CodeSignatureValidator>,
    pub entitlements: Box<dyn EntitlementEvaluator>,
    pub amfi: Box<dyn AmfiPolicy>,
    pub sip: Box<dyn SipPolicy>,
    pub hardened_runtime: Box<dyn HardenedRuntimePolicy>,
    pub library_validator: Box<dyn LibraryValidator>,
    pub sandbox: Box<dyn SandboxPolicy>,
}

impl TrustPolicy {
    /// Returns a fully permissive policy (all mocks, nothing enforced).
    pub fn permissive() -> Self {
        Self {
            quarantine: Box::new(MockQuarantine),
            code_signature: Box::new(MockCodeSignature),
            entitlements: Box::new(MockEntitlements),
            amfi: Box::new(MockAmfi),
            sip: Box::new(MockSip),
            hardened_runtime: Box::new(MockHardenedRuntime),
            library_validator: Box::new(MockLibraryValidator),
            sandbox: Box::new(MockSandbox),
        }
    }

    /// Returns a policy matching Apple's default macOS behavior.
    ///
    /// SIP is enforced, code signatures are checked (presence only),
    /// hardened runtime is modeled for system binaries, and library
    /// validation restricts cross-boundary dylib loading.
    pub fn apple_default() -> Self {
        Self {
            quarantine: Box::new(AppleQuarantine),
            code_signature: Box::new(AppleCodeSignature),
            entitlements: Box::new(AppleEntitlements),
            amfi: Box::new(AppleAmfi),
            sip: Box::new(AppleSip),
            hardened_runtime: Box::new(AppleHardenedRuntime),
            library_validator: Box::new(AppleLibraryValidator),
            sandbox: Box::new(AppleSandbox),
        }
    }

    /// Production policy — real CDHash + CMS verification, macOS-compatible defaults.
    ///
    /// - AMFI: `RealAmfi::production()` (ad-hoc allowed, unsigned allowed with warning)
    /// - Quarantine: real `com.apple.quarantine` xattr check
    /// - SIP: real path-based SIP enforcement
    /// - Other gates: Apple-default path-heuristic implementations
    pub fn production() -> Self {
        Self {
            quarantine: Box::new(real_amfi::RealQuarantine::production()),
            code_signature: Box::new(AppleCodeSignature),
            entitlements: Box::new(AppleEntitlements),
            amfi: Box::new(real_amfi::RealAmfi::production()),
            sip: Box::new(real_amfi::RealSip::enforcing()),
            hardened_runtime: Box::new(AppleHardenedRuntime),
            library_validator: Box::new(AppleLibraryValidator),
            sandbox: Box::new(AppleSandbox),
        }
    }

    /// Audit policy — real verification but never blocks execution.
    ///
    /// All checks run; failures produce warnings instead of denials.
    /// Use this to observe what `production()` would block without impact.
    pub fn audit() -> Self {
        Self {
            quarantine: Box::new(real_amfi::RealQuarantine::log_only()),
            code_signature: Box::new(AppleCodeSignature),
            entitlements: Box::new(AppleEntitlements),
            amfi: Box::new(real_amfi::RealAmfi::log_only()),
            sip: Box::new(real_amfi::RealSip::log_only()),
            hardened_runtime: Box::new(AppleHardenedRuntime),
            library_validator: Box::new(AppleLibraryValidator),
            sandbox: Box::new(AppleSandbox),
        }
    }

    /// Run all pre-load gates for a binary and collect a security report.
    ///
    /// This checks quarantine, code signature, entitlements, AMFI, SIP,
    /// and hardened runtime status for the main executable.
    #[allow(clippy::vec_init_then_push)] // builder pattern — interleaved computation between pushes
    pub fn evaluate_binary(
        &self,
        binary_path: &Path,
        flags: macho_types::header::MachFlags,
        has_code_signature: bool,
    ) -> SecurityReport {
        let mut decisions = Vec::new();

        decisions.push(GateDecision {
            gate: self.quarantine.name().to_string(),
            verdict: self.quarantine.check_quarantine(binary_path),
        });

        decisions.push(GateDecision {
            gate: self.code_signature.name().to_string(),
            verdict: self
                .code_signature
                .validate_signature(binary_path, has_code_signature),
        });

        decisions.push(GateDecision {
            gate: self.entitlements.name().to_string(),
            verdict: self.entitlements.evaluate(binary_path),
        });

        decisions.push(GateDecision {
            gate: self.amfi.name().to_string(),
            verdict: self
                .amfi
                .check_code_integrity(binary_path, has_code_signature),
        });

        let sip_verdict = if self.sip.is_path_protected(binary_path) {
            SecurityVerdict::AllowWithWarning {
                warning: "binary is in SIP-protected path".to_string(),
            }
        } else {
            SecurityVerdict::Allow
        };
        let strip_dyld_env = self.sip.should_strip_dyld_env(binary_path, flags);

        decisions.push(GateDecision {
            gate: self.sip.name().to_string(),
            verdict: sip_verdict,
        });

        let hardened = self.hardened_runtime.is_hardened(binary_path, flags);
        decisions.push(GateDecision {
            gate: self.hardened_runtime.name().to_string(),
            verdict: if hardened {
                SecurityVerdict::AllowWithWarning {
                    warning: "hardened runtime active — DYLD_INSERT_LIBRARIES restricted"
                        .to_string(),
                }
            } else {
                SecurityVerdict::Allow
            },
        });

        let denied: Vec<String> = decisions
            .iter()
            .filter_map(|d| {
                if let SecurityVerdict::Deny { reason } = &d.verdict {
                    Some(format!("{}: {}", d.gate, reason))
                } else {
                    None
                }
            })
            .collect();

        SecurityReport {
            decisions,
            allowed: denied.is_empty(),
            denied_reasons: denied,
            strip_dyld_env,
            hardened_runtime: hardened,
            cs_info: None,
        }
    }

    /// Check whether a dylib is allowed to be loaded.
    ///
    /// Runs library validation and sandbox file-access checks.
    pub fn validate_dylib(&self, loader_path: &Path, dylib_path: &Path) -> SecurityVerdict {
        let lib_verdict = self
            .library_validator
            .validate_dylib(loader_path, dylib_path);
        if let SecurityVerdict::Deny { .. } = &lib_verdict {
            return lib_verdict;
        }

        let sandbox_verdict = self.sandbox.check_file_access(dylib_path);
        if let SecurityVerdict::Deny { .. } = &sandbox_verdict {
            return sandbox_verdict;
        }

        SecurityVerdict::Allow
    }

    /// Run enriched security evaluation using parsed code signature data.
    ///
    /// This extends `evaluate_binary` by incorporating the real CS flags
    /// from the parsed code signature (CS_RUNTIME, CS_RESTRICT, CS_REQUIRE_LV)
    /// rather than inferring behavior from path-based heuristics alone.
    pub fn evaluate_binary_parsed(
        &self,
        binary_path: &Path,
        flags: macho_types::header::MachFlags,
        cs_info: &CodeSignatureInfo,
    ) -> SecurityReport {
        self.evaluate_binary_parsed_with_keys(binary_path, flags, cs_info, &[])
    }

    /// Run enriched security evaluation with parsed entitlement keys.
    pub fn evaluate_binary_parsed_with_keys(
        &self,
        binary_path: &Path,
        flags: macho_types::header::MachFlags,
        cs_info: &CodeSignatureInfo,
        entitlement_keys: &[(String, bool)],
    ) -> SecurityReport {
        // Start with the standard gate evaluation
        let mut report = self.evaluate_binary(binary_path, flags, cs_info.present);

        // Re-evaluate entitlements gate with parsed keys if available
        if !entitlement_keys.is_empty() {
            let ent_idx = report
                .decisions
                .iter()
                .position(|d| d.gate == self.entitlements.name());
            if let Some(idx) = ent_idx {
                report.decisions[idx].verdict = self
                    .entitlements
                    .evaluate_with_keys(binary_path, entitlement_keys);
            }
        }

        // Override strip_dyld_env if CS_RESTRICT is set in the actual code signature
        if cs_info.restrict {
            report.strip_dyld_env = true;
        }

        // Override hardened_runtime if CS_RUNTIME is set in the actual code signature
        if cs_info.runtime {
            report.hardened_runtime = true;
        }

        // Add the parsed code signature info to the report
        report.cs_info = Some(cs_info.clone());

        report
    }
}

/// Complete security evaluation report for a binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    /// Individual gate decisions.
    pub decisions: Vec<GateDecision>,
    /// Whether the binary is allowed to load (all gates passed).
    pub allowed: bool,
    /// Reasons for denial (empty if allowed).
    pub denied_reasons: Vec<String>,
    /// Whether DYLD_* environment variables should be stripped.
    pub strip_dyld_env: bool,
    /// Whether hardened runtime restrictions apply.
    pub hardened_runtime: bool,
    /// Parsed code signature info (populated by `evaluate_binary_parsed`).
    pub cs_info: Option<CodeSignatureInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn permissive_construction() {
        let policy = TrustPolicy::permissive();
        assert!(!policy.quarantine.is_enforcing());
        assert!(!policy.code_signature.is_enforcing());
        assert!(!policy.sip.is_enforcing());
        assert!(!policy.sandbox.is_enforcing());
    }

    #[test]
    fn apple_default_construction() {
        let policy = TrustPolicy::apple_default();
        assert!(policy.quarantine.is_enforcing());
        assert!(policy.code_signature.is_enforcing());
        assert!(policy.sip.is_enforcing());
        assert!(policy.sandbox.is_enforcing());
    }

    #[test]
    fn code_sig_info_default() {
        let info = CodeSignatureInfo::default();
        assert!(!info.present);
        assert!(!info.adhoc);
        assert!(!info.runtime);
        assert!(info.identity.is_empty());
        assert!(info.cd_hash.is_empty());
    }

    #[test]
    fn code_sig_info_fields() {
        let info = CodeSignatureInfo {
            present: true,
            adhoc: true,
            runtime: false,
            restrict: false,
            library_validation: false,
            get_task_allow: true,
            identity: "com.example.app".into(),
            team_id: "ABCDE12345".into(),
            cd_hash: "deadbeef".into(),
            has_cms_signature: false,
            has_entitlements: true,
        };
        assert!(info.present);
        assert!(info.adhoc);
        assert!(info.get_task_allow);
        assert_eq!(info.identity, "com.example.app");
        assert_eq!(info.team_id, "ABCDE12345");
        assert!(info.has_entitlements);
        assert!(!info.has_cms_signature);
    }

    #[test]
    fn permissive_allows_unsigned() {
        let policy = TrustPolicy::permissive();
        let report = policy.evaluate_binary(
            Path::new("/tmp/unsigned"),
            macho_types::header::MachFlags::empty(),
            false,
        );
        assert!(report.allowed);
        assert!(report.denied_reasons.is_empty());
    }

    #[test]
    fn apple_default_allows_signed_nonsip() {
        let policy = TrustPolicy::apple_default();
        let report = policy.evaluate_binary(
            Path::new("/tmp/my_app"),
            macho_types::header::MachFlags::empty(),
            true,
        );
        assert!(report.allowed);
    }

    #[test]
    fn apple_default_denies_unsigned_sip() {
        let policy = TrustPolicy::apple_default();
        let report = policy.evaluate_binary(
            Path::new("/usr/bin/ls"),
            macho_types::header::MachFlags::empty(),
            false,
        );
        assert!(!report.allowed);
        assert!(!report.denied_reasons.is_empty());
    }

    #[test]
    fn apple_sip_strips_dyld_env_for_protected_path() {
        let policy = TrustPolicy::apple_default();
        let report = policy.evaluate_binary(
            Path::new("/usr/bin/ls"),
            macho_types::header::MachFlags::empty(),
            true,
        );
        assert!(report.strip_dyld_env);
    }

    #[test]
    fn permissive_does_not_strip_dyld_env() {
        let policy = TrustPolicy::permissive();
        let report = policy.evaluate_binary(
            Path::new("/usr/bin/ls"),
            macho_types::header::MachFlags::empty(),
            true,
        );
        assert!(!report.strip_dyld_env);
    }

    #[test]
    fn apple_default_validate_dylib_sip_loader_rejects_nonsip() {
        let policy = TrustPolicy::apple_default();
        let verdict = policy.validate_dylib(
            Path::new("/usr/bin/ls"),
            Path::new("/tmp/evil.dylib"),
        );
        assert!(verdict.is_deny());
    }

    #[test]
    fn permissive_validate_dylib_allows_any() {
        let policy = TrustPolicy::permissive();
        let verdict = policy.validate_dylib(
            Path::new("/usr/bin/ls"),
            Path::new("/tmp/anything.dylib"),
        );
        assert!(verdict.is_allow());
    }

    #[test]
    fn apple_entitlements_allow_dyld_env_key() {
        let ent = AppleEntitlements;
        let keys = vec![(
            "com.apple.security.cs.allow-dyld-environment-variables".to_string(),
            true,
        )];
        let verdict = ent.evaluate_with_keys(Path::new("/tmp/app"), &keys);
        assert!(verdict.is_allow());
        assert!(matches!(verdict, SecurityVerdict::AllowWithWarning { .. }));
    }

    #[test]
    fn hardened_runtime_sip_path_blocks_insertion() {
        let hr = AppleHardenedRuntime;
        assert!(hr.is_hardened(
            Path::new("/usr/bin/ls"),
            macho_types::header::MachFlags::empty(),
        ));
        assert!(!hr.allows_dylib_insertion(
            Path::new("/usr/bin/ls"),
            macho_types::header::MachFlags::empty(),
        ));
    }
}
