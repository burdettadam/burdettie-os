//! Export trie parser for Mach-O binaries.
//!
//! The export trie is a compact representation of all symbols exported by a
//! dylib. It's referenced by LC_DYLD_INFO(_ONLY) export_off/export_size
//! or by LC_DYLD_EXPORTS_TRIE.
//!
//! The trie is a byte-stream where each node has:
//!   - terminal info (if this node is an export)
//!   - edge list (children with label bytes)

use macho_types::load_command::{LoadCommandPayload, LoadCommandType};
use serde::{Deserialize, Serialize};

use crate::leb128::read_uleb128;

/// Flags from the export trie terminal nodes.
pub const EXPORT_SYMBOL_FLAGS_KIND_MASK: u64 = 0x03;
pub const EXPORT_SYMBOL_FLAGS_KIND_REGULAR: u64 = 0x00;
pub const EXPORT_SYMBOL_FLAGS_KIND_THREAD_LOCAL: u64 = 0x01;
pub const EXPORT_SYMBOL_FLAGS_KIND_ABSOLUTE: u64 = 0x02;
pub const EXPORT_SYMBOL_FLAGS_WEAK_DEFINITION: u64 = 0x04;
pub const EXPORT_SYMBOL_FLAGS_REEXPORT: u64 = 0x08;
pub const EXPORT_SYMBOL_FLAGS_STUB_AND_RESOLVER: u64 = 0x10;

/// A single exported symbol from the export trie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedSymbol {
    pub name: String,
    pub flags: u64,
    pub address: u64,
    /// If re-exported, the ordinal of the source dylib.
    pub reexport_ordinal: Option<u64>,
    /// If re-exported under a different name, that name.
    pub reexport_name: Option<String>,
}

impl ExportedSymbol {
    pub fn is_weak(&self) -> bool {
        self.flags & EXPORT_SYMBOL_FLAGS_WEAK_DEFINITION != 0
    }

    pub fn is_reexport(&self) -> bool {
        self.flags & EXPORT_SYMBOL_FLAGS_REEXPORT != 0
    }

    pub fn kind(&self) -> &'static str {
        match self.flags & EXPORT_SYMBOL_FLAGS_KIND_MASK {
            EXPORT_SYMBOL_FLAGS_KIND_REGULAR => "regular",
            EXPORT_SYMBOL_FLAGS_KIND_THREAD_LOCAL => "thread-local",
            EXPORT_SYMBOL_FLAGS_KIND_ABSOLUTE => "absolute",
            _ => "unknown",
        }
    }
}

fn read_cstring<'a>(data: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
    let remaining = data.get(*pos..)?;
    let length = remaining.iter().position(|&byte| byte == 0)?;
    *pos += length + 1;
    Some(&remaining[..length])
}

fn parse_terminal(data: &[u8], name: String, name_budget: &mut usize) -> Option<ExportedSymbol> {
    let mut pos = 0;
    let flags = read_uleb128(data, &mut pos)?;
    if flags & EXPORT_SYMBOL_FLAGS_REEXPORT != 0 {
        let ordinal = read_uleb128(data, &mut pos)?;
        let reexport_name = read_cstring(data, &mut pos)?;
        // Lossy UTF-8 can expand each invalid byte into three bytes.
        *name_budget = name_budget.checked_sub(reexport_name.len().checked_mul(3)?)?;
        let reexport_name = String::from_utf8_lossy(reexport_name).into_owned();
        Some(ExportedSymbol {
            name,
            flags,
            address: 0,
            reexport_ordinal: Some(ordinal),
            reexport_name: (!reexport_name.is_empty()).then_some(reexport_name),
        })
    } else {
        let address = read_uleb128(data, &mut pos)?;
        let address = if flags & EXPORT_SYMBOL_FLAGS_STUB_AND_RESOLVER != 0 {
            read_uleb128(data, &mut pos)?
        } else {
            address
        };
        Some(ExportedSymbol {
            name,
            flags,
            address,
            reexport_ordinal: None,
            reexport_name: None,
        })
    }
}

/// Parse the export trie from raw bytes.
///
/// Malformed branches are skipped. Terminal records must fit inside their
/// declared lengths, and cycles are rejected without discarding valid aliases
/// that reach a shared node through different prefixes. To prevent compact DAGs
/// from expanding into unbounded output, the walk schedules at most four times
/// the input length (and at most one million nodes). Decoded prefix and symbol
/// names share a byte budget of 64 times the input length, capped at 64 MiB.
/// If a budget is exhausted, symbols successfully parsed so far are returned.
pub fn parse_export_trie(trie_data: &[u8]) -> Vec<ExportedSymbol> {
    enum StackEntry {
        Visit { offset: usize, prefix: String },
        Leave(usize),
    }

    let mut results = Vec::new();
    let max_nodes = trie_data.len().saturating_mul(4).min(1_000_000);
    let mut remaining_nodes = max_nodes.saturating_sub(1);
    let mut remaining_name_bytes = trie_data.len().saturating_mul(64).min(64 * 1024 * 1024);
    let mut active = std::collections::HashSet::new();
    let mut stack = vec![StackEntry::Visit {
        offset: 0,
        prefix: String::new(),
    }];

    'traversal: while let Some(entry) = stack.pop() {
        let (offset, prefix) = match entry {
            StackEntry::Visit { offset, prefix } => (offset, prefix),
            StackEntry::Leave(offset) => {
                active.remove(&offset);
                continue;
            }
        };
        if offset >= trie_data.len() || !active.insert(offset) {
            continue;
        }
        stack.push(StackEntry::Leave(offset));

        let mut pos = offset;
        let Some(terminal_size) =
            read_uleb128(trie_data, &mut pos).and_then(|value| usize::try_from(value).ok())
        else {
            continue;
        };
        let Some(terminal_end) = pos.checked_add(terminal_size) else {
            continue;
        };
        let Some(terminal) = trie_data.get(pos..terminal_end) else {
            continue;
        };
        if !terminal.is_empty() {
            let Some(budget) = remaining_name_bytes.checked_sub(prefix.len()) else {
                log::debug!("export trie decoded-name budget exhausted");
                break;
            };
            remaining_name_bytes = budget;
            let Some(symbol) = parse_terminal(terminal, prefix.clone(), &mut remaining_name_bytes)
            else {
                continue;
            };
            results.push(symbol);
            if results.len() >= max_nodes {
                log::debug!("export trie symbol budget exhausted");
                break;
            }
        }
        pos = terminal_end;

        let Some(&child_count) = trie_data.get(pos) else {
            continue;
        };
        pos += 1;
        for _ in 0..child_count {
            let Some(edge_label) = read_cstring(trie_data, &mut pos) else {
                break;
            };
            let Some(child_offset) =
                read_uleb128(trie_data, &mut pos).and_then(|value| usize::try_from(value).ok())
            else {
                break;
            };
            if child_offset >= trie_data.len() || active.contains(&child_offset) {
                continue;
            }
            if remaining_nodes == 0 {
                log::debug!("export trie traversal budget exhausted");
                break 'traversal;
            }
            let Some(name_size) = edge_label
                .len()
                .checked_mul(3)
                .and_then(|size| prefix.len().checked_add(size))
            else {
                break 'traversal;
            };
            let Some(budget) = remaining_name_bytes.checked_sub(name_size) else {
                log::debug!("export trie decoded-name budget exhausted");
                break 'traversal;
            };
            remaining_name_bytes = budget;
            remaining_nodes -= 1;
            let edge_label = String::from_utf8_lossy(edge_label);
            stack.push(StackEntry::Visit {
                offset: child_offset,
                prefix: format!("{prefix}{edge_label}"),
            });
        }
    }

    results
}

/// Extract exported symbols from a parsed Mach-O.
pub fn extract_exports(parsed: &super::ParsedMachO, raw_data: &[u8]) -> Vec<ExportedSymbol> {
    // Try LC_DYLD_EXPORTS_TRIE first (modern)
    for cmd in &parsed.commands {
        if cmd.cmd == LoadCommandType::DyldExportsTrie {
            if let LoadCommandPayload::LinkEdit {
                data_off,
                data_size,
            } = &cmd.payload
            {
                if let Some(data) =
                    crate::bytes_at(raw_data, u64::from(*data_off), u64::from(*data_size))
                {
                    return parse_export_trie(data);
                }
            }
        }
    }

    // Fall back to LC_DYLD_INFO(_ONLY) export_off/export_size
    for cmd in &parsed.commands {
        if cmd.cmd == LoadCommandType::DyldInfoOnly || cmd.cmd == LoadCommandType::DyldInfo {
            if let LoadCommandPayload::DyldInfo {
                export_off,
                export_size,
                ..
            } = &cmd.payload
            {
                if *export_size > 0 {
                    if let Some(data) =
                        crate::bytes_at(raw_data, u64::from(*export_off), u64::from(*export_size))
                    {
                        return parse_export_trie(data);
                    }
                }
            }
        }
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_symbol(name: &str, flags: u64, address: u64) -> ExportedSymbol {
        ExportedSymbol {
            name: name.to_string(),
            flags,
            address,
            reexport_ordinal: None,
            reexport_name: None,
        }
    }

    #[test]
    fn kind_regular() {
        let sym = make_symbol("_main", EXPORT_SYMBOL_FLAGS_KIND_REGULAR, 0x1000);
        assert_eq!(sym.kind(), "regular");
    }

    #[test]
    fn kind_thread_local() {
        let sym = make_symbol("_tls_var", EXPORT_SYMBOL_FLAGS_KIND_THREAD_LOCAL, 0x2000);
        assert_eq!(sym.kind(), "thread-local");
    }

    #[test]
    fn kind_absolute() {
        let sym = make_symbol("_abs", EXPORT_SYMBOL_FLAGS_KIND_ABSOLUTE, 0x3000);
        assert_eq!(sym.kind(), "absolute");
    }

    #[test]
    fn kind_unknown_for_out_of_range() {
        let sym = make_symbol("_mystery", 0x03, 0x4000);
        assert_eq!(sym.kind(), "unknown");
    }

    #[test]
    fn kind_masks_non_kind_bits() {
        // STUB_AND_RESOLVER (0x10) with kind bits = 0 → "regular"
        let sym = make_symbol("_stub", EXPORT_SYMBOL_FLAGS_STUB_AND_RESOLVER, 0x5000);
        assert_eq!(sym.kind(), "regular");
    }

    #[test]
    fn is_weak_flag() {
        let sym = make_symbol("_weak", EXPORT_SYMBOL_FLAGS_WEAK_DEFINITION, 0x6000);
        assert!(sym.is_weak());
        let sym2 = make_symbol("_strong", 0, 0x7000);
        assert!(!sym2.is_weak());
    }

    #[test]
    fn is_reexport_flag() {
        let sym = make_symbol("_re", EXPORT_SYMBOL_FLAGS_REEXPORT, 0);
        assert!(sym.is_reexport());
        let sym2 = make_symbol("_local", 0, 0x1000);
        assert!(!sym2.is_reexport());
    }

    #[test]
    fn read_uleb128_single_byte() {
        let data = [0x05];
        let mut pos = 0;
        assert_eq!(read_uleb128(&data, &mut pos), Some(5));
        assert_eq!(pos, 1);
    }

    #[test]
    fn read_uleb128_multi_byte() {
        // 624485 = 0xE5 0x8E 0x26
        let data = [0xE5, 0x8E, 0x26];
        let mut pos = 0;
        assert_eq!(read_uleb128(&data, &mut pos), Some(624485));
        assert_eq!(pos, 3);
    }

    #[test]
    fn read_uleb128_empty() {
        let data: [u8; 0] = [];
        let mut pos = 0;
        assert_eq!(read_uleb128(&data, &mut pos), None);
    }

    #[test]
    fn read_cstring_simple() {
        let data = b"hello\0world\0";
        let mut pos = 0;
        assert_eq!(read_cstring(data, &mut pos), Some(b"hello".as_slice()));
        assert_eq!(pos, 6); // past the NUL
        assert_eq!(read_cstring(data, &mut pos), Some(b"world".as_slice()));
    }

    #[test]
    fn read_cstring_empty_string() {
        let data = b"\0rest";
        let mut pos = 0;
        assert_eq!(read_cstring(data, &mut pos), Some(b"".as_slice()));
        assert_eq!(pos, 1);
    }

    #[test]
    fn parse_export_trie_empty() {
        assert!(parse_export_trie(&[]).is_empty());
    }
}
