//! ARM64 compact unwind and DWARF encoding constants.

// ── ARM64 compact unwind encoding constants ────────────────────────

pub(crate) const UNWIND_ARM64_MODE_MASK: u32 = 0x0F00_0000;
pub(crate) const UNWIND_ARM64_MODE_FRAMELESS: u32 = 0x0200_0000;
#[allow(dead_code)]
pub(crate) const UNWIND_ARM64_MODE_DWARF: u32 = 0x0300_0000;
pub(crate) const UNWIND_ARM64_MODE_FRAME: u32 = 0x0400_0000;
pub(crate) const UNWIND_HAS_LSDA: u32 = 0x4000_0000;
pub(crate) const UNWIND_PERSONALITY_MASK: u32 = 0x3000_0000;
pub(crate) const UNWIND_PERSONALITY_SHIFT: u32 = 28;

pub(crate) const UNWIND_ARM64_FRAMELESS_STACK_SIZE_MASK: u32 = 0x00FFF_000;

/// Register-pair bit flags in the low 12 bits of a FRAME-mode encoding.
pub(crate) const FRAME_REG_PAIRS: &[(u32, u8, u8)] = &[
    // (bit_flag, low_dwarf_reg, high_dwarf_reg)
    (0x0000_0001, 19, 20), // X19/X20
    (0x0000_0002, 21, 22), // X21/X22
    (0x0000_0004, 23, 24), // X23/X24
    (0x0000_0008, 25, 26), // X25/X26
    (0x0000_0010, 27, 28), // X27/X28
    (0x0000_0100, 72, 73), // D8/D9   (DWARF reg = 64 + Dn)
    (0x0000_0200, 74, 75), // D10/D11
    (0x0000_0400, 76, 77), // D12/D13
    (0x0000_0800, 78, 79), // D14/D15
];

// ── Compact unwind section format constants ────────────────────────

pub(crate) const UNWIND_SECOND_LEVEL_REGULAR: u32 = 2;
pub(crate) const UNWIND_SECOND_LEVEL_COMPRESSED: u32 = 3;

// ── DWARF ARM64 register numbers ───────────────────────────────────

pub(crate) const DW_REG_FP: u64 = 29;
pub(crate) const DW_REG_LR: u64 = 30;
pub(crate) const DW_REG_SP: u64 = 31;

// ── DWARF CFA opcodes ─────────────────────────────────────────────

pub(crate) const DW_CFA_NOP: u8 = 0x00;
pub(crate) const DW_CFA_DEF_CFA: u8 = 0x0c;
