//! Pointer Authentication Code (PAC) utilities for ARM64e binaries.
//!
//! On ARM64e, function pointers and data pointers may carry a PAC signature
//! in the upper bits. Before dereferencing or calling such pointers in our
//! in-process simulator, we must strip the PAC bits.
//!
//! ARM64e PAC layout (for user-space pointers):
//! - Bits 0-46: Virtual address (47-bit VA space)
//! - Bits 47-62: PAC signature (16 bits on current hardware)
//! - Bit 63: Top-byte-ignore / kernel-user discriminator
//!
//! Since our runtime executes in-process (same address space), we strip
//! PAC by masking to the lower 48 bits. We do NOT verify signatures —
//! we're simulating the loader, not enforcing PAC security.
//!
//! ## Cross-platform strategy
//!
//! On macOS aarch64, PAC hardware is always present — we sign GOT pointers
//! with hardware PACIA/PACIB and the guest binary's authenticated branches
//! (BRAA/BLRAA) work natively.
//!
//! On Linux aarch64 (e.g. Docker on Apple Silicon), we detect PAC support
//! at runtime via `AT_HWCAP` / `HWCAP_PACA`.  If present, we sign normally.
//! If absent (QEMU, non-PAC silicon), we leave GOT pointers unsigned and
//! patch authenticated branches to their unauthenticated equivalents.

use std::sync::OnceLock;

use macho_types::header::CpuType;

/// Strip PAC bits from a pointer, returning just the raw virtual address.
///
/// On ARM64 hardware with 48-bit VAs, PAC occupies bits 47-62.
/// We preserve bit 63 for kernel pointers but clear the PAC field.
///
/// For user-space pointers (which is all we handle), this simply
/// masks to the lower 48 bits.
#[inline]
pub fn ptrauth_strip(ptr: u64) -> u64 {
    // User-space virtual addresses on macOS use 47 bits (or 48 with TPRO).
    // PAC is stored in bits above the VA width but below bit 63.
    // For our in-process simulation, mask to 48 bits.
    ptr & 0x0000_FFFF_FFFF_FFFF
}

/// PAC key types used in ARM64e binaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacKey {
    /// Instruction Address key A (function pointers, vtables)
    IA = 0,
    /// Instruction Address key B (alternative code signing)
    IB = 1,
    /// Data Address key A (data pointers, ObjC class pointers)
    DA = 2,
    /// Data Address key B (alternative data signing)
    DB = 3,
}

impl PacKey {
    /// Decode a PAC key from the 2-bit key field in a chained fixup entry.
    pub fn from_bits(bits: u64) -> Self {
        match bits & 0x3 {
            0 => PacKey::IA,
            1 => PacKey::IB,
            2 => PacKey::DA,
            3 => PacKey::DB,
            _ => unreachable!(),
        }
    }
}

impl std::fmt::Display for PacKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacKey::IA => write!(f, "IA"),
            PacKey::IB => write!(f, "IB"),
            PacKey::DA => write!(f, "DA"),
            PacKey::DB => write!(f, "DB"),
        }
    }
}

/// Decoded auth metadata from an ARM64e chained fixup entry.
#[derive(Debug, Clone, Copy)]
pub struct AuthMetadata {
    /// The PAC key used (IA/IB/DA/DB).
    pub key: PacKey,
    /// Whether address diversity is enabled (the pointer address is
    /// mixed into the PAC computation).
    pub addr_diversity: bool,
    /// The 16-bit diversity value (discriminator).
    pub diversity: u16,
}

impl AuthMetadata {
    /// Extract auth metadata from an ARM64e auth bind/rebase raw value.
    ///
    /// ARM64e auth entries (bit 63 set) share this layout in the upper bits:
    /// - Bits 32-47: diversity (16 bits)
    /// - Bit 48: addrDiv (address diversity)
    /// - Bits 49-50: key (2 bits)
    /// - Bits 51-61: next (11 bits — not extracted here)
    /// - Bit 62: bind flag
    /// - Bit 63: auth flag
    ///
    /// For auth rebase, bits 0-31 are the target.
    /// For auth bind, bits 0-15 are the ordinal, bits 16-31 are zero.
    pub fn from_auth_entry(raw: u64) -> Self {
        let diversity = ((raw >> 32) & 0xFFFF) as u16;
        let addr_diversity = (raw >> 48) & 1 == 1;
        let key = PacKey::from_bits((raw >> 49) & 0x3);
        AuthMetadata {
            key,
            addr_diversity,
            diversity,
        }
    }

    /// Extract auth metadata from a bind entry (same layout as rebase for format 12).
    pub fn from_auth_bind(raw: u64) -> Self {
        Self::from_auth_entry(raw)
    }
}

/// Check if a raw u64 pointer value likely contains PAC bits.
///
/// On macOS, user-space addresses are typically below 0x0001_0000_0000_0000.
/// If the upper 16 bits contain non-zero values, they're likely PAC bits.
#[inline]
pub fn has_pac_bits(ptr: u64) -> bool {
    ptr & 0xFFFF_0000_0000_0000 != 0
}

/// Blend a pointer address and a 16-bit integer into a PAC discriminator.
///
/// Matches Apple's `ptrauth_blend_discriminator`:
///   `((addr & 0xFFFF) << 48) | diversity`
///
/// The low 16 bits of the pointer go into bits 48-63 and the 16-bit
/// diversity value goes into bits 0-15.
#[inline]
pub fn blend_discriminator(addr: u64, diversity: u16) -> u64 {
    ((addr & 0xFFFF) << 48) | (diversity as u64)
}

/// Sign a pointer with the specified PAC key and discriminator.
///
/// On ARM64 with PAC support (Apple Silicon), uses hardware PACIA/PACIB/PACDA/PACDB
/// instructions. On other architectures, returns the pointer unchanged.
///
/// # Arguments
/// * `value` — the raw pointer/address to sign
/// * `key` — which PAC key to use
/// * `slot_addr` — address of the GOT slot (for address diversity)
/// * `diversity` — 16-bit diversity value from the fixup entry
/// * `addr_div` — whether address diversity is enabled
///
/// # Safety
/// The caller must ensure `value` is a valid pointer for the target context.
pub unsafe fn pac_sign_pointer(
    value: u64,
    _key: PacKey,
    _slot_addr: u64,
    _diversity: u16,
    _addr_div: bool,
) -> u64 {
    // Our loader strips all PAC branches and signing instructions from the
    // guest binary (arm64e → arm64 downgrade).  Host code (glibc, libssl,
    // etc.) uses plain BLR and cannot handle PAC-signed pointers, so we
    // must never actually sign.
    value
}

// ── Runtime PAC detection ───────────────────────────────────────────

/// Returns `true` if the host CPU supports Pointer Authentication (PAC).
///
/// - macOS aarch64: always `true` (all Apple Silicon has PAC)
/// - Linux aarch64: checks `AT_HWCAP` for `HWCAP_PACA`
/// - Other architectures: `false`
pub fn has_hardware_pac() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(_detect_pac)
}

fn _detect_pac() -> bool {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        return true;
    }

    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    {
        // HWCAP_PACA = (1 << 30) from Linux asm/hwcap.h
        const HWCAP_PACA: libc::c_ulong = 1 << 30;
        let hwcap = unsafe { libc::getauxval(libc::AT_HWCAP) };
        return hwcap & HWCAP_PACA != 0;
    }

    #[allow(unreachable_code)]
    false
}

// ── PAC signing ─────────────────────────────────────────────────────

/// Sign a pointer using hardware PAC instructions (ARM64 only).
///
/// # Safety
/// Caller ensures `value` is a valid address and the CPU supports PAC.
#[cfg(target_arch = "aarch64")]
unsafe fn pac_sign_asm(value: u64, key: PacKey, modifier: u64) -> u64 {
    let result: u64;
    // `.arch_extension pauth` tells the assembler to accept PAC instructions
    // on targets where they aren't enabled by default (Linux aarch64).
    // On macOS aarch64 this is already enabled and the directive is a no-op.
    match key {
        PacKey::IA => {
            core::arch::asm!(
                ".arch_extension pauth",
                "pacia {ptr}, {mod}",
                ptr = inout(reg) value => result,
                mod = in(reg) modifier,
            );
        }
        PacKey::IB => {
            core::arch::asm!(
                ".arch_extension pauth",
                "pacib {ptr}, {mod}",
                ptr = inout(reg) value => result,
                mod = in(reg) modifier,
            );
        }
        PacKey::DA => {
            core::arch::asm!(
                ".arch_extension pauth",
                "pacda {ptr}, {mod}",
                ptr = inout(reg) value => result,
                mod = in(reg) modifier,
            );
        }
        PacKey::DB => {
            core::arch::asm!(
                ".arch_extension pauth",
                "pacdb {ptr}, {mod}",
                ptr = inout(reg) value => result,
                mod = in(reg) modifier,
            );
        }
    }
    result
}

/// Sign a pointer with a PAC key and pre-computed modifier.
///
/// On macOS aarch64, always signs (PAC always available).
/// On Linux aarch64, signs only when runtime detection confirms PAC.
/// On non-aarch64, returns the pointer unchanged.
///
/// # Safety
/// Caller ensures `value` is a valid address.
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
unsafe fn pac_sign_raw(value: u64, key: PacKey, modifier: u64) -> u64 {
    pac_sign_asm(value, key, modifier)
}

#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
unsafe fn pac_sign_raw(value: u64, key: PacKey, modifier: u64) -> u64 {
    if has_hardware_pac() {
        pac_sign_asm(value, key, modifier)
    } else {
        value
    }
}

#[cfg(not(target_arch = "aarch64"))]
unsafe fn pac_sign_raw(value: u64, _key: PacKey, _modifier: u64) -> u64 {
    value
}

// ── PAC branch stripping (fallback for non-PAC hardware) ───────────
//
// arm64e Mach-O binaries use authenticated branches (BRAA, BLRAA, etc.)
// in their stub trampolines.  When PAC hardware is available, we sign
// GOT pointers with PACIA/PACIB and the authenticated branches work.
//
// When PAC hardware is NOT available (QEMU, non-PAC Linux kernels), we
// leave GOT pointers unsigned and patch the guest binary's authenticated
// branch instructions to their unauthenticated equivalents.
//
// ARM64 "unconditional branch (register)" encoding:
//   bits[31:25] = 1101011   (group identifier)
//   bits[20:16] = 11111     (op2 — fixed for all variants)
//   bits[11:10]             (auth field: 00=none, 10=keyA, 11=keyB)
//
// Authenticated variants (BRAA/BRAB/BLRAA/BLRAB and Z forms) have
// bits[11:10] ≥ 2.  We convert them to their unauthenticated base
// (BR/BLR) by clearing:
//   - bit 24    (opc high bit distinguishing BRAA/BRAB from BR)
//   - bits[11:10] (auth discriminator)
//   - bits[4:0]   (Rm modifier register, becomes 00000 in non-auth)
//
// RETAA/RETAB (opc low-3 = 010) are replaced with RET (x30).

/// Test whether a 32-bit ARM64 instruction is an authenticated
/// branch-register instruction (BRAA, BRAB, BLRAA, BLRAB, or Z/RET forms).
#[inline]
fn is_pac_branch(insn: u32) -> bool {
    let group = (insn >> 25) & 0x7F; // bits[31:25]
    let op2 = (insn >> 16) & 0x1F; // bits[20:16]
    let auth = (insn >> 10) & 0x3; // bits[11:10]
    group == 0b1101011 && op2 == 0b11111 && auth >= 2
}

/// Convert an authenticated branch instruction to its unauthenticated
/// equivalent.  Caller must only pass instructions where `is_pac_branch`
/// returned true.
#[inline]
fn depac_branch(insn: u32) -> u32 {
    let opc_low3 = (insn >> 21) & 0x7; // bits[23:21]
    if opc_low3 == 0b010 {
        // RETAA / RETAB → RET (x30)
        return 0xD65F_03C0;
    }
    // BRAA/BRAB/BRAAZ/BRABZ → BR
    // BLRAA/BLRAB/BLRAAZ/BLRABZ → BLR
    // Clear bit 24 (opc high), bits[11:10] (auth), bits[4:0] (Rm).
    insn & !(0x0100_0000 | 0x0000_0C00 | 0x0000_001F)
}

/// Strip PAC-authenticated branches from all executable segments of a
/// mapped Mach-O image.
///
/// When PAC hardware is available (`has_hardware_pac() == true`), this is
/// a no-op — signed GOT pointers match the authenticated branches.
///
/// When PAC is absent, this patches BRAA/BLRAA/RETAA instructions to
/// BR/BLR/RET so the binary can execute with unsigned GOT pointers.
///
/// Must be called AFTER fixups are applied and BEFORE `protect()` makes
/// text segments read-only.
pub fn strip_pac_branches(image: &crate::MappedImage, cpu_type: CpuType) {
    if !should_strip_pac_branches(cpu_type) {
        return;
    }

    // Always strip PAC even on PAC hardware: the guest binary's signed
    // pointers cannot cross into host code (glibc qsort, libssl, etc.)
    // which uses plain BLR branches.
    let mut patched = 0u32;
    for seg in &image.segments {
        // Only patch executable segments (__TEXT and friends)
        if !seg.name.starts_with("__TEXT") {
            continue;
        }
        let count = seg.size / 4;
        let base = seg.base as *mut u32;
        for i in 0..count {
            let insn = unsafe { base.add(i).read_unaligned() };
            if is_pac_branch(insn) {
                let fixed = depac_branch(insn);
                unsafe {
                    base.add(i).write_unaligned(fixed);
                }
                patched += 1;
            } else if is_pac_data_insn(insn) {
                // NOP out PACIA/PACIB/PACDA/PACDB/AUTIA/AUTIB/AUTDA/AUTDB
                // and HINT-space variants (PACIASP, AUTIASP, etc.) so the
                // binary never signs or authenticates pointers.
                unsafe {
                    base.add(i).write_unaligned(NOP);
                }
                patched += 1;
            }
        }
    }
    if patched > 0 {
        log::info!("strip_pac_branches: patched {patched} PAC instructions (branches + data)");
    }
}

#[inline]
fn should_strip_pac_branches(cpu_type: CpuType) -> bool {
    matches!(cpu_type, CpuType::Arm64)
}

/// ARM64 NOP instruction.
const NOP: u32 = 0xD503_201F;

/// Test whether a 32-bit instruction is a PAC/AUT data-processing instruction.
///
/// Matches PACIA/PACIB/PACDA/PACDB/AUTIA/AUTIB/AUTDA/AUTDB and their Z
/// variants (Data-processing 1 source, opcodes 001_000..001_111), plus
/// HINT-space PAC instructions (PACIA1716, PACIASP, AUTIASP, etc.).
#[inline]
fn is_pac_data_insn(insn: u32) -> bool {
    // Data-processing (1 source) PAC/AUT family:
    //   bits[31:21] = 11011010110, bits[20:16] = 00001, bits[15:13] = 001
    //   Matches: PACIA..AUTDB and Z variants (opcode bits[12:10] = 000..111)
    if (insn & 0xFFFF_E000) == 0xDAC1_2000 {
        return true;
    }
    // HINT-space PAC instructions (function prologue/epilogue):
    //   PACIA1716  = HINT #8   = 0xD503_211F
    //   PACIB1716  = HINT #10  = 0xD503_215F
    //   AUTIA1716  = HINT #12  = 0xD503_219F
    //   AUTIB1716  = HINT #14  = 0xD503_21DF
    //   PACIASP    = HINT #25  = 0xD503_233F
    //   PACIBSP    = HINT #27  = 0xD503_237F
    //   AUTIASP    = HINT #29  = 0xD503_23BF
    //   AUTIBSP    = HINT #31  = 0xD503_23FF
    matches!(
        insn,
        0xD503_211F
            | 0xD503_215F
            | 0xD503_219F
            | 0xD503_21DF
            | 0xD503_233F
            | 0xD503_237F
            | 0xD503_23BF
            | 0xD503_23FF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_clean_pointer() {
        let addr = 0x0000_0001_0000_3F40u64;
        assert_eq!(ptrauth_strip(addr), addr);
    }

    #[test]
    fn strip_pac_signed_pointer() {
        // Simulate a pointer with PAC bits in the upper region
        let addr = 0x0000_0001_0000_3F40u64;
        let signed = addr | (0xA5B7u64 << 48); // fake PAC in bits 48-63
        assert_ne!(signed, addr);
        assert_eq!(ptrauth_strip(signed), addr);
    }

    #[test]
    fn strip_zero() {
        assert_eq!(ptrauth_strip(0), 0);
    }

    #[test]
    fn pac_key_from_bits() {
        assert_eq!(PacKey::from_bits(0), PacKey::IA);
        assert_eq!(PacKey::from_bits(1), PacKey::IB);
        assert_eq!(PacKey::from_bits(2), PacKey::DA);
        assert_eq!(PacKey::from_bits(3), PacKey::DB);
    }

    #[test]
    fn has_pac_bits_detection() {
        assert!(!has_pac_bits(0x0000_0001_0000_3F40));
        assert!(has_pac_bits(0x00A5_0001_0000_3F40));
        assert!(has_pac_bits(0x8000_0000_0000_0000));
    }

    #[test]
    fn auth_metadata_from_rebase() {
        // Simulate auth rebase: key=IA (0), addrDiv=1, diversity=0x1234
        // Bits 32-47 = diversity = 0x1234
        // Bit 48 = addrDiv = 1
        // Bits 49-50 = key = 0b00 (IA)
        // Bit 63 = auth = 1
        let raw: u64 = (0x1234u64 << 32) | (1u64 << 48) | (1u64 << 63);
        let meta = AuthMetadata::from_auth_entry(raw);
        assert_eq!(meta.key, PacKey::IA);
        assert!(meta.addr_diversity);
        assert_eq!(meta.diversity, 0x1234);
    }

    #[test]
    fn auth_metadata_from_bind() {
        // Simulate auth bind: key=DA (2), addrDiv=0, diversity=0xABCD
        // Bits 32-47 = diversity = 0xABCD
        // Bit 48 = addrDiv = 0
        // Bits 49-50 = key = 0b10 (DA)
        // Bit 62 = bind = 1
        // Bit 63 = auth = 1
        let raw: u64 = (0xABCDu64 << 32) | (2u64 << 49) | (1u64 << 62) | (1u64 << 63);
        let meta = AuthMetadata::from_auth_bind(raw);
        assert_eq!(meta.key, PacKey::DA);
        assert!(!meta.addr_diversity);
        assert_eq!(meta.diversity, 0xABCD);
    }

    #[test]
    fn strip_pac_branches_only_runs_for_arm64_images() {
        assert!(should_strip_pac_branches(CpuType::Arm64));
        assert!(!should_strip_pac_branches(CpuType::X86_64));
        assert!(!should_strip_pac_branches(CpuType::Arm));
        assert!(!should_strip_pac_branches(CpuType::X86));
    }
}
