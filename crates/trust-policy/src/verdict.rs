use serde::{Deserialize, Serialize};

/// Verdict from any security gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityVerdict {
    /// Gate allows execution unconditionally.
    Allow,
    /// Gate denies execution with a reason.
    Deny { reason: String },
    /// Gate allows but logs a warning (for mock/dev mode).
    AllowWithWarning { warning: String },
}

impl SecurityVerdict {
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow | Self::AllowWithWarning { .. })
    }

    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }
}

/// A single gate's decision, recorded in a SecurityReport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDecision {
    /// Name of the gate that made this decision.
    pub gate: String,
    /// The verdict.
    pub verdict: SecurityVerdict,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_is_allow() {
        assert!(SecurityVerdict::Allow.is_allow());
        assert!(!SecurityVerdict::Allow.is_deny());
    }

    #[test]
    fn deny_is_deny() {
        let v = SecurityVerdict::Deny {
            reason: "bad".into(),
        };
        assert!(v.is_deny());
        assert!(!v.is_allow());
    }

    #[test]
    fn allow_with_warning_is_allow() {
        let v = SecurityVerdict::AllowWithWarning {
            warning: "caution".into(),
        };
        assert!(v.is_allow());
        assert!(!v.is_deny());
    }
}
