//! Quarantine xattr parsing, notarization ticket verification, and
//! Gatekeeper assessment.
//!
//! On macOS, `com.apple.quarantine` is an extended attribute set by
//! web browsers, email clients, and other download agents. It triggers
//! Gatekeeper's first-launch check. This module parses the quarantine
//! xattr and models the Gatekeeper assessment flow.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::gates::{QuarantinePolicy, SecurityGate};
use crate::{CodeSignatureInfo, SecurityVerdict};

bitflags::bitflags! {
    /// Quarantine flags from the com.apple.quarantine xattr.
    ///
    /// The xattr value is a semicolon-delimited string:
    /// `flags;timestamp;agent_name;uuid`
    ///
    /// Example: `0083;5f3be4c0;Safari;12345678-1234-1234-1234-123456789012`
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct QuarantineFlags: u32 {
        /// User has approved execution (Gatekeeper dialog accepted).
        const USER_APPROVED     = 0x0001;
        /// File was downloaded from the internet.
        const FROM_WEB          = 0x0002;
        /// File was received via email.
        const FROM_EMAIL        = 0x0008;
        /// File has been scanned by Gatekeeper/XProtect.
        const SCANNED           = 0x0010;
        /// Notarization ticket is stapled.
        const NOTARIZED_STAPLED = 0x0040;
        /// File was quarantined by Launch Services.
        const LAUNCH_SERVICES   = 0x0080;
    }
}

impl Serialize for QuarantineFlags {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QuarantineFlags {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u32::deserialize(deserializer)?;
        Ok(Self::from_bits_truncate(bits))
    }
}

/// Parsed quarantine extended attribute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineInfo {
    /// Raw quarantine flags.
    pub flags: QuarantineFlags,
    /// Hex timestamp (seconds since epoch).
    pub timestamp: String,
    /// Agent that set the quarantine (e.g., "Safari", "Mail", "curl").
    pub agent_name: String,
    /// UUID of the download or quarantine event.
    pub uuid: String,
    /// The raw xattr string value.
    pub raw_value: String,
}

impl QuarantineInfo {
    /// Parse a quarantine xattr string value.
    ///
    /// Format: `flags;timestamp;agent_name;uuid`
    /// Example: `0083;5f3be4c0;Safari;12345678-1234-1234-1234-123456789012`
    pub fn parse(xattr_value: &str) -> Option<Self> {
        let parts: Vec<&str> = xattr_value.split(';').collect();
        if parts.len() < 3 {
            return None;
        }

        let flags_hex = parts[0].trim_start_matches("0x").trim_start_matches("0X");
        let flags_val = u32::from_str_radix(flags_hex, 16).unwrap_or(0);
        let flags = QuarantineFlags::from_bits_truncate(flags_val);

        let timestamp = parts[1].to_string();
        let agent_name = parts[2].to_string();
        let uuid = if parts.len() > 3 {
            parts[3].to_string()
        } else {
            String::new()
        };

        Some(Self {
            flags,
            timestamp,
            agent_name,
            uuid,
            raw_value: xattr_value.to_string(),
        })
    }

    /// Whether the user has already approved this file.
    pub fn is_user_approved(&self) -> bool {
        self.flags.contains(QuarantineFlags::USER_APPROVED)
    }

    /// Whether a notarization ticket is stapled.
    pub fn is_notarized_stapled(&self) -> bool {
        self.flags.contains(QuarantineFlags::NOTARIZED_STAPLED)
    }

    /// Whether the file has been scanned.
    pub fn is_scanned(&self) -> bool {
        self.flags.contains(QuarantineFlags::SCANNED)
    }

    /// Whether the file came from the web.
    pub fn is_from_web(&self) -> bool {
        self.flags.contains(QuarantineFlags::FROM_WEB)
    }
}

// ── Notarization ───────────────────────────────────────────────────

/// Notarization status of a binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotarizationStatus {
    /// Notarization ticket is stapled to the binary or found via xattr.
    Stapled,
    /// Not notarized (no ticket found).
    NotNotarized,
    /// Notarization check was not performed.
    Unknown,
}

/// Result of notarization verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotarizationResult {
    /// The notarization status.
    pub status: NotarizationStatus,
    /// CDHash that was checked (if available).
    pub cd_hash: String,
    /// Whether the CDHash was found in a ticket.
    pub ticket_found: bool,
    /// Human-readable summary.
    pub summary: String,
}

impl NotarizationResult {
    /// Create a result indicating notarization via stapled ticket.
    pub fn stapled(cd_hash: &str) -> Self {
        Self {
            status: NotarizationStatus::Stapled,
            cd_hash: cd_hash.to_string(),
            ticket_found: true,
            summary: format!("notarized (stapled ticket, CDHash: {cd_hash})"),
        }
    }

    /// Create a result indicating no notarization found.
    pub fn not_notarized() -> Self {
        Self {
            status: NotarizationStatus::NotNotarized,
            cd_hash: String::new(),
            ticket_found: false,
            summary: "not notarized".to_string(),
        }
    }

    /// Create an unknown result (check not performed).
    pub fn unknown() -> Self {
        Self {
            status: NotarizationStatus::Unknown,
            cd_hash: String::new(),
            ticket_found: false,
            summary: "notarization check not performed".to_string(),
        }
    }
}

// ── Gatekeeper assessment ──────────────────────────────────────────

/// Overall Gatekeeper assessment combining quarantine, code signature,
/// notarization, and trust cache status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatekeeperAssessment {
    /// Whether the binary would be allowed to execute.
    pub allowed: bool,
    /// Quarantine info (if xattr was found).
    pub quarantine: Option<QuarantineInfo>,
    /// Notarization result.
    pub notarization: NotarizationResult,
    /// Whether the binary is code-signed.
    pub code_signed: bool,
    /// Whether the CMS signature chains to Apple Root CA.
    pub chains_to_apple: bool,
    /// Whether the CDHash is in the trust cache.
    pub trust_cache_hit: bool,
    /// Assessment reason/summary.
    pub reason: String,
}

impl GatekeeperAssessment {
    /// Perform a Gatekeeper assessment of a binary.
    ///
    /// This combines quarantine status, code signature info, notarization,
    /// and trust cache into a single allow/deny decision matching macOS behavior.
    pub fn assess(
        quarantine: Option<&QuarantineInfo>,
        cs_info: &CodeSignatureInfo,
        notarization: &NotarizationResult,
        trust_cache_hit: bool,
    ) -> Self {
        // Trust cache entries bypass all other checks (like Apple first-party)
        if trust_cache_hit {
            return Self {
                allowed: true,
                quarantine: quarantine.cloned(),
                notarization: notarization.clone(),
                code_signed: cs_info.present,
                chains_to_apple: cs_info.has_cms_signature,
                trust_cache_hit: true,
                reason: "allowed: CDHash in trust cache".to_string(),
            };
        }

        // No quarantine xattr → not from a quarantined source → allow
        let Some(qinfo) = quarantine else {
            return Self {
                allowed: true,
                quarantine: None,
                notarization: notarization.clone(),
                code_signed: cs_info.present,
                chains_to_apple: false,
                trust_cache_hit: false,
                reason: "allowed: no quarantine xattr (not from quarantined source)".to_string(),
            };
        };

        // User already approved → allow
        if qinfo.is_user_approved() {
            return Self {
                allowed: true,
                quarantine: Some(qinfo.clone()),
                notarization: notarization.clone(),
                code_signed: cs_info.present,
                chains_to_apple: false,
                trust_cache_hit: false,
                reason: "allowed: user previously approved".to_string(),
            };
        }

        // Not code-signed → deny
        if !cs_info.present {
            return Self {
                allowed: false,
                quarantine: Some(qinfo.clone()),
                notarization: notarization.clone(),
                code_signed: false,
                chains_to_apple: false,
                trust_cache_hit: false,
                reason: "denied: quarantined binary has no code signature".to_string(),
            };
        }

        // Notarized → allow
        if notarization.status == NotarizationStatus::Stapled {
            return Self {
                allowed: true,
                quarantine: Some(qinfo.clone()),
                notarization: notarization.clone(),
                code_signed: true,
                chains_to_apple: cs_info.has_cms_signature,
                trust_cache_hit: false,
                reason: "allowed: quarantined binary is notarized".to_string(),
            };
        }

        // Developer ID signed (has CMS, chains to Apple) → allow with warning
        if cs_info.has_cms_signature && !cs_info.adhoc {
            return Self {
                allowed: true,
                quarantine: Some(qinfo.clone()),
                notarization: notarization.clone(),
                code_signed: true,
                chains_to_apple: true,
                trust_cache_hit: false,
                reason: "allowed: quarantined binary is Developer ID signed (not notarized)"
                    .to_string(),
            };
        }

        // Ad-hoc or unsigned → deny
        Self {
            allowed: false,
            quarantine: Some(qinfo.clone()),
            notarization: notarization.clone(),
            code_signed: cs_info.present,
            chains_to_apple: false,
            trust_cache_hit: false,
            reason: format!(
                "denied: quarantined binary is {} — Gatekeeper would block",
                if cs_info.adhoc {
                    "ad-hoc signed"
                } else {
                    "not properly signed"
                }
            ),
        }
    }
}

// ── Xattr reading ──────────────────────────────────────────────────

/// Read the com.apple.quarantine xattr from a file.
///
/// On macOS, uses the `getxattr` syscall. On other platforms, returns None.
pub fn read_quarantine_xattr(path: &Path) -> Option<QuarantineInfo> {
    #[cfg(target_os = "macos")]
    {
        read_quarantine_xattr_macos(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        None
    }
}

#[cfg(target_os = "macos")]
fn read_quarantine_xattr_macos(path: &Path) -> Option<QuarantineInfo> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let xattr_name = CString::new("com.apple.quarantine").ok()?;

    // First call to get the size
    let size = unsafe {
        libc::getxattr(
            c_path.as_ptr(),
            xattr_name.as_ptr(),
            std::ptr::null_mut(),
            0,
            0,
            0,
        )
    };

    let size = usize::try_from(size).ok().filter(|&s| s > 0)?;
    let mut buf = vec![0u8; size];

    let read = unsafe {
        libc::getxattr(
            c_path.as_ptr(),
            xattr_name.as_ptr(),
            buf.as_mut_ptr().cast(),
            size,
            0,
            0,
        )
    };

    let read = usize::try_from(read).ok().filter(|&r| r > 0)?;

    let value = std::str::from_utf8(&buf[..read]).ok()?;
    QuarantineInfo::parse(value)
}

// ── RealQuarantinePolicy ───────────────────────────────────────────

/// Quarantine policy that reads the real xattr on macOS.
///
/// Combines xattr reading with notarization/code signature checks
/// for a complete Gatekeeper-like assessment.
pub struct RealQuarantinePolicy {
    /// Whether to require notarization for quarantined binaries.
    require_notarization: bool,
}

impl RealQuarantinePolicy {
    /// Create with default settings (matches macOS 10.15+ behavior).
    pub fn new() -> Self {
        Self {
            require_notarization: false,
        }
    }

    /// Create requiring notarization (stricter, matches macOS 13+ default).
    pub fn strict() -> Self {
        Self {
            require_notarization: true,
        }
    }

    /// Perform enriched quarantine check with code signature info.
    pub fn check_with_cs_info(
        &self,
        binary_path: &Path,
        cs_info: &CodeSignatureInfo,
    ) -> (SecurityVerdict, GatekeeperAssessment) {
        let qinfo = read_quarantine_xattr(binary_path);
        let notarization = if qinfo.as_ref().is_some_and(QuarantineInfo::is_notarized_stapled) {
            NotarizationResult::stapled(&cs_info.cd_hash)
        } else {
            NotarizationResult::not_notarized()
        };

        let assessment =
            GatekeeperAssessment::assess(qinfo.as_ref(), cs_info, &notarization, false);

        let verdict = if assessment.allowed {
            if qinfo.is_some() && !assessment.trust_cache_hit {
                SecurityVerdict::AllowWithWarning {
                    warning: assessment.reason.clone(),
                }
            } else {
                SecurityVerdict::Allow
            }
        } else {
            SecurityVerdict::Deny {
                reason: assessment.reason.clone(),
            }
        };

        (verdict, assessment)
    }
}

impl Default for RealQuarantinePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityGate for RealQuarantinePolicy {
    fn name(&self) -> &str {
        "Quarantine (real)"
    }
    fn is_enforcing(&self) -> bool {
        true
    }
}

impl QuarantinePolicy for RealQuarantinePolicy {
    fn check_quarantine(&self, binary_path: &Path) -> SecurityVerdict {
        // Without CodeSignatureInfo, we can only check the xattr
        let qinfo = read_quarantine_xattr(binary_path);
        match qinfo {
            None => SecurityVerdict::Allow,
            Some(q) if q.is_user_approved() => SecurityVerdict::Allow,
            Some(q) if q.is_notarized_stapled() => SecurityVerdict::AllowWithWarning {
                warning: "quarantined binary has stapled notarization ticket".to_string(),
            },
            Some(_) => {
                if self.require_notarization {
                    SecurityVerdict::Deny {
                        reason: "quarantined binary: notarization required but not found"
                            .to_string(),
                    }
                } else {
                    SecurityVerdict::AllowWithWarning {
                        warning: "quarantined binary: no notarization ticket".to_string(),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_quarantine_xattr_safari() {
        let raw = "0083;5f3be4c0;Safari;12345678-1234-1234-1234-123456789012";
        let info = QuarantineInfo::parse(raw).unwrap();
        assert_eq!(info.flags, QuarantineFlags::FROM_WEB | QuarantineFlags::LAUNCH_SERVICES | QuarantineFlags::USER_APPROVED);
        assert_eq!(info.timestamp, "5f3be4c0");
        assert_eq!(info.agent_name, "Safari");
        assert_eq!(info.uuid, "12345678-1234-1234-1234-123456789012");
        assert_eq!(info.raw_value, raw);
    }

    #[test]
    fn parse_quarantine_xattr_curl() {
        let raw = "0002;64a2b000;curl;";
        let info = QuarantineInfo::parse(raw).unwrap();
        assert_eq!(info.flags, QuarantineFlags::FROM_WEB);
        assert_eq!(info.agent_name, "curl");
        assert!(info.uuid.is_empty());
    }

    #[test]
    fn parse_quarantine_xattr_invalid() {
        assert!(QuarantineInfo::parse("").is_none());
        assert!(QuarantineInfo::parse("x").is_none());
    }

    #[test]
    fn quarantine_flags_user_approved() {
        let info = QuarantineInfo::parse("0001;0;agent;uuid").unwrap();
        assert!(info.is_user_approved());
        assert!(!info.is_from_web());
    }

    #[test]
    fn quarantine_flags_notarized_stapled() {
        let info = QuarantineInfo::parse("0040;0;agent;uuid").unwrap();
        assert!(info.is_notarized_stapled());
        assert!(!info.is_user_approved());
    }

    #[test]
    fn quarantine_flags_from_web() {
        let info = QuarantineInfo::parse("0002;0;agent;uuid").unwrap();
        assert!(info.is_from_web());
        assert!(!info.is_scanned());
    }

    #[test]
    fn notarization_result_stapled() {
        let r = NotarizationResult::stapled("abcd1234");
        assert_eq!(r.status, NotarizationStatus::Stapled);
        assert_eq!(r.cd_hash, "abcd1234");
        assert!(r.ticket_found);
        assert!(r.summary.contains("notarized"));
    }

    #[test]
    fn notarization_result_not_notarized() {
        let r = NotarizationResult::not_notarized();
        assert_eq!(r.status, NotarizationStatus::NotNotarized);
        assert!(r.cd_hash.is_empty());
        assert!(!r.ticket_found);
    }

    fn make_cs_info(present: bool, adhoc: bool, has_cms: bool) -> CodeSignatureInfo {
        CodeSignatureInfo {
            present,
            adhoc,
            has_cms_signature: has_cms,
            cd_hash: "deadbeef".to_string(),
            ..Default::default()
        }
    }

    fn make_quarantine(flags: u32) -> QuarantineInfo {
        QuarantineInfo {
            flags: QuarantineFlags::from_bits_truncate(flags),
            timestamp: "0".to_string(),
            agent_name: "test".to_string(),
            uuid: "uuid".to_string(),
            raw_value: format!("{flags:04x};0;test;uuid"),
        }
    }

    #[test]
    fn gatekeeper_no_quarantine_allows() {
        let cs = make_cs_info(true, false, true);
        let n = NotarizationResult::not_notarized();
        let a = GatekeeperAssessment::assess(None, &cs, &n, false);
        assert!(a.allowed);
        assert!(a.reason.contains("no quarantine xattr"));
    }

    #[test]
    fn gatekeeper_user_approved_allows() {
        let q = make_quarantine(0x0003); // USER_APPROVED | FROM_WEB
        let cs = make_cs_info(true, false, true);
        let n = NotarizationResult::not_notarized();
        let a = GatekeeperAssessment::assess(Some(&q), &cs, &n, false);
        assert!(a.allowed);
        assert!(a.reason.contains("user previously approved"));
    }

    #[test]
    fn gatekeeper_unsigned_denies() {
        let q = make_quarantine(0x0002); // FROM_WEB only
        let cs = make_cs_info(false, false, false);
        let n = NotarizationResult::not_notarized();
        let a = GatekeeperAssessment::assess(Some(&q), &cs, &n, false);
        assert!(!a.allowed);
        assert!(a.reason.contains("no code signature"));
    }

    #[test]
    fn gatekeeper_notarized_allows() {
        let q = make_quarantine(0x0002); // FROM_WEB
        let cs = make_cs_info(true, false, true);
        let n = NotarizationResult::stapled("deadbeef");
        let a = GatekeeperAssessment::assess(Some(&q), &cs, &n, false);
        assert!(a.allowed);
        assert!(a.reason.contains("notarized"));
    }

    #[test]
    fn gatekeeper_developer_id_allows() {
        let q = make_quarantine(0x0002); // FROM_WEB
        let cs = make_cs_info(true, false, true); // has CMS, not adhoc
        let n = NotarizationResult::not_notarized();
        let a = GatekeeperAssessment::assess(Some(&q), &cs, &n, false);
        assert!(a.allowed);
        assert!(a.reason.contains("Developer ID signed"));
    }

    #[test]
    fn gatekeeper_adhoc_denies() {
        let q = make_quarantine(0x0002); // FROM_WEB
        let cs = make_cs_info(true, true, false); // adhoc, no CMS
        let n = NotarizationResult::not_notarized();
        let a = GatekeeperAssessment::assess(Some(&q), &cs, &n, false);
        assert!(!a.allowed);
        assert!(a.reason.contains("ad-hoc signed"));
    }

    #[test]
    fn gatekeeper_trust_cache_allows() {
        let q = make_quarantine(0x0002);
        let cs = make_cs_info(false, false, false); // no signature at all
        let n = NotarizationResult::not_notarized();
        let a = GatekeeperAssessment::assess(Some(&q), &cs, &n, true);
        assert!(a.allowed);
        assert!(a.trust_cache_hit);
        assert!(a.reason.contains("trust cache"));
    }

    #[test]
    fn real_quarantine_no_xattr_allows() {
        let policy = RealQuarantinePolicy::new();
        let verdict = policy.check_quarantine(Path::new("/tmp/nonexistent_binary_path"));
        assert!(verdict.is_allow());
    }

    #[test]
    fn real_quarantine_trait_impl() {
        let policy = RealQuarantinePolicy::new();
        assert_eq!(policy.name(), "Quarantine (real)");
        assert!(policy.is_enforcing());
    }

    #[test]
    fn real_quarantine_strict_mode() {
        let policy = RealQuarantinePolicy::strict();
        assert!(policy.require_notarization);
        assert!(policy.is_enforcing());
    }
}
