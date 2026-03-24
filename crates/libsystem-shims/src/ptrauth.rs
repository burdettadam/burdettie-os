//! Pointer authentication (PAC/ARM64e) shims.
//!
//! On real ARM64e hardware, `ptrauth_strip` removes the PAC signature
//! from a pointer and `ptrauth_sign_unauthenticated` adds one. In our
//! lab environment these are no-ops since we run in-process without
//! pointer signing.

use crate::table::ShimTable;

/// Install pointer authentication shims.
pub struct PtrauthShims;

impl PtrauthShims {
    /// Register PAC shims in the table.
    pub fn install(table: &mut ShimTable) {
        table.register(
            "_ptrauth_strip",
            shim_ptrauth_strip as u64,
            "ptrauth_strip → identity (no PAC in lab)",
        );
        table.register(
            "_ptrauth_sign_unauthenticated",
            shim_ptrauth_sign_unauthenticated as u64,
            "ptrauth_sign_unauthenticated → identity (no PAC in lab)",
        );

        log::debug!("PtrauthShims: registered 2 ptrauth shims");
    }
}

/// `void *ptrauth_strip(void *ptr, ptrauth_key key)` — strip PAC bits.
///
/// In the lab we simply return the pointer unchanged since all
/// pointers are already clean (PAC bits are stripped during fixup
/// application).
extern "C" fn shim_ptrauth_strip(ptr: u64, _key: u32) -> u64 {
    ptr & 0x0000_FFFF_FFFF_FFFF
}

/// `void *ptrauth_sign_unauthenticated(void *ptr, ptrauth_key key, void *disc)`
///
/// In the lab environment signing is a no-op — we return the pointer
/// as-is.
extern "C" fn shim_ptrauth_sign_unauthenticated(ptr: u64, _key: u32, _disc: u64) -> u64 {
    ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ptrauth_shims_registered() {
        let mut table = ShimTable::new();
        PtrauthShims::install(&mut table);
        assert!(table.resolve("_ptrauth_strip").is_some());
        assert!(table.resolve("_ptrauth_sign_unauthenticated").is_some());
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn strip_returns_lower_48() {
        let result = shim_ptrauth_strip(0x0080_1234_5678_9ABC, 0);
        assert_eq!(result, 0x0000_1234_5678_9ABC);
    }

    #[test]
    fn sign_returns_identity() {
        let result = shim_ptrauth_sign_unauthenticated(0x1234, 0, 0);
        assert_eq!(result, 0x1234);
    }
}
