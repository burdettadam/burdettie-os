//! Mach-O export extractor for swift-stub-gen.
//!
//! Reads a framework binary (possibly a fat/universal binary), selects the
//! arm64 slice, and returns the full export list via `macho-parse`.

use std::path::Path;

use macho_parse::{
    exports::{ExportedSymbol, EXPORT_SYMBOL_FLAGS_REEXPORT},
    fat::fat_slice_for_cpu,
    parse,
};
use macho_types::header::CpuType;

/// Classification of an extracted symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// Regular function or data export (not re-export).
    Regular,
    /// Re-exported from another dylib — only a name alias, no code here.
    Reexport,
}

/// A symbol extracted from a framework binary.
#[derive(Debug, Clone)]
pub struct ExtractedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    /// `true` for data symbols (kind=regular, address in a data segment, or
    /// the name carries metadata/witness-table patterns).
    pub is_data: bool,
}

impl ExtractedSymbol {
    fn from_export(ex: &ExportedSymbol) -> Self {
        let kind = if ex.flags & EXPORT_SYMBOL_FLAGS_REEXPORT != 0 {
            SymbolKind::Reexport
        } else {
            SymbolKind::Regular
        };
        // Heuristic: type-metadata accessors (`Ma`), nominal-type descriptors
        // (`Mn`), and protocol-witness tables (`WP`, `Wl`, `WL`) live in data
        // segments.  These suffixes are stable across Swift ABI versions.
        let is_data = {
            let n = &ex.name;
            n.ends_with("Ma")
                || n.ends_with("Mo")
                || n.contains("Mn")
                || n.contains("WP")
                || n.contains("Wl")
                || n.contains("WL")
                || n.contains("_metadata")
                || n.contains("_conformance")
        };
        ExtractedSymbol {
            name: ex.name.clone(),
            kind,
            is_data,
        }
    }
}

/// Extract all exported symbols from a Mach-O or fat-binary at `path`.
///
/// Prefers the `arm64` (`AARCH64`) slice when parsing a fat binary.  Falls
/// back to `arm64e` and then to the first slice if neither is present.
///
/// Only returns **regular** (non-re-export) symbols; re-exports are filtered
/// out by default because they have no code/data in the current binary.
#[allow(dead_code)]
pub fn extract_symbols(
    path: &Path,
    include_reexports: bool,
) -> Result<Vec<ExtractedSymbol>, String> {
    let raw = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    // Prefer arm64 (subtype 0/Arm64All); fat_slice_for_cpu already prefers
    // Arm64All over Arm64E automatically when CpuType::Arm64 is requested.
    let slice: &[u8] = if macho_parse::fat::is_fat(&raw) {
        fat_slice_for_cpu(&raw, CpuType::Arm64)
            .map_err(|_| format!("{}: no arm64 slice found in fat binary", path.display()))?
    } else {
        &raw
    };

    let parsed = parse(slice).map_err(|e| format!("{}: parse error: {e:?}", path.display()))?;

    let exports = macho_parse::exports::extract_exports(&parsed, slice);

    let mut out = Vec::with_capacity(exports.len());
    for ex in &exports {
        let sym = ExtractedSymbol::from_export(ex);
        if sym.kind == SymbolKind::Reexport && !include_reexports {
            continue;
        }
        out.push(sym);
    }
    Ok(out)
}

/// Like `extract_symbols` but also includes symbols found via the nlist
/// symbol table (LC_SYMTAB) that aren't in the export trie.  This catches
/// some older binaries that omit the trie.
pub fn extract_symbols_full(
    path: &Path,
    include_reexports: bool,
) -> Result<Vec<ExtractedSymbol>, String> {
    let raw = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    let slice: &[u8] = if macho_parse::fat::is_fat(&raw) {
        fat_slice_for_cpu(&raw, CpuType::Arm64)
            .map_err(|_| format!("{}: no arm64 slice found in fat binary", path.display()))?
    } else {
        &raw
    };

    let parsed = parse(slice).map_err(|e| format!("{}: parse error: {e:?}", path.display()))?;

    // Primary: export trie
    let trie_exports = macho_parse::exports::extract_exports(&parsed, slice);
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<ExtractedSymbol> = Vec::new();

    for ex in &trie_exports {
        let sym = ExtractedSymbol::from_export(ex);
        if sym.kind == SymbolKind::Reexport && !include_reexports {
            continue;
        }
        seen.insert(sym.name.clone());
        out.push(sym);
    }

    // Secondary: nlist symbols (Defined exports not in trie)
    if let Some(symtab) = &parsed.symbols {
        use macho_types::symbol::SymbolType;
        for sym in &symtab.symbols {
            if sym.sym_type != SymbolType::Defined {
                continue;
            }
            if sym.name.is_empty() || seen.contains(&sym.name) {
                continue;
            }
            seen.insert(sym.name.clone());
            out.push(ExtractedSymbol {
                name: sym.name.clone(),
                kind: SymbolKind::Regular,
                // nlist doesn't distinguish code vs. data; treat conservatively as code
                is_data: false,
            });
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_kind_reexport_flag() {
        let ex = macho_parse::exports::ExportedSymbol {
            name: "_foo".to_string(),
            flags: EXPORT_SYMBOL_FLAGS_REEXPORT,
            address: 0,
            reexport_ordinal: Some(1),
            reexport_name: None,
        };
        let sym = ExtractedSymbol::from_export(&ex);
        assert_eq!(sym.kind, SymbolKind::Reexport);
    }

    #[test]
    fn test_symbol_kind_regular() {
        let ex = macho_parse::exports::ExportedSymbol {
            name: "_$s7SwiftUI4TextV".to_string(),
            flags: 0,
            address: 0x1000,
            reexport_ordinal: None,
            reexport_name: None,
        };
        let sym = ExtractedSymbol::from_export(&ex);
        assert_eq!(sym.kind, SymbolKind::Regular);
        assert!(!sym.is_data);
    }

    #[test]
    fn test_data_heuristic_metadata() {
        let ex = macho_parse::exports::ExportedSymbol {
            name: "_$s7SwiftUI4TextVMa".to_string(),
            flags: 0,
            address: 0x2000,
            reexport_ordinal: None,
            reexport_name: None,
        };
        let sym = ExtractedSymbol::from_export(&ex);
        assert!(sym.is_data);
    }
}
