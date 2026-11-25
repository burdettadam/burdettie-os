//! Class reference for `___CFConstantStringClassReference`.
//!
//! When the compiler sees `@"hello"`, it emits a `__CFConstantString`
//! struct whose ISA field references `___CFConstantStringClassReference`.
//! This must be a valid address so ISA reads don't segfault.

use std::cell::UnsafeCell;

/// A wrapper around a byte array that can be safely shared across threads.
/// Class registration writes pointer fields, and ARM64 ISA normalization
/// clears the low three bits, so the class address must be eight-byte aligned.
#[repr(C, align(8))]
struct SyncArray(UnsafeCell<[u8; 40]>);

// SAFETY: Mutation is guarded by `std::sync::Once`; after init the data is read-only.
unsafe impl Sync for SyncArray {}

/// Fallback fake class struct (40 bytes of zeros, ISA points to self).
static CFCONSTANT_STRING_CLASS: SyncArray = SyncArray(UnsafeCell::new([0u8; 40]));

static CFSTRING_INIT: std::sync::Once = std::sync::Once::new();

pub(crate) fn cfconstant_string_class_addr() -> u64 {
    CFSTRING_INIT.call_once(|| {
        // SAFETY: Mutation is guarded by Once; the data is write-once.
        unsafe {
            let ptr = CFCONSTANT_STRING_CLASS.0.get() as *mut u8;
            let self_addr = ptr as u64;
            std::ptr::copy_nonoverlapping(self_addr.to_le_bytes().as_ptr(), ptr, 8);
        }
    });
    CFCONSTANT_STRING_CLASS.0.get() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfstring;
    use libsystem_shims::table::ShimTable;

    #[test]
    fn class_storage_has_pointer_alignment() {
        assert_eq!(std::mem::size_of::<SyncArray>(), 40);
        assert_eq!(std::mem::offset_of!(SyncArray, 0), 0);
        // The type must guarantee alignment even when a particular linker
        // happens to place byte-aligned storage at an aligned address.
        assert!(std::mem::align_of::<SyncArray>() >= std::mem::align_of::<u64>());

        let class = cfconstant_string_class_addr();
        assert_eq!(class & 7, 0);
        // SAFETY: the Once initializer has published the live class record.
        let isa = unsafe { std::ptr::read_unaligned(class as *const u64) };
        assert_eq!(isa, class);
    }

    fn assert_string_identity(obj: u64, class: u64, expected: &str) {
        // SAFETY: both callers supply a live, complete CFString record.
        let isa = unsafe { std::ptr::read_unaligned(obj as *const u64) };
        assert_eq!(isa, class);
        // Exercise the ARM64 ISA invariant on every test host, including
        // hosts whose production recognition path keeps the raw ISA.
        assert_eq!(isa & 0x007f_ffff_ffff_fff8, class);
        assert_eq!(
            cfstring::shim_cf_string_get_length(obj),
            expected.len() as i64
        );
        assert_eq!(cfstring::extract_cfstring_cstr(obj), Some(expected));
        assert!(!cfstring::shim_cf_string_get_cstring_ptr(
            obj,
            cfstring::K_CF_STRING_ENCODING_UTF8,
        )
        .is_null());
    }

    #[test]
    fn class_symbol_preserves_constant_and_created_string_identity() {
        let mut table = ShimTable::new();
        crate::foundation::FoundationShims::install(&mut table);
        let class = table.resolve("___CFConstantStringClassReference").unwrap();
        assert_eq!(class, cfconstant_string_class_addr());

        // Match the compiler-emitted 32-byte constant-string record without
        // relying on a platform compiler or a native Foundation framework.
        let literal = c"constant string";
        let record = [
            class,
            u64::from(cfstring::CF_STRING_FLAGS_UTF8),
            literal.as_ptr() as u64,
            literal.to_bytes().len() as u64,
        ];
        assert_string_identity(record.as_ptr() as u64, class, "constant string");

        let created = cfstring::shim_cf_string_create_with_cstring(
            0,
            c"created string".as_ptr(),
            cfstring::K_CF_STRING_ENCODING_UTF8,
        );
        assert_ne!(created, 0);
        assert_string_identity(created, class, "created string");

        // SAFETY: the constructor allocated this record and transferred a
        // CString into its payload field; neither is shared outside the test.
        unsafe {
            let payload = std::ptr::read_unaligned((created + 16) as *const u64);
            drop(std::ffi::CString::from_raw(
                payload as *mut std::ffi::c_char,
            ));
        }
        crate::pal_alloc().free(created as *mut u8);
    }
}
