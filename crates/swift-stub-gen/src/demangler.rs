//! Swift mangling detection and human-readable demangling.
//!
//! Swift mangled symbols come in two flavours:
//!
//!  - New-style (Swift ≥5): start with `_$s` or `_$S`
//!  - Old-style (Swift 1–3): start with `__T` followed by `F`, `M`, `W`,
//!    `T`, `C`, `P`, `V`, `O`, `E`, `a`, `o`, `d`…
//!  - ABI-stable runtime helpers: start with `_swift_`
//!
//! We do not implement a full demangler here.  Instead we call the system
//! `swift-demangle` binary (if Xcode is installed) to produce a comment
//! string, and fall back to the raw mangled name when that isn't available.

use std::process::Command;

/// Return `true` when the symbol looks like a Swift-mangled name.
///
/// Covers new-style (`_$s`/`_$S`) and old-style (`__TF`, `__TM`, `__TW`,
/// `__TT`, `__Tv`, `__Ti`, `__TO`, `__TFC`, `__TFO`, `__TFE`, `__TFa`,
/// `__TFo`, `__TFd`, `__TIV`, `__TIs`, `__TIa`) and the stdlib runtime
/// helper prefix `_swift_`.
pub fn is_swift_mangled(sym: &str) -> bool {
    // New-style ABI-stable encoding
    if sym.starts_with("_$s") || sym.starts_with("_$S") {
        return true;
    }
    // Old-style encoding — only match the valid FIRST characters after `__T`.
    // Valid kind markers: F=function, M=metadata, W=witness-table, C=class,
    // P=protocol, V=struct, O=enum, v=variable, I=type-info, S=stdlib-type.
    // Deliberately excluded: E (causes `__TEXT` false-positive — extensions
    // are encoded as `__TFE…`, so the FIRST char remains `F`), T (matches
    // `__THREAD`, `__TEXT`), X, H, etc.
    if sym.starts_with("__T") {
        const OLD_STYLE_TAGS: &[u8] = b"FMWCPVOvIS";
        if let Some(next) = sym.as_bytes().get(3) {
            if OLD_STYLE_TAGS.contains(next) {
                return true;
            }
        }
    }
    // Swift runtime helpers
    if sym.starts_with("_swift_") {
        return true;
    }
    false
}

/// Classify the symbol for use in the generated comment.
///
/// Returns a short category string based on the mangled name.  This is
/// heuristic and intentionally conservative.
pub fn classify(sym: &str) -> &'static str {
    if sym.starts_with("_$s") || sym.starts_with("_$S") {
        // After `_$s` the next characters encode the module + decl kind.
        // We look for common suffixes/infixes rather than full parsing.
        let body = &sym[3..];
        // Suffix `Ma`=metadata-accessor, `Mn`=nominal-type-descriptor, `Mo`=metadata-obj.
        // `CMa`/`VMa`/`SMa` etc all end with `Ma` — match broadly.
        if body.ends_with("Ma") || body.ends_with("Mo") || body.contains("Mn") {
            return "type-metadata";
        }
        if body.contains("WP") || body.contains("Wl") || body.contains("WL") {
            return "protocol-witness";
        }
        if body.ends_with("fC") || body.ends_with("fc") {
            return "initialiser";
        }
        if body.ends_with("fD") || body.ends_with("fd") {
            return "deinitialiser";
        }
        if body.ends_with("vg") || body.ends_with("vs") || body.ends_with("vm") {
            return "property-accessor";
        }
        if body.ends_with("Tu") || body.ends_with("Tv") {
            return "thunk";
        }
        return "function";
    }
    if sym.starts_with("_swift_") {
        return "runtime-helper";
    }
    "old-style"
}

/// Attempt to demangle `sym` using the system `swift-demangle` binary.
///
/// Returns `None` if the tool is not found, exits non-zero, or produces
/// output identical to the input (which `swift-demangle` does when it
/// cannot demangle).
pub fn demangle(sym: &str) -> Option<String> {
    let out = Command::new("swift-demangle")
        .arg("--compact")
        .arg(sym)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let demangled = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    // swift-demangle echoes the input unchanged when it cannot decode it.
    if demangled.is_empty() || demangled == sym {
        None
    } else {
        Some(demangled)
    }
}

/// Convert a mangled Swift symbol name into a valid Rust identifier suffix.
///
/// The suffix is used to deduplicate multiple stubs registered to the same
/// null function (we use a single `swift_null_stub` for all stubs, so we
/// only need uniqueness in the `register!` call comment, not in a per-symbol
/// function name).
///
/// The returned string is safe to embed in an identifier:
///   - leading `_$s` / `_$S` / `__T` stripped
///   - `$` replaced with `D`  (dollar-sign encodes context in Swift ABI)
///   - non-alphanumeric → `_`
pub fn to_rust_ident_suffix(sym: &str) -> String {
    let stripped = if let Some(s) = sym.strip_prefix("_$s") {
        s
    } else if let Some(s) = sym.strip_prefix("_$S") {
        s
    } else if let Some(s) = sym.strip_prefix("__T") {
        s
    } else if let Some(s) = sym.strip_prefix('_') {
        s
    } else {
        sym
    };

    stripped
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else if c == '$' {
                'D'
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_style_detected() {
        assert!(is_swift_mangled("_$s7SwiftUI4ViewP"));
        assert!(is_swift_mangled("_$S7SwiftUI4TextV3fooyyF"));
    }

    #[test]
    fn test_old_style_detected() {
        assert!(is_swift_mangled("__TFSs6printlnU__FT3valx_T_"));
        assert!(is_swift_mangled("__TMSi"));
    }

    #[test]
    fn test_false_positives_rejected() {
        assert!(!is_swift_mangled("__TEXT"));
        assert!(!is_swift_mangled("__THREAD"));
        assert!(!is_swift_mangled("_printf"));
        assert!(!is_swift_mangled("_NSString"));
    }

    #[test]
    fn test_runtime_helper_detected() {
        assert!(is_swift_mangled("_swift_retain"));
        assert!(is_swift_mangled("_swift_release"));
        assert!(is_swift_mangled("_swift_bridgeObjectRetain_n"));
    }

    #[test]
    fn test_rust_ident_roundtrip() {
        let id = to_rust_ident_suffix("_$s7SwiftUI4ViewP");
        assert!(id.chars().all(|c| c.is_alphanumeric() || c == '_'));
        assert!(!id.is_empty());
    }

    #[test]
    fn test_classify() {
        assert_eq!(classify("_$s7SwiftUI4TextVMa"), "type-metadata");
        assert_eq!(classify("_$s7SwiftUI4TextVACycfC"), "initialiser");
        assert_eq!(classify("_swift_retain"), "runtime-helper");
    }
}
