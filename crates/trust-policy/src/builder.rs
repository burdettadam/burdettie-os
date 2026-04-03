use serde::{Deserialize, Serialize};

use crate::apple::{
    AppleAmfi, AppleCodeSignature, AppleEntitlements, AppleHardenedRuntime, AppleLibraryValidator,
    AppleQuarantine, AppleSandbox, AppleSip,
};
use crate::gates::{
    AmfiPolicy, CodeSignatureValidator, EntitlementEvaluator, HardenedRuntimePolicy,
    LibraryValidator, QuarantinePolicy, SandboxPolicy, SipPolicy,
};
use crate::mock::{
    MockAmfi, MockCodeSignature, MockEntitlements, MockHardenedRuntime, MockLibraryValidator,
    MockQuarantine, MockSandbox, MockSip,
};
use crate::real_amfi::{RealAmfi, RealQuarantine, RealSip};
use crate::TrustPolicy;

/// Preset configurations for common policy combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyPreset {
    /// All gates permissive (mock implementations).
    Permissive,
    /// Apple-default enforcement (SIP, code signing, hardened runtime, etc.).
    /// Uses path-heuristic implementations.
    #[serde(alias = "apple")]
    AppleDefault,
    /// Production policy: real CDHash + CMS verification, macOS-compatible.
    /// AMFI enforcing, real quarantine xattr, real SIP paths.
    Production,
    /// Audit mode: real verification, never blocks — observe without impact.
    /// Same as Production but all gates in LogOnly mode.
    Audit,
    /// Strict mode: Apple-signed only, no ad-hoc, no unsigned binaries.
    /// AMFI enforcing with `allow_adhoc=false` and `allow_unsigned=false`.
    Strict,
}

/// Builder for constructing [`TrustPolicy`] instances with fine-grained control.
pub struct TrustPolicyBuilder {
    quarantine: Option<Box<dyn QuarantinePolicy>>,
    code_signature: Option<Box<dyn CodeSignatureValidator>>,
    entitlements: Option<Box<dyn EntitlementEvaluator>>,
    amfi: Option<Box<dyn AmfiPolicy>>,
    sip: Option<Box<dyn SipPolicy>>,
    hardened_runtime: Option<Box<dyn HardenedRuntimePolicy>>,
    library_validator: Option<Box<dyn LibraryValidator>>,
    sandbox: Option<Box<dyn SandboxPolicy>>,
}

impl TrustPolicyBuilder {
    /// Creates an empty builder with all gates unset.
    pub fn new() -> Self {
        Self {
            quarantine: None,
            code_signature: None,
            entitlements: None,
            amfi: None,
            sip: None,
            hardened_runtime: None,
            library_validator: None,
            sandbox: None,
        }
    }

    /// Creates a builder pre-filled with permissive mock implementations.
    pub fn from_permissive() -> Self {
        Self {
            quarantine: Some(Box::new(MockQuarantine)),
            code_signature: Some(Box::new(MockCodeSignature)),
            entitlements: Some(Box::new(MockEntitlements)),
            amfi: Some(Box::new(MockAmfi)),
            sip: Some(Box::new(MockSip)),
            hardened_runtime: Some(Box::new(MockHardenedRuntime)),
            library_validator: Some(Box::new(MockLibraryValidator)),
            sandbox: Some(Box::new(MockSandbox)),
        }
    }

    /// Creates a builder pre-filled with Apple-default implementations.
    pub fn from_apple_default() -> Self {
        Self {
            quarantine: Some(Box::new(AppleQuarantine)),
            code_signature: Some(Box::new(AppleCodeSignature)),
            entitlements: Some(Box::new(AppleEntitlements)),
            amfi: Some(Box::new(AppleAmfi)),
            sip: Some(Box::new(AppleSip)),
            hardened_runtime: Some(Box::new(AppleHardenedRuntime)),
            library_validator: Some(Box::new(AppleLibraryValidator)),
            sandbox: Some(Box::new(AppleSandbox)),
        }
    }

    /// Creates a builder pre-filled with production real-AMFI implementations.
    ///
    /// - Real CDHash + CMS verification (macOS-compatible defaults)
    /// - Real quarantine xattr check
    /// - Real SIP path enforcement
    /// - Apple-default for other gates
    pub fn from_production() -> Self {
        Self {
            quarantine: Some(Box::new(RealQuarantine::production())),
            code_signature: Some(Box::new(AppleCodeSignature)),
            entitlements: Some(Box::new(AppleEntitlements)),
            amfi: Some(Box::new(RealAmfi::production())),
            sip: Some(Box::new(RealSip::enforcing())),
            hardened_runtime: Some(Box::new(AppleHardenedRuntime)),
            library_validator: Some(Box::new(AppleLibraryValidator)),
            sandbox: Some(Box::new(AppleSandbox)),
        }
    }

    /// Creates a builder in audit mode: real verification, never blocks.
    ///
    /// All real gates run in `LogOnly` mode — failures produce warnings
    /// but never deny execution. Use to observe coverage before going live.
    pub fn from_audit() -> Self {
        Self {
            quarantine: Some(Box::new(RealQuarantine::log_only())),
            code_signature: Some(Box::new(AppleCodeSignature)),
            entitlements: Some(Box::new(AppleEntitlements)),
            amfi: Some(Box::new(RealAmfi::log_only())),
            sip: Some(Box::new(RealSip::log_only())),
            hardened_runtime: Some(Box::new(AppleHardenedRuntime)),
            library_validator: Some(Box::new(AppleLibraryValidator)),
            sandbox: Some(Box::new(AppleSandbox)),
        }
    }

    /// Creates a builder in strict mode: Apple-signed only, no ad-hoc, no unsigned.
    pub fn from_strict() -> Self {
        use crate::enforcement::EnforcementMode;
        Self {
            quarantine: Some(Box::new(RealQuarantine::production())),
            code_signature: Some(Box::new(AppleCodeSignature)),
            entitlements: Some(Box::new(AppleEntitlements)),
            amfi: Some(Box::new(
                RealAmfi::builder()
                    .mode(EnforcementMode::Enforcing)
                    .allow_adhoc(false)
                    .allow_unsigned(false)
                    .verify_cms_chain(true)
                    .build(),
            )),
            sip: Some(Box::new(RealSip::enforcing())),
            hardened_runtime: Some(Box::new(AppleHardenedRuntime)),
            library_validator: Some(Box::new(AppleLibraryValidator)),
            sandbox: Some(Box::new(AppleSandbox)),
        }
    }

    /// Creates a builder from a [`PolicyPreset`].
    pub fn from_preset(preset: PolicyPreset) -> Self {
        match preset {
            PolicyPreset::Permissive => Self::from_permissive(),
            PolicyPreset::AppleDefault => Self::from_apple_default(),
            PolicyPreset::Production => Self::from_production(),
            PolicyPreset::Audit => Self::from_audit(),
            PolicyPreset::Strict => Self::from_strict(),
        }
    }

    pub fn quarantine(mut self, gate: impl QuarantinePolicy + 'static) -> Self {
        self.quarantine = Some(Box::new(gate));
        self
    }

    pub fn code_signature(mut self, gate: impl CodeSignatureValidator + 'static) -> Self {
        self.code_signature = Some(Box::new(gate));
        self
    }

    pub fn entitlements(mut self, gate: impl EntitlementEvaluator + 'static) -> Self {
        self.entitlements = Some(Box::new(gate));
        self
    }

    pub fn amfi(mut self, gate: impl AmfiPolicy + 'static) -> Self {
        self.amfi = Some(Box::new(gate));
        self
    }

    pub fn sip(mut self, gate: impl SipPolicy + 'static) -> Self {
        self.sip = Some(Box::new(gate));
        self
    }

    pub fn hardened_runtime(mut self, gate: impl HardenedRuntimePolicy + 'static) -> Self {
        self.hardened_runtime = Some(Box::new(gate));
        self
    }

    pub fn library_validator(mut self, gate: impl LibraryValidator + 'static) -> Self {
        self.library_validator = Some(Box::new(gate));
        self
    }

    pub fn sandbox(mut self, gate: impl SandboxPolicy + 'static) -> Self {
        self.sandbox = Some(Box::new(gate));
        self
    }

    /// Builds the [`TrustPolicy`], filling any unset gates with mock defaults.
    pub fn build(self) -> TrustPolicy {
        TrustPolicy {
            quarantine: self.quarantine.unwrap_or_else(|| Box::new(MockQuarantine)),
            code_signature: self
                .code_signature
                .unwrap_or_else(|| Box::new(MockCodeSignature)),
            entitlements: self
                .entitlements
                .unwrap_or_else(|| Box::new(MockEntitlements)),
            amfi: self.amfi.unwrap_or_else(|| Box::new(MockAmfi)),
            sip: self.sip.unwrap_or_else(|| Box::new(MockSip)),
            hardened_runtime: self
                .hardened_runtime
                .unwrap_or_else(|| Box::new(MockHardenedRuntime)),
            library_validator: self
                .library_validator
                .unwrap_or_else(|| Box::new(MockLibraryValidator)),
            sandbox: self.sandbox.unwrap_or_else(|| Box::new(MockSandbox)),
        }
    }
}

impl Default for TrustPolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use macho_types::header::MachFlags;

    use super::*;

    #[test]
    fn builder_default_fills_mocks() {
        let policy = TrustPolicyBuilder::new().build();

        assert!(!policy.quarantine.is_enforcing());
        assert!(!policy.code_signature.is_enforcing());
        assert!(!policy.entitlements.is_enforcing());
        assert!(!policy.amfi.is_enforcing());
        assert!(!policy.sip.is_enforcing());
        assert!(!policy.hardened_runtime.is_enforcing());
        assert!(!policy.library_validator.is_enforcing());
        assert!(!policy.sandbox.is_enforcing());
    }

    #[test]
    fn builder_from_apple_overrides_one() {
        let policy = TrustPolicyBuilder::from_apple_default()
            .sandbox(MockSandbox)
            .build();

        assert!(policy.quarantine.is_enforcing());
        assert!(policy.code_signature.is_enforcing());
        assert!(policy.entitlements.is_enforcing());
        assert!(policy.amfi.is_enforcing());
        assert!(policy.sip.is_enforcing());
        assert!(policy.hardened_runtime.is_enforcing());
        assert!(policy.library_validator.is_enforcing());
        assert!(!policy.sandbox.is_enforcing());
    }

    #[test]
    fn builder_from_permissive_all_allow() {
        let policy = TrustPolicyBuilder::from_permissive().build();
        let path = Path::new("/tmp/test_binary");
        let flags = MachFlags::empty();

        let report = policy.evaluate_binary(path, flags, true);
        assert!(report.allowed);
    }

    #[test]
    fn preset_round_trip() {
        let permissive = TrustPolicyBuilder::from_preset(PolicyPreset::Permissive).build();
        assert!(!permissive.quarantine.is_enforcing());
        assert!(!permissive.sip.is_enforcing());

        let apple = TrustPolicyBuilder::from_preset(PolicyPreset::AppleDefault).build();
        assert!(apple.quarantine.is_enforcing());
        assert!(apple.sip.is_enforcing());
    }
}
