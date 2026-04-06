//! `vproc.h` / `vproc_priv.h` — process lifecycle management API.
//!
//! Implements the public `vproc_transaction_begin/end` and
//! `vproc_standby_begin/end` APIs, plus the commonly-used private
//! `_vproc_*` and `vproc_swap_*` functions.

use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::constants::VprocGsk;

// ── Opaque handle types ─────────────────────────────────────────────

/// Opaque vproc handle.  In Apple's implementation this references a
/// server-side process record.  We use a sentinel pointer.
pub type VprocT = *mut libc::c_void;
/// Error type.  NULL = success, non-NULL = error.
pub type VprocErrT = *const libc::c_void;

/// Null (success) error.
pub const VPROC_ERR_NONE: VprocErrT = std::ptr::null();
/// Generic error sentinel.
static VPROC_ERR_GENERIC: u8 = 1;

fn err_generic() -> VprocErrT {
    &VPROC_ERR_GENERIC as *const u8 as VprocErrT
}

// ── Transaction tracking ────────────────────────────────────────────

/// Outstanding transaction count.  Positive = dirty, zero = clean.
static TRANSACTION_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Standby count.
static STANDBY_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Whether transactions are enabled for this process.
static TRANSACTIONS_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

// ── Opaque transaction/standby handle IDs ───────────────────────────

static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(1);

fn alloc_handle() -> usize {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

// ═══════════════════════════════════════════════════════════════════
//  Public API — vproc.h
// ═══════════════════════════════════════════════════════════════════

/// `vproc_strerror(err)` → string.
pub fn vproc_strerror(err: VprocErrT) -> &'static str {
    if err.is_null() {
        "Success"
    } else {
        "vproc error"
    }
}

/// Opaque transaction handle.
pub type VprocTransactionT = *mut libc::c_void;

/// `vproc_transaction_begin(vp)` → handle.
pub fn vproc_transaction_begin(_vp: VprocT) -> VprocTransactionT {
    let id = alloc_handle();
    TRANSACTION_COUNT.fetch_add(1, Ordering::Relaxed);
    log::debug!("vproc_transaction_begin → handle {}", id);
    id as VprocTransactionT
}

/// `vproc_transaction_end(vp, handle)`.
pub fn vproc_transaction_end(_vp: VprocT, _handle: VprocTransactionT) {
    let prev = TRANSACTION_COUNT.fetch_sub(1, Ordering::Relaxed);
    log::debug!("vproc_transaction_end (was {})", prev);
}

/// Opaque standby handle.
pub type VprocStandbyT = *mut libc::c_void;

/// `vproc_standby_begin(vp)` → handle.
pub fn vproc_standby_begin(_vp: VprocT) -> VprocStandbyT {
    let id = alloc_handle();
    STANDBY_COUNT.fetch_add(1, Ordering::Relaxed);
    id as VprocStandbyT
}

/// `vproc_standby_end(vp, handle)`.
pub fn vproc_standby_end(_vp: VprocT, _handle: VprocStandbyT) {
    STANDBY_COUNT.fetch_sub(1, Ordering::Relaxed);
}

// ═══════════════════════════════════════════════════════════════════
//  Private API — vproc_priv.h
// ═══════════════════════════════════════════════════════════════════

/// `vprocmgr_lookup_vproc(label)` → vproc_t.
pub fn vprocmgr_lookup_vproc(_label: *const libc::c_char) -> VprocT {
    // Return a non-null sentinel so callers don't think it failed.
    1usize as VprocT
}

/// `vproc_retain(vp)` → vp.
pub fn vproc_retain(vp: VprocT) -> VprocT {
    vp
}

/// `vproc_release(vp)`.
pub fn vproc_release(_vp: VprocT) {}

// ── Swap operations ─────────────────────────────────────────────────

/// Global swap integer state.
static SWAP_STATE: OnceLock<Mutex<SwapState>> = OnceLock::new();

struct SwapState {
    last_exit_status: i32,
    global_on_demand: bool,
    mgr_uid: i32,
    mgr_pid: i32,
    is_managed: bool,
    mgr_name: String,
    idle_timeout: i64,
    exit_timeout: i64,
    transactions_enabled: bool,
}

fn with_swap<R>(f: impl FnOnce(&mut SwapState) -> R) -> R {
    let mutex = SWAP_STATE.get_or_init(|| {
        Mutex::new(SwapState {
            last_exit_status: 0,
            global_on_demand: true,
            mgr_uid: unsafe { libc::getuid() } as i32,
            mgr_pid: unsafe { libc::getpid() },
            is_managed: true,
            mgr_name: "com.apple.xpc.launchd".to_string(),
            idle_timeout: 0,
            exit_timeout: 20,
            transactions_enabled: false,
        })
    });
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

/// `vproc_swap_integer(vp, key, inval, outval)` → vproc_err_t.
pub fn vproc_swap_integer(
    _vp: VprocT,
    key: u32,
    inval: *const i64,
    outval: *mut i64,
) -> VprocErrT {
    let Some(gsk) = gsk_from_raw(key) else {
        return err_generic();
    };

    with_swap(|state| {
        // Read current value if outval requested
        if !outval.is_null() {
            let val = match gsk {
                VprocGsk::LastExitStatus => i64::from(state.last_exit_status),
                VprocGsk::GlobalOnDemand => i64::from(state.global_on_demand),
                VprocGsk::MgrUid => i64::from(state.mgr_uid),
                VprocGsk::MgrPid => i64::from(state.mgr_pid),
                VprocGsk::IsManaged => i64::from(state.is_managed),
                VprocGsk::IdleTimeout => state.idle_timeout,
                VprocGsk::ExitTimeout => state.exit_timeout,
                VprocGsk::TransactionsEnabled => i64::from(state.transactions_enabled),
                _ => 0,
            };
            unsafe { *outval = val; }
        }

        // Write new value if inval provided
        if !inval.is_null() {
            let val = unsafe { *inval };
            match gsk {
                VprocGsk::LastExitStatus => state.last_exit_status = val as i32,
                VprocGsk::GlobalOnDemand => state.global_on_demand = val != 0,
                VprocGsk::IdleTimeout => state.idle_timeout = val,
                VprocGsk::ExitTimeout => state.exit_timeout = val,
                VprocGsk::TransactionsEnabled => {
                    state.transactions_enabled = val != 0;
                    TRANSACTIONS_ENABLED.store(val != 0, Ordering::Relaxed);
                }
                _ => {} // read-only keys are no-ops for writes
            }
        }
    });

    VPROC_ERR_NONE
}

/// `vproc_swap_complex(vp, key, inval, outval)` → vproc_err_t.
pub fn vproc_swap_complex(
    _vp: VprocT,
    key: u32,
    _inval: crate::launch_data::LaunchDataT,
    outval: *mut crate::launch_data::LaunchDataT,
) -> VprocErrT {
    let Some(gsk) = gsk_from_raw(key) else {
        return err_generic();
    };

    match gsk {
        VprocGsk::Environment => {
            // Return a dictionary of environment variables
            if !outval.is_null() {
                use crate::launch_data::*;
                let dict = alloc(LaunchDataType::Dictionary as u32);
                for (k, v) in std::env::vars() {
                    let c_key = std::ffi::CString::new(k).unwrap_or_default();
                    let c_val = std::ffi::CString::new(v).unwrap_or_default();
                    dict_insert(dict, new_string(c_val.as_ptr()), c_key.as_ptr());
                }
                unsafe { *outval = dict; }
            }
            VPROC_ERR_NONE
        }
        VprocGsk::AllJobs => {
            // Return job registry as a dictionary
            if !outval.is_null() {
                use crate::launch_data::*;
                let dict = alloc(LaunchDataType::Dictionary as u32);
                for label in crate::job::get_all_job_labels() {
                    let c_key = std::ffi::CString::new(label).unwrap_or_default();
                    dict_insert(dict, new_integer(0), c_key.as_ptr());
                }
                unsafe { *outval = dict; }
            }
            VPROC_ERR_NONE
        }
        _ => err_generic(),
    }
}

/// `vproc_swap_string(vp, key, instr, outstr)` → vproc_err_t.
pub fn vproc_swap_string(
    _vp: VprocT,
    key: u32,
    _instr: *const libc::c_char,
    outstr: *mut *mut libc::c_char,
) -> VprocErrT {
    let Some(gsk) = gsk_from_raw(key) else {
        return err_generic();
    };

    match gsk {
        VprocGsk::MgrName => {
            if !outstr.is_null() {
                let name = with_swap(|s| s.mgr_name.clone());
                let cs = std::ffi::CString::new(name).unwrap_or_default();
                unsafe {
                    let ptr = libc::strdup(cs.as_ptr());
                    *outstr = ptr;
                }
            }
            VPROC_ERR_NONE
        }
        _ => err_generic(),
    }
}

// ── Convenience private APIs ────────────────────────────────────────

/// Last exit status tracking.
static LAST_EXIT_STATUS: AtomicI32 = AtomicI32::new(0);

/// `_vproc_get_last_exit_status(&wstatus)` → vproc_err_t.
pub fn _vproc_get_last_exit_status(wstatus: *mut i32) -> VprocErrT {
    if wstatus.is_null() {
        return err_generic();
    }
    unsafe { *wstatus = LAST_EXIT_STATUS.load(Ordering::Relaxed); }
    VPROC_ERR_NONE
}

/// `_vproc_set_global_on_demand(val)` → vproc_err_t.
pub fn _vproc_set_global_on_demand(_val: bool) -> VprocErrT {
    VPROC_ERR_NONE
}

/// `_vproc_send_signal_by_label(label, sig)` → vproc_err_t.
pub fn _vproc_send_signal_by_label(
    label: *const libc::c_char,
    sig: i32,
) -> VprocErrT {
    if label.is_null() {
        return err_generic();
    }
    let cs = unsafe { std::ffi::CStr::from_ptr(label) };
    if let Ok(label_str) = cs.to_str() {
        log::debug!("_vproc_send_signal_by_label(\"{}\", {})", label_str, sig);
    }
    // In emulation, we can't actually send signals to managed jobs.
    VPROC_ERR_NONE
}

/// `_vproc_kickstart_by_label(label, &pid, &port, &obsrvr, flags)` → vproc_err_t.
pub fn _vproc_kickstart_by_label(
    _label: *const libc::c_char,
    _out_pid: *mut i32,
    _out_port: *mut u32,
    _out_obsrvr: *mut u32,
    _flags: u32,
) -> VprocErrT {
    err_generic() // not supported in emulation
}

/// `_vproc_transaction_begin()`.
pub fn _vproc_transaction_begin() {
    TRANSACTION_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// `_vproc_transaction_end()`.
pub fn _vproc_transaction_end() {
    TRANSACTION_COUNT.fetch_sub(1, Ordering::Relaxed);
}

/// `_vproc_transaction_count()` → count.
pub fn _vproc_transaction_count() -> usize {
    TRANSACTION_COUNT.load(Ordering::Relaxed)
}

/// `_vproc_transactions_enable()`.
pub fn _vproc_transactions_enable() {
    TRANSACTIONS_ENABLED.store(true, Ordering::Relaxed);
}

/// `_vproc_standby_begin()`.
pub fn _vproc_standby_begin() {
    STANDBY_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// `_vproc_standby_end()`.
pub fn _vproc_standby_end() {
    STANDBY_COUNT.fetch_sub(1, Ordering::Relaxed);
}

/// `_vproc_standby_count()` → count.
pub fn _vproc_standby_count() -> usize {
    STANDBY_COUNT.load(Ordering::Relaxed)
}

/// `_vproc_standby_timeout()` → seconds.
pub fn _vproc_standby_timeout() -> usize {
    0 // no timeout in emulation
}

/// `_vproc_transaction_count_for_pid(pid, &count, &condemned)` → kern_return_t.
pub fn _vproc_transaction_count_for_pid(
    _pid: i32,
    count: *mut i32,
    condemned: *mut bool,
) -> u32 {
    if !count.is_null() {
        unsafe { *count = TRANSACTION_COUNT.load(Ordering::Relaxed) as i32; }
    }
    if !condemned.is_null() {
        unsafe { *condemned = false; }
    }
    0 // KERN_SUCCESS
}

/// `_vproc_pid_is_managed(pid)` → bool.
pub fn _vproc_pid_is_managed(_pid: i32) -> bool {
    true // in our emulation, everything is "managed"
}

/// `_vproc_transaction_try_exit(status)`.
pub fn _vproc_transaction_try_exit(status: i32) {
    LAST_EXIT_STATUS.store(status, Ordering::Relaxed);
    // In real launchd this might actually exit; we just record the status
}

/// `_vprocmgr_move_subset_to_user(uid, session_type, flags)` → vproc_err_t.
pub fn _vprocmgr_move_subset_to_user(
    _uid: u32,
    _session_type: *const libc::c_char,
    _flags: u64,
) -> VprocErrT {
    VPROC_ERR_NONE
}

/// `_vprocmgr_switch_to_session(session, flags)` → vproc_err_t.
pub fn _vprocmgr_switch_to_session(
    _session: *const libc::c_char,
    _flags: u32,
) -> VprocErrT {
    VPROC_ERR_NONE
}

/// `_vprocmgr_detach_from_console(flags)` → vproc_err_t.
pub fn _vprocmgr_detach_from_console(_flags: u32) -> VprocErrT {
    VPROC_ERR_NONE
}

// ── Set last exit status (for testing) ──────────────────────────────

pub fn set_last_exit_status(status: i32) {
    LAST_EXIT_STATUS.store(status, Ordering::Relaxed);
}

// ── Helpers ─────────────────────────────────────────────────────────

fn gsk_from_raw(raw: u32) -> Option<VprocGsk> {
    // Safety: we check bounds manually. This matches the enum discriminants.
    if raw <= VprocGsk::EmbeddedRootEquivalent as u32 {
        // Safe: all values 0..=27 are valid VprocGsk variants
        Some(unsafe { std::mem::transmute::<u32, VprocGsk>(raw) })
    } else {
        None
    }
}

/// Reset vproc state for testing.
pub fn reset_state() {
    TRANSACTION_COUNT.store(0, Ordering::Relaxed);
    STANDBY_COUNT.store(0, Ordering::Relaxed);
    TRANSACTIONS_ENABLED.store(false, Ordering::Relaxed);
    LAST_EXIT_STATUS.store(0, Ordering::Relaxed);
    NEXT_HANDLE.store(1, Ordering::Relaxed);
}

// ═══════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        reset_state();
    }

    #[test]
    fn transaction_begin_end() {
        setup();
        assert_eq!(_vproc_transaction_count(), 0);
        let h = vproc_transaction_begin(std::ptr::null_mut());
        assert!(!h.is_null());
        assert_eq!(_vproc_transaction_count(), 1);
        vproc_transaction_end(std::ptr::null_mut(), h);
        assert_eq!(_vproc_transaction_count(), 0);
    }

    #[test]
    fn multiple_transactions() {
        setup();
        _vproc_transaction_begin();
        _vproc_transaction_begin();
        _vproc_transaction_begin();
        assert_eq!(_vproc_transaction_count(), 3);
        _vproc_transaction_end();
        assert_eq!(_vproc_transaction_count(), 2);
    }

    #[test]
    fn standby_begin_end() {
        setup();
        assert_eq!(_vproc_standby_count(), 0);
        _vproc_standby_begin();
        assert_eq!(_vproc_standby_count(), 1);
        _vproc_standby_end();
        assert_eq!(_vproc_standby_count(), 0);
    }

    #[test]
    fn swap_integer_last_exit_status() {
        setup();
        let mut out: i64 = -1;
        let r = vproc_swap_integer(
            std::ptr::null_mut(),
            VprocGsk::LastExitStatus as u32,
            std::ptr::null(),
            &mut out,
        );
        assert!(r.is_null()); // success
        assert_eq!(out, 0);
    }

    #[test]
    fn swap_integer_write_and_read() {
        setup();
        let new_val: i64 = 42;
        vproc_swap_integer(
            std::ptr::null_mut(),
            VprocGsk::IdleTimeout as u32,
            &new_val,
            std::ptr::null_mut(),
        );

        let mut out: i64 = 0;
        vproc_swap_integer(
            std::ptr::null_mut(),
            VprocGsk::IdleTimeout as u32,
            std::ptr::null(),
            &mut out,
        );
        assert_eq!(out, 42);
    }

    #[test]
    fn swap_string_mgr_name() {
        let mut outstr: *mut libc::c_char = std::ptr::null_mut();
        let r = vproc_swap_string(
            std::ptr::null_mut(),
            VprocGsk::MgrName as u32,
            std::ptr::null(),
            &mut outstr,
        );
        assert!(r.is_null());
        assert!(!outstr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(outstr) }.to_str().unwrap_or("");
        assert_eq!(name, "com.apple.xpc.launchd");
        unsafe { libc::free(outstr as *mut libc::c_void); }
    }

    #[test]
    fn last_exit_status() {
        setup();
        _vproc_transaction_try_exit(7);
        let mut wstatus: i32 = 0;
        let r = _vproc_get_last_exit_status(&mut wstatus);
        assert!(r.is_null());
        assert_eq!(wstatus, 7);
    }

    #[test]
    fn pid_is_managed() {
        assert!(_vproc_pid_is_managed(1));
    }

    #[test]
    fn strerror_works() {
        assert_eq!(vproc_strerror(VPROC_ERR_NONE), "Success");
        assert_eq!(vproc_strerror(err_generic()), "vproc error");
    }

    #[test]
    fn lookup_vproc_returns_non_null() {
        let vp = vprocmgr_lookup_vproc(c"com.test".as_ptr());
        assert!(!vp.is_null());
        vproc_retain(vp);
        vproc_release(vp);
    }

    #[test]
    fn enable_transactions() {
        setup();
        _vproc_transactions_enable();
        assert!(TRANSACTIONS_ENABLED.load(Ordering::Relaxed));
    }

    #[test]
    fn transaction_count_for_pid() {
        setup();
        _vproc_transaction_begin();
        _vproc_transaction_begin();
        let mut count: i32 = 0;
        let mut condemned = true;
        _vproc_transaction_count_for_pid(1, &mut count, &mut condemned);
        assert_eq!(count, 2);
        assert!(!condemned);
    }
}
