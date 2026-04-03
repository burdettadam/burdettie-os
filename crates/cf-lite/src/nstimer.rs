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
    let obj = crate::pal_alloc().calloc(1, NSTIMER_SIZE) as *mut u64;
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
/// Creates a timer and schedules it on the current run loop.
/// The block signature is `void (^)(NSTimer *)`.
pub extern "C" fn shim_nstimer_scheduled_with_block(
    _cls: u64,
    _cmd: u64,
    interval: f64,
    repeats: u64,
    block: u64,
) -> u64 {
    let t = alloc_nstimer(interval, block, repeats != 0);
    if t != 0 {
        crate::nsrunloop::shim_nsrunloop_add_timer_for_mode(0, 0, t, 0);
    }
    t
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
/// Target/selector variant — creates and schedules on the current run loop.
/// We store the target pointer in the block field; the selector cannot be
/// captured so invoking the target is a no-op.
pub extern "C" fn shim_nstimer_scheduled_with_target(
    _cls: u64,
    _cmd: u64,
    interval: f64,
    _target: u64,
    _sel: u64,
    _info: u64,
    repeats: u64,
) -> u64 {
    let t = alloc_nstimer(interval, 0, repeats != 0);
    if t != 0 {
        crate::nsrunloop::shim_nsrunloop_add_timer_for_mode(0, 0, t, 0);
    }
    t
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

/// `-[NSTimer fireDate]` — returns an NSDate representing when the timer fires.
///
/// Simplified: returns an NSDate with the current time.
pub extern "C" fn shim_nstimer_fire_date(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::nsdate::shim_nsdate_date(0, 0)
}

/// `-[NSTimer setFireDate:]` — no-op stub.
pub extern "C" fn shim_nstimer_set_fire_date(_obj: u64, _cmd: u64, _date: u64) {}

/// `-[NSTimer tolerance]` — returns 0.0.
pub extern "C" fn shim_nstimer_tolerance(_obj: u64, _cmd: u64) -> f64 {
    0.0
}

/// `-[NSTimer setTolerance:]` — no-op stub.
pub extern "C" fn shim_nstimer_set_tolerance(_obj: u64, _cmd: u64, _tolerance: f64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn timer_with_block_is_valid() {
        let t = shim_nstimer_timer_with_block(0, 0, 1.5, 0, 0);
        assert_ne!(t, 0);
        assert_eq!(shim_nstimer_is_valid(t, 0), 1);
        let interval = shim_nstimer_time_interval(t, 0);
        assert!((interval - 1.5).abs() < 1e-9, "expected 1.5 got {interval}");
        free_obj(t);
    }

    #[test]
    fn scheduled_with_block_also_valid() {
        let t = shim_nstimer_scheduled_with_block(0, 0, 2.0, 1, 0);
        assert_ne!(t, 0);
        assert_eq!(shim_nstimer_is_valid(t, 0), 1);
        assert!((shim_nstimer_time_interval(t, 0) - 2.0).abs() < 1e-9);
        free_obj(t);
    }

    #[test]
    fn time_interval_nil_returns_zero() {
        assert!((shim_nstimer_time_interval(0, 0)).abs() < 1e-9);
    }

    #[test]
    fn is_valid_nil_returns_zero() {
        assert_eq!(shim_nstimer_is_valid(0, 0), 0);
    }

    #[test]
    fn invalidate_clears_valid_flag() {
        let t = shim_nstimer_timer_with_block(0, 0, 0.5, 0, 0);
        assert_eq!(shim_nstimer_is_valid(t, 0), 1);
        shim_nstimer_invalidate(t, 0);
        assert_eq!(shim_nstimer_is_valid(t, 0), 0);
        free_obj(t);
    }

    #[test]
    fn fire_nil_is_noop() {
        shim_nstimer_fire(0, 0); // must not crash
    }

    #[test]
    fn fire_non_repeating_without_block_auto_invalidates() {
        // repeats=0, block=0: fire should auto-invalidate a non-repeating timer
        let t = shim_nstimer_scheduled_with_block(0, 0, 0.1, 0, 0);
        assert_eq!(shim_nstimer_is_valid(t, 0), 1);
        shim_nstimer_fire(t, 0);
        assert_eq!(shim_nstimer_is_valid(t, 0), 0, "non-repeating timer must auto-invalidate after fire");
        free_obj(t);
    }

    #[test]
    fn scheduled_with_target_stores_interval() {
        let t = shim_nstimer_scheduled_with_target(0, 0, 5.0, 0, 0, 0, 0);
        assert_ne!(t, 0);
        assert_eq!(shim_nstimer_is_valid(t, 0), 1);
        assert!((shim_nstimer_time_interval(t, 0) - 5.0).abs() < 1e-9);
        free_obj(t);
    }

    #[test]
    fn description_returns_nonempty_string() {
        let t = shim_nstimer_timer_with_block(0, 0, 1.0, 0, 0);
        let desc = shim_nstimer_description(t, 0);
        assert_ne!(desc, 0);
        free_obj(t);
    }

    #[test]
    fn description_nil_returns_zero() {
        assert_eq!(shim_nstimer_description(0, 0), 0);
    }

    #[test]
    fn nstimer_create_valid() {
        let t = shim_nstimer_scheduled_with_block(0, 0, 1.0, 0, 0);
        assert_ne!(t, 0);
        assert_eq!(shim_nstimer_is_valid(t, 0), 1);
        free_obj(t);
    }

    #[test]
    fn nstimer_invalidate_makes_invalid() {
        let t = shim_nstimer_timer_with_block(0, 0, 0.5, 0, 0);
        assert_eq!(shim_nstimer_is_valid(t, 0), 1);
        shim_nstimer_invalidate(t, 0);
        assert_eq!(shim_nstimer_is_valid(t, 0), 0);
        free_obj(t);
    }

    #[test]
    fn fire_date_returns_nsdate() {
        let t = shim_nstimer_timer_with_block(0, 0, 1.0, 0, 0);
        let date = shim_nstimer_fire_date(t, 0);
        assert_ne!(date, 0, "fireDate should return a non-nil NSDate");
        free_obj(t);
    }

    #[test]
    fn fire_date_nil_returns_zero() {
        assert_eq!(shim_nstimer_fire_date(0, 0), 0);
    }

    #[test]
    fn set_fire_date_noop() {
        let t = shim_nstimer_timer_with_block(0, 0, 1.0, 0, 0);
        shim_nstimer_set_fire_date(t, 0, 0); // must not crash
        assert_eq!(shim_nstimer_is_valid(t, 0), 1);
        free_obj(t);
    }

    #[test]
    fn tolerance_returns_zero() {
        let t = shim_nstimer_timer_with_block(0, 0, 1.0, 0, 0);
        assert!((shim_nstimer_tolerance(t, 0)).abs() < 1e-9);
        free_obj(t);
    }

    #[test]
    fn set_tolerance_noop() {
        let t = shim_nstimer_timer_with_block(0, 0, 1.0, 0, 0);
        shim_nstimer_set_tolerance(t, 0, 0.5); // must not crash
        free_obj(t);
    }
}
