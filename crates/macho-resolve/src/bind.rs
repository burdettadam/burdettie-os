//! Two-level namespace symbol binding.
//!
//! In Mach-O's two-level namespace model, each undefined symbol records
//! which dylib ordinal it expects to find the symbol in. This module resolves
//! undefined symbols from the main binary (and its dylibs) against the
//! export tries of the corresponding dylibs.

use std::collections::HashMap;
use std::path::Path;

use macho_parse::{extract_exports, parse, ExportedSymbol};
use serde::{Deserialize, Serialize};

use crate::ResolutionPlan;

/// Result of binding a single undefined symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolBinding {
    /// The symbol name being resolved.
    pub name: String,
    /// The dylib ordinal from n_desc (1-based index into LC_LOAD_DYLIB list).
    pub dylib_ordinal: i64,
    /// Where the symbol was found (dylib install name or path).
    pub resolved_in: Option<String>,
    /// The address within the providing dylib.
    pub address: Option<u64>,
    /// Whether the symbol is a weak import (missing = ok).
    pub is_weak: bool,
    /// Whether binding succeeded.
    pub bound: bool,
}

/// Complete symbol binding report for a binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingReport {
    /// All bindings attempted.
    pub bindings: Vec<SymbolBinding>,
    /// Symbols that failed to bind and are NOT weak.
    pub failed: Vec<String>,
    /// Number of successfully bound symbols.
    pub bound_count: usize,
    /// Number of weak symbols left unbound (acceptable).
    pub weak_unbound_count: usize,
}

impl BindingReport {
    pub fn is_complete(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Cache of loaded export tries keyed by file path.
struct ExportCache {
    cache: HashMap<String, Vec<ExportedSymbol>>,
}

impl ExportCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    fn get_exports(&mut self, path: &Path) -> &[ExportedSymbol] {
        let key = path.to_string_lossy().into_owned();
        self.cache.entry(key).or_insert_with(|| {
            let data = match std::fs::read(path) {
                Ok(d) => d,
                Err(e) => {
                    log::warn!("ExportCache: failed to read {}: {e}", path.display());
                    return Vec::new();
                }
            };
            match parse(&data) {
                Ok(parsed) => extract_exports(&parsed, &data),
                Err(e) => {
                    log::warn!("ExportCache: failed to parse {}: {e}", path.display());
                    Vec::new()
                }
            }
        })
    }

    fn lookup(&mut self, path: &Path, symbol_name: &str) -> Option<(String, u64)> {
        let exports = self.get_exports(path);
        exports
            .iter()
            .find(|e| e.name == symbol_name)
            .map(|e| (path.to_string_lossy().into_owned(), e.address))
    }
}

/// Perform two-level namespace symbol binding.
///
/// For each undefined symbol in the main binary:
/// 1. Look at its dylib ordinal (from n_desc field, or from bind opcodes)
/// 2. Find the corresponding dylib in the resolution plan
/// 3. Search that dylib's export trie for the symbol
pub fn bind_symbols(main_data: &[u8], resolution: &ResolutionPlan) -> BindingReport {
    let parsed = match parse(main_data) {
        Ok(p) => p,
        Err(e) => {
            log::error!("bind_symbols: failed to parse main binary: {e}");
            return BindingReport {
                bindings: Vec::new(),
                failed: Vec::new(),
                bound_count: 0,
                weak_unbound_count: 0,
            };
        }
    };

    let mut cache = ExportCache::new();
    let mut bindings = Vec::new();

    // Collect the ordered dylib list (matching LC_LOAD_DYLIB order)
    // Ordinal 1 = first LC_LOAD_DYLIB, ordinal 2 = second, etc.
    let dylib_order: Vec<&str> = parsed.dylibs.iter().map(|d| d.name.as_str()).collect();

    // Method 1: Use bind opcode entries from fixups (most reliable for ordinal info)
    let fixups = macho_parse::extract_fixups(&parsed, main_data);
    let all_binds: Vec<_> = fixups
        .binds
        .iter()
        .chain(fixups.lazy_binds.iter())
        .collect();

    if !all_binds.is_empty() {
        for bind in &all_binds {
            let ordinal = bind.dylib_ordinal;
            let is_weak = bind.is_weak;

            let resolved = resolve_by_ordinal(
                ordinal,
                &bind.symbol_name,
                &dylib_order,
                resolution,
                &mut cache,
            );

            let (resolved_in, address, bound) = match resolved {
                Some((lib, addr)) => (Some(lib), Some(addr), true),
                None => (None, None, false),
            };

            bindings.push(SymbolBinding {
                name: bind.symbol_name.clone(),
                dylib_ordinal: ordinal,
                resolved_in,
                address,
                is_weak,
                bound,
            });
        }
    } else {
        // Method 2: Fall back to symtab undefined symbols
        // The dylib ordinal is encoded in n_desc: GET_LIBRARY_ORDINAL(n_desc)
        if let Some(ref symtab) = parsed.symbols {
            for sym in &symtab.symbols {
                if sym.sym_type != macho_types::SymbolType::Undefined {
                    continue;
                }
                if sym.name.is_empty() {
                    continue;
                }

                let ordinal = ((sym.desc >> 8) & 0xFF) as i64;
                let is_weak = (sym.desc & 0x0080) != 0; // N_WEAK_REF

                let resolved =
                    resolve_by_ordinal(ordinal, &sym.name, &dylib_order, resolution, &mut cache);

                let (resolved_in, address, bound) = match resolved {
                    Some((lib, addr)) => (Some(lib), Some(addr), true),
                    None => (None, None, false),
                };

                bindings.push(SymbolBinding {
                    name: sym.name.clone(),
                    dylib_ordinal: ordinal,
                    resolved_in,
                    address,
                    is_weak,
                    bound,
                });
            }
        }
    }

    let bound_count = bindings.iter().filter(|b| b.bound).count();
    let weak_unbound_count = bindings.iter().filter(|b| !b.bound && b.is_weak).count();
    let failed: Vec<String> = bindings
        .iter()
        .filter(|b| !b.bound && !b.is_weak)
        .map(|b| b.name.clone())
        .collect();

    BindingReport {
        bindings,
        failed,
        bound_count,
        weak_unbound_count,
    }
}

/// Resolve a symbol by its dylib ordinal.
///
/// Ordinals:
///   0 = BIND_SPECIAL_DYLIB_SELF
///  -1 = BIND_SPECIAL_DYLIB_MAIN_EXECUTABLE
///  -2 = BIND_SPECIAL_DYLIB_FLAT_LOOKUP
///  1+ = index into LC_LOAD_DYLIB list (1-based)
fn resolve_by_ordinal(
    ordinal: i64,
    symbol_name: &str,
    dylib_order: &[&str],
    resolution: &ResolutionPlan,
    cache: &mut ExportCache,
) -> Option<(String, u64)> {
    match ordinal {
        // Specific dylib ordinal
        n if n >= 1 => {
            let idx = (n - 1) as usize;
            if idx >= dylib_order.len() {
                return None;
            }
            let raw_name = dylib_order[idx];

            // Find this dylib in the resolved nodes
            find_in_resolved_dylib(raw_name, symbol_name, resolution, cache)
        }
        // BIND_SPECIAL_DYLIB_SELF (0): look in the image's own exports first,
        // then fall back to flat lookup (for umbrella framework re-exports).
        0 => {
            if !resolution.graph.nodes.is_empty() {
                let self_path = &resolution.graph.nodes[0].path;
                if let Some(result) = cache.lookup(self_path, symbol_name) {
                    return Some(result);
                }
            }
            flat_lookup(symbol_name, resolution, cache)
        }
        // BIND_SPECIAL_DYLIB_MAIN_EXECUTABLE (-1): look in main exe
        -1 => {
            let main_path = &resolution.graph.nodes[0].path;
            cache.lookup(main_path, symbol_name)
        }
        // BIND_SPECIAL_DYLIB_FLAT_LOOKUP (-2): search all
        -2 => flat_lookup(symbol_name, resolution, cache),
        // BIND_SPECIAL_DYLIB_WEAK_LOOKUP (-3): search all, null if not found
        -3 => {
            // Weak lookup: same as flat but callers should treat None as null (0)
            // rather than an error. The actual null-fill is done at the call site.
            flat_lookup(symbol_name, resolution, cache)
        }
        _ => None,
    }
}

fn find_in_resolved_dylib(
    raw_name: &str,
    symbol_name: &str,
    resolution: &ResolutionPlan,
    cache: &mut ExportCache,
) -> Option<(String, u64)> {
    // Find the edge with this raw_name from node 0
    for edge in &resolution.graph.edges {
        if edge.raw_name == raw_name {
            let node = &resolution.graph.nodes[edge.to];
            if let Some(result) = cache.lookup(&node.path, symbol_name) {
                return Some(result);
            }
        }
    }
    // Also try matching by install_name
    for node in &resolution.graph.nodes {
        if node.install_name.as_deref() == Some(raw_name) {
            if let Some(result) = cache.lookup(&node.path, symbol_name) {
                return Some(result);
            }
        }
    }
    None
}

fn flat_lookup(
    symbol_name: &str,
    resolution: &ResolutionPlan,
    cache: &mut ExportCache,
) -> Option<(String, u64)> {
    for node in &resolution.graph.nodes {
        if let Some(result) = cache.lookup(&node.path, symbol_name) {
            return Some(result);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_binding(name: &str, bound: bool, is_weak: bool) -> SymbolBinding {
        SymbolBinding {
            name: name.to_string(),
            dylib_ordinal: 1,
            resolved_in: if bound {
                Some("/usr/lib/libSystem.B.dylib".into())
            } else {
                None
            },
            address: if bound { Some(0x1000) } else { None },
            is_weak,
            bound,
        }
    }

    #[test]
    fn binding_report_empty_is_complete() {
        let report = BindingReport {
            bindings: vec![],
            failed: vec![],
            bound_count: 0,
            weak_unbound_count: 0,
        };
        assert!(report.is_complete());
    }

    #[test]
    fn binding_report_all_resolved_is_complete() {
        let report = BindingReport {
            bindings: vec![
                make_binding("_foo", true, false),
                make_binding("_bar", true, false),
            ],
            failed: vec![],
            bound_count: 2,
            weak_unbound_count: 0,
        };
        assert!(report.is_complete());
    }

    #[test]
    fn binding_report_unresolved_not_complete() {
        let report = BindingReport {
            bindings: vec![make_binding("_foo", false, false)],
            failed: vec!["_foo".to_string()],
            bound_count: 0,
            weak_unbound_count: 0,
        };
        assert!(!report.is_complete());
    }
}
