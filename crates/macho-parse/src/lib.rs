pub mod codesig;
pub mod data_in_code;
mod error;
pub mod exports;
pub mod fat;
pub mod fixups;
pub mod func_starts;
pub mod indirect;
mod parser;
pub mod relocations;
pub mod strings;
pub mod tlv;

use std::sync::{Arc, OnceLock};

pub use codesig::{parse_code_signature, verify_page_hashes, PageHashMismatch, PageHashResult};
pub use data_in_code::parse_data_in_code;
pub use error::ParseError;
pub use exports::{extract_exports, parse_export_trie, ExportedSymbol};
pub use fat::{fat_slice_for_cpu, fat_slice_for_cpu_subtype, is_fat, parse_auto, parse_fat};
pub use fixups::{extract_fixups, BindEntry, ChainedFixup, FixupModel, Fixups, RebaseEntry};
pub use func_starts::parse_function_starts;
pub use indirect::{read_indirect_symbol_table, resolve_indirect_symbols, IndirectSymbolEntry};
pub use parser::{parse, ParsedMachO};
pub use relocations::{parse_relocations, SectionRelocations};
pub use strings::{extract_cstrings, SectionStrings};
pub use tlv::parse_tlv_descriptors;

/// Global platform hasher, set once by the runtime before parsing.
static PLATFORM_HASHER: OnceLock<Arc<dyn platform_core::hash::Hasher>> = OnceLock::new();

/// Set the global platform hasher (called once from runtime initialization).
pub fn set_platform_hasher(hasher: Arc<dyn platform_core::hash::Hasher>) {
    if PLATFORM_HASHER.set(hasher).is_err() {
        log::debug!("macho-parse: set_platform_hasher called more than once (harmless if reusing runtime)");
    }
}

/// Get the global platform hasher (if set).
pub(crate) fn platform_hasher() -> Option<&'static dyn platform_core::hash::Hasher> {
    PLATFORM_HASHER.get().map(|arc| arc.as_ref())
}

/// Convert a virtual memory address to a file offset using segment mappings.
///
/// Scans parsed segments to find which contains the given `vmaddr`, then
/// computes the file offset as `vmaddr - seg.vmaddr + seg.fileoff`.
pub fn vmaddr_to_file_offset(parsed: &ParsedMachO, vmaddr: u64) -> Option<usize> {
    for seg in &parsed.segments {
        if vmaddr >= seg.vmaddr && vmaddr < seg.vmaddr + seg.vmsize {
            let offset_in_seg = vmaddr - seg.vmaddr;
            if offset_in_seg < seg.filesize {
                return Some((seg.fileoff + offset_in_seg) as usize);
            }
        }
    }
    None
}
