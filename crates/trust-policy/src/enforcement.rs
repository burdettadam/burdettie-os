//! Enforcement mode — controls whether gate failures block execution.
//!
//! AMFI on Apple hardware operates in one of three modes:
//! - **Enforcing** — violations are fatal; the binary is denied.
//! - **LogOnly**  — violations are recorded but execution proceeds.
//! - **Promiscuous** — all checks pass unconditionally; disables AMFI entirely.
//!
//! Our `EnforcementMode` models all three. The `promiscuous` flag maps to
//! Apple's `amfi_get_out_of_my_way=1` boot argument (or `--amfi-allow-any-signature`).

use serde::{Deserialize, Serialize};

use crate::SecurityVerdict;

/// How strictly a gate enforces its policy.
///
/// Applied at the gate level — each gate can use a different mode.
/// `TrustPolicyBuilder::with_enforcement_mode()` applies the same mode to all gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementMode {
    /// All checks disabled — every binary is unconditionally allowed.
    ///
    /// Maps to Apple's `amfi_get_out_of_my_way=1` boot argument.
    /// Use this for development, fuzzing, or compatibility testing.
    Promiscuous,

    /// Checks run but failures are downgraded to warnings — execution always proceeds.
    ///
    /// Use this for audit/observability: see what *would* have been blocked
    /// without actually blocking anything. Useful for measuring coverage
    /// before enabling enforcing mode.
    LogOnly,

    /// Full enforcement — check failures deny execution.
    ///
    /// This is the production setting. Matches Apple macOS default behavior.
    Enforcing,
}

impl Default for EnforcementMode {
    fn default() -> Self {
        Self::Enforcing
    }
}

impl EnforcementMode {
    /// Whether this mode performs real checks at all.
    #[inline]
    pub fn is_active(self) -> bool {
        self != Self::Promiscuous
    }

    /// Whether failures in this mode result in a `Deny` verdict.
    #[inline]
    pub fn is_enforcing(self) -> bool {
        self == Self::Enforcing
    }

    /// Apply the mode to a potential denial verdict.
    ///
    /// - `Promiscuous` → `Allow` (unconditionally)
    /// - `LogOnly` → `AllowWithWarning` (failure recorded but not fatal)
    /// - `Enforcing` → `Deny` (the original denial is preserved)
    pub fn apply(&self, verdict: SecurityVerdict) -> SecurityVerdict {
        match self {
            Self::Promiscuous => SecurityVerdict::Allow,
            Self::LogOnly => match verdict {
                SecurityVerdict::Deny { reason } => SecurityVerdict::AllowWithWarning {
                    warning: format!("[log-only] would have denied: {reason}"),
                },
                other => other,
            },
            Self::Enforcing => verdict,
        }
    }

    /// Apply the mode to a warning verdict (warnings are always kept unless promiscuous).
    pub fn apply_warning(&self, verdict: SecurityVerdict) -> SecurityVerdict {
        match self {
            Self::Promiscuous => SecurityVerdict::Allow,
            _ => verdict, // LogOnly and Enforcing preserve warnings
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promiscuous_converts_deny_to_allow() {
        let deny = SecurityVerdict::Deny {
            reason: "test".to_string(),
        };
        assert_eq!(
            EnforcementMode::Promiscuous.apply(deny),
            SecurityVerdict::Allow
        );
    }

    #[test]
    fn log_only_converts_deny_to_warning() {
        let deny = SecurityVerdict::Deny {
            reason: "unsigned binary".to_string(),
        };
        match EnforcementMode::LogOnly.apply(deny) {
            SecurityVerdict::AllowWithWarning { warning } => {
                assert!(warning.contains("[log-only]"));
                assert!(warning.contains("unsigned binary"));
            }
            other => panic!("expected AllowWithWarning, got {other:?}"),
        }
    }

    #[test]
    fn enforcing_preserves_deny() {
        let deny = SecurityVerdict::Deny {
            reason: "bad".to_string(),
        };
        assert!(matches!(
            EnforcementMode::Enforcing.apply(deny),
            SecurityVerdict::Deny { .. }
        ));
    }

    #[test]
    fn log_only_passes_allow_through() {
        assert_eq!(
            EnforcementMode::LogOnly.apply(SecurityVerdict::Allow),
            SecurityVerdict::Allow
        );
    }

    #[test]
    fn is_active() {
        assert!(!EnforcementMode::Promiscuous.is_active());
        assert!(EnforcementMode::LogOnly.is_active());
        assert!(EnforcementMode::Enforcing.is_active());
    }

    #[test]
    fn is_enforcing() {
        assert!(!EnforcementMode::Promiscuous.is_enforcing());
        assert!(!EnforcementMode::LogOnly.is_enforcing());
        assert!(EnforcementMode::Enforcing.is_enforcing());
    }

    #[test]
    fn promiscuous_applies_to_warning_too() {
        let warn = SecurityVerdict::AllowWithWarning {
            warning: "risky".to_string(),
        };
        assert_eq!(
            EnforcementMode::Promiscuous.apply_warning(warn),
            SecurityVerdict::Allow
        );
    }
}
