//! Class reference for `___CFConstantStringClassReference`.
//!
//! When the compiler sees `@"hello"`, it emits a `__CFConstantString`
//! struct whose ISA field references `___CFConstantStringClassReference`.
//! This must be a valid address so ISA reads don't segfault.

use std::cell::UnsafeCell;

/// A wrapper around a byte array that can be safely shared across threads.
#[repr(transparent)]
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
