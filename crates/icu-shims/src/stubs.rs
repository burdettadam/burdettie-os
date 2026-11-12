//! Remaining ICU stubs for symbols that don't have direct ICU4X equivalents.
//!
//! These cover uregex_*, uenum_*, uregion_* — APIs where
//! ICU4X uses a different architecture. Date and number formatting
//! are now handled by the `datetime` and `decimal` modules.

use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

/// U_UNSUPPORTED_ERROR
const U_UNSUPPORTED_ERROR: i32 = 13;
/// U_ZERO_ERROR
const U_ZERO_ERROR: i32 = 0;

pub fn install_remaining_stubs(table: &mut ShimTable) {
    // ── uregex (regex) ──────────────────────────────────────
    // ICU4X doesn't wrap ICU regex — Rust `regex` crate is used instead.
    register_shims!(
        table,
        (
            "_uregex_open",
            shim_uregex_open as *const () as u64,
            "uregex_open (stub → NULL + U_UNSUPPORTED_ERROR)"
        ),
        (
            "_uregex_openC",
            shim_uregex_open_c as *const () as u64,
            "uregex_openC (stub → NULL)"
        ),
        (
            "_uregex_close",
            shim_void1 as *const () as u64,
            "uregex_close (stub)"
        ),
        (
            "_uregex_setText",
            shim_void4 as *const () as u64,
            "uregex_setText (stub)"
        ),
        (
            "_uregex_find",
            shim_false2 as *const () as u64,
            "uregex_find (stub → false)"
        ),
        (
            "_uregex_findNext",
            shim_false1 as *const () as u64,
            "uregex_findNext (stub → false)"
        ),
        (
            "_uregex_group",
            shim_null3 as *const () as u64,
            "uregex_group (stub → NULL)"
        ),
        (
            "_uregex_groupCount",
            shim_zero1 as *const () as u64,
            "uregex_groupCount (stub → 0)"
        ),
        (
            "_uregex_start",
            shim_zero2 as *const () as u64,
            "uregex_start (stub → 0)"
        ),
        (
            "_uregex_end",
            shim_zero2 as *const () as u64,
            "uregex_end (stub → 0)"
        ),
        (
            "_uregex_reset",
            shim_void2 as *const () as u64,
            "uregex_reset (stub)"
        ),
        (
            "_uregex_matches",
            shim_false2 as *const () as u64,
            "uregex_matches (stub → false)"
        ),
        (
            "_uregex_replaceAll",
            shim_zero4 as *const () as u64,
            "uregex_replaceAll (stub → 0)"
        ),
        (
            "_uregex_replaceFirst",
            shim_zero4 as *const () as u64,
            "uregex_replaceFirst (stub → 0)"
        ),
        (
            "_uregex_pattern",
            shim_null2 as *const () as u64,
            "uregex_pattern (stub → NULL)"
        )
    );

    // ── uregion ─────────────────────────────────────────────
    register_shims!(
        table,
        (
            "_uregion_getRegionFromCode",
            shim_null1 as *const () as u64,
            "uregion_getRegionFromCode (stub → NULL)"
        ),
        (
            "_uregion_getContainedRegions",
            shim_null2 as *const () as u64,
            "uregion_getContainedRegions (stub → NULL)"
        ),
        (
            "_uregion_getNumericCode",
            shim_zero1 as *const () as u64,
            "uregion_getNumericCode (stub → 0)"
        )
    );

    // ── uenum (enumeration) ─────────────────────────────────
    register_shims!(
        table,
        (
            "_uenum_close",
            shim_void1 as *const () as u64,
            "uenum_close (stub)"
        ),
        (
            "_uenum_count",
            shim_uenum_count as *const () as u64,
            "uenum_count (stub → 0)"
        ),
        (
            "_uenum_next",
            shim_uenum_next as *const () as u64,
            "uenum_next (stub → NULL)"
        ),
        (
            "_uenum_reset",
            shim_void2 as *const () as u64,
            "uenum_reset (stub)"
        )
    );
}

// ── Generic stubs ───────────────────────────────────────────

pub extern "C" fn shim_void1(_a: u64) {}
pub extern "C" fn shim_void2(_a: u64, _b: u64) {}
pub extern "C" fn shim_void4(_a: u64, _b: u64, _c: u64, _d: u64) {}
pub extern "C" fn shim_null1(_a: u64) -> u64 {
    0
}
pub extern "C" fn shim_null2(_a: u64, _b: u64) -> u64 {
    0
}
pub extern "C" fn shim_null3(_a: u64, _b: u64, _c: u64) -> u64 {
    0
}
pub extern "C" fn shim_null5(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64) -> u64 {
    0
}
pub extern "C" fn shim_null6(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64) -> u64 {
    0
}
pub extern "C" fn shim_zero1(_a: u64) -> i32 {
    0
}
pub extern "C" fn shim_zero2(_a: u64, _b: u64) -> i32 {
    0
}
pub extern "C" fn shim_zero4(_a: u64, _b: u64, _c: u64, _d: u64) -> i32 {
    0
}
pub extern "C" fn shim_false1(_a: u64) -> i32 {
    0
}
pub extern "C" fn shim_false2(_a: u64, _b: u64) -> i32 {
    0
}

// ── Specific stubs ──────────────────────────────────────────

pub extern "C" fn shim_uregex_open(
    _pattern: u64,
    _len: i32,
    _flags: u32,
    _pe: u64,
    status: *mut i32,
) -> u64 {
    if !status.is_null() {
        unsafe {
            *status = U_UNSUPPORTED_ERROR;
        }
    }
    0
}

pub extern "C" fn shim_uregex_open_c(
    _pattern: u64,
    _flags: u32,
    _pe: u64,
    status: *mut i32,
) -> u64 {
    if !status.is_null() {
        unsafe {
            *status = U_UNSUPPORTED_ERROR;
        }
    }
    0
}

pub extern "C" fn shim_uenum_count(_en: u64, status: *mut i32) -> i32 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    0
}

pub extern "C" fn shim_uenum_next(_en: u64, _result_len: *mut i32, status: *mut i32) -> u64 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsystem_shims::table::ShimTable;

    #[test]
    fn remaining_stubs_registered() {
        let mut table = ShimTable::new();
        install_remaining_stubs(&mut table);
        assert!(table.resolve("_uregex_open").is_some());
        assert!(table.resolve("_uenum_count").is_some());
    }

    #[test]
    fn uregex_open_returns_error() {
        let mut status = 0i32;
        let result = shim_uregex_open(0, 0, 0, 0, &mut status);
        assert_eq!(result, 0);
        assert_eq!(status, U_UNSUPPORTED_ERROR);
    }
}
