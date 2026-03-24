//! Thread-Local Variable (TLV) descriptor parser.
//!
//! Parses `__thread_vars` sections (type `S_THREAD_LOCAL_VARIABLES` / 0x13)
//! to extract TLV descriptors. Each descriptor is 3 pointers (24 bytes on 64-bit).

use macho_types::segment::{SectionType, TlvDescriptor};

use crate::ParsedMachO;

/// Parse TLV descriptors from all `__thread_vars` sections in the binary.
pub fn parse_tlv_descriptors(parsed: &ParsedMachO, data: &[u8]) -> Vec<TlvDescriptor> {
    let is_64 = parsed.header.magic == 0xFEED_FACF || parsed.header.magic == 0xCFFA_EDFE;
    let ptr_size: usize = if is_64 { 8 } else { 4 };
    let desc_size = ptr_size * 3;

    let mut descriptors = Vec::new();

    for section in &parsed.sections {
        if section.section_type_name() != SectionType::ThreadLocalVariables {
            continue;
        }

        let file_offset = section.offset as usize;
        let section_size = section.size as usize;

        if file_offset + section_size > data.len() {
            continue;
        }

        let count = section_size / desc_size;
        for i in 0..count {
            let base = file_offset + i * desc_size;
            let addr = section.addr + (i * desc_size) as u64;

            let (thunk, key, offset) = if is_64 {
                if base + 24 > data.len() {
                    break;
                }
                (
                    u64::from_le_bytes(data[base..base + 8].try_into().unwrap()),
                    u64::from_le_bytes(data[base + 8..base + 16].try_into().unwrap()),
                    u64::from_le_bytes(data[base + 16..base + 24].try_into().unwrap()),
                )
            } else {
                if base + 12 > data.len() {
                    break;
                }
                (
                    u32::from_le_bytes(data[base..base + 4].try_into().unwrap()) as u64,
                    u32::from_le_bytes(data[base + 4..base + 8].try_into().unwrap()) as u64,
                    u32::from_le_bytes(data[base + 8..base + 12].try_into().unwrap()) as u64,
                )
            };

            descriptors.push(TlvDescriptor {
                address: addr,
                thunk,
                key,
                offset,
            });
        }
    }

    descriptors
}
