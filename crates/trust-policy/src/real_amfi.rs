//! Real AMFI (Apple Mobile File Integrity) policy implementation.
//!
//! Models the behavior of the `AppleMobileFileIntegrity.kext` in XNU:
//!
//! 1. **CDHash verification** — computes SHA-256 of the raw CodeDirectory blob
//!    and compares to the stored `cd_hash` field. Any manipulation of the binary
//!    changes the CDHash and fails this check.
//!
//! 2. **Trust cache lookup** — checks the computed CDHash against a registry of
//!    known-good hashes (`TrustAnchorRegistry`). Matching the trust cache fast-paths
//!    the binary (Apple ships thousands of hashes for OS binaries in the static
//!    trust cache embedded in the kernel).
//!
//! 3. **CMS chain-to-Apple-Root** — if a CMS (PKCS#7) signature is present, verifies
//!    that it chains up through Apple's intermediate CAs to a known Apple Root CA.
//!    SHA-256 fingerprints of Apple Root CA, Apple Root CA G2, and Apple Root CA G3
//!    are hardcoded as trust anchors.
//!
//! 4. **Ad-hoc policy** — per Apple's macOS policy, ad-hoc signed binaries are
//!    allowed to execute by default. This is modeled with `allow_adhoc: bool`
//!    (default `true` matching macOS behavior). Disable for strict environments.
//!
//! 5. **Enforcement mode** — every check is mediated through an `EnforcementMode`:
//!    - `Promiscuous` → unconditional pass (equivalent to `amfi_get_out_of_my_way=1`)
//!    - `LogOnly` → checks run, failures warn but never block
//!    - `Enforcing` → failures block execution
//!
//! # Relationship to Apple's AMFI
//!
//! | Apple AMFI | burdettieOS RealAmfi |
//! |------------|---------------------------|
//! | Static trust cache (in kernel) | `TrustAnchorRegistry::is_cd_hash_trusted()` |
//! | Supplemental trust cache | loadable JSON via `TrustAnchorRegistry::load_file()` |
//! | CDHash computation | `compute_cd_hash()` via SHA-256 of CD blob |
//! | CMS signature chain-to-Apple | `cert::verify_cms_signature()` |
//! | `amfi_get_out_of_my_way=1` | `EnforcementMode::Promiscuous` |
//! | CS_ADHOC policy | `allow_adhoc` flag (default: `true`) |
//! | `amfi_skip_cs_enforcement` | `EnforcementMode::LogOnly` |
//! | Entitlement check | delegated to `EntitlementEvaluator` gate |

use std::path::Path;
use std::sync::Arc;

use sha2::Digest;

use crate::anchor::TrustAnchorRegistry;
use crate::cert::verify_cms_signature;
use crate::enforcement::EnforcementMode;
use crate::gates::{AmfiPolicy, QuarantinePolicy, SecurityGate, SipPolicy};
use crate::{CodeSignatureInfo, SecurityVerdict};

// ── CDHash computation ─────────────────────────────────────────────

/// Compute the CDHash (SHA-256 of the raw CodeDirectory blob).
///
/// The CDHash is the full SHA-256 hash (32 bytes) of the CodeDirectory
/// blob as it appears on disk in the __LINKEDIT segment.
/// Apple also stores a 20-byte truncated form for backward compat;
/// `verify_cd_hash` handles both lengths.
pub fn compute_cd_hash_sha256(cd_blob: &[u8]) -> Vec<u8> {
    sha2::Sha256::digest(cd_blob).to_vec()
}

/// Compute the CDHash as a lowercase hex string (64 hex chars).
pub fn compute_cd_hash_hex(cd_blob: &[u8]) -> String {
    compute_cd_hash_sha256(cd_blob)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Verify that the CDHash stored in `CodeSignatureInfo` matches the actual
/// SHA-256 hash of the raw CodeDirectory bytes.
///
/// This is the fundamental integrity check: if the binary was modified
/// after signing, the page hashes in the CD change, which changes the CD
/// blob, which changes the CDHash. Any tampering is immediately detectable.
pub fn verify_cd_hash(cs_info: &CodeSignatureInfo, cd_data: &[u8]) -> CdHashResult {
    if cd_data.is_empty() {
        return CdHashResult::NoCdData;
    }

    let computed = compute_cd_hash_hex(cd_data);
    let stored = cs_info.cd_hash.to_lowercase();

    if stored.is_empty() {
        return CdHashResult::NoStoredHash { computed };
    }

    // Apple stores a 20-byte truncated CDHash for backward compat;
    // our computed hash is 64 hex chars (32 bytes). Accept a prefix match
    // for the stored 40-hex (20-byte) truncated form.
    let matched = if stored.len() == 40 {
        computed.starts_with(&stored)
    } else {
        computed == stored
    };

    if matched {
        CdHashResult::Valid { hash: computed }
    } else {
        CdHashResult::Mismatch {
            computed,
            stored: stored.to_string(),
        }
    }
}

/// Result of CDHash verification.
#[derive(Debug, Clone)]
pub enum CdHashResult {
    /// CDHash matches — binary is integral.
    Valid { hash: String },
    /// CDHash mismatches — binary has been tampered with.
    Mismatch { computed: String, stored: String },
    /// No raw CodeDirectory data was provided; can't verify.
    NoCdData,
    /// No stored CDHash in the signature; verification skipped.
    NoStoredHash { computed: String },
}

impl CdHashResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid { .. } | Self::NoStoredHash { .. })
    }

    pub fn computed_hash(&self) -> Option<&str> {
        match self {
            Self::Valid { hash } | Self::NoStoredHash { computed: hash } => Some(hash),
            Self::Mismatch { computed, .. } => Some(computed),
            Self::NoCdData => None,
        }
    }
}

// ── RealAmfi ──────────────────────────────────────────────────────

/// Real AMFI gate — performs actual code integrity verification.
///
/// See module-level docs for the full description.
pub struct RealAmfi {
    /// How strictly to enforce failures.
    mode: EnforcementMode,

    /// Allow ad-hoc signed binaries (no CMS, CS_ADHOC flag set).
    ///
    /// macOS allows ad-hoc by default. Set to `false` for environments
    /// that require Apple-signed binaries.
    allow_adhoc: bool,

    /// Allow unsigned binaries (no LC_CODE_SIGNATURE at all).
    ///
    /// When `true`, unsigned binaries are allowed with a warning.
    /// When `false`, they are denied (or warned in LogOnly mode).
    allow_unsigned: bool,

    /// Trust anchor registry for CDHash/team ID lookups (optional).
    registry: Option<Arc<TrustAnchorRegistry>>,

    /// Whether to verify CMS signature chains to Apple Root CA.
    ///
    /// When `false`, CMS is not validated (faster). Ad-hoc binaries
    /// are not affected (they never have CMS).
    verify_cms_chain: bool,
}

impl RealAmfi {
    /// Return a builder for fine-grained control.
    pub fn builder() -> RealAmfiBuilder {
        RealAmfiBuilder::default()
    }

    /// Promiscuous mode — equivalent to `amfi_get_out_of_my_way=1`.
    ///
    /// All binaries pass unconditionally. No verification is performed.
    pub fn promiscuous() -> Self {
        Self {
            mode: EnforcementMode::Promiscuous,
            allow_adhoc: true,
            allow_unsigned: true,
            registry: None,
            verify_cms_chain: false,
        }
    }

    /// Log-only mode — verifies but never blocks.
    ///
    /// All binaries pass with warnings on failures.
    pub fn log_only() -> Self {
        Self {
            mode: EnforcementMode::LogOnly,
            allow_adhoc: true,
            allow_unsigned: true,
            registry: None,
            verify_cms_chain: true,
        }
    }

    /// Production mode — same as Apple macOS default.
    ///
    /// - Ad-hoc binaries: allowed (macOS policy)
    /// - Unsigned binaries: allowed with warning on non-SIP paths
    /// - CMS verification: required for binaries that have it
    pub fn production() -> Self {
        Self {
            mode: EnforcementMode::Enforcing,
            allow_adhoc: true,
            allow_unsigned: true, // macOS allows unsigned on non-SIP paths
            registry: None,
            verify_cms_chain: true,
        }
    }

    /// Strict mode — Apple-signed binaries only, no ad-hoc, no unsigned.
    pub fn strict() -> Self {
        Self {
            mode: EnforcementMode::Enforcing,
            allow_adhoc: false,
            allow_unsigned: false,
            registry: None,
            verify_cms_chain: true,
        }
    }

    /// Perform enriched AMFI evaluation with raw code signature data.
    ///
    /// This is the primary evaluation path when called from the loader pipeline
    /// with full code signature data available.
    pub fn evaluate_with_data(
        &self,
        cs_info: &CodeSignatureInfo,
        cms_data: Option<&[u8]>,
        cd_data: Option<&[u8]>,
    ) -> AmfiEvaluation {
        // Short-circuit in promiscuous mode
        if self.mode == EnforcementMode::Promiscuous {
            return AmfiEvaluation::allowed("promiscuous mode — all binaries pass");
        }

        // ── Phase 1: Presence check ──────────────────────────────

        if !cs_info.present {
            let verdict = if self.allow_unsigned {
                SecurityVerdict::AllowWithWarning {
                    warning: "binary has no code signature".to_string(),
                }
            } else {
                SecurityVerdict::Deny {
                    reason: "binary has no code signature (unsigned binaries blocked)".to_string(),
                }
            };
            return AmfiEvaluation {
                verdict: self.mode.apply(verdict),
                cd_hash_result: CdHashResult::NoCdData,
                trust_cache_hit: false,
                cms_chains_to_apple: false,
                adhoc: false,
                summary: "unsigned binary".to_string(),
            };
        }

        // ── Phase 2: Ad-hoc policy ───────────────────────────────

        if cs_info.adhoc {
            // Trust cache check for ad-hoc binaries — Apple ships OS binaries as
            // ad-hoc signed with their CDHashes in the static trust cache.
            if let Some(reg) = &self.registry {
                if reg.is_cd_hash_trusted(&cs_info.cd_hash) {
                    return AmfiEvaluation {
                        verdict: SecurityVerdict::Allow,
                        cd_hash_result: CdHashResult::Valid {
                            hash: cs_info.cd_hash.clone(),
                        },
                        trust_cache_hit: true,
                        cms_chains_to_apple: false,
                        adhoc: true,
                        summary: format!(
                            "ad-hoc binary trusted via CDHash trust cache ({})",
                            short_hash(&cs_info.cd_hash)
                        ),
                    };
                }
            }

            let verdict = if self.allow_adhoc {
                SecurityVerdict::AllowWithWarning {
                    warning: format!(
                        "ad-hoc signed binary (CS_ADHOC, CDHash: {})",
                        short_hash(&cs_info.cd_hash)
                    ),
                }
            } else {
                SecurityVerdict::Deny {
                    reason: "ad-hoc signed binaries are not permitted".to_string(),
                }
            };
            return AmfiEvaluation {
                verdict: self.mode.apply(verdict),
                cd_hash_result: CdHashResult::NoStoredHash {
                    computed: cs_info.cd_hash.clone(),
                },
                trust_cache_hit: false,
                cms_chains_to_apple: false,
                adhoc: true,
                summary: "ad-hoc signed binary".to_string(),
            };
        }

        // ── Phase 3: CDHash verification ─────────────────────────

        let cd_hash_result = if let Some(cd) = cd_data {
            verify_cd_hash(cs_info, cd)
        } else {
            CdHashResult::NoCdData
        };

        // Trust cache lookup by computed CDHash
        let trust_cache_hit = self.registry.as_ref().map_or(false, |reg| {
            let hash = cd_hash_result
                .computed_hash()
                .unwrap_or(cs_info.cd_hash.as_str());
            reg.is_cd_hash_trusted(hash)
        });

        if trust_cache_hit {
            return AmfiEvaluation {
                verdict: SecurityVerdict::Allow,
                trust_cache_hit: true,
                cms_chains_to_apple: false,
                adhoc: false,
                summary: "trusted via CDHash trust cache".to_string(),
                cd_hash_result,
            };
        }

        // CDHash mismatch is always a hard error — binary has been tampered with
        if let CdHashResult::Mismatch {
            ref computed,
            ref stored,
        } = cd_hash_result
        {
            let verdict = self.mode.apply(SecurityVerdict::Deny {
                reason: format!(
                    "CDHash mismatch: stored={}, computed={}",
                    short_hash(stored),
                    short_hash(computed)
                ),
            });
            return AmfiEvaluation {
                verdict,
                cd_hash_result,
                trust_cache_hit: false,
                cms_chains_to_apple: false,
                adhoc: false,
                summary: "CDHash mismatch — binary tampered".to_string(),
            };
        }

        // ── Phase 4: CMS chain-to-Apple-Root ────────────────────

        let mut cms_chains_to_apple = false;
        let mut cms_summary = "no CMS data".to_string();

        if self.verify_cms_chain {
            if let Some(cms) = cms_data {
                let cms_result = verify_cms_signature(cms, cd_data.unwrap_or(&[]));
                cms_chains_to_apple = cms_result.chains_to_apple_root;
                cms_summary = cms_result.summary.clone();

                if !cms_result.structure_valid {
                    let verdict = self.mode.apply(SecurityVerdict::Deny {
                        reason: format!(
                            "CMS signature is structurally invalid: {}",
                            cms_result.summary
                        ),
                    });
                    return AmfiEvaluation {
                        verdict,
                        cd_hash_result,
                        trust_cache_hit: false,
                        cms_chains_to_apple: false,
                        adhoc: false,
                        summary: cms_summary,
                    };
                }

                if !cms_chains_to_apple {
                    log::debug!(
                        "RealAmfi: CMS signature does not chain to Apple Root CA: {}",
                        cms_result.summary
                    );

                    // Check if signer team ID / identity is in our trust registry
                    let team_trusted = self.registry.as_ref().map_or(false, |reg| {
                        reg.is_team_id_trusted(&cs_info.team_id)
                            || reg.is_identity_trusted(&cs_info.identity)
                    });

                    if !team_trusted && !cs_info.team_id.is_empty() {
                        let verdict = SecurityVerdict::AllowWithWarning {
                            warning: format!(
                                "CMS does not chain to Apple Root CA; team={} — {}",
                                cs_info.team_id, cms_result.summary
                            ),
                        };
                        return AmfiEvaluation {
                            verdict: self.mode.apply_warning(verdict),
                            cd_hash_result,
                            trust_cache_hit: false,
                            cms_chains_to_apple: false,
                            adhoc: false,
                            summary: cms_result.summary,
                        };
                    }
                }
            } else if cs_info.has_cms_signature {
                log::warn!(
                    "RealAmfi: has_cms_signature=true but cms_data is None for {:?}",
                    cs_info.identity
                );
            }
        }

        // ── Phase 5: Verdict ─────────────────────────────────────

        let final_verdict = if cms_chains_to_apple || trust_cache_hit {
            SecurityVerdict::Allow
        } else if cs_info.has_cms_signature {
            SecurityVerdict::AllowWithWarning {
                warning: format!("CMS present but chain-to-Apple not confirmed: {cms_summary}"),
            }
        } else {
            SecurityVerdict::AllowWithWarning {
                warning: "code signature present without CMS (linker-signed or corrupted)"
                    .to_string(),
            }
        };

        AmfiEvaluation {
            verdict: self.mode.apply_warning(final_verdict),
            cd_hash_result,
            trust_cache_hit,
            cms_chains_to_apple,
            adhoc: false,
            summary: cms_summary,
        }
    }
}

/// Truncate a hex hash for display (first 20 chars).
fn short_hash(hash: &str) -> &str {
    &hash[..20.min(hash.len())]
}

/// Detailed result of a `RealAmfi::evaluate_with_data()` call.
#[derive(Debug, Clone)]
pub struct AmfiEvaluation {
    /// The final security verdict (after applying enforcement mode).
    pub verdict: SecurityVerdict,
    /// CDHash verification result.
    pub cd_hash_result: CdHashResult,
    /// Whether the CDHash was found in the trust cache (fast-path).
    pub trust_cache_hit: bool,
    /// Whether the CMS signature chains to Apple Root CA.
    pub cms_chains_to_apple: bool,
    /// Whether the binary is ad-hoc signed.
    pub adhoc: bool,
    /// Human-readable evaluation summary.
    pub summary: String,
}

impl AmfiEvaluation {
    fn allowed(summary: &str) -> Self {
        Self {
            verdict: SecurityVerdict::Allow,
            cd_hash_result: CdHashResult::NoCdData,
            trust_cache_hit: false,
            cms_chains_to_apple: false,
            adhoc: false,
            summary: summary.to_string(),
        }
    }
}

// ── Builder ────────────────────────────────────────────────────────

/// Builder for `RealAmfi`.
#[derive(Default)]
pub struct RealAmfiBuilder {
    mode: Option<EnforcementMode>,
    allow_adhoc: Option<bool>,
    allow_unsigned: Option<bool>,
    registry: Option<Arc<TrustAnchorRegistry>>,
    verify_cms_chain: Option<bool>,
}

impl RealAmfiBuilder {
    /// Set the enforcement mode.
    pub fn mode(mut self, mode: EnforcementMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Set whether ad-hoc signed binaries are allowed.
    pub fn allow_adhoc(mut self, allow: bool) -> Self {
        self.allow_adhoc = Some(allow);
        self
    }

    /// Set whether unsigned binaries (no LC_CODE_SIGNATURE) are allowed.
    pub fn allow_unsigned(mut self, allow: bool) -> Self {
        self.allow_unsigned = Some(allow);
        self
    }

    /// Set the trust anchor registry for CDHash/team ID lookups.
    pub fn trust_registry(mut self, registry: Arc<TrustAnchorRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Set whether CMS chain-to-Apple-Root should be verified.
    pub fn verify_cms_chain(mut self, verify: bool) -> Self {
        self.verify_cms_chain = Some(verify);
        self
    }

    /// Build the `RealAmfi` gate.
    pub fn build(self) -> RealAmfi {
        RealAmfi {
            mode: self.mode.unwrap_or(EnforcementMode::Enforcing),
            allow_adhoc: self.allow_adhoc.unwrap_or(true),
            allow_unsigned: self.allow_unsigned.unwrap_or(true),
            registry: self.registry,
            verify_cms_chain: self.verify_cms_chain.unwrap_or(true),
        }
    }
}

// ── SecurityGate + AmfiPolicy impls ───────────────────────────────

impl SecurityGate for RealAmfi {
    fn name(&self) -> &str {
        match self.mode {
            EnforcementMode::Promiscuous => "AMFI (promiscuous)",
            EnforcementMode::LogOnly => "AMFI (log-only)",
            EnforcementMode::Enforcing => "AMFI (enforcing)",
        }
    }

    fn is_enforcing(&self) -> bool {
        self.mode.is_enforcing()
    }
}

impl AmfiPolicy for RealAmfi {
    /// Lightweight check using only the path-level code signature info.
    ///
    /// For full verification (CDHash, CMS), use `evaluate_with_data()` directly.
    fn check_code_integrity(
        &self,
        _binary_path: &Path,
        has_code_signature: bool,
    ) -> SecurityVerdict {
        if self.mode == EnforcementMode::Promiscuous {
            return SecurityVerdict::Allow;
        }

        if !has_code_signature {
            let verdict = if self.allow_unsigned {
                SecurityVerdict::AllowWithWarning {
                    warning: "binary has no code signature".to_string(),
                }
            } else {
                SecurityVerdict::Deny {
                    reason: "binary has no code signature (unsigned binaries blocked)".to_string(),
                }
            };
            return self.mode.apply(verdict);
        }

        // Presence confirmed; full verification requires evaluate_with_data()
        SecurityVerdict::Allow
    }
}

// ── Real Quarantine gate ───────────────────────────────────────────

/// Real quarantine xattr reader gate.
///
/// Reads `com.apple.quarantine` extended attribute from the binary and
/// evaluates it according to Gatekeeper policy.
pub struct RealQuarantine {
    mode: EnforcementMode,
    block_unapproved: bool,
}

impl RealQuarantine {
    /// Production quarantine: block unapproved downloads.
    pub fn production() -> Self {
        Self {
            mode: EnforcementMode::Enforcing,
            block_unapproved: true,
        }
    }

    /// Permissive quarantine: log only.
    pub fn log_only() -> Self {
        Self {
            mode: EnforcementMode::LogOnly,
            block_unapproved: false,
        }
    }

    /// Promiscuous: skip quarantine check entirely.
    pub fn promiscuous() -> Self {
        Self {
            mode: EnforcementMode::Promiscuous,
            block_unapproved: false,
        }
    }

    /// Read the quarantine xattr from a path using `libc::getxattr`.
    ///
    /// Returns `None` if the xattr is absent, `Some(raw_string)` if found.
    fn read_quarantine_xattr(path: &Path) -> Option<String> {
        Self::read_quarantine_xattr_impl(path)
    }

    #[cfg(target_os = "macos")]
    fn read_quarantine_xattr_impl(path: &Path) -> Option<String> {
        use std::ffi::CString;
        let c_path = CString::new(path.to_str()?).ok()?;
        // SAFETY: attribute name is a valid null-terminated C string literal.
        let attr_name = b"com.apple.quarantine\0".as_ptr() as *const libc::c_char;

        // First call: get the size
        let size =
            unsafe { libc::getxattr(c_path.as_ptr(), attr_name, std::ptr::null_mut(), 0, 0, 0) };
        if size <= 0 {
            return None;
        }

        // Second call: read the data
        let mut buf = vec![0u8; size as usize];
        let read = unsafe {
            libc::getxattr(
                c_path.as_ptr(),
                attr_name,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
                0,
                0,
            )
        };
        if read <= 0 {
            return None;
        }

        String::from_utf8(buf[..(read as usize)].to_vec()).ok()
    }

    #[cfg(target_os = "linux")]
    fn read_quarantine_xattr_impl(path: &Path) -> Option<String> {
        use std::ffi::CString;
        let c_path = CString::new(path.to_str()?).ok()?;
        // Linux uses "user." namespace for quarantine simulation
        let attr_name = b"user.quarantine\0".as_ptr() as *const libc::c_char;

        let size = unsafe { libc::getxattr(c_path.as_ptr(), attr_name, std::ptr::null_mut(), 0) };
        if size <= 0 {
            return None;
        }

        let mut buf = vec![0u8; size as usize];
        let read = unsafe {
            libc::getxattr(
                c_path.as_ptr(),
                attr_name,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
            )
        };
        if read <= 0 {
            return None;
        }

        String::from_utf8(buf[..(read as usize)].to_vec()).ok()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn read_quarantine_xattr_impl(_path: &Path) -> Option<String> {
        None
    }
}

impl SecurityGate for RealQuarantine {
    fn name(&self) -> &str {
        match self.mode {
            EnforcementMode::Promiscuous => "Quarantine (promiscuous)",
            EnforcementMode::LogOnly => "Quarantine (log-only)",
            EnforcementMode::Enforcing => "Quarantine (real)",
        }
    }

    fn is_enforcing(&self) -> bool {
        self.mode.is_enforcing()
    }
}

impl QuarantinePolicy for RealQuarantine {
    fn check_quarantine(&self, binary_path: &Path) -> SecurityVerdict {
        if self.mode == EnforcementMode::Promiscuous {
            return SecurityVerdict::Allow;
        }

        match Self::read_quarantine_xattr(binary_path) {
            None => {
                // No quarantine xattr — binary was not downloaded from the internet
                SecurityVerdict::Allow
            }
            Some(ref xattr) => match crate::quarantine::QuarantineInfo::parse(xattr) {
                None => self.mode.apply(SecurityVerdict::AllowWithWarning {
                    warning: format!("malformed quarantine xattr: {xattr:?}"),
                }),
                Some(qi) => {
                    if qi.is_user_approved() {
                        SecurityVerdict::AllowWithWarning {
                            warning: format!(
                                "quarantined binary (user-approved, agent={})",
                                qi.agent_name
                            ),
                        }
                    } else if self.block_unapproved {
                        self.mode.apply(SecurityVerdict::Deny {
                            reason: format!(
                                "quarantined binary not user-approved \
                                     (agent={}, flags=0x{:04x}). Remove quarantine: \
                                     xattr -d com.apple.quarantine <path>",
                                qi.agent_name,
                                qi.flags.bits()
                            ),
                        })
                    } else {
                        SecurityVerdict::AllowWithWarning {
                            warning: format!(
                                "quarantined binary (agent={}, flags=0x{:04x})",
                                qi.agent_name,
                                qi.flags.bits()
                            ),
                        }
                    }
                }
            },
        }
    }
}

// ── Real SIP gate ──────────────────────────────────────────────────

/// Real SIP (System Integrity Protection) gate.
///
/// Checks SIP-protected paths as Apple defines them on macOS.
/// On non-macOS targets, path checks still run but DYLD stripping
/// is based only on flags.
pub struct RealSip {
    mode: EnforcementMode,
}

impl RealSip {
    pub fn enforcing() -> Self {
        Self {
            mode: EnforcementMode::Enforcing,
        }
    }

    pub fn log_only() -> Self {
        Self {
            mode: EnforcementMode::LogOnly,
        }
    }

    pub fn promiscuous() -> Self {
        Self {
            mode: EnforcementMode::Promiscuous,
        }
    }
}

/// SIP-protected path prefixes (macOS source: AMFI + SIP policy).
const SIP_PROTECTED: &[&str] = &["/usr/", "/System/", "/bin/", "/sbin/", "/private/var/db/"];
/// Paths under SIP_PROTECTED that are explicitly user-writable.
const SIP_EXEMPT: &[&str] = &["/usr/local/", "/usr/share/"];

/// Whether a path is covered by SIP.
pub fn is_sip_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if SIP_EXEMPT.iter().any(|e| s.starts_with(e)) {
        return false;
    }
    SIP_PROTECTED.iter().any(|p| s.starts_with(p))
}

impl SecurityGate for RealSip {
    fn name(&self) -> &str {
        match self.mode {
            EnforcementMode::Promiscuous => "SIP (disabled)",
            EnforcementMode::LogOnly => "SIP (log-only)",
            EnforcementMode::Enforcing => "SIP",
        }
    }

    fn is_enforcing(&self) -> bool {
        self.mode.is_enforcing()
    }
}

impl SipPolicy for RealSip {
    fn is_path_protected(&self, path: &Path) -> bool {
        self.mode.is_active() && is_sip_path(path)
    }

    fn should_strip_dyld_env(
        &self,
        binary_path: &Path,
        _flags: macho_types::header::MachFlags,
    ) -> bool {
        if self.mode == EnforcementMode::Promiscuous {
            return false;
        }
        // Strip DYLD_* for SIP-protected binaries
        is_sip_path(binary_path)
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_cd_hash_sha256_basic() {
        let data = b"fake codedirectory blob";
        let hash = compute_cd_hash_hex(data);
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn cd_hash_mismatch_detected() {
        let cs_info = CodeSignatureInfo {
            present: true,
            cd_hash: "deadbeef".to_string(),
            ..Default::default()
        };
        let result = verify_cd_hash(&cs_info, b"some binary data");
        assert!(matches!(result, CdHashResult::Mismatch { .. }));
    }

    #[test]
    fn cd_hash_valid_full_hash() {
        let data = b"test codedirectory";
        let expected = compute_cd_hash_hex(data);
        let cs_info = CodeSignatureInfo {
            present: true,
            cd_hash: expected.clone(),
            ..Default::default()
        };
        let result = verify_cd_hash(&cs_info, data);
        assert!(result.is_valid());
        assert_eq!(result.computed_hash(), Some(expected.as_str()));
    }

    #[test]
    fn cd_hash_truncated_prefix_match() {
        let data = b"test codedirectory";
        let full_hash = compute_cd_hash_hex(data);
        // 20-byte (40 hex char) truncated form — matches Apple's stored format
        let truncated = full_hash[..40].to_string();
        let cs_info = CodeSignatureInfo {
            present: true,
            cd_hash: truncated,
            ..Default::default()
        };
        let result = verify_cd_hash(&cs_info, data);
        assert!(result.is_valid());
    }

    #[test]
    fn cd_hash_no_cd_data() {
        let cs_info = CodeSignatureInfo {
            present: true,
            cd_hash: "abc".to_string(),
            ..Default::default()
        };
        let result = verify_cd_hash(&cs_info, &[]);
        assert!(matches!(result, CdHashResult::NoCdData));
    }

    #[test]
    fn promiscuous_amfi_always_allows() {
        let amfi = RealAmfi::promiscuous();
        assert_eq!(
            amfi.check_code_integrity(Path::new("/bin/ls"), false),
            SecurityVerdict::Allow
        );
    }

    #[test]
    fn production_amfi_warns_unsigned() {
        let amfi = RealAmfi::production();
        let cs = CodeSignatureInfo {
            present: false,
            ..Default::default()
        };
        let eval = amfi.evaluate_with_data(&cs, None, None);
        assert!(matches!(
            eval.verdict,
            SecurityVerdict::AllowWithWarning { .. }
        ));
    }

    #[test]
    fn strict_amfi_denies_unsigned() {
        let amfi = RealAmfi::strict();
        let cs = CodeSignatureInfo {
            present: false,
            ..Default::default()
        };
        let eval = amfi.evaluate_with_data(&cs, None, None);
        assert!(matches!(eval.verdict, SecurityVerdict::Deny { .. }));
    }

    #[test]
    fn strict_amfi_denies_adhoc() {
        let amfi = RealAmfi::strict();
        let cs = CodeSignatureInfo {
            present: true,
            adhoc: true,
            cd_hash: "aabbcc".to_string(),
            ..Default::default()
        };
        let eval = amfi.evaluate_with_data(&cs, None, None);
        assert!(matches!(eval.verdict, SecurityVerdict::Deny { .. }));
    }

    #[test]
    fn production_amfi_allows_adhoc() {
        let amfi = RealAmfi::production();
        let cs = CodeSignatureInfo {
            present: true,
            adhoc: true,
            cd_hash: "aabbcc".to_string(),
            ..Default::default()
        };
        let eval = amfi.evaluate_with_data(&cs, None, None);
        assert!(!matches!(eval.verdict, SecurityVerdict::Deny { .. }));
        assert!(eval.adhoc);
    }

    #[test]
    fn log_only_amfi_warns_not_denies() {
        let amfi = RealAmfi::builder()
            .mode(EnforcementMode::LogOnly)
            .allow_unsigned(false)
            .build();
        let cs = CodeSignatureInfo {
            present: false,
            ..Default::default()
        };
        let eval = amfi.evaluate_with_data(&cs, None, None);
        // Even though allow_unsigned=false, LogOnly upgrades deny to warning
        assert!(matches!(
            eval.verdict,
            SecurityVerdict::AllowWithWarning { .. }
        ));
    }

    #[test]
    fn builder_creates_custom_amfi() {
        let amfi = RealAmfi::builder()
            .mode(EnforcementMode::Enforcing)
            .allow_adhoc(false)
            .allow_unsigned(false)
            .verify_cms_chain(false)
            .build();
        assert!(amfi.is_enforcing());
        assert!(!amfi.allow_adhoc);
        assert!(!amfi.allow_unsigned);
    }

    #[test]
    fn cd_hash_mismatch_blocked_in_enforcing() {
        let amfi = RealAmfi::builder()
            .mode(EnforcementMode::Enforcing)
            .verify_cms_chain(false)
            .build();
        let cs = CodeSignatureInfo {
            present: true,
            adhoc: false,
            cd_hash: "deadbeef".to_string(), // wrong hash
            has_cms_signature: false,
            ..Default::default()
        };
        let eval = amfi.evaluate_with_data(&cs, None, Some(b"some binary bytes"));
        assert!(matches!(eval.verdict, SecurityVerdict::Deny { .. }));
        assert!(matches!(eval.cd_hash_result, CdHashResult::Mismatch { .. }));
    }

    #[test]
    fn cd_hash_mismatch_warned_in_log_only() {
        let amfi = RealAmfi::builder()
            .mode(EnforcementMode::LogOnly)
            .verify_cms_chain(false)
            .build();
        let cs = CodeSignatureInfo {
            present: true,
            adhoc: false,
            cd_hash: "deadbeef".to_string(),
            has_cms_signature: false,
            ..Default::default()
        };
        let eval = amfi.evaluate_with_data(&cs, None, Some(b"some binary bytes"));
        assert!(matches!(
            eval.verdict,
            SecurityVerdict::AllowWithWarning { .. }
        ));
    }

    #[test]
    fn sip_path_detection() {
        assert!(is_sip_path(Path::new("/usr/bin/ls")));
        assert!(is_sip_path(Path::new("/System/Library/Frameworks")));
        assert!(is_sip_path(Path::new("/bin/sh")));
        assert!(!is_sip_path(Path::new("/usr/local/bin/custom")));
        assert!(!is_sip_path(Path::new("/Users/alice/Desktop/app")));
    }

    #[test]
    fn trust_cache_bypasses_adhoc_restriction() {
        let mut registry = TrustAnchorRegistry::new();
        registry.add_trusted_cd_hash("cafebabe1234");
        let amfi = RealAmfi::builder()
            .mode(EnforcementMode::Enforcing)
            .allow_adhoc(false) // strict: no ad-hoc
            .trust_registry(Arc::new(registry))
            .build();
        let cs = CodeSignatureInfo {
            present: true,
            adhoc: true,
            cd_hash: "cafebabe1234".to_string(), // in trust cache
            ..Default::default()
        };
        let eval = amfi.evaluate_with_data(&cs, None, None);
        // Trust cache hit → allow even though allow_adhoc=false
        assert_eq!(eval.verdict, SecurityVerdict::Allow);
        assert!(eval.trust_cache_hit);
    }

    #[test]
    fn real_sip_path_protected() {
        let sip = RealSip::enforcing();
        assert!(sip.is_path_protected(Path::new("/usr/bin/ls")));
        assert!(!sip.is_path_protected(Path::new("/usr/local/bin/custom")));
        assert!(!sip.is_path_protected(Path::new("/Users/alice/app")));
    }

    #[test]
    fn real_sip_promiscuous_never_enforces() {
        let sip = RealSip::promiscuous();
        assert!(!sip.is_path_protected(Path::new("/usr/bin/ls")));
        assert!(!sip.should_strip_dyld_env(
            Path::new("/usr/bin/ls"),
            macho_types::header::MachFlags::empty()
        ));
    }
}
