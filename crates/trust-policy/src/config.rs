use std::path::Path;

use macho_types::header::MachFlags;
use serde::{Deserialize, Serialize};

use crate::builder::{PolicyPreset, TrustPolicyBuilder};
use crate::gates::{EntitlementEvaluator, SecurityGate, SipPolicy};
use crate::mock::*;
use crate::SecurityVerdict;
use crate::TrustPolicy;

// ── Configuration structs ───────────────────────────────────────────

/// Top-level policy configuration, loadable from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PolicyConfig {
    /// Base preset: "permissive" or "apple"
    pub preset: PolicyPreset,
    /// Per-gate configuration
    pub quarantine: GateConfig,
    pub code_signature: CodeSignatureConfig,
    pub sip: SipConfig,
    pub hardened_runtime: HardenedRuntimeConfig,
    pub entitlements: EntitlementConfig,
    pub library_validator: GateConfig,
    pub sandbox: GateConfig,
    pub amfi: GateConfig,
}

/// Simple gate toggle — just enabled/disabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GateConfig {
    pub enabled: bool,
}

/// Code signature gate configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CodeSignatureConfig {
    pub enabled: bool,
    pub require_signature: bool,
    pub allow_adhoc: bool,
    pub allow_unsigned_non_sip: bool,
}

/// SIP gate configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SipConfig {
    pub enabled: bool,
    pub protected_paths: Vec<String>,
    pub exempt_paths: Vec<String>,
}

/// Hardened runtime gate configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HardenedRuntimeConfig {
    pub enabled: bool,
    pub enforce: bool,
    pub allow_dylib_insertion_with_entitlement: bool,
}

/// Entitlement evaluation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EntitlementConfig {
    pub enabled: bool,
    /// Entitlement keys that produce warnings when present (and true).
    pub warn_on: Vec<String>,
    /// Entitlement keys that cause denial when present (and true).
    pub deny_on: Vec<String>,
}

// ── Default implementations ─────────────────────────────────────────

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            preset: PolicyPreset::Permissive,
            quarantine: GateConfig::default(),
            code_signature: CodeSignatureConfig::default(),
            sip: SipConfig::default(),
            hardened_runtime: HardenedRuntimeConfig::default(),
            entitlements: EntitlementConfig::default(),
            library_validator: GateConfig::default(),
            sandbox: GateConfig::default(),
            amfi: GateConfig::default(),
        }
    }
}

impl Default for GateConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Default for CodeSignatureConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_signature: true,
            allow_adhoc: true,
            allow_unsigned_non_sip: true,
        }
    }
}

impl Default for SipConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            protected_paths: vec![
                "/usr/".into(),
                "/System/".into(),
                "/bin/".into(),
                "/sbin/".into(),
            ],
            exempt_paths: vec!["/usr/local/".into()],
        }
    }
}

impl Default for HardenedRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enforce: true,
            allow_dylib_insertion_with_entitlement: true,
        }
    }
}

impl Default for EntitlementConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            warn_on: vec![
                "com.apple.security.cs.disable-library-validation".into(),
                "com.apple.security.cs.allow-dyld-environment-variables".into(),
            ],
            deny_on: vec![],
        }
    }
}

// ── ConfigError ─────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "config I/O error: {e}"),
            Self::Parse(e) => write!(f, "config parse error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        Self::Parse(e)
    }
}

// ── PolicyConfig methods ────────────────────────────────────────────

impl PolicyConfig {
    /// Load from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Load from a file path.
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Build a [`TrustPolicy`] from this configuration.
    ///
    /// Starts from the preset, then overrides gates that have custom config.
    pub fn build_policy(&self) -> TrustPolicy {
        let mut builder = TrustPolicyBuilder::from_preset(self.preset);

        // Disable gates by replacing with mock variants
        if !self.quarantine.enabled {
            builder = builder.quarantine(MockQuarantine);
        }
        if !self.code_signature.enabled {
            builder = builder.code_signature(MockCodeSignature);
        }
        if !self.amfi.enabled {
            builder = builder.amfi(MockAmfi);
        }
        if !self.hardened_runtime.enabled {
            builder = builder.hardened_runtime(MockHardenedRuntime);
        }
        if !self.library_validator.enabled {
            builder = builder.library_validator(MockLibraryValidator);
        }
        if !self.sandbox.enabled {
            builder = builder.sandbox(MockSandbox);
        }

        // SIP: use configurable impl if custom paths are provided, or mock if disabled
        if !self.sip.enabled {
            builder = builder.sip(MockSip);
        } else if self.has_custom_sip_paths() {
            builder = builder.sip(ConfigurableSip::new(
                self.sip.protected_paths.clone(),
                self.sip.exempt_paths.clone(),
            ));
        }

        // Entitlements: use configurable impl if custom lists, or mock if disabled
        if !self.entitlements.enabled {
            builder = builder.entitlements(MockEntitlements);
        } else if self.has_custom_entitlement_lists() {
            builder = builder.entitlements(ConfigurableEntitlements::new(
                self.entitlements.warn_on.clone(),
                self.entitlements.deny_on.clone(),
            ));
        }

        builder.build()
    }

    fn has_custom_sip_paths(&self) -> bool {
        let default = SipConfig::default();
        self.sip.protected_paths != default.protected_paths
            || self.sip.exempt_paths != default.exempt_paths
    }

    fn has_custom_entitlement_lists(&self) -> bool {
        let default = EntitlementConfig::default();
        self.entitlements.warn_on != default.warn_on
            || self.entitlements.deny_on != default.deny_on
    }
}

// ── ConfigurableSip ─────────────────────────────────────────────────

pub struct ConfigurableSip {
    protected_paths: Vec<String>,
    exempt_paths: Vec<String>,
}

impl ConfigurableSip {
    pub fn new(protected_paths: Vec<String>, exempt_paths: Vec<String>) -> Self {
        Self {
            protected_paths,
            exempt_paths,
        }
    }
}

impl SecurityGate for ConfigurableSip {
    fn name(&self) -> &str {
        "SIP (configured)"
    }

    fn is_enforcing(&self) -> bool {
        true
    }
}

impl SipPolicy for ConfigurableSip {
    fn is_path_protected(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();
        for exempt in &self.exempt_paths {
            if s.starts_with(exempt.as_str()) {
                return false;
            }
        }
        for prefix in &self.protected_paths {
            if s.starts_with(prefix.as_str()) {
                return true;
            }
        }
        false
    }

    fn should_strip_dyld_env(&self, binary_path: &Path, _flags: MachFlags) -> bool {
        self.is_path_protected(binary_path)
    }
}

// ── ConfigurableEntitlements ────────────────────────────────────────

pub struct ConfigurableEntitlements {
    warn_on: Vec<String>,
    deny_on: Vec<String>,
}

impl ConfigurableEntitlements {
    pub fn new(warn_on: Vec<String>, deny_on: Vec<String>) -> Self {
        Self { warn_on, deny_on }
    }
}

impl SecurityGate for ConfigurableEntitlements {
    fn name(&self) -> &str {
        "Entitlements (configured)"
    }

    fn is_enforcing(&self) -> bool {
        true
    }
}

impl EntitlementEvaluator for ConfigurableEntitlements {
    fn evaluate(&self, _binary_path: &Path) -> SecurityVerdict {
        SecurityVerdict::Allow
    }

    fn evaluate_with_keys(&self, _binary_path: &Path, keys: &[(String, bool)]) -> SecurityVerdict {
        for (key, value) in keys {
            if *value && self.deny_on.contains(key) {
                return SecurityVerdict::Deny {
                    reason: format!("entitlement denied by policy: {key}"),
                };
            }
        }
        for (key, value) in keys {
            if *value && self.warn_on.contains(key) {
                return SecurityVerdict::AllowWithWarning {
                    warning: format!("entitlement flagged by policy: {key}"),
                };
            }
        }
        SecurityVerdict::Allow
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_round_trip() {
        let config = PolicyConfig::default();
        let toml_str = toml::to_string(&config).expect("serialize");
        let parsed: PolicyConfig = PolicyConfig::from_toml(&toml_str).expect("parse");

        assert_eq!(parsed.preset, config.preset);
        assert_eq!(parsed.quarantine.enabled, config.quarantine.enabled);
        assert_eq!(parsed.code_signature.enabled, config.code_signature.enabled);
        assert_eq!(
            parsed.code_signature.require_signature,
            config.code_signature.require_signature
        );
        assert_eq!(parsed.sip.enabled, config.sip.enabled);
        assert_eq!(parsed.sip.protected_paths, config.sip.protected_paths);
        assert_eq!(parsed.sip.exempt_paths, config.sip.exempt_paths);
        assert_eq!(
            parsed.hardened_runtime.enforce,
            config.hardened_runtime.enforce
        );
        assert_eq!(parsed.entitlements.warn_on, config.entitlements.warn_on);
        assert_eq!(parsed.entitlements.deny_on, config.entitlements.deny_on);
        assert_eq!(parsed.sandbox.enabled, config.sandbox.enabled);
        assert_eq!(parsed.amfi.enabled, config.amfi.enabled);
    }

    #[test]
    fn from_toml_custom_sip() {
        let toml_str = r#"
            preset = "permissive"

            [sip]
            enabled = true
            protected_paths = ["/opt/secure/", "/private/"]
            exempt_paths = ["/opt/secure/dev/"]
        "#;
        let config = PolicyConfig::from_toml(toml_str).expect("parse");
        assert_eq!(config.sip.protected_paths, vec!["/opt/secure/", "/private/"]);
        assert_eq!(config.sip.exempt_paths, vec!["/opt/secure/dev/"]);

        let policy = config.build_policy();
        assert!(policy.sip.is_path_protected(Path::new("/opt/secure/bin/tool")));
        assert!(!policy.sip.is_path_protected(Path::new("/opt/secure/dev/test")));
        assert!(!policy.sip.is_path_protected(Path::new("/usr/bin/ls")));
    }

    #[test]
    fn disabled_gate_becomes_mock() {
        let toml_str = r#"
            preset = "apple"

            [sandbox]
            enabled = false
        "#;
        let config = PolicyConfig::from_toml(toml_str).expect("parse");
        let policy = config.build_policy();

        assert!(!policy.sandbox.is_enforcing());
        assert!(policy.sip.is_enforcing());
        assert!(policy.code_signature.is_enforcing());
    }

    #[test]
    fn configurable_entitlements_deny() {
        let ent = ConfigurableEntitlements::new(
            vec![],
            vec!["com.apple.private.dangerous".into()],
        );
        let keys = vec![("com.apple.private.dangerous".to_string(), true)];
        let verdict = ent.evaluate_with_keys(Path::new("/tmp/app"), &keys);
        assert!(verdict.is_deny());
    }

    #[test]
    fn configurable_entitlements_warn() {
        let ent = ConfigurableEntitlements::new(
            vec!["com.apple.security.cs.disable-library-validation".into()],
            vec![],
        );
        let keys = vec![(
            "com.apple.security.cs.disable-library-validation".to_string(),
            true,
        )];
        let verdict = ent.evaluate_with_keys(Path::new("/tmp/app"), &keys);
        assert!(verdict.is_allow());
        assert!(matches!(verdict, SecurityVerdict::AllowWithWarning { .. }));
    }

    #[test]
    fn configurable_sip_custom_paths() {
        let sip = ConfigurableSip::new(
            vec!["/custom/protected/".into()],
            vec!["/custom/protected/exempt/".into()],
        );

        assert!(sip.is_path_protected(Path::new("/custom/protected/bin/tool")));
        assert!(!sip.is_path_protected(Path::new("/custom/protected/exempt/dev")));
        assert!(!sip.is_path_protected(Path::new("/tmp/random")));
        assert!(sip.should_strip_dyld_env(
            Path::new("/custom/protected/bin/tool"),
            MachFlags::empty()
        ));
        assert!(!sip.should_strip_dyld_env(
            Path::new("/tmp/random"),
            MachFlags::empty()
        ));
    }
}
