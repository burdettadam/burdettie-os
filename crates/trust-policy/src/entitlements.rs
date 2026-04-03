use serde::{Deserialize, Serialize};
use std::path::Path;

/// A typed entitlement value parsed from the entitlements plist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntitlementValue {
    /// Boolean entitlement (most common).
    Bool(bool),
    /// String entitlement (e.g., app group identifiers).
    Str(String),
    /// Array of strings (e.g., keychain access groups).
    StringArray(Vec<String>),
    /// Numeric value.
    Number(i64),
}

impl EntitlementValue {
    /// Returns true if the value is Bool(true).
    pub fn is_true(&self) -> bool {
        matches!(self, Self::Bool(true))
    }

    /// Returns the boolean value, or false for non-boolean values.
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            _ => false,
        }
    }

    /// Returns the string value if this is a Str variant.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// A single parsed entitlement with its key and value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entitlement {
    pub key: String,
    pub value: EntitlementValue,
}

/// A complete set of entitlements for a binary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntitlementSet {
    entries: Vec<Entitlement>,
}

impl EntitlementSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from key/bool pairs (backward compatible with existing extraction).
    pub fn from_keys(keys: &[(String, bool)]) -> Self {
        Self {
            entries: keys
                .iter()
                .map(|(k, v)| Entitlement {
                    key: k.clone(),
                    value: EntitlementValue::Bool(*v),
                })
                .collect(),
        }
    }

    /// Parse entitlements from raw plist XML (lightweight scanner, no XML dep).
    /// Extends the existing key/bool scanner to also detect string values.
    pub fn from_plist_xml(xml: &str) -> Self {
        let mut entries = Vec::new();
        let key_open = "<key>";
        let key_close = "</key>";
        let mut search_from = 0;
        while let Some(start) = xml[search_from..].find(key_open) {
            let abs_start = search_from + start + key_open.len();
            let Some(end) = xml[abs_start..].find(key_close) else {
                break;
            };
            let abs_end = abs_start + end;
            let key = xml[abs_start..abs_end].trim().to_string();
            let after = xml[abs_end + key_close.len()..].trim_start();
            let value = if after.starts_with("<true/>") {
                EntitlementValue::Bool(true)
            } else if after.starts_with("<false/>") {
                EntitlementValue::Bool(false)
            } else if after.starts_with("<string>") {
                if let Some(s_end) = after.find("</string>") {
                    let s = after["<string>".len()..s_end].to_string();
                    EntitlementValue::Str(s)
                } else {
                    EntitlementValue::Bool(true)
                }
            } else if after.starts_with("<array>") {
                let arr_end_pos = after.find("</array>").unwrap_or(after.len());
                let arr_content = &after["<array>".len()..arr_end_pos];
                let mut strings = Vec::new();
                let mut arr_pos = 0;
                while let Some(s_start) = arr_content[arr_pos..].find("<string>") {
                    let val_start = arr_pos + s_start + "<string>".len();
                    if let Some(s_end) = arr_content[val_start..].find("</string>") {
                        strings.push(arr_content[val_start..val_start + s_end].to_string());
                        arr_pos = val_start + s_end + "</string>".len();
                    } else {
                        break;
                    }
                }
                EntitlementValue::StringArray(strings)
            } else {
                EntitlementValue::Bool(true)
            };
            entries.push(Entitlement { key, value });
            search_from = abs_end + key_close.len();
        }
        Self { entries }
    }

    /// Get a specific entitlement by key.
    pub fn get(&self, key: &str) -> Option<&EntitlementValue> {
        self.entries.iter().find(|e| e.key == key).map(|e| &e.value)
    }

    /// Check if a boolean entitlement is present and true.
    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some_and(EntitlementValue::is_true)
    }

    /// Return all entitlements as key/bool pairs (backward compat).
    pub fn as_key_bool_pairs(&self) -> Vec<(String, bool)> {
        self.entries
            .iter()
            .map(|e| (e.key.clone(), e.value.as_bool()))
            .collect()
    }

    /// Return all entries.
    pub fn entries(&self) -> &[Entitlement] {
        &self.entries
    }

    /// Number of entitlements.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are no entitlements.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Well-known Apple entitlement keys that affect runtime security behavior.
pub mod known {
    // ── Hardened runtime relaxations ──
    pub const ALLOW_JIT: &str = "com.apple.security.cs.allow-jit";
    pub const ALLOW_UNSIGNED_MEMORY: &str =
        "com.apple.security.cs.allow-unsigned-executable-memory";
    pub const ALLOW_DYLD_ENV: &str = "com.apple.security.cs.allow-dyld-environment-variables";
    pub const DISABLE_LIBRARY_VALIDATION: &str =
        "com.apple.security.cs.disable-library-validation";
    pub const DISABLE_EXECUTABLE_PAGE_PROTECTION: &str =
        "com.apple.security.cs.disable-executable-page-protection";
    pub const DEBUGGER: &str = "com.apple.security.cs.debugger";

    // ── Sandbox ──
    pub const APP_SANDBOX: &str = "com.apple.security.app-sandbox";
    pub const NETWORK_CLIENT: &str = "com.apple.security.network.client";
    pub const NETWORK_SERVER: &str = "com.apple.security.network.server";
    pub const FILES_USER_SELECTED_RO: &str = "com.apple.security.files.user-selected.read-only";
    pub const FILES_USER_SELECTED_RW: &str = "com.apple.security.files.user-selected.read-write";
    pub const FILES_DOWNLOADS_RO: &str = "com.apple.security.files.downloads.read-only";
    pub const FILES_DOWNLOADS_RW: &str = "com.apple.security.files.downloads.read-write";

    // ── Process ──
    pub const GET_TASK_ALLOW: &str = "com.apple.security.get-task-allow";
    pub const INHERIT: &str = "com.apple.security.inherit";

    // ── Keychain ──
    pub const KEYCHAIN_ACCESS_GROUPS: &str = "keychain-access-groups";
    pub const APPLICATION_GROUPS: &str = "com.apple.security.application-groups";

    // ── Device ──
    pub const DEVICE_CAMERA: &str = "com.apple.security.device.camera";
    pub const DEVICE_MICROPHONE: &str = "com.apple.security.device.microphone";
    pub const DEVICE_USB: &str = "com.apple.security.device.usb";
}

/// Category of entitlement effect on system security.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntitlementCategory {
    /// Relaxes hardened runtime restrictions.
    RuntimeRelaxation,
    /// Controls sandbox behavior.
    Sandbox,
    /// Affects process security (debugging, inheritance).
    Process,
    /// Grants resource access (keychain, device).
    ResourceAccess,
    /// Unknown/uncategorized entitlement.
    Unknown,
}

/// Categorize an entitlement key.
pub fn categorize_entitlement(key: &str) -> EntitlementCategory {
    if key.starts_with("com.apple.security.cs.") {
        EntitlementCategory::RuntimeRelaxation
    } else if key.starts_with("com.apple.security.app-sandbox")
        || key.starts_with("com.apple.security.network.")
        || key.starts_with("com.apple.security.files.")
    {
        EntitlementCategory::Sandbox
    } else if key == known::GET_TASK_ALLOW || key == known::INHERIT {
        EntitlementCategory::Process
    } else if key.starts_with("com.apple.security.device.")
        || key == known::KEYCHAIN_ACCESS_GROUPS
        || key == known::APPLICATION_GROUPS
    {
        EntitlementCategory::ResourceAccess
    } else {
        EntitlementCategory::Unknown
    }
}

/// Entitlement evaluator with full semantic understanding.
///
/// Goes beyond `AppleEntitlements` by:
/// - Understanding the full known entitlement catalog
/// - Supporting configurable warn/deny lists
/// - Categorizing entitlements by security impact
pub struct SemanticEntitlementEvaluator {
    /// Entitlement keys that should produce warnings.
    warn_on: Vec<String>,
    /// Entitlement keys that should cause denial.
    deny_on: Vec<String>,
    /// Whether to warn on all runtime relaxation entitlements.
    warn_all_relaxations: bool,
}

impl SemanticEntitlementEvaluator {
    pub fn new() -> Self {
        Self {
            warn_on: vec![
                known::DISABLE_LIBRARY_VALIDATION.to_string(),
                known::ALLOW_DYLD_ENV.to_string(),
                known::ALLOW_UNSIGNED_MEMORY.to_string(),
            ],
            warn_all_relaxations: false,
            deny_on: Vec::new(),
        }
    }

    /// Create a strict evaluator that warns on all runtime relaxations.
    pub fn strict() -> Self {
        Self {
            warn_on: Vec::new(),
            warn_all_relaxations: true,
            deny_on: Vec::new(),
        }
    }

    /// Add a key to the deny list.
    pub fn deny_key(mut self, key: impl Into<String>) -> Self {
        self.deny_on.push(key.into());
        self
    }

    /// Add a key to the warn list.
    pub fn warn_key(mut self, key: impl Into<String>) -> Self {
        self.warn_on.push(key.into());
        self
    }

    /// Evaluate an EntitlementSet directly (richer than evaluate_with_keys).
    pub fn evaluate_set(
        &self,
        _binary_path: &Path,
        ent_set: &EntitlementSet,
    ) -> crate::SecurityVerdict {
        for key in &self.deny_on {
            if ent_set.has(key) {
                return crate::SecurityVerdict::Deny {
                    reason: format!("entitlement '{key}' is denied by policy"),
                };
            }
        }

        let mut warnings = Vec::new();
        for key in &self.warn_on {
            if ent_set.has(key) {
                warnings.push(key.clone());
            }
        }

        if self.warn_all_relaxations {
            for e in ent_set.entries() {
                if e.value.is_true()
                    && categorize_entitlement(&e.key) == EntitlementCategory::RuntimeRelaxation
                    && !warnings.contains(&e.key)
                {
                    warnings.push(e.key.clone());
                }
            }
        }

        if warnings.is_empty() {
            crate::SecurityVerdict::Allow
        } else {
            crate::SecurityVerdict::AllowWithWarning {
                warning: format!("security-relevant entitlements: {}", warnings.join(", ")),
            }
        }
    }
}

impl Default for SemanticEntitlementEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::gates::SecurityGate for SemanticEntitlementEvaluator {
    fn name(&self) -> &str {
        "Entitlements (semantic)"
    }

    fn is_enforcing(&self) -> bool {
        true
    }
}

impl crate::gates::EntitlementEvaluator for SemanticEntitlementEvaluator {
    fn evaluate(&self, _binary_path: &Path) -> crate::SecurityVerdict {
        crate::SecurityVerdict::Allow
    }

    fn evaluate_with_keys(
        &self,
        binary_path: &Path,
        keys: &[(String, bool)],
    ) -> crate::SecurityVerdict {
        let ent_set = EntitlementSet::from_keys(keys);
        self.evaluate_set(binary_path, &ent_set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn entitlement_value_bool() {
        assert!(EntitlementValue::Bool(true).is_true());
        assert!(!EntitlementValue::Bool(false).is_true());
    }

    #[test]
    fn entitlement_value_str() {
        assert_eq!(EntitlementValue::Str("test".into()).as_str(), Some("test"));
        assert_eq!(EntitlementValue::Bool(true).as_str(), None);
    }

    #[test]
    fn entitlement_set_from_keys() {
        let keys = vec![
            ("key1".to_string(), true),
            ("key2".to_string(), false),
            ("key3".to_string(), true),
        ];
        let set = EntitlementSet::from_keys(&keys);
        assert_eq!(set.len(), 3);
        assert!(set.has("key1"));
        assert!(!set.has("key2"));
    }

    #[test]
    fn entitlement_set_has() {
        let keys = vec![
            ("present".to_string(), true),
            ("absent".to_string(), false),
        ];
        let set = EntitlementSet::from_keys(&keys);
        assert!(set.has("present"));
        assert!(!set.has("absent"));
        assert!(!set.has("missing"));
    }

    #[test]
    fn from_plist_xml_booleans() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.get-task-allow</key>
    <false/>
</dict>
</plist>"#;
        let set = EntitlementSet::from_plist_xml(xml);
        assert_eq!(set.len(), 3);
        assert!(set.has(known::APP_SANDBOX));
        assert!(set.has(known::NETWORK_CLIENT));
        assert!(!set.has(known::GET_TASK_ALLOW));
    }

    #[test]
    fn from_plist_xml_strings() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>com.apple.security.application-groups</key>
    <string>group.com.example.app</string>
</dict>
</plist>"#;
        let set = EntitlementSet::from_plist_xml(xml);
        assert_eq!(set.len(), 1);
        let val = set.get(known::APPLICATION_GROUPS).unwrap();
        assert_eq!(val.as_str(), Some("group.com.example.app"));
    }

    #[test]
    fn from_plist_xml_arrays() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>keychain-access-groups</key>
    <array>
        <string>TEAM123.com.example.app</string>
        <string>TEAM123.com.example.shared</string>
    </array>
</dict>
</plist>"#;
        let set = EntitlementSet::from_plist_xml(xml);
        assert_eq!(set.len(), 1);
        let val = set.get(known::KEYCHAIN_ACCESS_GROUPS).unwrap();
        assert_eq!(
            *val,
            EntitlementValue::StringArray(vec![
                "TEAM123.com.example.app".to_string(),
                "TEAM123.com.example.shared".to_string(),
            ])
        );
    }

    #[test]
    fn as_key_bool_pairs_compat() {
        let keys = vec![
            ("a".to_string(), true),
            ("b".to_string(), false),
            ("c".to_string(), true),
        ];
        let set = EntitlementSet::from_keys(&keys);
        let pairs = set.as_key_bool_pairs();
        assert_eq!(pairs, keys);
    }

    #[test]
    fn categorize_runtime_relaxation() {
        assert_eq!(
            categorize_entitlement(known::ALLOW_JIT),
            EntitlementCategory::RuntimeRelaxation
        );
    }

    #[test]
    fn categorize_sandbox() {
        assert_eq!(
            categorize_entitlement(known::APP_SANDBOX),
            EntitlementCategory::Sandbox
        );
    }

    #[test]
    fn categorize_process() {
        assert_eq!(
            categorize_entitlement(known::GET_TASK_ALLOW),
            EntitlementCategory::Process
        );
    }

    #[test]
    fn categorize_unknown() {
        assert_eq!(
            categorize_entitlement("com.example.custom"),
            EntitlementCategory::Unknown
        );
    }

    #[test]
    fn semantic_evaluator_warns_on_disable_lv() {
        let eval = SemanticEntitlementEvaluator::new();
        let keys = vec![(known::DISABLE_LIBRARY_VALIDATION.to_string(), true)];
        let set = EntitlementSet::from_keys(&keys);
        let verdict = eval.evaluate_set(Path::new("/bin/test"), &set);
        assert!(matches!(
            verdict,
            crate::SecurityVerdict::AllowWithWarning { .. }
        ));
    }

    #[test]
    fn semantic_evaluator_denies_configured() {
        let eval = SemanticEntitlementEvaluator::new().deny_key("foo");
        let keys = vec![("foo".to_string(), true)];
        let set = EntitlementSet::from_keys(&keys);
        let verdict = eval.evaluate_set(Path::new("/bin/test"), &set);
        assert!(verdict.is_deny());
    }

    #[test]
    fn semantic_strict_warns_all_relaxations() {
        let eval = SemanticEntitlementEvaluator::strict();
        let keys = vec![(known::ALLOW_JIT.to_string(), true)];
        let set = EntitlementSet::from_keys(&keys);
        let verdict = eval.evaluate_set(Path::new("/bin/test"), &set);
        assert!(matches!(
            verdict,
            crate::SecurityVerdict::AllowWithWarning { .. }
        ));
    }

    #[test]
    fn semantic_allows_clean() {
        let eval = SemanticEntitlementEvaluator::new();
        let set = EntitlementSet::new();
        let verdict = eval.evaluate_set(Path::new("/bin/test"), &set);
        assert!(matches!(verdict, crate::SecurityVerdict::Allow));
    }

    #[test]
    fn trait_evaluate_with_keys() {
        use crate::gates::EntitlementEvaluator;
        let eval = SemanticEntitlementEvaluator::new();
        let keys = vec![(known::DISABLE_LIBRARY_VALIDATION.to_string(), true)];
        let verdict = eval.evaluate_with_keys(Path::new("/bin/test"), &keys);
        assert!(verdict.is_allow());
        assert!(matches!(
            verdict,
            crate::SecurityVerdict::AllowWithWarning { .. }
        ));
    }
}
