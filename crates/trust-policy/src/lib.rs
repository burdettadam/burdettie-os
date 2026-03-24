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

mod apple;
mod gates;
mod mock;
mod verdict;

pub use apple::{
    AppleAmfi, AppleCodeSignature, AppleEntitlements, AppleHardenedRuntime, AppleLibraryValidator,
    AppleQuarantine, AppleSandbox, AppleSip,
};
pub use gates::{
    AmfiPolicy, CodeSignatureValidator, EntitlementEvaluator, HardenedRuntimePolicy,
    LibraryValidator, QuarantinePolicy, SandboxPolicy, SecurityGate, SipPolicy,
};
pub use mock::{
    MockAmfi, MockCodeSignature, MockEntitlements, MockHardenedRuntime, MockLibraryValidator,
    MockQuarantine, MockSandbox, MockSip,
};
pub use verdict::{GateDecision, SecurityVerdict};

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
