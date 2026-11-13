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

        let Some(section_data) = crate::bytes_at(data, u64::from(section.offset), section.size)
        else {
            continue;
        };

        for (i, descriptor) in section_data.chunks_exact(desc_size).enumerate() {
            let Ok(byte_offset) = u64::try_from(i * desc_size) else {
                break;
            };
            let Some(addr) = section.addr.checked_add(byte_offset) else {
                break;
            };
            let read_pointer = |offset: usize| {
                let bytes = &descriptor[offset..offset + ptr_size];
                if is_64 {
                    u64::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ])
                } else {
                    u64::from(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                }
            };
            let (thunk, key, offset) = (
                read_pointer(0),
                read_pointer(ptr_size),
                read_pointer(ptr_size * 2),
            );

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
