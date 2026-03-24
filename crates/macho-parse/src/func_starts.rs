//! LC_FUNCTION_STARTS decoder.
//!
//! The function starts data is a ULEB128-delta-compressed list of
//! function entry point offsets from the __TEXT segment base. Each
//! delta is added to a running address to produce the next function
//! start address.

use macho_types::load_command::{LoadCommandPayload, LoadCommandType};

/// Parse the LC_FUNCTION_STARTS blob into a list of function start
/// virtual addresses.
///
/// The `text_vmaddr` parameter is the vmaddr of the __TEXT segment,
/// used as the base address for the first function.
pub fn parse_function_starts(parsed: &super::ParsedMachO, raw_data: &[u8]) -> Vec<u64> {
    // Find the __TEXT vmaddr as base address
    let text_vmaddr = parsed
        .segments
        .iter()
        .find(|s| s.name == "__TEXT")
        .map(|s| s.vmaddr)
        .unwrap_or(0);

    // Find the LC_FUNCTION_STARTS link-edit data
    let Some((off, size)) = parsed.commands.iter().find_map(|cmd| {
        if cmd.cmd == LoadCommandType::FunctionStarts {
            if let LoadCommandPayload::LinkEdit {
                data_off,
                data_size,
            } = &cmd.payload
            {
                return Some((*data_off as usize, *data_size as usize));
            }
        }
        None
    }) else {
        return Vec::new();
    };

    let end = off + size;
    if end > raw_data.len() {
        return Vec::new();
    }

    decode_uleb128_deltas(&raw_data[off..end], text_vmaddr)
}

/// Decode ULEB128-delta-encoded function starts from raw bytes.
fn decode_uleb128_deltas(data: &[u8], base: u64) -> Vec<u64> {
    let mut addrs = Vec::new();
    let mut addr = base;
    let mut pos = 0;

    while pos < data.len() {
        let delta = read_uleb128(data, &mut pos);
        if delta == 0 {
            break; // terminator
        }
        addr += delta;
        addrs.push(addr);
    }

    addrs
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
