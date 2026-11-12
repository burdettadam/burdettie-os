//! ICU (International Components for Unicode) stub shims.
//!
//! These provide minimal stub implementations for ICU functions that
//! guest binaries import. On a fully configured system with libicu
//! installed, these could delegate to real ICU; for now they return
//! empty/error results that won't crash callers.
//!
//! ICU error code: U_ZERO_ERROR = 0, U_UNSUPPORTED_ERROR = 13

use crate::table::ShimTable;

pub struct IcuShims;

/// U_ZERO_ERROR
const U_ZERO_ERROR: i32 = 0;
/// U_UNSUPPORTED_ERROR
const U_UNSUPPORTED_ERROR: i32 = 13;

impl IcuShims {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── uregex (regex) ──────────────────────────────────────
        register_shims!(
            table,
            (
                "_uregex_open",
                shim_uregex_open as *const () as u64,
                "uregex_open (stub → NULL)"
            ),
            (
                "_uregex_openC",
                shim_uregex_open_c as *const () as u64,
                "uregex_openC (stub → NULL)"
            ),
            (
                "_uregex_close",
                shim_icu_void1 as *const () as u64,
                "uregex_close (stub → void)"
            ),
            (
                "_uregex_setText",
                shim_icu_void4 as *const () as u64,
                "uregex_setText (stub → void)"
            ),
            (
                "_uregex_find",
                shim_icu_false2 as *const () as u64,
                "uregex_find (stub → false)"
            ),
            (
                "_uregex_findNext",
                shim_icu_false1 as *const () as u64,
                "uregex_findNext (stub → false)"
            ),
            (
                "_uregex_group",
                shim_icu_null3 as *const () as u64,
                "uregex_group (stub → NULL)"
            ),
            (
                "_uregex_groupCount",
                shim_icu_zero1 as *const () as u64,
                "uregex_groupCount (stub → 0)"
            ),
            (
                "_uregex_start",
                shim_icu_zero2 as *const () as u64,
                "uregex_start (stub → 0)"
            ),
            (
                "_uregex_end",
                shim_icu_zero2 as *const () as u64,
                "uregex_end (stub → 0)"
            ),
            (
                "_uregex_reset",
                shim_icu_void2 as *const () as u64,
                "uregex_reset (stub → void)"
            ),
            (
                "_uregex_matches",
                shim_icu_false2 as *const () as u64,
                "uregex_matches (stub → false)"
            ),
            (
                "_uregex_replaceAll",
                shim_icu_zero4 as *const () as u64,
                "uregex_replaceAll (stub → 0)"
            ),
            (
                "_uregex_replaceFirst",
                shim_icu_zero4 as *const () as u64,
                "uregex_replaceFirst (stub → 0)"
            ),
            (
                "_uregex_pattern",
                shim_icu_null2 as *const () as u64,
                "uregex_pattern (stub → NULL)"
            )
        );

        // ── uregion ─────────────────────────────────────────────
        register_shims!(
            table,
            (
                "_uregion_getRegionFromCode",
                shim_icu_null1 as *const () as u64,
                "uregion_getRegionFromCode (stub → NULL)"
            ),
            (
                "_uregion_getContainedRegions",
                shim_icu_null2 as *const () as u64,
                "uregion_getContainedRegions (stub → NULL)"
            ),
            (
                "_uregion_getNumericCode",
                shim_icu_zero1 as *const () as u64,
                "uregion_getNumericCode (stub → 0)"
            )
        );

        // ── ucol (collation) ────────────────────────────────────
        register_shims!(
            table,
            (
                "_ucol_open",
                shim_ucol_open as *const () as u64,
                "ucol_open (stub → NULL)"
            ),
            (
                "_ucol_close",
                shim_icu_void1 as *const () as u64,
                "ucol_close (stub → void)"
            ),
            (
                "_ucol_strcoll",
                shim_ucol_strcoll as *const () as u64,
                "ucol_strcoll (stub → 0/equal)"
            ),
            (
                "_ucol_getSortKey",
                shim_icu_zero4 as *const () as u64,
                "ucol_getSortKey (stub → 0)"
            )
        );

        // ── uenum (enumeration) ─────────────────────────────────
        register_shims!(
            table,
            (
                "_uenum_close",
                shim_icu_void1 as *const () as u64,
                "uenum_close (stub → void)"
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
                shim_icu_void2 as *const () as u64,
                "uenum_reset (stub → void)"
            )
        );

        // ── udat (date formatting) ──────────────────────────────
        register_shims!(
            table,
            (
                "_udat_open",
                shim_icu_null6 as *const () as u64,
                "udat_open (stub → NULL)"
            ),
            (
                "_udat_close",
                shim_icu_void1 as *const () as u64,
                "udat_close (stub → void)"
            ),
            (
                "_udat_format",
                shim_icu_zero4 as *const () as u64,
                "udat_format (stub → 0)"
            ),
            (
                "_udat_parse",
                shim_icu_zero4 as *const () as u64,
                "udat_parse (stub → 0)"
            )
        );

        // ── unum (number formatting) ────────────────────────────
        register_shims!(
            table,
            (
                "_unum_open",
                shim_icu_null5 as *const () as u64,
                "unum_open (stub → NULL)"
            ),
            (
                "_unum_close",
                shim_icu_void1 as *const () as u64,
                "unum_close (stub → void)"
            ),
            (
                "_unum_formatInt64",
                shim_icu_zero4 as *const () as u64,
                "unum_formatInt64 (stub → 0)"
            ),
            (
                "_unum_formatDouble",
                shim_icu_zero4 as *const () as u64,
                "unum_formatDouble (stub → 0)"
            ),
            (
                "_unum_parseToNumber",
                shim_icu_void4 as *const () as u64,
                "unum_parseToNumber (stub → void)"
            )
        );

        // ── uloc (locale) ───────────────────────────────────────
        register_shims!(
            table,
            (
                "_uloc_getDefault",
                shim_uloc_get_default as *const () as u64,
                "uloc_getDefault (stub → en_US)"
            ),
            (
                "_uloc_getName",
                shim_icu_zero4 as *const () as u64,
                "uloc_getName (stub → 0)"
            ),
            (
                "_uloc_getLanguage",
                shim_icu_zero4 as *const () as u64,
                "uloc_getLanguage (stub → 0)"
            ),
            (
                "_uloc_getCountry",
                shim_icu_zero4 as *const () as u64,
                "uloc_getCountry (stub → 0)"
            ),
            (
                "_uloc_canonicalize",
                shim_icu_zero4 as *const () as u64,
                "uloc_canonicalize (stub → 0)"
            )
        );

        // ── ubrk (break iterator) ──────────────────────────────
        register_shims!(
            table,
            (
                "_ubrk_open",
                shim_icu_null4 as *const () as u64,
                "ubrk_open (stub → NULL)"
            ),
            (
                "_ubrk_close",
                shim_icu_void1 as *const () as u64,
                "ubrk_close (stub → void)"
            ),
            (
                "_ubrk_setText",
                shim_icu_void3 as *const () as u64,
                "ubrk_setText (stub → void)"
            ),
            (
                "_ubrk_first",
                shim_icu_zero1 as *const () as u64,
                "ubrk_first (stub → 0)"
            ),
            (
                "_ubrk_next",
                shim_ubrk_next as *const () as u64,
                "ubrk_next (stub → UBRK_DONE)"
            )
        );

        log::debug!("IcuShims: registered 44 ICU stub shims");
    }
}

// ── Generic ICU stubs ───────────────────────────────────────────

pub extern "C" fn shim_icu_void1(_a: u64) {}
pub extern "C" fn shim_icu_void2(_a: u64, _b: u64) {}
pub extern "C" fn shim_icu_void3(_a: u64, _b: u64, _c: u64) {}
pub extern "C" fn shim_icu_void4(_a: u64, _b: u64, _c: u64, _d: u64) {}
pub extern "C" fn shim_icu_null1(_a: u64) -> u64 {
    0
}
pub extern "C" fn shim_icu_null2(_a: u64, _b: u64) -> u64 {
    0
}
pub extern "C" fn shim_icu_null3(_a: u64, _b: u64, _c: u64) -> u64 {
    0
}
pub extern "C" fn shim_icu_null4(_a: u64, _b: u64, _c: u64, _d: u64) -> u64 {
    0
}
pub extern "C" fn shim_icu_null5(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64) -> u64 {
    0
}
pub extern "C" fn shim_icu_null6(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64) -> u64 {
    0
}
pub extern "C" fn shim_icu_zero1(_a: u64) -> i32 {
    0
}
pub extern "C" fn shim_icu_zero2(_a: u64, _b: u64) -> i32 {
    0
}
pub extern "C" fn shim_icu_zero4(_a: u64, _b: u64, _c: u64, _d: u64) -> i32 {
    0
}
pub extern "C" fn shim_icu_false1(_a: u64) -> i32 {
    0
}
pub extern "C" fn shim_icu_false2(_a: u64, _b: u64) -> i32 {
    0
}

// ── Specific stubs ──────────────────────────────────────────────

/// uregex_open(pattern, patternLength, flags, pe, status) → URegularExpression*
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
    0 // NULL
}

/// uregex_openC(pattern, flags, pe, status) → URegularExpression*
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

/// ucol_open(locale, status) → UCollator*
pub extern "C" fn shim_ucol_open(_locale: u64, status: *mut i32) -> u64 {
    if !status.is_null() {
        unsafe {
            *status = U_UNSUPPORTED_ERROR;
        }
    }
    0
}

/// ucol_strcoll(coll, source, sourceLen, target, targetLen) → UCollationResult
/// Returns 0 = UCOL_EQUAL
pub extern "C" fn shim_ucol_strcoll(
    _coll: u64,
    _src: u64,
    _src_len: i32,
    _tgt: u64,
    _tgt_len: i32,
) -> i32 {
    0 // UCOL_EQUAL
}

/// uenum_count(en, status) → int32_t
pub extern "C" fn shim_uenum_count(_en: u64, status: *mut i32) -> i32 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    0
}

/// uenum_next(en, resultLength, status) → const char*
pub extern "C" fn shim_uenum_next(_en: u64, _result_len: *mut i32, status: *mut i32) -> u64 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    0 // NULL (no more elements)
}

/// Default locale string.
static DEFAULT_LOCALE: &[u8] = b"en_US\0";

/// uloc_getDefault() → const char*
pub extern "C" fn shim_uloc_get_default() -> u64 {
    DEFAULT_LOCALE.as_ptr() as u64
}

/// UBRK_DONE = -1
pub extern "C" fn shim_ubrk_next(_bi: u64) -> i32 {
    -1 // UBRK_DONE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::ShimTable;

    #[test]
    fn icu_shims_registered() {
        let mut table = ShimTable::new();
        IcuShims::install(&mut table);
        assert!(table.resolve("_uregex_open").is_some());
        assert!(table.resolve("_ucol_open").is_some());
        assert!(table.resolve("_uloc_getDefault").is_some());
        assert!(table.resolve("_ubrk_next").is_some());
    }

    #[test]
    fn uloc_get_default_returns_en_us() {
        let ptr = shim_uloc_get_default() as *const u8;
        let s = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::ffi::c_char) };
        assert_eq!(s.to_str().unwrap(), "en_US");
    }

    #[test]
    fn uregex_open_sets_error() {
        let mut status: i32 = 0;
        let result = shim_uregex_open(0, 0, 0, 0, &mut status);
        assert_eq!(result, 0);
        assert_eq!(status, U_UNSUPPORTED_ERROR);
    }

    #[test]
    fn ubrk_next_returns_done() {
        assert_eq!(shim_ubrk_next(0), -1);
    }
}
