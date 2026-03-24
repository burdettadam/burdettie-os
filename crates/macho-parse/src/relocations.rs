//! Relocation entry parser for Mach-O sections.
//!
//! Each section's `reloff`/`nreloc` fields point to an array of 8-byte
//! `relocation_info` structures in the `__LINKEDIT` segment.
//!
//! Layout of each 8-byte entry:
//!   - `r_address` (u32): offset within the section
//!   - packed u32:
//!     - bits 0–23:  `r_symbolnum` (symbol table index or section ordinal)
//!     - bit 24:     `r_pcrel` (PC-relative)
//!     - bits 25–26: `r_length` (0=byte, 1=word, 2=long, 3=quad)
//!     - bit 27:     `r_extern` (1=symbol index, 0=section ordinal)
//!     - bits 28–31: `r_type` (RelocationType)

use macho_types::relocation::{Relocation, RelocationType};

use crate::ParsedMachO;

/// Parse relocation entries from all sections that have `reloff != 0 && nreloc > 0`.
pub fn parse_relocations(parsed: &ParsedMachO, data: &[u8]) -> Vec<SectionRelocations> {
    let mut result = Vec::new();

    for section in &parsed.sections {
        if section.reloff == 0 || section.nreloc == 0 {
            continue;
        }

        let entries =
            parse_relocation_entries(data, section.reloff as usize, section.nreloc as usize);

        if !entries.is_empty() {
            result.push(SectionRelocations {
                section: section.sectname.clone(),
                segment: section.segname.clone(),
                entries,
            });
        }
    }

    result
}

/// Relocation entries associated with a specific section.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SectionRelocations {
    /// Section name (e.g. `__text`).
    pub section: String,
    /// Segment name (e.g. `__TEXT`).
    pub segment: String,
    /// Parsed relocation entries.
    pub entries: Vec<Relocation>,
}

/// Parse `count` relocation entries starting at `offset` in the binary data.
fn parse_relocation_entries(data: &[u8], offset: usize, count: usize) -> Vec<Relocation> {
    let entry_size = 8;
    let end = offset + count * entry_size;
    if end > data.len() {
        return Vec::new();
    }

    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * entry_size;
        let address = u32::from_le_bytes(data[base..base + 4].try_into().unwrap());
        let packed = u32::from_le_bytes(data[base + 4..base + 8].try_into().unwrap());

        let symbol_index = packed & 0x00FF_FFFF; // bits 0-23
        let pc_relative = (packed >> 24) & 1 != 0; // bit 24
        let length = ((packed >> 25) & 3) as u8; // bits 25-26
        let extern_sym = (packed >> 27) & 1 != 0; // bit 27
        let reloc_type_raw = ((packed >> 28) & 0xF) as u8; // bits 28-31

        entries.push(Relocation {
            address,
            symbol_index,
            pc_relative,
            length,
            extern_sym,
            reloc_type: RelocationType::from(reloc_type_raw),
        });
    }

    entries
}
