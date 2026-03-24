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

fn read_uleb128(data: &[u8], pos: &mut usize) -> u64 {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        if *pos >= data.len() {
            break;
        }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            break;
        }
    }
    result
}

fn read_cstring(data: &[u8], pos: &mut usize) -> String {
    let start = *pos;
    while *pos < data.len() && data[*pos] != 0 {
        *pos += 1;
    }
    let s = String::from_utf8_lossy(&data[start..*pos]).into_owned();
    if *pos < data.len() {
        *pos += 1;
    }
    s
}

/// Parse the export trie from raw bytes.
pub fn parse_export_trie(trie_data: &[u8]) -> Vec<ExportedSymbol> {
    let mut results = Vec::new();
    if trie_data.is_empty() {
        return results;
    }

    // DFS through the trie
    struct StackEntry {
        offset: usize,
        prefix: String,
    }

    let mut stack = vec![StackEntry {
        offset: 0,
        prefix: String::new(),
    }];

    while let Some(entry) = stack.pop() {
        if entry.offset >= trie_data.len() {
            continue;
        }

        let mut pos = entry.offset;
        let terminal_size = read_uleb128(trie_data, &mut pos) as usize;

        if terminal_size > 0 {
            let terminal_start = pos;
            let flags = read_uleb128(trie_data, &mut pos);

            if flags & EXPORT_SYMBOL_FLAGS_REEXPORT != 0 {
                let ordinal = read_uleb128(trie_data, &mut pos);
                let reexport_name = read_cstring(trie_data, &mut pos);
                results.push(ExportedSymbol {
                    name: entry.prefix.clone(),
                    flags,
                    address: 0,
                    reexport_ordinal: Some(ordinal),
                    reexport_name: if reexport_name.is_empty() {
                        None
                    } else {
                        Some(reexport_name)
                    },
                });
            } else {
                let address = read_uleb128(trie_data, &mut pos);
                let final_address = if flags & EXPORT_SYMBOL_FLAGS_STUB_AND_RESOLVER != 0 {
                    // STUB_AND_RESOLVER: `address` above consumed stub_offset.
                    // Read the resolver_offset as the real address.
                    read_uleb128(trie_data, &mut pos)
                } else {
                    address
                };
                results.push(ExportedSymbol {
                    name: entry.prefix.clone(),
                    flags,
                    address: final_address,
                    reexport_ordinal: None,
                    reexport_name: None,
                });
            }

            // Skip any remaining terminal bytes
            pos = terminal_start + terminal_size;
        }

        // Read children
        if pos >= trie_data.len() {
            continue;
        }
        let child_count = trie_data[pos] as usize;
        pos += 1;

        for _ in 0..child_count {
            if pos >= trie_data.len() {
                break;
            }
            let edge_label = read_cstring(trie_data, &mut pos);
            let child_offset = read_uleb128(trie_data, &mut pos) as usize;

            stack.push(StackEntry {
                offset: child_offset,
                prefix: format!("{}{}", entry.prefix, edge_label),
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
                let start = *data_off as usize;
                let end = start + *data_size as usize;
                if end <= raw_data.len() {
                    return parse_export_trie(&raw_data[start..end]);
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
                    let start = *export_off as usize;
                    let end = start + *export_size as usize;
                    if end <= raw_data.len() {
                        return parse_export_trie(&raw_data[start..end]);
                    }
                }
            }
        }
    }

    Vec::new()
}
