//! Trust anchor registry and AMFI trust cache.
//!
//! Provides a central registry of trusted CDHashes, team IDs, and signing
//! identities, plus security gate implementations backed by the registry.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;

use crate::gates::{AmfiPolicy, LibraryValidator, SecurityGate};
use crate::{CodeSignatureInfo, SecurityVerdict};

// ── TrustRule trait ─────────────────────────────────────────────────

/// A custom rule that can make trust decisions based on code signature info.
pub trait TrustRule: Send + Sync {
    /// Rule name for logging/diagnostics.
    fn name(&self) -> &str;

    /// Evaluate the rule. Returns `Some(verdict)` if the rule has an opinion,
    /// `None` to defer to other rules/gates.
    fn evaluate(&self, cs_info: &CodeSignatureInfo) -> Option<SecurityVerdict>;
}

// ── TrustAnchorRegistry ─────────────────────────────────────────────

/// Central registry of trusted entities and custom trust rules.
///
/// Used by security gates to make enriched trust decisions based on
/// CDHash trust caches, known team IDs, and custom rules.
#[derive(Default)]
pub struct TrustAnchorRegistry {
    /// CDHashes that are unconditionally trusted (like Apple's static trust cache).
    /// Hex-encoded lowercase strings.
    trusted_cd_hashes: HashSet<String>,

    /// Team IDs that are trusted for library validation bypass.
    trusted_team_ids: HashSet<String>,

    /// Signing identities that are trusted.
    trusted_identities: HashSet<String>,

    /// Custom trust rules evaluated in order.
    custom_rules: Vec<Box<dyn TrustRule>>,
}

#[derive(Deserialize)]
struct TrustAnchorFile {
    #[serde(default)]
    cd_hashes: Vec<String>,
    #[serde(default)]
    team_ids: Vec<String>,
    #[serde(default)]
    identities: Vec<String>,
}

impl TrustAnchorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // ── CDHash trust cache ──

    pub fn add_trusted_cd_hash(&mut self, hash: impl Into<String>) {
        self.trusted_cd_hashes.insert(hash.into().to_lowercase());
    }

    pub fn add_trusted_cd_hashes(&mut self, hashes: impl IntoIterator<Item = impl Into<String>>) {
        for h in hashes {
            self.add_trusted_cd_hash(h);
        }
    }

    pub fn is_cd_hash_trusted(&self, hash: &str) -> bool {
        self.trusted_cd_hashes.contains(&hash.to_lowercase())
    }

    pub fn trusted_cd_hash_count(&self) -> usize {
        self.trusted_cd_hashes.len()
    }

    // ── Team IDs ──

    pub fn add_trusted_team_id(&mut self, team_id: impl Into<String>) {
        self.trusted_team_ids.insert(team_id.into());
    }

    pub fn is_team_id_trusted(&self, team_id: &str) -> bool {
        self.trusted_team_ids.contains(team_id)
    }

    // ── Identities ──

    pub fn add_trusted_identity(&mut self, identity: impl Into<String>) {
        self.trusted_identities.insert(identity.into());
    }

    pub fn is_identity_trusted(&self, identity: &str) -> bool {
        self.trusted_identities.contains(identity)
    }

    // ── Custom rules ──

    pub fn add_rule(&mut self, rule: impl TrustRule + 'static) {
        self.custom_rules.push(Box::new(rule));
    }

    // ── Composite evaluation ──

    /// Evaluate all trust sources for the given code signature.
    ///
    /// Returns `Allow` if CDHash, team ID, or identity is in the trusted set,
    /// or if any custom rule returns a verdict. Returns `None` if no trust
    /// source has an opinion.
    pub fn evaluate(&self, cs_info: &CodeSignatureInfo) -> Option<SecurityVerdict> {
        // CDHash match
        if !cs_info.cd_hash.is_empty() && self.is_cd_hash_trusted(&cs_info.cd_hash) {
            return Some(SecurityVerdict::Allow);
        }

        // Team ID match
        if !cs_info.team_id.is_empty() && self.is_team_id_trusted(&cs_info.team_id) {
            return Some(SecurityVerdict::Allow);
        }

        // Identity match
        if !cs_info.identity.is_empty() && self.is_identity_trusted(&cs_info.identity) {
            return Some(SecurityVerdict::Allow);
        }

        // Custom rules (first opinion wins)
        for rule in &self.custom_rules {
            if let Some(verdict) = rule.evaluate(cs_info) {
                return Some(verdict);
            }
        }

        None
    }

    // ── Serialization helpers ──

    /// Load trusted CDHashes, team IDs, and identities from a JSON string.
    ///
    /// Expected format:
    /// ```json
    /// { "cd_hashes": ["abc123", ...], "team_ids": ["TEAM1", ...], "identities": ["com.example", ...] }
    /// ```
    pub fn load_from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let file: TrustAnchorFile = serde_json::from_str(json_str)?;
        let mut registry = Self::new();
        registry.add_trusted_cd_hashes(file.cd_hashes);
        for tid in file.team_ids {
            registry.add_trusted_team_id(tid);
        }
        for id in file.identities {
            registry.add_trusted_identity(id);
        }
        Ok(registry)
    }

    /// Merge another registry into this one (union of all sets, append rules).
    pub fn merge(&mut self, other: TrustAnchorRegistry) {
        self.trusted_cd_hashes.extend(other.trusted_cd_hashes);
        self.trusted_team_ids.extend(other.trusted_team_ids);
        self.trusted_identities.extend(other.trusted_identities);
        self.custom_rules.extend(other.custom_rules);
    }
}

// ── SIP path helper ─────────────────────────────────────────────────

fn is_sip_protected(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if s.starts_with("/usr/local/") {
        return false;
    }
    s.starts_with("/usr/")
        || s.starts_with("/System/")
        || s.starts_with("/bin/")
        || s.starts_with("/sbin/")
}

// ── TrustCacheAmfi ──────────────────────────────────────────────────

/// AMFI implementation that checks CDHash against the trust anchor registry.
/// Falls back to path-based heuristics (like `AppleAmfi`) when no CDHash match.
pub struct TrustCacheAmfi {
    registry: Arc<TrustAnchorRegistry>,
}

impl TrustCacheAmfi {
    pub fn new(registry: Arc<TrustAnchorRegistry>) -> Self {
        Self { registry }
    }

    /// Enriched check using parsed code signature info.
    pub fn check_with_cs_info(&self, cs_info: &CodeSignatureInfo) -> SecurityVerdict {
        // Check registry first
        if let Some(verdict) = self.registry.evaluate(cs_info) {
            return verdict;
        }
        // Fall back: if signed → Allow, else → AllowWithWarning
        if cs_info.present {
            SecurityVerdict::Allow
        } else {
            SecurityVerdict::AllowWithWarning {
                warning: "AMFI: no code signature and not in trust cache".to_string(),
            }
        }
    }
}

impl SecurityGate for TrustCacheAmfi {
    fn name(&self) -> &str {
        "AMFI (trust cache)"
    }

    fn is_enforcing(&self) -> bool {
        true
    }
}

impl AmfiPolicy for TrustCacheAmfi {
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

// ── TeamIdLibraryValidator ──────────────────────────────────────────

/// Library validator that checks team ID matching between loader and dylib.
/// Trusted team IDs from the registry bypass the matching requirement.
pub struct TeamIdLibraryValidator {
    registry: Arc<TrustAnchorRegistry>,
}

impl TeamIdLibraryValidator {
    pub fn new(registry: Arc<TrustAnchorRegistry>) -> Self {
        Self { registry }
    }

    /// Enriched validation using code signature info for both loader and dylib.
    pub fn validate_with_cs_info(
        &self,
        loader_cs: &CodeSignatureInfo,
        dylib_cs: &CodeSignatureInfo,
    ) -> SecurityVerdict {
        // If dylib's CDHash is in trust cache → Allow
        if self.registry.is_cd_hash_trusted(&dylib_cs.cd_hash) {
            return SecurityVerdict::Allow;
        }
        // If dylib's team ID is trusted → Allow
        if self.registry.is_team_id_trusted(&dylib_cs.team_id) {
            return SecurityVerdict::Allow;
        }
        // If loader requires library validation (CS_REQUIRE_LV), team IDs must match
        if loader_cs.library_validation {
            if dylib_cs.team_id.is_empty() {
                return SecurityVerdict::Deny {
                    reason: "library validation: dylib has no team ID".to_string(),
                };
            }
            if loader_cs.team_id != dylib_cs.team_id {
                return SecurityVerdict::Deny {
                    reason: format!(
                        "library validation: team ID mismatch (loader={}, dylib={})",
                        loader_cs.team_id, dylib_cs.team_id
                    ),
                };
            }
        }
        SecurityVerdict::Allow
    }
}

impl SecurityGate for TeamIdLibraryValidator {
    fn name(&self) -> &str {
        "Library Validator (team ID)"
    }

    fn is_enforcing(&self) -> bool {
        true
    }
}

impl LibraryValidator for TeamIdLibraryValidator {
    fn validate_dylib(&self, loader_path: &Path, dylib_path: &Path) -> SecurityVerdict {
        if is_sip_protected(loader_path) && !is_sip_protected(dylib_path) {
            SecurityVerdict::Deny {
                reason: "SIP-protected loader cannot load non-SIP dylib".to_string(),
            }
        } else {
            SecurityVerdict::Allow
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cs_info(
        cd_hash: &str,
        team_id: &str,
        identity: &str,
        present: bool,
    ) -> CodeSignatureInfo {
        CodeSignatureInfo {
            present,
            cd_hash: cd_hash.to_string(),
            team_id: team_id.to_string(),
            identity: identity.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn empty_registry_no_opinion() {
        let reg = TrustAnchorRegistry::new();
        let cs = make_cs_info("abc123", "TEAM1", "com.example", true);
        assert!(reg.evaluate(&cs).is_none());
    }

    #[test]
    fn cd_hash_trusted() {
        let mut reg = TrustAnchorRegistry::new();
        reg.add_trusted_cd_hash("abc123");
        let cs = make_cs_info("abc123", "", "", true);
        let verdict = reg.evaluate(&cs).unwrap();
        assert!(verdict.is_allow());
    }

    #[test]
    fn cd_hash_case_insensitive() {
        let mut reg = TrustAnchorRegistry::new();
        reg.add_trusted_cd_hash("ABCDEF");
        let cs = make_cs_info("abcdef", "", "", true);
        assert!(reg.evaluate(&cs).unwrap().is_allow());

        let cs2 = make_cs_info("AbCdEf", "", "", true);
        assert!(reg.evaluate(&cs2).unwrap().is_allow());
    }

    #[test]
    fn team_id_trusted() {
        let mut reg = TrustAnchorRegistry::new();
        reg.add_trusted_team_id("TEAM1");
        let cs = make_cs_info("", "TEAM1", "", true);
        assert!(reg.evaluate(&cs).unwrap().is_allow());
    }

    #[test]
    fn identity_trusted() {
        let mut reg = TrustAnchorRegistry::new();
        reg.add_trusted_identity("com.apple.finder");
        let cs = make_cs_info("", "", "com.apple.finder", true);
        assert!(reg.evaluate(&cs).unwrap().is_allow());
    }

    #[test]
    fn custom_rule_fires() {
        struct AllowSpecificIdentity;
        impl TrustRule for AllowSpecificIdentity {
            fn name(&self) -> &str {
                "allow-specific"
            }
            fn evaluate(&self, cs_info: &CodeSignatureInfo) -> Option<SecurityVerdict> {
                if cs_info.identity == "com.test.app" {
                    Some(SecurityVerdict::Allow)
                } else {
                    None
                }
            }
        }

        let mut reg = TrustAnchorRegistry::new();
        reg.add_rule(AllowSpecificIdentity);
        let cs = make_cs_info("", "", "com.test.app", true);
        assert!(reg.evaluate(&cs).unwrap().is_allow());

        let cs2 = make_cs_info("", "", "com.other.app", true);
        assert!(reg.evaluate(&cs2).is_none());
    }

    #[test]
    fn load_from_json() {
        let json = r#"{
            "cd_hashes": ["aaa111", "bbb222"],
            "team_ids": ["TEAM1", "TEAM2", "TEAM3"],
            "identities": ["com.example.app"]
        }"#;
        let reg = TrustAnchorRegistry::load_from_json(json).unwrap();
        assert_eq!(reg.trusted_cd_hash_count(), 2);
        assert!(reg.is_cd_hash_trusted("aaa111"));
        assert!(reg.is_cd_hash_trusted("bbb222"));
        assert!(reg.is_team_id_trusted("TEAM1"));
        assert!(reg.is_team_id_trusted("TEAM2"));
        assert!(reg.is_team_id_trusted("TEAM3"));
        assert!(reg.is_identity_trusted("com.example.app"));
    }

    #[test]
    fn merge_registries() {
        let mut reg1 = TrustAnchorRegistry::new();
        reg1.add_trusted_cd_hash("hash1");
        reg1.add_trusted_team_id("TEAM_A");

        let mut reg2 = TrustAnchorRegistry::new();
        reg2.add_trusted_cd_hash("hash2");
        reg2.add_trusted_team_id("TEAM_B");
        reg2.add_trusted_identity("com.b.app");

        reg1.merge(reg2);
        assert!(reg1.is_cd_hash_trusted("hash1"));
        assert!(reg1.is_cd_hash_trusted("hash2"));
        assert!(reg1.is_team_id_trusted("TEAM_A"));
        assert!(reg1.is_team_id_trusted("TEAM_B"));
        assert!(reg1.is_identity_trusted("com.b.app"));
    }

    #[test]
    fn trust_cache_amfi_trusted_hash() {
        let mut reg = TrustAnchorRegistry::new();
        reg.add_trusted_cd_hash("deadbeef");
        let amfi = TrustCacheAmfi::new(Arc::new(reg));
        let cs = make_cs_info("deadbeef", "", "", true);
        assert!(amfi.check_with_cs_info(&cs).is_allow());
    }

    #[test]
    fn trust_cache_amfi_unsigned() {
        let reg = TrustAnchorRegistry::new();
        let amfi = TrustCacheAmfi::new(Arc::new(reg));
        let cs = make_cs_info("", "", "", false);
        let verdict = amfi.check_with_cs_info(&cs);
        assert!(verdict.is_allow());
        assert!(matches!(verdict, SecurityVerdict::AllowWithWarning { .. }));
    }

    #[test]
    fn team_id_validator_matching() {
        let reg = TrustAnchorRegistry::new();
        let validator = TeamIdLibraryValidator::new(Arc::new(reg));
        let loader = CodeSignatureInfo {
            library_validation: true,
            team_id: "TEAM_X".to_string(),
            ..Default::default()
        };
        let dylib = CodeSignatureInfo {
            team_id: "TEAM_X".to_string(),
            ..Default::default()
        };
        assert!(validator.validate_with_cs_info(&loader, &dylib).is_allow());
    }

    #[test]
    fn team_id_validator_mismatch() {
        let reg = TrustAnchorRegistry::new();
        let validator = TeamIdLibraryValidator::new(Arc::new(reg));
        let loader = CodeSignatureInfo {
            library_validation: true,
            team_id: "TEAM_X".to_string(),
            ..Default::default()
        };
        let dylib = CodeSignatureInfo {
            team_id: "TEAM_Y".to_string(),
            ..Default::default()
        };
        assert!(validator.validate_with_cs_info(&loader, &dylib).is_deny());
    }

    #[test]
    fn team_id_validator_trusted_override() {
        let mut reg = TrustAnchorRegistry::new();
        reg.add_trusted_team_id("TEAM_Y");
        let validator = TeamIdLibraryValidator::new(Arc::new(reg));
        let loader = CodeSignatureInfo {
            library_validation: true,
            team_id: "TEAM_X".to_string(),
            ..Default::default()
        };
        let dylib = CodeSignatureInfo {
            team_id: "TEAM_Y".to_string(),
            ..Default::default()
        };
        // TEAM_Y is trusted, so mismatch is bypassed
        assert!(validator.validate_with_cs_info(&loader, &dylib).is_allow());
    }
}
