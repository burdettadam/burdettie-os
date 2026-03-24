//! Apple-default (restrictive) security policy implementations.
//!
//! These model the behavior of a stock macOS system with SIP enabled:
//! - SIP protects /usr, /System, /bin, /sbin paths
//! - SIP-protected binaries have DYLD_* env vars stripped
//! - Code signatures are required for execution
//! - AMFI checks code integrity (requires signature for protected binaries)
//! - Hardened runtime restricts DYLD_INSERT_LIBRARIES
//! - Library validator requires same-team or Apple signatures
//!
//! These are modeled implementations — they check path patterns and flags
//! rather than reading actual code signatures or xattrs from disk.

use std::path::Path;

use macho_types::header::MachFlags;

use crate::gates::*;
use crate::SecurityVerdict;

/// SIP-protected path prefixes on macOS.
const SIP_PROTECTED_PREFIXES: &[&str] = &["/usr/", "/System/", "/bin/", "/sbin/"];

/// Paths exempted from SIP protection (user-writable under /usr).
const SIP_EXEMPT_PREFIXES: &[&str] = &["/usr/local/"];

/// Check if a path falls under SIP protection.
fn is_sip_protected(path: &Path) -> bool {
    let s = path.to_string_lossy();
    for exempt in SIP_EXEMPT_PREFIXES {
        if s.starts_with(exempt) {
            return false;
        }
    }
    for prefix in SIP_PROTECTED_PREFIXES {
        if s.starts_with(prefix) {
            return true;
        }
    }
    false
}

// ── Gate 1: Apple Quarantine ────────────────────────────────────────

/// Models Gatekeeper behavior: quarantine xattr check.
///
/// In a real implementation this would read com.apple.quarantine xattr.
/// Our model always allows (quarantine is a first-launch UX gate, not
/// a programmatic enforcement that blocks our pipeline).
pub struct AppleQuarantine;

impl SecurityGate for AppleQuarantine {
    fn name(&self) -> &str {
        "Quarantine"
    }
    fn is_enforcing(&self) -> bool {
        true
    }
}

impl QuarantinePolicy for AppleQuarantine {
    fn check_quarantine(&self, _binary_path: &Path) -> SecurityVerdict {
        // In practice, Gatekeeper shows a dialog — we model as allow-with-warning
        // since we can't read the xattr meaningfully in our pipeline.
        SecurityVerdict::AllowWithWarning {
            warning: "quarantine check modeled (xattr not read)".to_string(),
        }
    }
}

// ── Gate 2: Apple Code Signature ────────────────────────────────────

/// Models Apple code signature enforcement.
///
/// Requires LC_CODE_SIGNATURE to be present. Does not validate the
/// actual CMS signature or page hashes (that would require libSecurity).
pub struct AppleCodeSignature;

impl SecurityGate for AppleCodeSignature {
    fn name(&self) -> &str {
        "Code Signature"
    }
    fn is_enforcing(&self) -> bool {
        true
    }
}

impl CodeSignatureValidator for AppleCodeSignature {
    fn validate_signature(&self, binary_path: &Path, has_code_signature: bool) -> SecurityVerdict {
        if has_code_signature {
            SecurityVerdict::Allow
        } else if is_sip_protected(binary_path) {
            SecurityVerdict::Deny {
                reason: "SIP-protected binary lacks code signature".to_string(),
            }
        } else {
            // Non-SIP binaries can run without signatures on macOS,
            // but with a warning (ad-hoc signing is common for dev builds).
            SecurityVerdict::AllowWithWarning {
                warning: "binary has no LC_CODE_SIGNATURE".to_string(),
            }
        }
    }
}

// ── Gate 3: Apple Entitlements ──────────────────────────────────────

/// Models entitlement evaluation.
///
/// In our model, entitlements are always granted (we don't parse the
/// embedded plist). Real enforcement is per-entitlement.
pub struct AppleEntitlements;

impl SecurityGate for AppleEntitlements {
    fn name(&self) -> &str {
        "Entitlements"
    }
    fn is_enforcing(&self) -> bool {
        true
    }
}

impl EntitlementEvaluator for AppleEntitlements {
    fn evaluate(&self, _binary_path: &Path) -> SecurityVerdict {
        SecurityVerdict::Allow
    }

    fn evaluate_with_keys(&self, _binary_path: &Path, keys: &[(String, bool)]) -> SecurityVerdict {
        // Check for dangerous entitlements that weaken security
        for (key, value) in keys {
            if *value && key == "com.apple.security.cs.disable-library-validation" {
                return SecurityVerdict::AllowWithWarning {
                    warning: "entitlement disables library validation".to_string(),
                };
            }
            if *value && key == "com.apple.security.cs.allow-dyld-environment-variables" {
                return SecurityVerdict::AllowWithWarning {
                    warning: "entitlement allows DYLD_* environment variables".to_string(),
                };
            }
        }
        SecurityVerdict::Allow
    }
}

// ── Gate 4: Apple AMFI ──────────────────────────────────────────────

/// Models AMFI code integrity enforcement.
///
/// AMFI requires a valid code signature for SIP-protected binaries.
/// For non-protected binaries, it allows execution with a warning.
pub struct AppleAmfi;

impl SecurityGate for AppleAmfi {
    fn name(&self) -> &str {
        "AMFI"
    }
    fn is_enforcing(&self) -> bool {
        true
    }
}

impl AmfiPolicy for AppleAmfi {
    fn check_code_integrity(
        &self,
        binary_path: &Path,
        has_code_signature: bool,
    ) -> SecurityVerdict {
        if has_code_signature {
            SecurityVerdict::Allow
        } else if is_sip_protected(binary_path) {
            SecurityVerdict::Deny {
                reason: "AMFI: SIP-protected binary requires valid code signature".to_string(),
            }
        } else {
            SecurityVerdict::AllowWithWarning {
                warning: "AMFI: binary not code-signed (allowed on non-SIP path)".to_string(),
            }
        }
    }
}

// ── Gate 5: Apple SIP ───────────────────────────────────────────────

/// Models System Integrity Protection.
///
/// SIP protects /usr, /System, /bin, /sbin (except /usr/local).
/// Binaries in SIP-protected paths have DYLD_* env vars stripped.
pub struct AppleSip;

impl SecurityGate for AppleSip {
    fn name(&self) -> &str {
        "SIP"
    }
    fn is_enforcing(&self) -> bool {
        true
    }
}

impl SipPolicy for AppleSip {
    fn should_strip_dyld_env(&self, binary_path: &Path, _flags: MachFlags) -> bool {
        is_sip_protected(binary_path)
    }

    fn is_path_protected(&self, path: &Path) -> bool {
        is_sip_protected(path)
    }
}

// ── Gate 6: Apple Hardened Runtime ──────────────────────────────────

/// Models hardened runtime enforcement.
///
/// In a real implementation, hardened runtime is indicated by the
/// CS_RUNTIME flag in the code signature. We model it as active for
/// SIP-protected binaries (Apple ships everything hardened).
pub struct AppleHardenedRuntime;

impl SecurityGate for AppleHardenedRuntime {
    fn name(&self) -> &str {
        "Hardened Runtime"
    }
    fn is_enforcing(&self) -> bool {
        true
    }
}

impl HardenedRuntimePolicy for AppleHardenedRuntime {
    fn is_hardened(&self, binary_path: &Path, _flags: MachFlags) -> bool {
        // All SIP-protected (Apple-shipped) binaries use hardened runtime
        is_sip_protected(binary_path)
    }

    fn allows_dylib_insertion(&self, binary_path: &Path, _flags: MachFlags) -> bool {
        // Hardened binaries do NOT allow DYLD_INSERT_LIBRARIES
        !is_sip_protected(binary_path)
    }
}

// ── Gate 7: Apple Library Validator ─────────────────────────────────

/// Models Apple's library validation.
///
/// For SIP-protected loaders, only Apple-signed dylibs (in SIP paths)
/// are allowed. Non-SIP loaders can load any dylib.
pub struct AppleLibraryValidator;

impl SecurityGate for AppleLibraryValidator {
    fn name(&self) -> &str {
        "Library Validator"
    }
    fn is_enforcing(&self) -> bool {
        true
    }
}

impl LibraryValidator for AppleLibraryValidator {
    fn validate_dylib(&self, loader_path: &Path, dylib_path: &Path) -> SecurityVerdict {
        if is_sip_protected(loader_path) && !is_sip_protected(dylib_path) {
            SecurityVerdict::Deny {
                reason: format!(
                    "SIP-protected loader cannot load dylib from non-protected path: {}",
                    dylib_path.display()
                ),
            }
        } else {
            SecurityVerdict::Allow
        }
    }
}

// ── Gate 8: Apple Sandbox ───────────────────────────────────────────

/// Models sandbox file access checks.
///
/// Our model allows all file access (sandboxing is profile-based and
/// we don't parse sandbox profiles). A real implementation would check
/// the running profile's file access rules.
pub struct AppleSandbox;

impl SecurityGate for AppleSandbox {
    fn name(&self) -> &str {
        "Sandbox"
    }
    fn is_enforcing(&self) -> bool {
        true
    }
}

impl SandboxPolicy for AppleSandbox {
    fn check_file_access(&self, _path: &Path) -> SecurityVerdict {
        SecurityVerdict::Allow
    }
}
