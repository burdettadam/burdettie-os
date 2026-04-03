//! Flag-based hardened runtime enforcement.
//!
//! Uses `CodeSignatureInfo` to determine hardened runtime status from
//! the actual CS_RUNTIME flag rather than path-based heuristics.

use std::path::Path;

use macho_types::header::MachFlags;
use serde::{Deserialize, Serialize};

use crate::gates::{HardenedRuntimePolicy, SecurityGate};
use crate::CodeSignatureInfo;

/// Computed effects of hardened runtime on the binary's execution environment.
/// These effects flow into other gates (SIP env stripping, library validation).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HardenedRuntimeEffects {
    /// Whether hardened runtime is active (CS_RUNTIME flag or SIP path).
    pub hardened: bool,
    /// Whether DYLD_INSERT_LIBRARIES is allowed.
    pub allows_dylib_insertion: bool,
    /// Whether DYLD_* environment variables should be stripped (CS_RESTRICT or hardened).
    pub strip_dyld_env: bool,
    /// Whether unsigned executable memory is allowed.
    pub allows_unsigned_memory: bool,
    /// Whether JIT is allowed (MAP_JIT).
    pub allows_jit: bool,
    /// Whether debugging is allowed (get-task-allow).
    pub allows_debugging: bool,
    /// Whether library validation is required.
    pub requires_library_validation: bool,
}

/// Hardened runtime policy that uses CodeSignatureInfo flags.
///
/// Falls back to path-based heuristics when CodeSignatureInfo is not available
/// (via the trait methods which don't receive cs_info).
pub struct FlagBasedHardenedRuntime {
    /// Whether to also consider SIP paths as hardened (Apple default: true).
    sip_paths_hardened: bool,
}

impl Default for FlagBasedHardenedRuntime {
    fn default() -> Self {
        Self {
            sip_paths_hardened: true,
        }
    }
}

impl FlagBasedHardenedRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with SIP-path heuristic disabled (only CS_RUNTIME flag matters).
    pub fn flags_only() -> Self {
        Self {
            sip_paths_hardened: false,
        }
    }

    /// Compute full hardened runtime effects from CodeSignatureInfo and entitlement keys.
    ///
    /// This is the main enriched evaluation method. It considers:
    /// - CS_RUNTIME flag → hardened runtime active
    /// - CS_RESTRICT flag → strip DYLD env vars
    /// - CS_REQUIRE_LV → library validation required
    /// - CS_GET_TASK_ALLOW → debugging allowed
    /// - Entitlements that modify hardened runtime behavior
    pub fn compute_effects(
        &self,
        cs_info: &CodeSignatureInfo,
        binary_path: &Path,
        entitlement_keys: &[(String, bool)],
    ) -> HardenedRuntimeEffects {
        let hardened =
            cs_info.runtime || (self.sip_paths_hardened && is_sip_protected(binary_path));

        let mut allows_dylib_insertion = !hardened;
        let mut strip_dyld_env = cs_info.restrict || (hardened && !cs_info.get_task_allow);
        let mut allows_unsigned_memory = !hardened;
        let mut allows_jit = !hardened;
        let allows_debugging = cs_info.get_task_allow;
        let requires_library_validation = cs_info.library_validation || hardened;

        // Entitlements can relax hardened runtime restrictions
        for (key, value) in entitlement_keys {
            if !value {
                continue;
            }
            match key.as_str() {
                "com.apple.security.cs.allow-dyld-environment-variables" => {
                    strip_dyld_env = false;
                    allows_dylib_insertion = true;
                }
                "com.apple.security.cs.allow-unsigned-executable-memory" => {
                    allows_unsigned_memory = true;
                }
                "com.apple.security.cs.allow-jit" => {
                    allows_jit = true;
                }
                _ => {}
            }
        }

        HardenedRuntimeEffects {
            hardened,
            allows_dylib_insertion,
            strip_dyld_env,
            allows_unsigned_memory,
            allows_jit,
            allows_debugging,
            requires_library_validation,
        }
    }
}

impl SecurityGate for FlagBasedHardenedRuntime {
    fn name(&self) -> &str {
        "Hardened Runtime (flag-based)"
    }

    fn is_enforcing(&self) -> bool {
        true
    }
}

impl HardenedRuntimePolicy for FlagBasedHardenedRuntime {
    fn is_hardened(&self, binary_path: &Path, _flags: MachFlags) -> bool {
        // Fallback: use SIP path heuristic if enabled
        self.sip_paths_hardened && is_sip_protected(binary_path)
    }

    fn allows_dylib_insertion(&self, binary_path: &Path, _flags: MachFlags) -> bool {
        // Fallback: hardened binaries don't allow insertion
        !(self.sip_paths_hardened && is_sip_protected(binary_path))
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cs_info(f: impl FnOnce(&mut CodeSignatureInfo)) -> CodeSignatureInfo {
        let mut info = CodeSignatureInfo::default();
        f(&mut info);
        info
    }

    #[test]
    fn hardened_from_cs_runtime_flag() {
        let gate = FlagBasedHardenedRuntime::new();
        let cs = make_cs_info(|i| i.runtime = true);
        let effects = gate.compute_effects(&cs, Path::new("/tmp/app"), &[]);
        assert!(effects.hardened);
    }

    #[test]
    fn not_hardened_without_flag() {
        let gate = FlagBasedHardenedRuntime::new();
        let cs = make_cs_info(|_| {});
        let effects = gate.compute_effects(&cs, Path::new("/tmp/app"), &[]);
        assert!(!effects.hardened);
    }

    #[test]
    fn sip_path_fallback_hardened() {
        let gate = FlagBasedHardenedRuntime::new();
        let cs = make_cs_info(|_| {});
        let effects = gate.compute_effects(&cs, Path::new("/usr/bin/ls"), &[]);
        assert!(effects.hardened);
    }

    #[test]
    fn flags_only_ignores_sip() {
        let gate = FlagBasedHardenedRuntime::flags_only();
        let cs = make_cs_info(|_| {});
        let effects = gate.compute_effects(&cs, Path::new("/usr/bin/ls"), &[]);
        assert!(!effects.hardened);
    }

    #[test]
    fn cs_restrict_strips_env() {
        let gate = FlagBasedHardenedRuntime::new();
        let cs = make_cs_info(|i| i.restrict = true);
        let effects = gate.compute_effects(&cs, Path::new("/tmp/app"), &[]);
        assert!(effects.strip_dyld_env);
    }

    #[test]
    fn get_task_allow_enables_debug() {
        let gate = FlagBasedHardenedRuntime::new();
        let cs = make_cs_info(|i| i.get_task_allow = true);
        let effects = gate.compute_effects(&cs, Path::new("/tmp/app"), &[]);
        assert!(effects.allows_debugging);
    }

    #[test]
    fn dyld_entitlement_relaxes() {
        let gate = FlagBasedHardenedRuntime::new();
        let cs = make_cs_info(|i| i.runtime = true);
        let ents = vec![(
            "com.apple.security.cs.allow-dyld-environment-variables".to_string(),
            true,
        )];
        let effects = gate.compute_effects(&cs, Path::new("/tmp/app"), &ents);
        assert!(!effects.strip_dyld_env);
        assert!(effects.allows_dylib_insertion);
    }

    #[test]
    fn jit_entitlement_relaxes() {
        let gate = FlagBasedHardenedRuntime::new();
        let cs = make_cs_info(|i| i.runtime = true);
        let ents = vec![(
            "com.apple.security.cs.allow-jit".to_string(),
            true,
        )];
        let effects = gate.compute_effects(&cs, Path::new("/tmp/app"), &ents);
        assert!(effects.allows_jit);
    }

    #[test]
    fn unsigned_memory_entitlement() {
        let gate = FlagBasedHardenedRuntime::new();
        let cs = make_cs_info(|i| i.runtime = true);
        let ents = vec![(
            "com.apple.security.cs.allow-unsigned-executable-memory".to_string(),
            true,
        )];
        let effects = gate.compute_effects(&cs, Path::new("/tmp/app"), &ents);
        assert!(effects.allows_unsigned_memory);
    }

    #[test]
    fn library_validation_from_flag() {
        let gate = FlagBasedHardenedRuntime::new();
        let cs = make_cs_info(|i| i.library_validation = true);
        let effects = gate.compute_effects(&cs, Path::new("/tmp/app"), &[]);
        assert!(effects.requires_library_validation);
    }

    #[test]
    fn hardened_requires_library_validation() {
        let gate = FlagBasedHardenedRuntime::new();
        let cs = make_cs_info(|i| i.runtime = true);
        let effects = gate.compute_effects(&cs, Path::new("/tmp/app"), &[]);
        assert!(effects.requires_library_validation);
    }

    #[test]
    fn trait_fallback_sip_path() {
        let gate = FlagBasedHardenedRuntime::new();
        let flags = MachFlags::empty();
        assert!(gate.is_hardened(Path::new("/usr/bin/ls"), flags));
        assert!(!gate.allows_dylib_insertion(Path::new("/usr/bin/ls"), flags));
        assert!(!gate.is_hardened(Path::new("/tmp/app"), flags));
        assert!(gate.allows_dylib_insertion(Path::new("/tmp/app"), flags));
    }
}
