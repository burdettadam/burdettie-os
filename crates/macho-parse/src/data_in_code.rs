//! Parser for LC_DATA_IN_CODE entries.
//!
//! The LC_DATA_IN_CODE load command points to an array of
//! `data_in_code_entry` structs (8 bytes each) in __LINKEDIT.

use macho_types::data_in_code::{DataInCodeEntry, DataInCodeKind};
use macho_types::load_command::{LoadCommandPayload, LoadCommandType};

use crate::ParsedMachO;

/// Parse data-in-code entries from the linked binary data.
///
/// Locates the LC_DATA_IN_CODE load command and decodes the entry array.
pub fn parse_data_in_code(parsed: &ParsedMachO, data: &[u8]) -> Vec<DataInCodeEntry> {
    for cmd in &parsed.commands {
        if cmd.cmd == LoadCommandType::DataInCode {
            if let LoadCommandPayload::LinkEdit {
                data_off,
                data_size,
            } = &cmd.payload
            {
                let start = *data_off as usize;
                let end = start + *data_size as usize;
                if end <= data.len() {
                    return decode_entries(&data[start..end]);
                }
            }
        }
    }
    Vec::new()
}

/// Decode raw data_in_code_entry structs from a byte slice.
/// Each entry is 8 bytes: offset(u32) + length(u16) + kind(u16).
fn decode_entries(data: &[u8]) -> Vec<DataInCodeEntry> {
    let entry_size = 8;
    let count = data.len() / entry_size;
    let mut entries = Vec::with_capacity(count);

    for i in 0..count {
        let base = i * entry_size;
        let offset =
            u32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]);
        let length = u16::from_le_bytes([data[base + 4], data[base + 5]]);
        let kind_raw = u16::from_le_bytes([data[base + 6], data[base + 7]]);
        entries.push(DataInCodeEntry {
            offset,
            length,
            kind: DataInCodeKind::from(kind_raw),
        });
    }

    entries
}
