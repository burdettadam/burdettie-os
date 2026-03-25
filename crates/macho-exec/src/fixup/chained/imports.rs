//! Chained fixup import table parsing.

use super::ChainedImport;

const DYLD_CHAINED_IMPORT_ADDEND: u32 = 2;
const DYLD_CHAINED_IMPORT_ADDEND64: u32 = 3;

pub(super) fn parse_imports(
    buf: &[u8],
    imports_offset: usize,
    imports_count: usize,
    symbols_offset: usize,
    imports_format: u32,
) -> Vec<ChainedImport> {
    let mut imports = Vec::with_capacity(imports_count);

    let entry_size = match imports_format {
        DYLD_CHAINED_IMPORT_ADDEND => 8,
        DYLD_CHAINED_IMPORT_ADDEND64 => 16,
        _ => 4,
    };

    for i in 0..imports_count {
        let pos = imports_offset + i * entry_size;
        if pos + entry_size > buf.len() {
            break;
        }

        let (lib_ordinal, weak, name_offset) = match imports_format {
            DYLD_CHAINED_IMPORT_ADDEND64 => {
                let lib = u16::from_le_bytes([buf[pos], buf[pos + 1]]) as u8;
                let flags = u16::from_le_bytes([buf[pos + 2], buf[pos + 3]]);
                let w = (flags & 1) != 0;
                let noff =
                    u32::from_le_bytes([buf[pos + 4], buf[pos + 5], buf[pos + 6], buf[pos + 7]])
                        as usize;
                (lib, w, noff)
            }
            _ => {
                let raw = u32::from_le_bytes([buf[pos], buf[pos + 1], buf[pos + 2], buf[pos + 3]]);
                let lib = (raw & 0xFF) as u8;
                let w = ((raw >> 8) & 1) != 0;
                let noff = (raw >> 9) as usize;
                (lib, w, noff)
            }
        };

        let name = read_cstring_at(buf, symbols_offset + name_offset);
        imports.push(ChainedImport {
            lib_ordinal,
            weak,
            name,
        });
    }

    imports
}

pub(super) fn read_cstring_at(buf: &[u8], offset: usize) -> String {
    if offset >= buf.len() {
        return String::new();
    }
    let end = buf[offset..]
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(buf.len() - offset);
    String::from_utf8_lossy(&buf[offset..offset + end]).into_owned()
}
