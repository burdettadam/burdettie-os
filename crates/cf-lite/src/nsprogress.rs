//! NSProgress shim: progress reporting object.
//!
//! Object layout (48 bytes):
//!   offset  0: isa             (u64)
//!   offset  8: total_count     (i64)
//!   offset 16: completed_count (i64)
//!   offset 24: cancellable     (u64 — bool)
//!   offset 32: cancelled       (u64 — bool)
//!   offset 40: desc_ptr        (u64 — NSString)

use crate::cfstring;
use crate::nsobj;

const TOTAL_OFF: usize = 8;
const COMPLETED_OFF: usize = 16;
const CANCELLABLE_OFF: usize = 24;
const CANCELLED_OFF: usize = 32;
const DESC_OFF: usize = 40;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn alloc_progress(total: i64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 48) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsprogress_class_addr();
        *((obj as usize + TOTAL_OFF) as *mut i64) = total;
        *((obj as usize + CANCELLABLE_OFF) as *mut u64) = 1; // default cancellable
    }
    obj
}

// ── Class methods ──────────────────────────────────────────────

/// `+[NSProgress progressWithTotalUnitCount:]`
pub extern "C" fn shim_nsprogress_with_total(
    _cls: u64, _cmd: u64, total: i64,
) -> u64 {
    alloc_progress(total)
}

/// `+[NSProgress discreteProgressWithTotalUnitCount:]`
pub extern "C" fn shim_nsprogress_discrete(
    _cls: u64, _cmd: u64, total: i64,
) -> u64 {
    alloc_progress(total)
}

// ── Instance methods ───────────────────────────────────────────

pub extern "C" fn shim_nsprogress_total_unit_count(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + TOTAL_OFF) as *const i64) }
}

pub extern "C" fn shim_nsprogress_set_total_unit_count(obj: u64, _cmd: u64, val: i64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + TOTAL_OFF) as *mut i64) = val; }
    0
}

pub extern "C" fn shim_nsprogress_completed_unit_count(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + COMPLETED_OFF) as *const i64) }
}

pub extern "C" fn shim_nsprogress_set_completed_unit_count(obj: u64, _cmd: u64, val: i64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + COMPLETED_OFF) as *mut i64) = val; }
    0
}

/// `-[NSProgress fractionCompleted]` — returns double (0.0..1.0).
pub extern "C" fn shim_nsprogress_fraction_completed(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let total = unsafe { *((obj as usize + TOTAL_OFF) as *const i64) };
    let completed = unsafe { *((obj as usize + COMPLETED_OFF) as *const i64) };
    let frac: f64 = if total > 0 { completed as f64 / total as f64 } else { 0.0 };
    frac.to_bits()
}

pub extern "C" fn shim_nsprogress_is_cancellable(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + CANCELLABLE_OFF) as *const u64) as u8 }
}

pub extern "C" fn shim_nsprogress_set_cancellable(obj: u64, _cmd: u64, val: u8) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + CANCELLABLE_OFF) as *mut u64) = val as u64; }
    0
}

pub extern "C" fn shim_nsprogress_is_cancelled(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + CANCELLED_OFF) as *const u64) as u8 }
}

/// `-[NSProgress cancel]`
pub extern "C" fn shim_nsprogress_cancel(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + CANCELLED_OFF) as *mut u64) = 1; }
    0
}

pub extern "C" fn shim_nsprogress_localized_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let desc = unsafe { *((obj as usize + DESC_OFF) as *const u64) };
    if desc != 0 { return desc; }
    let total = shim_nsprogress_total_unit_count(obj, 0);
    let completed = shim_nsprogress_completed_unit_count(obj, 0);
    make_nsstring(&format!("{completed} of {total}"))
}

pub extern "C" fn shim_nsprogress_set_localized_description(obj: u64, _cmd: u64, val: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + DESC_OFF) as *mut u64) = val; }
    0
}

pub extern "C" fn shim_nsprogress_description(obj: u64, _cmd: u64) -> u64 {
    shim_nsprogress_localized_description(obj, 0)
}
