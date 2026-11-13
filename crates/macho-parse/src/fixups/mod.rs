//! Fixup (rebase/bind) parsing for Mach-O binaries.
//!
//! Two models exist:
//! - Classic: LC_DYLD_INFO / LC_DYLD_INFO_ONLY → opcode streams
//! - Modern: LC_DYLD_CHAINED_FIXUPS → pointer chains in __DATA segments

mod decode;

pub use decode::{parse_binds, parse_chained_fixups, parse_rebases};

use macho_types::load_command::{LoadCommandPayload, LoadCommandType};
use serde::{Deserialize, Serialize};

// ── Opcode constants ───────────────────────────────────────────────

// Rebase
pub(crate) const REBASE_OPCODE_MASK: u8 = 0xF0;
pub(crate) const REBASE_IMMEDIATE_MASK: u8 = 0x0F;
pub(crate) const REBASE_OPCODE_DONE: u8 = 0x00;
pub(crate) const REBASE_OPCODE_SET_TYPE_IMM: u8 = 0x10;
pub(crate) const REBASE_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB: u8 = 0x20;
pub(crate) const REBASE_OPCODE_ADD_ADDR_ULEB: u8 = 0x30;
pub(crate) const REBASE_OPCODE_ADD_ADDR_IMM_SCALED: u8 = 0x40;
pub(crate) const REBASE_OPCODE_DO_REBASE_IMM_TIMES: u8 = 0x50;
pub(crate) const REBASE_OPCODE_DO_REBASE_ULEB_TIMES: u8 = 0x60;
pub(crate) const REBASE_OPCODE_DO_REBASE_ADD_ADDR_ULEB: u8 = 0x70;
pub(crate) const REBASE_OPCODE_DO_REBASE_ULEB_TIMES_SKIPPING_ULEB: u8 = 0x80;

// Bind
pub(crate) const BIND_OPCODE_MASK: u8 = 0xF0;
pub(crate) const BIND_IMMEDIATE_MASK: u8 = 0x0F;
pub(crate) const BIND_OPCODE_DONE: u8 = 0x00;
pub(crate) const BIND_OPCODE_SET_DYLIB_ORDINAL_IMM: u8 = 0x10;
pub(crate) const BIND_OPCODE_SET_DYLIB_ORDINAL_ULEB: u8 = 0x20;
pub(crate) const BIND_OPCODE_SET_DYLIB_SPECIAL_IMM: u8 = 0x30;
pub(crate) const BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM: u8 = 0x40;
pub(crate) const BIND_OPCODE_SET_TYPE_IMM: u8 = 0x50;
pub(crate) const BIND_OPCODE_SET_ADDEND_SLEB: u8 = 0x60;
pub(crate) const BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB: u8 = 0x70;
pub(crate) const BIND_OPCODE_ADD_ADDR_ULEB: u8 = 0x80;
pub(crate) const BIND_OPCODE_DO_BIND: u8 = 0x90;
pub(crate) const BIND_OPCODE_DO_BIND_ADD_ADDR_ULEB: u8 = 0xA0;
pub(crate) const BIND_OPCODE_DO_BIND_ADD_ADDR_IMM_SCALED: u8 = 0xB0;
pub(crate) const BIND_OPCODE_DO_BIND_ULEB_TIMES_SKIPPING_ULEB: u8 = 0xC0;

#[allow(dead_code)]
pub(crate) const REBASE_TYPE_POINTER: u8 = 1;
#[allow(dead_code)]
pub(crate) const REBASE_TYPE_TEXT_ABSOLUTE32: u8 = 2;
#[allow(dead_code)]
pub(crate) const REBASE_TYPE_TEXT_PCREL32: u8 = 3;

#[allow(dead_code)]
pub(crate) const BIND_TYPE_POINTER: u8 = 1;
#[allow(dead_code)]
pub(crate) const BIND_TYPE_TEXT_ABSOLUTE32: u8 = 2;
#[allow(dead_code)]
pub(crate) const BIND_TYPE_TEXT_PCREL32: u8 = 3;

// Chained
#[allow(dead_code)]
pub(crate) const DYLD_CHAINED_PTR_64: u16 = 2;
#[allow(dead_code)]
pub(crate) const DYLD_CHAINED_PTR_ARM64E: u16 = 1;
#[allow(dead_code)]
pub(crate) const DYLD_CHAINED_PTR_64_OFFSET: u16 = 6;
#[allow(dead_code)]
pub(crate) const DYLD_CHAINED_PTR_ARM64E_USERLAND: u16 = 12;
#[allow(dead_code)]
pub(crate) const DYLD_CHAINED_PTR_ARM64E_USERLAND24: u16 = 13;

// ── Public types ───────────────────────────────────────────────────

/// A single rebase fixup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebaseEntry {
    pub segment_index: usize,
    pub segment_offset: u64,
    pub rebase_type: u8,
}

/// A single bind fixup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindEntry {
    pub segment_index: usize,
    pub segment_offset: u64,
    pub bind_type: u8,
    pub dylib_ordinal: i64,
    pub symbol_name: String,
    pub addend: i64,
    pub is_weak: bool,
    pub is_lazy: bool,
}

/// A single chained fixup pointer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainedFixup {
    Rebase {
        segment_index: usize,
        page_offset: u32,
        target: u64,
    },
    Bind {
        segment_index: usize,
        page_offset: u32,
        ordinal: u32,
        addend: i64,
    },
}

/// Which fixup model the binary uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FixupModel {
    Classic,
    Chained { pointer_format: u16 },
    None,
}

/// All fixup information from a Mach-O binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixups {
    pub model: FixupModel,
    pub rebases: Vec<RebaseEntry>,
    pub binds: Vec<BindEntry>,
    pub weak_binds: Vec<BindEntry>,
    pub lazy_binds: Vec<BindEntry>,
    pub chained: Vec<ChainedFixup>,
}

impl Fixups {
    pub fn empty() -> Self {
        Self {
            model: FixupModel::None,
            rebases: Vec::new(),
            binds: Vec::new(),
            weak_binds: Vec::new(),
            lazy_binds: Vec::new(),
            chained: Vec::new(),
        }
    }
}

// ── Codec helpers ──────────────────────────────────────────────────

pub(crate) fn read_uleb128(data: &[u8], pos: &mut usize) -> u64 {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        if *pos >= data.len() {
            break;
        }
        let byte = data[*pos];
        *pos += 1;
        result |= u64::from(byte & 0x7F) << shift;
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

pub(crate) fn read_sleb128(data: &[u8], pos: &mut usize) -> i64 {
    let mut result: i64 = 0;
    let mut shift = 0u32;
    let mut byte;
    loop {
        if *pos >= data.len() {
            return result;
        }
        byte = data[*pos];
        *pos += 1;
        result |= i64::from(byte & 0x7F) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
        if shift >= 64 {
            break;
        }
    }
    if shift < 64 && (byte & 0x40) != 0 {
        result |= !0i64 << shift;
    }
    result
}

pub(crate) fn read_cstring(data: &[u8], pos: &mut usize) -> String {
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

// ── Top-level fixup extraction ─────────────────────────────────────

/// Extract all fixup information from a parsed Mach-O.
pub fn extract_fixups(parsed: &crate::ParsedMachO, raw_data: &[u8]) -> Fixups {
    for cmd in &parsed.commands {
        if cmd.cmd == LoadCommandType::DyldChainedFixups {
            if let LoadCommandPayload::LinkEdit {
                data_off,
                data_size,
            } = &cmd.payload
            {
                let (chained, ptr_fmt) = parse_chained_fixups(raw_data, *data_off, *data_size);
                return Fixups {
                    model: FixupModel::Chained {
                        pointer_format: ptr_fmt,
                    },
                    rebases: Vec::new(),
                    binds: Vec::new(),
                    weak_binds: Vec::new(),
                    lazy_binds: Vec::new(),
                    chained,
                };
            }
        }
    }

    for cmd in &parsed.commands {
        if cmd.cmd == LoadCommandType::DyldInfoOnly || cmd.cmd == LoadCommandType::DyldInfo {
            if let LoadCommandPayload::DyldInfo {
                rebase_off,
                rebase_size,
                bind_off,
                bind_size,
                weak_bind_off,
                weak_bind_size,
                lazy_bind_off,
                lazy_bind_size,
                export_off: _,
                export_size: _,
            } = &cmd.payload
            {
                let rebases = parse_rebases(raw_data, *rebase_off, *rebase_size);
                let binds = parse_binds(raw_data, *bind_off, *bind_size, false);
                let weak_binds = parse_binds(raw_data, *weak_bind_off, *weak_bind_size, false);
                let lazy_binds = parse_binds(raw_data, *lazy_bind_off, *lazy_bind_size, true);
                return Fixups {
                    model: FixupModel::Classic,
                    rebases,
                    binds,
                    weak_binds,
                    lazy_binds,
                    chained: Vec::new(),
                };
            }
        }
    }

    Fixups::empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixups_empty_has_no_entries() {
        let f = Fixups::empty();
        assert!(matches!(f.model, FixupModel::None));
        assert!(f.rebases.is_empty());
        assert!(f.binds.is_empty());
        assert!(f.weak_binds.is_empty());
        assert!(f.lazy_binds.is_empty());
        assert!(f.chained.is_empty());
    }

    #[test]
    fn uleb128_single_byte() {
        let data = [0x7F];
        let mut pos = 0;
        assert_eq!(read_uleb128(&data, &mut pos), 127);
        assert_eq!(pos, 1);
    }

    #[test]
    fn uleb128_multi_byte() {
        // 300 = 0xAC 0x02
        let data = [0xAC, 0x02];
        let mut pos = 0;
        assert_eq!(read_uleb128(&data, &mut pos), 300);
        assert_eq!(pos, 2);
    }

    #[test]
    fn uleb128_zero() {
        let data = [0x00];
        let mut pos = 0;
        assert_eq!(read_uleb128(&data, &mut pos), 0);
    }

    #[test]
    fn sleb128_positive() {
        // 42 = 0x2A (single byte, high bit clear, sign bit clear)
        let data = [0x2A];
        let mut pos = 0;
        assert_eq!(read_sleb128(&data, &mut pos), 42);
    }

    #[test]
    fn sleb128_negative() {
        // -1 = 0x7F (single byte, bit 6 set → sign extend)
        let data = [0x7F];
        let mut pos = 0;
        assert_eq!(read_sleb128(&data, &mut pos), -1);
    }

    #[test]
    fn sleb128_negative_multi_byte() {
        // -128 = 0x80 0x7F
        let data = [0x80, 0x7F];
        let mut pos = 0;
        assert_eq!(read_sleb128(&data, &mut pos), -128);
    }

    #[test]
    fn cstring_basic() {
        let data = b"foo\0bar";
        let mut pos = 0;
        assert_eq!(read_cstring(data, &mut pos), "foo");
        assert_eq!(pos, 4);
    }

    #[test]
    fn cstring_at_end_no_nul() {
        let data = b"abc";
        let mut pos = 0;
        assert_eq!(read_cstring(data, &mut pos), "abc");
        assert_eq!(pos, 3);
    }

    #[test]
    fn fixup_model_debug() {
        let m = FixupModel::Chained { pointer_format: 6 };
        let dbg = format!("{:?}", m);
        assert!(dbg.contains("Chained"));
        assert!(dbg.contains("6"));
    }
}
