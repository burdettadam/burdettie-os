//! DWARF .eh_frame emission helpers for ARM64.

use super::constants::*;

const DW_EH_PE_ABSPTR: u8 = 0x00;

/// Emit a CIE for ARM64 into `buf`.
pub(crate) fn emit_cie_arm64(buf: &mut Vec<u8>, personality: Option<usize>, uses_lsda: bool) {
    let cie_start = buf.len();
    buf.extend_from_slice(&0u32.to_ne_bytes()); // placeholder for length

    buf.extend_from_slice(&0u32.to_ne_bytes()); // CIE id = 0
    buf.push(1); // version

    let mut augmentation = Vec::from([b'z']);
    if personality.is_some() {
        augmentation.push(b'P');
    }
    if uses_lsda {
        augmentation.push(b'L');
    }
    augmentation.push(b'R');
    augmentation.push(0);
    buf.extend_from_slice(&augmentation);

    write_uleb128(buf, 4); // code_alignment_factor (ARM64 = 4-byte instrs)
    write_sleb128(buf, -8); // data_alignment_factor
    write_uleb128(buf, DW_REG_LR); // return_address_register = x30

    let mut augmentation_data = Vec::new();
    if let Some(personality_addr) = personality {
        augmentation_data.push(DW_EH_PE_ABSPTR);
        augmentation_data.extend_from_slice(&personality_addr.to_ne_bytes());
    }
    if uses_lsda {
        augmentation_data.push(DW_EH_PE_ABSPTR);
    }
    augmentation_data.push(DW_EH_PE_ABSPTR); // FDE addresses are absolute pointers.
    write_uleb128(buf, augmentation_data.len() as u64);
    buf.extend_from_slice(&augmentation_data);

    // Initial instructions: DW_CFA_def_cfa SP, 0
    buf.push(DW_CFA_DEF_CFA);
    write_uleb128(buf, DW_REG_SP);
    write_uleb128(buf, 0);

    // Pad to pointer-size alignment.
    while !(buf.len() - cie_start - 4).is_multiple_of(std::mem::size_of::<usize>()) {
        buf.push(DW_CFA_NOP);
    }

    // Patch length (excludes the 4-byte length field itself).
    let length = (buf.len() - cie_start - 4) as u32;
    buf[cie_start..cie_start + 4].copy_from_slice(&length.to_ne_bytes());
}

/// Emit one FDE for an ARM64 function.
pub(crate) fn emit_fde_arm64(
    buf: &mut Vec<u8>,
    cie_start: usize,
    func_addr: usize,
    func_length: usize,
    encoding: u32,
    lsda: Option<usize>,
) {
    let fde_start = buf.len();
    buf.extend_from_slice(&0u32.to_ne_bytes()); // placeholder for length

    // CIE pointer: byte offset from *this field* back to CIE start.
    let cie_ptr = (fde_start + 4 - cie_start) as u32;
    buf.extend_from_slice(&cie_ptr.to_ne_bytes());

    // initial_location (absolute, 8 bytes for DW_EH_PE_absptr on 64-bit).
    buf.extend_from_slice(&(func_addr as u64).to_ne_bytes());

    // address_range.
    buf.extend_from_slice(&(func_length as u64).to_ne_bytes());

    // Augmentation data length (for 'z' augmentation — optional LSDA pointer).
    let lsda_size = lsda.map(|_| std::mem::size_of::<usize>()).unwrap_or(0);
    write_uleb128(buf, lsda_size as u64);
    if let Some(lsda_addr) = lsda {
        buf.extend_from_slice(&lsda_addr.to_ne_bytes());
    }

    // ── CFA instructions based on compact encoding ─────────────────
    let mode = encoding & UNWIND_ARM64_MODE_MASK;
    match mode {
        UNWIND_ARM64_MODE_FRAME => emit_frame_cfa(buf, encoding),
        UNWIND_ARM64_MODE_FRAMELESS => emit_frameless_cfa(buf, encoding),
        _ => {} // unknown / DWARF mode — no-op (shouldn't reach here)
    }

    // Pad to pointer-size alignment.
    while !(buf.len() - fde_start - 4).is_multiple_of(std::mem::size_of::<usize>()) {
        buf.push(DW_CFA_NOP);
    }

    // Patch length.
    let length = (buf.len() - fde_start - 4) as u32;
    buf[fde_start..fde_start + 4].copy_from_slice(&length.to_ne_bytes());
}

/// Emit CFA instructions for ARM64 FRAME mode.
///
/// Layout: CFA = FP + 16; FP at CFA-16; LR at CFA-8; then saved pairs below FP.
pub(crate) fn emit_frame_cfa(buf: &mut Vec<u8>, encoding: u32) {
    // CFA = FP + 16
    buf.push(DW_CFA_DEF_CFA);
    write_uleb128(buf, DW_REG_FP);
    write_uleb128(buf, 16);

    // FP saved at CFA - 16  →  factored offset = 16/8 = 2
    buf.push(0x80 | (DW_REG_FP as u8 & 0x3F));
    write_uleb128(buf, 2);

    // LR saved at CFA - 8  →  factored offset = 8/8 = 1
    buf.push(0x80 | (DW_REG_LR as u8 & 0x3F));
    write_uleb128(buf, 1);

    // Saved register pairs below FP, in order.
    let mut slot = 0u64; // pair index (0 = first pair below FP)
    for &(flag, low_reg, high_reg) in FRAME_REG_PAIRS {
        if encoding & flag != 0 {
            // High register at CFA - (24 + 16*slot) → factored = (24 + 16*slot) / 8
            let high_factored = (24 + 16 * slot) / 8;
            buf.push(0x80 | (high_reg & 0x3F));
            write_uleb128(buf, high_factored);

            // Low register at CFA - (32 + 16*slot) → factored = (32 + 16*slot) / 8
            let low_factored = (32 + 16 * slot) / 8;
            buf.push(0x80 | (low_reg & 0x3F));
            write_uleb128(buf, low_factored);

            slot += 1;
        }
    }
}

/// Emit CFA instructions for ARM64 FRAMELESS mode.
///
/// Leaf function with known stack allocation. CFA = SP + stack_size.
pub(crate) fn emit_frameless_cfa(buf: &mut Vec<u8>, encoding: u32) {
    let stack_size = ((encoding & UNWIND_ARM64_FRAMELESS_STACK_SIZE_MASK) >> 12) * 16;
    buf.push(DW_CFA_DEF_CFA);
    write_uleb128(buf, DW_REG_SP);
    write_uleb128(buf, stack_size as u64);
}

// ── Helpers ────────────────────────────────────────────────────────

pub(crate) fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

pub(crate) fn read_u16(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

pub(crate) fn write_uleb128(buf: &mut Vec<u8>, mut val: u64) {
    loop {
        let byte = (val & 0x7F) as u8;
        val >>= 7;
        if val == 0 {
            buf.push(byte);
            return;
        }
        buf.push(byte | 0x80);
    }
}

pub(crate) fn write_sleb128(buf: &mut Vec<u8>, mut val: i64) {
    loop {
        let byte = (val & 0x7F) as u8;
        val >>= 7;
        let done = (val == 0 && byte & 0x40 == 0) || (val == -1 && byte & 0x40 != 0);
        if done {
            buf.push(byte);
            return;
        }
        buf.push(byte | 0x80);
    }
}
