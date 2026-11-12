use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::anchor::TrustAnchorRegistry;
use crate::cert::{CmsVerificationResult, PageHashVerificationResult};
use crate::gates::{CodeSignatureValidator, SecurityGate};
use crate::{CodeSignatureInfo, SecurityVerdict};

// ── VerificationLevel ───────────────────────────────────────────────

/// How thorough the code signature verification should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VerificationLevel {
    /// Just check if LC_CODE_SIGNATURE is present (like AppleCodeSignature).
    PresenceOnly,
    /// Check presence + verify page hashes match CodeDirectory.
    PageHashes,
    /// Check presence + page hashes + CMS structure + certificate chain.
    Full,
}

// ── CodeSignatureVerification ───────────────────────────────────────

/// Detailed result of code signature verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSignatureVerification {
    /// The verification level that was applied.
    pub level: VerificationLevel,
    /// Whether LC_CODE_SIGNATURE was present.
    pub presence: bool,
    /// Whether the signature is ad-hoc.
    pub adhoc: bool,
    /// Page hash verification result (if checked).
    pub page_hashes: Option<PageHashVerificationResult>,
    /// CMS verification result (if checked).
    pub cms_result: Option<CmsVerificationResult>,
    /// Whether the CDHash was found in the trust cache.
    pub trust_cache_hit: bool,
    /// Overall verdict.
    pub verdict: SecurityVerdict,
}

// ── VerifyingCodeSignature ──────────────────────────────────────────

/// Code signature validator that performs real verification.
///
/// Unlike `AppleCodeSignature` which only checks for the presence of
/// LC_CODE_SIGNATURE, this validator can verify page hashes and
/// CMS signature chains.
pub struct VerifyingCodeSignature {
    /// How thorough to verify.
    level: VerificationLevel,
    /// Trust anchor registry for CDHash and certificate checks.
    registry: Option<Arc<TrustAnchorRegistry>>,
    /// Whether to deny unsigned binaries on non-SIP paths.
    /// If false, unsigned binaries get AllowWithWarning (like Apple default).
    deny_unsigned: bool,
}

impl VerifyingCodeSignature {
    /// Create with just a verification level (no trust anchors).
    pub fn new(level: VerificationLevel) -> Self {
        Self {
            level,
            registry: None,
            deny_unsigned: false,
        }
    }

    /// Create with a trust anchor registry.
    pub fn with_registry(level: VerificationLevel, registry: Arc<TrustAnchorRegistry>) -> Self {
        Self {
            level,
            registry: Some(registry),
            deny_unsigned: false,
        }
    }

    /// Set whether unsigned binaries should be denied outright.
    pub fn deny_unsigned(mut self, deny: bool) -> Self {
        self.deny_unsigned = deny;
        self
    }

    /// Perform enriched verification with full code signature data.
    ///
    /// This is the method that does real work — the trait method `validate_signature`
    /// can only do presence checking since it doesn't have the raw data.
    pub fn verify_with_data(
        &self,
        cs_info: &CodeSignatureInfo,
        cms_data: Option<&[u8]>,
        cd_data: Option<&[u8]>,
        page_hash_result: Option<&PageHashVerificationResult>,
    ) -> CodeSignatureVerification {
        let mut result = CodeSignatureVerification {
            level: self.level,
            presence: cs_info.present,
            adhoc: cs_info.adhoc,
            page_hashes: None,
            cms_result: None,
            trust_cache_hit: false,
            verdict: SecurityVerdict::Allow,
        };

        if !cs_info.present {
            result.verdict = if self.deny_unsigned {
                SecurityVerdict::Deny {
                    reason: "binary has no code signature".to_string(),
                }
            } else {
                SecurityVerdict::AllowWithWarning {
                    warning: "binary has no code signature".to_string(),
                }
            };
            return result;
        }

        // Check trust cache first — if CDHash is trusted, skip further checks
        if let Some(reg) = &self.registry {
            if reg.is_cd_hash_trusted(&cs_info.cd_hash) {
                result.trust_cache_hit = true;
                result.verdict = SecurityVerdict::Allow;
                return result;
            }
        }

        // Page hash verification
        if self.level >= VerificationLevel::PageHashes {
            if let Some(ph) = page_hash_result {
                result.page_hashes = Some(ph.clone());
                if !ph.all_valid {
                    result.verdict = SecurityVerdict::Deny {
                        reason: format!(
                            "page hash verification failed: {}/{} pages mismatched",
                            ph.mismatches, ph.total_slots
                        ),
                    };
                    return result;
                }
            }
        }

        // CMS chain verification
        if self.level >= VerificationLevel::Full {
            if let (Some(cms), Some(cd)) = (cms_data, cd_data) {
                let cms_result = crate::cert::verify_cms_signature(cms, cd);
                if !cms_result.structure_valid {
                    result.verdict = SecurityVerdict::AllowWithWarning {
                        warning: "CMS signature structure invalid".to_string(),
                    };
                } else if cms_result.chains_to_apple_root {
                    result.verdict = SecurityVerdict::Allow;
                } else {
                    result.verdict = SecurityVerdict::AllowWithWarning {
                        warning: format!(
                            "CMS signature does not chain to Apple Root CA: {}",
                            cms_result.summary
                        ),
                    };
                }
                result.cms_result = Some(cms_result);
            } else if cs_info.adhoc {
                // Ad-hoc signed — no CMS expected
                result.verdict = SecurityVerdict::AllowWithWarning {
                    warning: "ad-hoc signed binary (no CMS signature)".to_string(),
                };
            }
        }

        result
    }
}

impl SecurityGate for VerifyingCodeSignature {
    fn name(&self) -> &str {
        "Code Signature (verifying)"
    }

    fn is_enforcing(&self) -> bool {
        true
    }
}

impl CodeSignatureValidator for VerifyingCodeSignature {
    fn validate_signature(&self, binary_path: &Path, has_code_signature: bool) -> SecurityVerdict {
        // Without the raw data, we can only do presence checking
        // (same as AppleCodeSignature). The enriched path is via verify_with_data().
        if has_code_signature {
            SecurityVerdict::Allow
        } else if is_sip_protected(binary_path) {
            SecurityVerdict::Deny {
                reason: "SIP-protected binary lacks code signature".to_string(),
            }
        } else if self.deny_unsigned {
            SecurityVerdict::Deny {
                reason: "unsigned binary not allowed by policy".to_string(),
            }
        } else {
            SecurityVerdict::AllowWithWarning {
                warning: "binary has no LC_CODE_SIGNATURE".to_string(),
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn is_sip_protected(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if s.starts_with("/usr/local/") {
        return false;
    }
    s.starts_with("/usr/")
        || s.starts_with("/System/")
        || s.starts_with("/bin/")
        || s.starts_with("/sbin/")
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn signed_cs_info() -> CodeSignatureInfo {
        CodeSignatureInfo {
            present: true,
            adhoc: false,
            runtime: true,
            restrict: false,
            library_validation: false,
            get_task_allow: false,
            identity: "com.example.app".to_string(),
            team_id: "TEAM123".to_string(),
            cd_hash: "abc123def456".to_string(),
            has_cms_signature: true,
            has_entitlements: false,
        }
    }

    fn unsigned_cs_info() -> CodeSignatureInfo {
        CodeSignatureInfo {
            present: false,
            ..Default::default()
        }
    }

    fn adhoc_cs_info() -> CodeSignatureInfo {
        CodeSignatureInfo {
            present: true,
            adhoc: true,
            identity: "app".to_string(),
            cd_hash: "deadbeef".to_string(),
            ..Default::default()
        }
    }

    fn valid_page_hashes() -> PageHashVerificationResult {
        PageHashVerificationResult {
            total_slots: 10,
            verified: 10,
            mismatches: 0,
            all_valid: true,
        }
    }

    fn invalid_page_hashes() -> PageHashVerificationResult {
        PageHashVerificationResult {
            total_slots: 10,
            verified: 7,
            mismatches: 3,
            all_valid: false,
        }
    }

    #[test]
    fn presence_only_allows_signed() {
        let gate = VerifyingCodeSignature::new(VerificationLevel::PresenceOnly);
        let result = gate.verify_with_data(&signed_cs_info(), None, None, None);
        assert!(result.verdict.is_allow());
        assert!(!result.verdict.is_deny());
    }

    #[test]
    fn presence_only_warns_unsigned() {
        let gate = VerifyingCodeSignature::new(VerificationLevel::PresenceOnly);
        let result = gate.verify_with_data(&unsigned_cs_info(), None, None, None);
        assert!(matches!(
            result.verdict,
            SecurityVerdict::AllowWithWarning { .. }
        ));
    }

    #[test]
    fn presence_only_denies_unsigned_sip() {
        let gate = VerifyingCodeSignature::new(VerificationLevel::PresenceOnly);
        let verdict = gate.validate_signature(Path::new("/usr/bin/ls"), false);
        assert!(verdict.is_deny());
    }

    #[test]
    fn deny_unsigned_mode() {
        let gate = VerifyingCodeSignature::new(VerificationLevel::PresenceOnly).deny_unsigned(true);
        let result = gate.verify_with_data(&unsigned_cs_info(), None, None, None);
        assert!(result.verdict.is_deny());
    }

    #[test]
    fn trust_cache_hit_skips_checks() {
        let mut reg = TrustAnchorRegistry::new();
        reg.add_trusted_cd_hash("abc123def456");
        let gate = VerifyingCodeSignature::with_registry(VerificationLevel::Full, Arc::new(reg));
        let bad_pages = invalid_page_hashes();
        let result = gate.verify_with_data(&signed_cs_info(), None, None, Some(&bad_pages));
        assert!(result.trust_cache_hit);
        assert!(result.verdict.is_allow());
        assert!(result.page_hashes.is_none());
    }

    #[test]
    fn page_hash_failure_denies() {
        let gate = VerifyingCodeSignature::new(VerificationLevel::PageHashes);
        let bad = invalid_page_hashes();
        let result = gate.verify_with_data(&signed_cs_info(), None, None, Some(&bad));
        assert!(result.verdict.is_deny());
    }

    #[test]
    fn page_hash_success_allows() {
        let gate = VerifyingCodeSignature::new(VerificationLevel::PageHashes);
        let good = valid_page_hashes();
        let result = gate.verify_with_data(&signed_cs_info(), None, None, Some(&good));
        assert!(result.verdict.is_allow());
    }

    #[test]
    fn full_level_adhoc_warns() {
        let gate = VerifyingCodeSignature::new(VerificationLevel::Full);
        let result = gate.verify_with_data(&adhoc_cs_info(), None, None, None);
        assert!(matches!(
            result.verdict,
            SecurityVerdict::AllowWithWarning { .. }
        ));
    }

    #[test]
    fn full_level_no_data() {
        let gate = VerifyingCodeSignature::new(VerificationLevel::Full);
        let result = gate.verify_with_data(&signed_cs_info(), None, None, None);
        // No CMS/CD data provided, not adhoc → defaults to initial Allow
        assert!(result.verdict.is_allow());
    }

    #[test]
    fn verification_result_fields() {
        let gate = VerifyingCodeSignature::new(VerificationLevel::PageHashes);
        let good = valid_page_hashes();
        let cs = signed_cs_info();
        let result = gate.verify_with_data(&cs, None, None, Some(&good));
        assert_eq!(result.level, VerificationLevel::PageHashes);
        assert!(result.presence);
        assert!(!result.adhoc);
        assert!(result.page_hashes.is_some());
        let ph = result.page_hashes.unwrap();
        assert_eq!(ph.total_slots, 10);
        assert_eq!(ph.verified, 10);
        assert!(ph.all_valid);
        assert!(result.cms_result.is_none());
        assert!(!result.trust_cache_hit);
    }
}
