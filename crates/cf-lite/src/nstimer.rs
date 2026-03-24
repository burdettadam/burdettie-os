//! NSTimer shim: single-threaded timer with block invocation.
//!
//! Object layout: 32 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  interval (f64 bits) — time interval in seconds
//!   [16..24): block_ptr (u64) — block to invoke on fire
//!   [24..32): flags (u64) — bit 0 = valid, bit 1 = repeats
//!
//! Since we're single-threaded with no run loop, `scheduledTimerWithTimeInterval:`
//! creates the timer but does NOT automatically fire it. The fixture must
//! call `[timer fire]` explicitly. This is sufficient for testing.

use crate::cfstring;
use crate::nsobj;

const NSTIMER_SIZE: usize = 32;
const FLAG_VALID: u64 = 1;
const FLAG_REPEATS: u64 = 2;

fn alloc_nstimer(interval: f64, block: u64, repeats: bool) -> u64 {
    let obj = unsafe { libc::calloc(1, NSTIMER_SIZE) } as *mut u64;
    if obj.is_null() {
        return 0;
    }
    let flags = FLAG_VALID | if repeats { FLAG_REPEATS } else { 0 };
    unsafe {
        *obj = nsobj::nstimer_class_addr();
        *obj.add(1) = interval.to_bits();
        *obj.add(2) = block;
        *obj.add(3) = flags;
    }
    obj as u64
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSTimer scheduledTimerWithTimeInterval:repeats:block:]`
///
/// Creates a timer; does NOT actually schedule it (no run loop).
/// The block signature is `void (^)(NSTimer *)`.
pub extern "C" fn shim_nstimer_scheduled_with_block(
    _cls: u64,
    _cmd: u64,
    interval: f64,
    repeats: u64,
    block: u64,
) -> u64 {
    alloc_nstimer(interval, block, repeats != 0)
}

/// `+[NSTimer timerWithTimeInterval:repeats:block:]`
pub extern "C" fn shim_nstimer_timer_with_block(
    _cls: u64,
    _cmd: u64,
    interval: f64,
    repeats: u64,
    block: u64,
) -> u64 {
    alloc_nstimer(interval, block, repeats != 0)
}

/// `+[NSTimer scheduledTimerWithTimeInterval:target:selector:userInfo:repeats:]`
///
/// Target/selector variant — we store the target pointer in the block field
/// and the selector can't easily be captured, so this is a no-op stub.
pub extern "C" fn shim_nstimer_scheduled_with_target(
    _cls: u64,
    _cmd: u64,
    interval: f64,
    _target: u64,
    _sel: u64,
    _info: u64,
    repeats: u64,
) -> u64 {
    alloc_nstimer(interval, 0, repeats != 0)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSTimer fire]` — invokes the block immediately.
pub extern "C" fn shim_nstimer_fire(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let flags = unsafe { *((obj as *const u64).add(3)) };
    if flags & FLAG_VALID == 0 {
        return; // invalidated
    }
    let block = unsafe { *((obj as *const u64).add(2)) };
    if block != 0 {
        // Block invoke pointer is at offset 16 of the block struct
        let invoke = unsafe { *((block as *const u64).add(2)) };
        if invoke != 0 {
            // Call: invoke(block, timer)
            let f: extern "C" fn(u64, u64) = unsafe { std::mem::transmute(invoke) };
            f(block, obj);
        }
    }
    // If non-repeating, auto-invalidate after fire
    if flags & FLAG_REPEATS == 0 {
        unsafe {
            *((obj as *mut u64).add(3)) = flags & !FLAG_VALID;
        }
    }
}

/// `-[NSTimer invalidate]` — marks the timer as invalid.
pub extern "C" fn shim_nstimer_invalidate(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let flags = unsafe { *((obj as *const u64).add(3)) };
    unsafe {
        *((obj as *mut u64).add(3)) = flags & !FLAG_VALID;
    }
}

/// `-[NSTimer isValid]`
pub extern "C" fn shim_nstimer_is_valid(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = unsafe { *((obj as *const u64).add(3)) };
    u8::from(flags & FLAG_VALID != 0)
}

/// `-[NSTimer timeInterval]`
pub extern "C" fn shim_nstimer_time_interval(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    let bits = unsafe { *((obj as *const u64).add(1)) };
    f64::from_bits(bits)
}

/// `-[NSTimer description]`
pub extern "C" fn shim_nstimer_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let interval = shim_nstimer_time_interval(obj, _cmd);
    let valid = shim_nstimer_is_valid(obj, _cmd) != 0;
    let desc = format!("<NSTimer: interval={interval}, valid={valid}>");
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}
