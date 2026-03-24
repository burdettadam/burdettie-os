//! Permissive mock implementations of all security gates.
//!
//! Every gate unconditionally allows execution — used for testing
//! and development without real code signing or SIP checks.

use std::path::Path;

use macho_types::header::MachFlags;

use crate::gates::*;
use crate::SecurityVerdict;

// ── Gate 1: Mock Quarantine ─────────────────────────────────────────

pub struct MockQuarantine;

impl SecurityGate for MockQuarantine {
    fn name(&self) -> &str {
        "Quarantine (mock)"
    }
    fn is_enforcing(&self) -> bool {
        false
    }
}

impl QuarantinePolicy for MockQuarantine {
    fn check_quarantine(&self, _binary_path: &Path) -> SecurityVerdict {
        SecurityVerdict::Allow
    }
}

// ── Gate 2: Mock Code Signature ─────────────────────────────────────

pub struct MockCodeSignature;

impl SecurityGate for MockCodeSignature {
    fn name(&self) -> &str {
        "Code Signature (mock)"
    }
    fn is_enforcing(&self) -> bool {
        false
    }
}

impl CodeSignatureValidator for MockCodeSignature {
    fn validate_signature(
        &self,
        _binary_path: &Path,
        _has_code_signature: bool,
    ) -> SecurityVerdict {
        SecurityVerdict::Allow
    }
}

// ── Gate 3: Mock Entitlements ───────────────────────────────────────

pub struct MockEntitlements;

impl SecurityGate for MockEntitlements {
    fn name(&self) -> &str {
        "Entitlements (mock)"
    }
    fn is_enforcing(&self) -> bool {
        false
    }
}

impl EntitlementEvaluator for MockEntitlements {
    fn evaluate(&self, _binary_path: &Path) -> SecurityVerdict {
        SecurityVerdict::Allow
    }
}

// ── Gate 4: Mock AMFI ───────────────────────────────────────────────

pub struct MockAmfi;

impl SecurityGate for MockAmfi {
    fn name(&self) -> &str {
        "AMFI (mock)"
    }
    fn is_enforcing(&self) -> bool {
        false
    }
}

impl AmfiPolicy for MockAmfi {
    fn check_code_integrity(
        &self,
        _binary_path: &Path,
        _has_code_signature: bool,
    ) -> SecurityVerdict {
        SecurityVerdict::Allow
    }
}

// ── Gate 5: Mock SIP ────────────────────────────────────────────────

pub struct MockSip;

impl SecurityGate for MockSip {
    fn name(&self) -> &str {
        "SIP (mock)"
    }
    fn is_enforcing(&self) -> bool {
        false
    }
}

impl SipPolicy for MockSip {
    fn should_strip_dyld_env(&self, _binary_path: &Path, _flags: MachFlags) -> bool {
        false // never strip in mock mode
    }

    fn is_path_protected(&self, _path: &Path) -> bool {
        false // no paths protected in mock mode
    }
}

// ── Gate 6: Mock Hardened Runtime ───────────────────────────────────

pub struct MockHardenedRuntime;

impl SecurityGate for MockHardenedRuntime {
    fn name(&self) -> &str {
        "Hardened Runtime (mock)"
    }
    fn is_enforcing(&self) -> bool {
        false
    }
}

impl HardenedRuntimePolicy for MockHardenedRuntime {
    fn is_hardened(&self, _binary_path: &Path, _flags: MachFlags) -> bool {
        false // nothing is hardened in mock mode
    }

    fn allows_dylib_insertion(&self, _binary_path: &Path, _flags: MachFlags) -> bool {
        true // always allow in mock mode
    }
}

// ── Gate 7: Mock Library Validator ──────────────────────────────────

pub struct MockLibraryValidator;

impl SecurityGate for MockLibraryValidator {
    fn name(&self) -> &str {
        "Library Validator (mock)"
    }
    fn is_enforcing(&self) -> bool {
        false
    }
}

impl LibraryValidator for MockLibraryValidator {
    fn validate_dylib(&self, _loader_path: &Path, _dylib_path: &Path) -> SecurityVerdict {
        SecurityVerdict::Allow
    }
}

// ── Gate 8: Mock Sandbox ────────────────────────────────────────────

pub struct MockSandbox;

impl SecurityGate for MockSandbox {
    fn name(&self) -> &str {
        "Sandbox (mock)"
    }
    fn is_enforcing(&self) -> bool {
        false
    }
}

impl SandboxPolicy for MockSandbox {
    fn check_file_access(&self, _path: &Path) -> SecurityVerdict {
        SecurityVerdict::Allow
    }
}
