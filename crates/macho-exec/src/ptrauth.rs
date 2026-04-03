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
///   `(uint64_t(diversity) << 48) | (addr & 0xFFFF)`
#[inline]
pub fn blend_discriminator(addr: u64, diversity: u16) -> u64 {
    ((diversity as u64) << 48) | (addr & 0xFFFF)
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
    key: PacKey,
    slot_addr: u64,
    diversity: u16,
    addr_div: bool,
) -> u64 {
    if value == 0 {
        return 0;
    }
    let modifier = if addr_div {
        blend_discriminator(slot_addr, diversity)
    } else {
        diversity as u64
    };
    pac_sign_raw(value, key, modifier)
}

/// Sign a pointer with a PAC key and pre-computed modifier.
///
/// # Safety
/// Caller ensures `value` is a valid address.
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
unsafe fn pac_sign_raw(value: u64, key: PacKey, modifier: u64) -> u64 {
    let result: u64;
    match key {
        PacKey::IA => {
            core::arch::asm!(
                "pacia {ptr}, {mod}",
                ptr = inout(reg) value => result,
                mod = in(reg) modifier,
            );
        }
        PacKey::IB => {
            core::arch::asm!(
                "pacib {ptr}, {mod}",
                ptr = inout(reg) value => result,
                mod = in(reg) modifier,
            );
        }
        PacKey::DA => {
            core::arch::asm!(
                "pacda {ptr}, {mod}",
                ptr = inout(reg) value => result,
                mod = in(reg) modifier,
            );
        }
        PacKey::DB => {
            core::arch::asm!(
                "pacdb {ptr}, {mod}",
                ptr = inout(reg) value => result,
                mod = in(reg) modifier,
            );
        }
    }
    result
}

#[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
unsafe fn pac_sign_raw(value: u64, _key: PacKey, _modifier: u64) -> u64 {
    // No hardware PAC support; return unsigned pointer.
    value
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
}
