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
    /// For auth entries (bit 63 set), the layout is:
    /// - Bits 49-50: key (2 bits)
    /// - Bit 51: addrDiv (address diversity)
    /// - Bits 52-63: diversity (16 bits, but only 12 available in some formats)
    ///
    /// For format 1/12 auth bind:
    /// - Bits 0-15: ordinal
    /// - Bits 16-31: diversity (16 bits)
    /// - Bits 32-42: next
    /// - Bits 43-44: key
    /// - Bit 45: addrDiv
    ///
    /// For format 1/12 auth rebase:
    /// - Bits 0-31: target
    /// - Bits 32-42: next
    /// - Bits 43-44: key
    /// - Bit 45: addrDiv
    /// - Bits 46-61: diversity
    pub fn from_auth_entry(raw: u64) -> Self {
        let key = PacKey::from_bits((raw >> 43) & 0x3);
        let addr_diversity = (raw >> 45) & 1 == 1;
        let diversity = ((raw >> 46) & 0xFFFF) as u16;
        AuthMetadata {
            key,
            addr_diversity,
            diversity,
        }
    }

    /// Extract auth metadata from a bind entry where diversity is at bits 16-31.
    pub fn from_auth_bind(raw: u64) -> Self {
        let key = PacKey::from_bits((raw >> 43) & 0x3);
        let addr_diversity = (raw >> 45) & 1 == 1;
        let diversity = ((raw >> 16) & 0xFFFF) as u16;
        AuthMetadata {
            key,
            addr_diversity,
            diversity,
        }
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
        // Simulate: key=IA (0), addrDiv=1, diversity=0x1234
        // Bits 43-44 = 0b00, bit 45 = 1, bits 46-61 = 0x1234
        let raw: u64 = (0u64 << 43) | (1u64 << 45) | (0x1234u64 << 46) | (1u64 << 63);
        let meta = AuthMetadata::from_auth_entry(raw);
        assert_eq!(meta.key, PacKey::IA);
        assert!(meta.addr_diversity);
        assert_eq!(meta.diversity, 0x1234);
    }

    #[test]
    fn auth_metadata_from_bind() {
        // Simulate auth bind: key=IA (0), addrDiv=1, diversity at bits 16-31 = 0x0000
        let raw: u64 = (0u64 << 43) | (1u64 << 45) | (1u64 << 63) | (1u64 << 62);
        let meta = AuthMetadata::from_auth_bind(raw);
        assert_eq!(meta.key, PacKey::IA);
        assert!(meta.addr_diversity);
        assert_eq!(meta.diversity, 0);
    }
}
