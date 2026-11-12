//! Real collation shims backed by icu_collator.
//!
//! Provides Unicode-aware string comparison using ICU4X's collation engine.

use icu_collator::options::CollatorOptions;
use icu_collator::{Collator, CollatorBorrowed};
use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

/// Create a default collator from compiled data.
/// With compiled_data, creation is essentially pointer setup to baked-in statics.
fn default_collator() -> CollatorBorrowed<'static> {
    Collator::try_new(Default::default(), CollatorOptions::default()).unwrap_or_else(|e| {
        log::warn!("icu-shims: failed to create default collator: {e}, using fallback");
        Collator::try_new(Default::default(), CollatorOptions::default())
            .expect("icu-shims: default collator must succeed")
    })
}

pub fn install(table: &mut ShimTable) {
    register_shims!(
        table,
        (
            "_ucol_open",
            shim_ucol_open as *const () as u64,
            "ucol_open (icu4x → real collator)"
        ),
        (
            "_ucol_close",
            shim_ucol_close as *const () as u64,
            "ucol_close (icu4x)"
        ),
        (
            "_ucol_strcoll",
            shim_ucol_strcoll as *const () as u64,
            "ucol_strcoll (icu4x → real comparison)"
        ),
        (
            "_ucol_getSortKey",
            shim_ucol_get_sort_key as *const () as u64,
            "ucol_getSortKey (icu4x → stub)"
        )
    );
}

/// ucol_open(locale, status) → UCollator*
/// We return a sentinel since we use the global default collator.
static COLLATOR_SENTINEL: u64 = 0x4943_5543_4F4C;

pub extern "C" fn shim_ucol_open(_locale: u64, status: *mut i32) -> u64 {
    // Force initialization of collator
    let _ = default_collator();
    if !status.is_null() {
        unsafe {
            *status = 0;
        } // U_ZERO_ERROR
    }
    &COLLATOR_SENTINEL as *const u64 as u64
}

pub extern "C" fn shim_ucol_close(_coll: u64) {
    // No-op — we use a global collator
}

/// ucol_strcoll(coll, source, sourceLen, target, targetLen) → UCollationResult
/// Returns: UCOL_LESS = -1, UCOL_EQUAL = 0, UCOL_GREATER = 1
pub extern "C" fn shim_ucol_strcoll(
    _coll: u64,
    src: *const u16,
    src_len: i32,
    tgt: *const u16,
    tgt_len: i32,
) -> i32 {
    if src.is_null() || tgt.is_null() {
        return 0;
    }

    let collator = default_collator();

    let src_slice = if src_len < 0 {
        // Null-terminated — find length
        unsafe { read_u16_until_null(src) }
    } else {
        unsafe { std::slice::from_raw_parts(src, src_len as usize) }
    };

    let tgt_slice = if tgt_len < 0 {
        unsafe { read_u16_until_null(tgt) }
    } else {
        unsafe { std::slice::from_raw_parts(tgt, tgt_len as usize) }
    };

    // Convert UTF-16 to String for ICU4X comparison
    let src_str = String::from_utf16_lossy(src_slice);
    let tgt_str = String::from_utf16_lossy(tgt_slice);

    match collator.compare(&src_str, &tgt_str) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// ucol_getSortKey(coll, source, sourceLen, result, resultLen) → int32_t
/// Stub — returns 0 (no sort key generated). Full sort keys require more work.
pub extern "C" fn shim_ucol_get_sort_key(
    _coll: u64,
    _src: u64,
    _src_len: i32,
    _result: u64,
    _result_len: i32,
) -> i32 {
    0
}

/// Read UTF-16 code units until a null terminator (0x0000).
unsafe fn read_u16_until_null<'a>(ptr: *const u16) -> &'a [u16] {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
        if len > 1_000_000 {
            break;
        } // safety cap
    }
    std::slice::from_raw_parts(ptr, len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collator_initializes() {
        let c = default_collator();
        assert_eq!(c.compare("abc", "abc"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn ucol_open_returns_non_null() {
        let mut status = 0i32;
        let handle = shim_ucol_open(0, &mut status);
        assert_ne!(handle, 0);
        assert_eq!(status, 0);
    }

    #[test]
    fn ucol_strcoll_compares_correctly() {
        let a: Vec<u16> = "apple".encode_utf16().collect();
        let b: Vec<u16> = "banana".encode_utf16().collect();
        let result = shim_ucol_strcoll(0, a.as_ptr(), a.len() as i32, b.as_ptr(), b.len() as i32);
        assert_eq!(result, -1); // apple < banana

        let result2 = shim_ucol_strcoll(0, b.as_ptr(), b.len() as i32, a.as_ptr(), a.len() as i32);
        assert_eq!(result2, 1); // banana > apple

        let result3 = shim_ucol_strcoll(0, a.as_ptr(), a.len() as i32, a.as_ptr(), a.len() as i32);
        assert_eq!(result3, 0); // apple == apple
    }
}
