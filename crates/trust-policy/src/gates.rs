//! Security gate trait definitions.
//!
//! Each trait represents one gate in Apple's security enforcement chain.
//! Gates 1-8 are checked in sequence during binary loading.

use std::path::Path;

use macho_types::header::MachFlags;

use crate::SecurityVerdict;

/// Base trait shared by all security gates.
pub trait SecurityGate: Send + Sync {
    /// Human-readable name for logging.
    fn name(&self) -> &str;

    /// Whether this gate is currently enforcing (true) or permissive (false).
    fn is_enforcing(&self) -> bool;
}

// ── Gate 1: Quarantine (Gatekeeper) ─────────────────────────────────

/// Checks com.apple.quarantine xattr and notarization ticket.
pub trait QuarantinePolicy: SecurityGate {
    /// Check whether the binary passes quarantine (first-launch) checks.
    fn check_quarantine(&self, binary_path: &Path) -> SecurityVerdict;
}

// ── Gate 2: Code Signature ──────────────────────────────────────────

/// Verifies CMS signature, CDHash, and page hashes.
pub trait CodeSignatureValidator: SecurityGate {
    /// Validate the code signature of a binary.
    ///
    /// `has_code_signature` indicates whether LC_CODE_SIGNATURE was present.
    fn validate_signature(&self, binary_path: &Path, has_code_signature: bool) -> SecurityVerdict;
}

// ── Gate 3: Entitlements ────────────────────────────────────────────

/// Parses and evaluates the embedded entitlements plist.
pub trait EntitlementEvaluator: SecurityGate {
    /// Evaluate entitlements for a binary (path-only, legacy).
    fn evaluate(&self, binary_path: &Path) -> SecurityVerdict;

    /// Evaluate entitlements with pre-parsed keys available.
    ///
    /// The `keys` slice contains `(entitlement_name, bool_value)` pairs
    /// extracted from the entitlements plist. Implementations can make
    /// key-specific decisions (e.g. deny `com.apple.security.cs.disable-library-validation`).
    ///
    /// Default delegates to `evaluate()` for backward compatibility.
    fn evaluate_with_keys(&self, binary_path: &Path, keys: &[(String, bool)]) -> SecurityVerdict {
        let _ = keys;
        self.evaluate(binary_path)
    }
}

// ── Gate 4: AMFI (Apple Mobile File Integrity) ──────────────────────

/// Kernel code-integrity enforcement: CDHash trust cache + page verification.
pub trait AmfiPolicy: SecurityGate {
    /// Check code integrity for execution permission.
    fn check_code_integrity(&self, binary_path: &Path, has_code_signature: bool)
        -> SecurityVerdict;
}

// ── Gate 5: SIP (System Integrity Protection) ──────────────────────

/// Controls protected paths and DYLD_* environment variable stripping.
pub trait SipPolicy: SecurityGate {
    /// Whether DYLD_* environment variables should be stripped for this binary.
    fn should_strip_dyld_env(&self, binary_path: &Path, flags: MachFlags) -> bool;

    /// Whether the given path is SIP-protected.
    fn is_path_protected(&self, path: &Path) -> bool;
}

// ── Gate 6: Hardened Runtime ────────────────────────────────────────

/// Controls library validation, JIT, debugging restrictions.
pub trait HardenedRuntimePolicy: SecurityGate {
    /// Whether the binary has hardened runtime active.
    fn is_hardened(&self, binary_path: &Path, flags: MachFlags) -> bool;

    /// Whether DYLD_INSERT_LIBRARIES is allowed for this binary.
    fn allows_dylib_insertion(&self, binary_path: &Path, flags: MachFlags) -> bool;
}

// ── Gate 7: Library Validation ──────────────────────────────────────

/// Per-dylib load-time validation: must be signed by same team or Apple.
pub trait LibraryValidator: SecurityGate {
    /// Validate whether a dylib is allowed to load into this process.
    fn validate_dylib(&self, loader_path: &Path, dylib_path: &Path) -> SecurityVerdict;
}

// ── Gate 8: Sandbox ─────────────────────────────────────────────────

/// File access and IPC restrictions from sandbox profile.
pub trait SandboxPolicy: SecurityGate {
    /// Check whether the sandboxed process can access a file path.
    fn check_file_access(&self, path: &Path) -> SecurityVerdict;
}
