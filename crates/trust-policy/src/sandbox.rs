use serde::{Deserialize, Serialize};
use std::path::Path;

/// Operations that can be controlled by the sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxOperation {
    /// Read file data.
    FileRead,
    /// Write file data.
    FileWrite,
    /// Read file metadata (stat, readdir).
    FileReadMetadata,
    /// Execute a binary.
    ProcessExec,
    /// Outbound network connections.
    NetworkOutbound,
    /// Inbound network connections.
    NetworkInbound,
    /// Mach IPC lookup.
    MachLookup,
    /// Signal delivery.
    Signal,
    /// sysctl read.
    SysctlRead,
}

/// Filter that determines which paths/resources a rule applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SandboxFilter {
    Literal { literal: String },
    Subpath { subpath: String },
    Regex { regex: String },
    Extension { extension: String },
    Any { any: bool },
}

impl SandboxFilter {
    /// Test whether this filter matches the given path.
    pub fn matches(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();
        match self {
            Self::Literal { literal } => s == *literal,
            Self::Subpath { subpath } => s.starts_with(subpath.as_str()),
            Self::Regex { regex } => {
                // Simple glob-like matching without regex dependency
                // Supports * as wildcard
                if regex.contains('*') {
                    glob_match(regex, &s)
                } else {
                    s.contains(regex.as_str())
                }
            }
            Self::Extension { extension } => path
                .extension()
                .is_some_and(|e| e.to_string_lossy() == *extension),
            Self::Any { .. } => true,
        }
    }
}

/// Action to take when a rule matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxAction {
    Allow,
    Deny,
}

/// A single sandbox rule: action + operation + filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxRule {
    pub action: SandboxAction,
    pub operation: SandboxOperation,
    pub filter: SandboxFilter,
}

/// A complete sandbox profile: default action + ordered rules.
///
/// Rules are evaluated in order; first match wins. If no rule matches,
/// the default action is applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxProfile {
    /// Default action when no rule matches.
    #[serde(default = "default_action_deny")]
    pub default_action: SandboxAction,
    /// Ordered list of rules (first match wins).
    #[serde(default)]
    pub rules: Vec<SandboxRule>,
    /// Human-readable name for the profile.
    #[serde(default)]
    pub name: String,
}

fn default_action_deny() -> SandboxAction {
    SandboxAction::Deny
}

impl SandboxProfile {
    /// Create an empty deny-default profile.
    pub fn deny_all() -> Self {
        Self {
            default_action: SandboxAction::Deny,
            rules: Vec::new(),
            name: "deny-all".to_string(),
        }
    }

    /// Create a permissive allow-all profile.
    pub fn allow_all() -> Self {
        Self {
            default_action: SandboxAction::Allow,
            rules: Vec::new(),
            name: "allow-all".to_string(),
        }
    }

    /// Create a standard macOS App Sandbox profile.
    ///
    /// This models the default container-based sandbox that macOS applies
    /// to apps with the `com.apple.security.app-sandbox` entitlement.
    pub fn app_sandbox_default() -> Self {
        let rules = vec![
            // Allow reading system frameworks and libraries
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileRead,
                filter: SandboxFilter::Subpath {
                    subpath: "/System/".to_string(),
                },
            },
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileRead,
                filter: SandboxFilter::Subpath {
                    subpath: "/usr/lib/".to_string(),
                },
            },
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileRead,
                filter: SandboxFilter::Subpath {
                    subpath: "/usr/share/".to_string(),
                },
            },
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileRead,
                filter: SandboxFilter::Subpath {
                    subpath: "/Library/Frameworks/".to_string(),
                },
            },
            // Allow reading app bundle
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileRead,
                filter: SandboxFilter::Subpath {
                    subpath: "/Applications/".to_string(),
                },
            },
            // Allow /tmp and /var/tmp for temporary files
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileRead,
                filter: SandboxFilter::Subpath {
                    subpath: "/tmp/".to_string(),
                },
            },
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileWrite,
                filter: SandboxFilter::Subpath {
                    subpath: "/tmp/".to_string(),
                },
            },
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileRead,
                filter: SandboxFilter::Subpath {
                    subpath: "/var/tmp/".to_string(),
                },
            },
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileWrite,
                filter: SandboxFilter::Subpath {
                    subpath: "/var/tmp/".to_string(),
                },
            },
            // Allow /dev for device access
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileRead,
                filter: SandboxFilter::Subpath {
                    subpath: "/dev/".to_string(),
                },
            },
            // Allow process execution of system binaries
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::ProcessExec,
                filter: SandboxFilter::Subpath {
                    subpath: "/usr/bin/".to_string(),
                },
            },
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::ProcessExec,
                filter: SandboxFilter::Subpath {
                    subpath: "/bin/".to_string(),
                },
            },
            // Allow sysctl reads
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::SysctlRead,
                filter: SandboxFilter::Any { any: true },
            },
            // Allow metadata reads everywhere (needed for path resolution)
            SandboxRule {
                action: SandboxAction::Allow,
                operation: SandboxOperation::FileReadMetadata,
                filter: SandboxFilter::Any { any: true },
            },
        ];

        Self {
            default_action: SandboxAction::Deny,
            rules,
            name: "app-sandbox-default".to_string(),
        }
    }

    /// Load a profile from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Evaluate an operation against this profile.
    ///
    /// Returns the action (Allow/Deny) for the given operation on the given path.
    pub fn evaluate(&self, operation: SandboxOperation, path: &Path) -> SandboxAction {
        for rule in &self.rules {
            if rule.operation == operation && rule.filter.matches(path) {
                return rule.action;
            }
        }
        self.default_action
    }

    /// Evaluate file read access.
    pub fn check_file_read(&self, path: &Path) -> SandboxAction {
        self.evaluate(SandboxOperation::FileRead, path)
    }

    /// Evaluate file write access.
    pub fn check_file_write(&self, path: &Path) -> SandboxAction {
        self.evaluate(SandboxOperation::FileWrite, path)
    }

    /// Evaluate process execution.
    pub fn check_process_exec(&self, path: &Path) -> SandboxAction {
        self.evaluate(SandboxOperation::ProcessExec, path)
    }

    /// Add a rule to the profile.
    pub fn add_rule(&mut self, rule: SandboxRule) {
        self.rules.push(rule);
    }

    /// Number of rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for SandboxProfile {
    fn default() -> Self {
        Self::allow_all()
    }
}

/// Sandbox enforcement gate backed by a [`SandboxProfile`].
///
/// When activated (via entitlement or explicit configuration), evaluates
/// file access against the loaded profile.
pub struct SandboxEngine {
    /// The active sandbox profile (None = not sandboxed).
    profile: Option<SandboxProfile>,
    /// Whether sandbox is active.
    active: bool,
}

impl SandboxEngine {
    /// Create an inactive sandbox (allows everything).
    pub fn inactive() -> Self {
        Self {
            profile: None,
            active: false,
        }
    }

    /// Create with the default App Sandbox profile.
    pub fn app_sandbox() -> Self {
        Self {
            profile: Some(SandboxProfile::app_sandbox_default()),
            active: true,
        }
    }

    /// Create with a custom profile.
    pub fn with_profile(profile: SandboxProfile) -> Self {
        Self {
            profile: Some(profile),
            active: true,
        }
    }

    /// Activate sandbox with a profile.
    pub fn activate(&mut self, profile: SandboxProfile) {
        self.profile = Some(profile);
        self.active = true;
    }

    /// Whether the sandbox is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get the profile name if active.
    pub fn profile_name(&self) -> Option<&str> {
        self.profile.as_ref().map(|p| p.name.as_str())
    }

    /// Evaluate a specific operation (richer than the trait method).
    pub fn evaluate_operation(
        &self,
        operation: SandboxOperation,
        path: &Path,
    ) -> crate::SecurityVerdict {
        match &self.profile {
            Some(profile) if self.active => match profile.evaluate(operation, path) {
                SandboxAction::Allow => crate::SecurityVerdict::Allow,
                SandboxAction::Deny => crate::SecurityVerdict::Deny {
                    reason: format!(
                        "sandbox({}) denied {:?} on {}",
                        profile.name,
                        operation,
                        path.display()
                    ),
                },
            },
            _ => crate::SecurityVerdict::Allow,
        }
    }
}

impl crate::gates::SecurityGate for SandboxEngine {
    fn name(&self) -> &str {
        "Sandbox (engine)"
    }
    fn is_enforcing(&self) -> bool {
        self.active
    }
}

impl crate::gates::SandboxPolicy for SandboxEngine {
    fn check_file_access(&self, path: &Path) -> crate::SecurityVerdict {
        // The trait method checks file read access (most common check)
        self.evaluate_operation(SandboxOperation::FileRead, path)
    }
}

/// Simple glob matching: supports `*` as wildcard for any characters.
fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return text == pattern;
    }

    // Check prefix
    if !parts[0].is_empty() && !text.starts_with(parts[0]) {
        return false;
    }

    // Check suffix
    if let Some(last) = parts.last() {
        if !last.is_empty() && !text.ends_with(last) {
            return false;
        }
    }

    // Check all parts appear in order
    let mut pos = 0;
    for part in &parts {
        if part.is_empty() {
            continue;
        }
        match text[pos..].find(part) {
            Some(idx) => pos += idx + part.len(),
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::{SandboxPolicy, SecurityGate};

    #[test]
    fn deny_all_denies_everything() {
        let profile = SandboxProfile::deny_all();
        assert_eq!(
            profile.check_file_read(Path::new("/any/path")),
            SandboxAction::Deny
        );
    }

    #[test]
    fn allow_all_allows_everything() {
        let profile = SandboxProfile::allow_all();
        assert_eq!(
            profile.check_file_read(Path::new("/any/path")),
            SandboxAction::Allow
        );
    }

    #[test]
    fn filter_literal_match() {
        let filter = SandboxFilter::Literal {
            literal: "/usr/bin/ls".to_string(),
        };
        assert!(filter.matches(Path::new("/usr/bin/ls")));
    }

    #[test]
    fn filter_literal_no_match() {
        let filter = SandboxFilter::Literal {
            literal: "/usr/bin/ls".to_string(),
        };
        assert!(!filter.matches(Path::new("/usr/bin/cat")));
    }

    #[test]
    fn filter_subpath_match() {
        let filter = SandboxFilter::Subpath {
            subpath: "/usr/".to_string(),
        };
        assert!(filter.matches(Path::new("/usr/lib/foo")));
    }

    #[test]
    fn filter_subpath_no_match() {
        let filter = SandboxFilter::Subpath {
            subpath: "/usr/".to_string(),
        };
        assert!(!filter.matches(Path::new("/tmp/foo")));
    }

    #[test]
    fn filter_extension_match() {
        let filter = SandboxFilter::Extension {
            extension: "dylib".to_string(),
        };
        assert!(filter.matches(Path::new("/usr/lib/foo.dylib")));
    }

    #[test]
    fn filter_any_matches_all() {
        let filter = SandboxFilter::Any { any: true };
        assert!(filter.matches(Path::new("/literally/anything")));
    }

    #[test]
    fn filter_regex_glob() {
        let filter = SandboxFilter::Regex {
            regex: "*.dylib".to_string(),
        };
        assert!(filter.matches(Path::new("/lib/foo.dylib")));
    }

    #[test]
    fn app_sandbox_allows_system_read() {
        let profile = SandboxProfile::app_sandbox_default();
        assert_eq!(
            profile.check_file_read(Path::new("/System/Library/foo")),
            SandboxAction::Allow
        );
    }

    #[test]
    fn app_sandbox_denies_home_read() {
        let profile = SandboxProfile::app_sandbox_default();
        assert_eq!(
            profile.check_file_read(Path::new("/Users/test/Documents/secret")),
            SandboxAction::Deny
        );
    }

    #[test]
    fn app_sandbox_allows_tmp_write() {
        let profile = SandboxProfile::app_sandbox_default();
        assert_eq!(
            profile.check_file_write(Path::new("/tmp/foo")),
            SandboxAction::Allow
        );
    }

    #[test]
    fn app_sandbox_denies_arbitrary_write() {
        let profile = SandboxProfile::app_sandbox_default();
        assert_eq!(
            profile.check_file_write(Path::new("/usr/local/foo")),
            SandboxAction::Deny
        );
    }

    #[test]
    fn app_sandbox_allows_system_exec() {
        let profile = SandboxProfile::app_sandbox_default();
        assert_eq!(
            profile.check_process_exec(Path::new("/usr/bin/env")),
            SandboxAction::Allow
        );
    }

    #[test]
    fn profile_from_toml() {
        let toml_str = r#"
name = "test-profile"
default_action = "deny"

[[rules]]
action = "allow"
operation = "file-read"

[rules.filter]
subpath = "/usr/lib/"

[[rules]]
action = "allow"
operation = "file-write"

[rules.filter]
subpath = "/tmp/"
"#;
        let profile = SandboxProfile::from_toml(toml_str).expect("valid TOML");
        assert_eq!(profile.name, "test-profile");
        assert_eq!(profile.default_action, SandboxAction::Deny);
        assert_eq!(profile.rules.len(), 2);
        assert_eq!(profile.rules[0].operation, SandboxOperation::FileRead);
        assert_eq!(
            profile.check_file_read(Path::new("/usr/lib/libSystem.dylib")),
            SandboxAction::Allow
        );
        assert_eq!(
            profile.check_file_write(Path::new("/tmp/scratch")),
            SandboxAction::Allow
        );
        assert_eq!(
            profile.check_file_read(Path::new("/home/secret")),
            SandboxAction::Deny
        );
    }

    #[test]
    fn sandbox_engine_inactive_allows() {
        let engine = SandboxEngine::inactive();
        assert!(!engine.is_active());
        let verdict =
            engine.evaluate_operation(SandboxOperation::FileRead, Path::new("/etc/passwd"));
        assert!(verdict.is_allow());
    }

    #[test]
    fn sandbox_engine_active_denies() {
        let engine = SandboxEngine::with_profile(SandboxProfile::deny_all());
        assert!(engine.is_active());
        let verdict =
            engine.evaluate_operation(SandboxOperation::FileRead, Path::new("/etc/passwd"));
        assert!(verdict.is_deny());
    }

    #[test]
    fn sandbox_engine_app_sandbox() {
        let engine = SandboxEngine::app_sandbox();
        assert!(engine.is_active());
        assert_eq!(engine.profile_name(), Some("app-sandbox-default"));
        let verdict =
            engine.evaluate_operation(SandboxOperation::FileRead, Path::new("/System/Library/foo"));
        assert!(verdict.is_allow());
    }

    #[test]
    fn sandbox_engine_trait_check() {
        let engine = SandboxEngine::app_sandbox();
        assert_eq!(engine.name(), "Sandbox (engine)");
        assert!(engine.is_enforcing());
        let verdict = engine.check_file_access(Path::new("/System/Library/Frameworks/foo"));
        assert!(verdict.is_allow());
    }

    #[test]
    fn evaluate_operation_network() {
        let engine = SandboxEngine::app_sandbox();
        let verdict =
            engine.evaluate_operation(SandboxOperation::NetworkOutbound, Path::new("0.0.0.0:443"));
        assert!(verdict.is_deny());
    }

    #[test]
    fn first_match_wins() {
        let mut profile = SandboxProfile::deny_all();
        profile.add_rule(SandboxRule {
            action: SandboxAction::Allow,
            operation: SandboxOperation::FileRead,
            filter: SandboxFilter::Subpath {
                subpath: "/data/".to_string(),
            },
        });
        profile.add_rule(SandboxRule {
            action: SandboxAction::Deny,
            operation: SandboxOperation::FileRead,
            filter: SandboxFilter::Subpath {
                subpath: "/data/".to_string(),
            },
        });
        assert_eq!(
            profile.check_file_read(Path::new("/data/file.txt")),
            SandboxAction::Allow
        );
    }

    #[test]
    fn glob_match_basic() {
        assert!(glob_match("*.dylib", "/lib/foo.dylib"));
        assert!(glob_match("/usr/*/lib", "/usr/local/lib"));
        assert!(!glob_match("*.dylib", "/lib/foo.so"));
        assert!(glob_match("*", "anything"));
    }
}
