//! Indirect symbol table parsing.
//!
//! The indirect symbol table is an array of `u32` indices into the main symbol
//! table. Sections like `__stubs`, `__got`, and `__la_symbol_ptr` use
//! `reserved1` to specify their starting index into this table. Each entry in
//! the section corresponds to one indirect symbol entry, allowing the linker
//! to know which symbol each stub/GOT slot refers to.
//!
//! Special values:
//! - `INDIRECT_SYMBOL_LOCAL` (0x8000_0000): local symbol, not in symbol table
//! - `INDIRECT_SYMBOL_ABS`  (0x4000_0000): absolute symbol

use serde::{Deserialize, Serialize};

pub const INDIRECT_SYMBOL_LOCAL: u32 = 0x8000_0000;
pub const INDIRECT_SYMBOL_ABS: u32 = 0x4000_0000;

/// A resolved indirect symbol entry, cross-referencing a section slot to a symbol name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndirectSymbolEntry {
    /// The section containing this entry (e.g. "__stubs", "__got", "__la_symbol_ptr").
    pub section: String,
    /// Index of this slot within the section (0-based).
    pub slot_index: usize,
    /// Virtual address of this slot.
    pub slot_addr: u64,
    /// The resolved symbol name, or a sentinel description.
    pub symbol_name: String,
    /// Raw index into the symbol table (before sentinel check).
    pub raw_index: u32,
}

/// Read the raw indirect symbol table from binary data.
///
/// Returns a `Vec<u32>` of symbol table indices (or sentinel values).
pub fn read_indirect_symbol_table(
    data: &[u8],
    indirectsymoff: u32,
    nindirectsyms: u32,
) -> Vec<u32> {
    let off = indirectsymoff as usize;
    let count = nindirectsyms as usize;
    let end = off + count * 4;
    if end > data.len() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let pos = off + i * 4;
        let val = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        result.push(val);
    }
    result
}

/// Cross-reference indirect symbol table entries with sections that use them.
///
/// For each slot in `__stubs`, `__got`, `__la_symbol_ptr`, and `__nl_symbol_ptr`,
/// resolves the corresponding symbol name from the symbol table.
pub fn resolve_indirect_symbols(
    parsed: &super::ParsedMachO,
    data: &[u8],
) -> Vec<IndirectSymbolEntry> {
    // Find DYSYMTAB to get indirect symbol table location
    let Some((indirectsymoff, nindirectsyms)) = find_dysymtab(parsed) else {
        return Vec::new();
    };

    let indirect_table = read_indirect_symbol_table(data, indirectsymoff, nindirectsyms);
    if indirect_table.is_empty() {
        return Vec::new();
    }

    let symbols = parsed.symbols.as_ref();
    let mut result = Vec::new();

    // Sections that use indirect symbols
    let indirect_sections = [
        "__stubs",
        "__got",
        "__la_symbol_ptr",
        "__nl_symbol_ptr",
        "__auth_got",
        "__auth_stubs",
    ];

    for seg in &parsed.segments {
        for sect in &seg.sections {
            if !indirect_sections.contains(&sect.sectname.as_str()) {
                continue;
            }

            let start_idx = sect.reserved1 as usize;
            // Calculate number of entries: section_size / entry_size
            let entry_size = match sect.sectname.as_str() {
                "__stubs" => {
                    if sect.reserved2 > 0 {
                        sect.reserved2 as usize
                    } else {
                        8
                    }
                }
                _ => 8, // pointer-sized slots (64-bit)
            };
            let n_entries = if entry_size > 0 {
                sect.size as usize / entry_size
            } else {
                0
            };

            for i in 0..n_entries {
                let table_idx = start_idx + i;
                if table_idx >= indirect_table.len() {
                    break;
                }

                let raw_index = indirect_table[table_idx];
                let symbol_name = if raw_index == INDIRECT_SYMBOL_LOCAL {
                    "<local>".to_string()
                } else if raw_index == INDIRECT_SYMBOL_ABS {
                    "<absolute>".to_string()
                } else if raw_index == (INDIRECT_SYMBOL_LOCAL | INDIRECT_SYMBOL_ABS) {
                    "<local+absolute>".to_string()
                } else if let Some(syms) = symbols {
                    syms.symbols
                        .get(raw_index as usize)
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| format!("<invalid index {raw_index}>"))
                } else {
                    format!("<sym[{raw_index}]>")
                };

                result.push(IndirectSymbolEntry {
                    section: sect.sectname.clone(),
                    slot_index: i,
                    slot_addr: sect.addr + (i * entry_size) as u64,
                    symbol_name,
                    raw_index,
                });
            }
        }
    }

    result
}

fn find_dysymtab(parsed: &super::ParsedMachO) -> Option<(u32, u32)> {
    use macho_types::load_command::{LoadCommandPayload, LoadCommandType};

    for cmd in &parsed.commands {
        if cmd.cmd == LoadCommandType::Dysymtab {
            if let LoadCommandPayload::Dysymtab {
                indirectsymoff,
                nindirectsyms,
                ..
            } = &cmd.payload
            {
                return Some((*indirectsymoff, *nindirectsyms));
            }
        }
    }
    None
}
