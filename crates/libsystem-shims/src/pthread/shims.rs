//! Thin pthread shim functions that delegate to the host OS.

/// macOS `_PTHREAD_MUTEX_SIG_init` — the 4-byte signature Darwin writes
/// at the start of every `pthread_mutex_t` to mark it as initialised.
/// Linux interprets those same bytes as lock-state / owner, causing
/// deadlocks when we hand a macOS-format mutex to `pthread_mutex_lock`.
const DARWIN_PTHREAD_MUTEX_SIG_INIT: u32 = 0x32AA_ABA7;

/// If `mutex` starts with the macOS signature, zero it and call
/// `pthread_mutex_init` so Linux gets a valid mutex.
#[cfg(not(target_os = "macos"))]
unsafe fn maybe_reinit_darwin_mutex(mutex: *mut libc::pthread_mutex_t) {
    let sig = (mutex as *const u32).read_unaligned();
    if sig == DARWIN_PTHREAD_MUTEX_SIG_INIT {
        log::debug!(
            "pthread_mutex: detected macOS sig {sig:#x}, reinitializing for Linux"
        );
        std::ptr::write_bytes(mutex as *mut u8, 0, std::mem::size_of::<libc::pthread_mutex_t>());
        libc::pthread_mutex_init(mutex, std::ptr::null());
    }
}

#[cfg(target_os = "macos")]
unsafe fn maybe_reinit_darwin_mutex(_mutex: *mut libc::pthread_mutex_t) {
    // On macOS the signature is expected — nothing to do.
}

// ── Thread lifecycle ────────────────────────────────────────────────

/// `int pthread_create(pthread_t *, const pthread_attr_t *, start_routine, void *arg)`
pub(super) extern "C" fn shim_pthread_create(
    thread: *mut libc::pthread_t,
    attr: *const libc::pthread_attr_t,
    start_routine: Option<extern "C" fn(*mut libc::c_void) -> *mut libc::c_void>,
    arg: *mut libc::c_void,
) -> libc::c_int {
    let Some(f) = start_routine else {
        return libc::EINVAL;
    };
    log::trace!("pthread_create: spawning new thread");
    unsafe { libc::pthread_create(thread, attr, f, arg) }
}

/// `int pthread_join(pthread_t thread, void **retval)`
pub(super) extern "C" fn shim_pthread_join(
    thread: libc::pthread_t,
    retval: *mut *mut libc::c_void,
) -> libc::c_int {
    log::trace!("pthread_join: waiting for thread");
    unsafe { libc::pthread_join(thread, retval) }
}

/// `int pthread_detach(pthread_t thread)`
pub(super) extern "C" fn shim_pthread_detach(thread: libc::pthread_t) -> libc::c_int {
    unsafe { libc::pthread_detach(thread) }
}

/// `void pthread_exit(void *retval)` — never returns.
pub(super) extern "C" fn shim_pthread_exit(retval: *mut libc::c_void) {
    unsafe { libc::pthread_exit(retval) }
}

/// `pthread_t pthread_self(void)`
pub(super) extern "C" fn shim_pthread_self() -> libc::pthread_t {
    unsafe { libc::pthread_self() }
}

/// `int pthread_equal(pthread_t t1, pthread_t t2)`
pub(super) extern "C" fn shim_pthread_equal(t1: libc::pthread_t, t2: libc::pthread_t) -> libc::c_int {
    if t1 == t2 {
        1
    } else {
        0
    }
}

/// `int pthread_cancel(pthread_t thread)`
pub(super) extern "C" fn shim_pthread_cancel(thread: libc::pthread_t) -> libc::c_int {
    unsafe { libc::pthread_cancel(thread) }
}

// ── Thread attributes ───────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_attr_init(attr: *mut libc::pthread_attr_t) -> libc::c_int {
    unsafe { libc::pthread_attr_init(attr) }
}

pub(super) extern "C" fn shim_pthread_attr_destroy(attr: *mut libc::pthread_attr_t) -> libc::c_int {
    unsafe { libc::pthread_attr_destroy(attr) }
}

pub(super) extern "C" fn shim_pthread_attr_setdetachstate(
    attr: *mut libc::pthread_attr_t,
    detachstate: libc::c_int,
) -> libc::c_int {
    unsafe { libc::pthread_attr_setdetachstate(attr, detachstate) }
}

pub(super) extern "C" fn shim_pthread_attr_getdetachstate(
    attr: *const libc::pthread_attr_t,
    detachstate: *mut libc::c_int,
) -> libc::c_int {
    // libc crate doesn't expose pthread_attr_getdetachstate on all targets,
    // so we link it directly via POSIX.
    extern "C" {
        fn pthread_attr_getdetachstate(
            attr: *const libc::pthread_attr_t,
            detachstate: *mut libc::c_int,
        ) -> libc::c_int;
    }
    unsafe { pthread_attr_getdetachstate(attr, detachstate) }
}

pub(super) extern "C" fn shim_pthread_attr_setstacksize(
    attr: *mut libc::pthread_attr_t,
    stacksize: libc::size_t,
) -> libc::c_int {
    unsafe { libc::pthread_attr_setstacksize(attr, stacksize) }
}

pub(super) extern "C" fn shim_pthread_attr_getstacksize(
    attr: *const libc::pthread_attr_t,
    stacksize: *mut libc::size_t,
) -> libc::c_int {
    unsafe { libc::pthread_attr_getstacksize(attr, stacksize) }
}

// ── Mutexes ─────────────────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_mutex_init(
    mutex: *mut libc::pthread_mutex_t,
    attr: *const libc::pthread_mutexattr_t,
) -> libc::c_int {
    // Zero out first to clear any macOS signature bytes.
    unsafe {
        std::ptr::write_bytes(mutex as *mut u8, 0, std::mem::size_of::<libc::pthread_mutex_t>());
        libc::pthread_mutex_init(mutex, attr)
    }
}

pub(super) extern "C" fn shim_pthread_mutex_lock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { maybe_reinit_darwin_mutex(mutex) };
    unsafe { libc::pthread_mutex_lock(mutex) }
}

pub(super) extern "C" fn shim_pthread_mutex_trylock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { maybe_reinit_darwin_mutex(mutex) };
    unsafe { libc::pthread_mutex_trylock(mutex) }
}

pub(super) extern "C" fn shim_pthread_mutex_unlock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { libc::pthread_mutex_unlock(mutex) }
}

pub(super) extern "C" fn shim_pthread_mutex_destroy(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { libc::pthread_mutex_destroy(mutex) }
}

// ── Mutex attributes ────────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_mutexattr_init(attr: *mut libc::pthread_mutexattr_t) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_init(attr) }
}

pub(super) extern "C" fn shim_pthread_mutexattr_destroy(attr: *mut libc::pthread_mutexattr_t) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_destroy(attr) }
}

pub(super) extern "C" fn shim_pthread_mutexattr_settype(
    attr: *mut libc::pthread_mutexattr_t,
    kind: libc::c_int,
) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_settype(attr, kind) }
}

// ── Read-write locks ────────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_rwlock_init(
    rwlock: *mut libc::pthread_rwlock_t,
    attr: *const libc::pthread_rwlockattr_t,
) -> libc::c_int {
    unsafe { libc::pthread_rwlock_init(rwlock, attr) }
}

pub(super) extern "C" fn shim_pthread_rwlock_rdlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_rdlock(rwlock) }
}

pub(super) extern "C" fn shim_pthread_rwlock_wrlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_wrlock(rwlock) }
}

pub(super) extern "C" fn shim_pthread_rwlock_tryrdlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_tryrdlock(rwlock) }
}

pub(super) extern "C" fn shim_pthread_rwlock_trywrlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_trywrlock(rwlock) }
}

pub(super) extern "C" fn shim_pthread_rwlock_unlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_unlock(rwlock) }
}

pub(super) extern "C" fn shim_pthread_rwlock_destroy(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_destroy(rwlock) }
}

// ── Condition variables ─────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_cond_init(
    cond: *mut libc::pthread_cond_t,
    attr: *const libc::pthread_condattr_t,
) -> libc::c_int {
    unsafe { libc::pthread_cond_init(cond, attr) }
}

pub(super) extern "C" fn shim_pthread_cond_wait(
    cond: *mut libc::pthread_cond_t,
    mutex: *mut libc::pthread_mutex_t,
) -> libc::c_int {
    log::debug!("pthread_cond_wait(cond={cond:?}, mutex={mutex:?})");
    let ret = unsafe { libc::pthread_cond_wait(cond, mutex) };
    log::debug!("pthread_cond_wait returned {ret}");
    ret
}

pub(super) extern "C" fn shim_pthread_cond_timedwait(
    cond: *mut libc::pthread_cond_t,
    mutex: *mut libc::pthread_mutex_t,
    abstime: *const libc::timespec,
) -> libc::c_int {
    let ret = unsafe { libc::pthread_cond_timedwait(cond, mutex, abstime) };
    // Translate Linux ETIMEDOUT (110) → macOS ETIMEDOUT (60).
    // Guest code (e.g. BIND9) checks the return value against the macOS
    // constant and asserts on unexpected values.
    #[cfg(target_os = "linux")]
    if ret == libc::ETIMEDOUT {
        return DARWIN_ETIMEDOUT;
    }
    ret
}

// macOS ETIMEDOUT = 60 (Operation timed out).
#[cfg(target_os = "linux")]
const DARWIN_ETIMEDOUT: i32 = 60;

pub(super) extern "C" fn shim_pthread_cond_signal(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    unsafe { libc::pthread_cond_signal(cond) }
}

pub(super) extern "C" fn shim_pthread_cond_broadcast(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    unsafe { libc::pthread_cond_broadcast(cond) }
}

pub(super) extern "C" fn shim_pthread_cond_destroy(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    unsafe { libc::pthread_cond_destroy(cond) }
}

// ── Thread-specific data ────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_key_create(
    key: *mut libc::pthread_key_t,
    destructor: Option<unsafe extern "C" fn(*mut libc::c_void)>,
) -> libc::c_int {
    unsafe { libc::pthread_key_create(key, destructor) }
}

pub(super) extern "C" fn shim_pthread_key_delete(key: libc::pthread_key_t) -> libc::c_int {
    unsafe { libc::pthread_key_delete(key) }
}

pub(super) extern "C" fn shim_pthread_getspecific(key: libc::pthread_key_t) -> *mut libc::c_void {
    unsafe { libc::pthread_getspecific(key) }
}

pub(super) extern "C" fn shim_pthread_setspecific(
    key: libc::pthread_key_t,
    value: *const libc::c_void,
) -> libc::c_int {
    unsafe { libc::pthread_setspecific(key, value) }
}

// ── Once control ────────────────────────────────────────────────────

// macOS pthread_once_t layout (16 bytes):
//   __sig: i64    = 0x30B1BCBA (PTHREAD_ONCE_INIT magic)
//   __opaque: i64 = 0
// Linux pthread_once_t layout (4 bytes):
//   __once: i32   = 0 (PTHREAD_ONCE_INIT)
//
// If the macOS once-control has the DARWIN magic and hasn't run (__opaque == 0),
// we must zero the first 4 bytes before passing to Linux's pthread_once.
const DARWIN_PTHREAD_ONCE_SIG: i64 = 0x30B1_BCBA;

pub(super) extern "C" fn shim_pthread_once(
    once_control: *mut libc::pthread_once_t,
    init_routine: Option<extern "C" fn()>,
) -> libc::c_int {
    match init_routine {
        Some(f) => {
            // macOS libc wants Option<extern "C" fn()>, Linux wants bare fn ptr.
            #[cfg(target_os = "macos")]
            { unsafe { libc::pthread_once(once_control, Some(f)) } }
            #[cfg(not(target_os = "macos"))]
            {
                // Translate macOS pthread_once_t → Linux pthread_once_t.
                // The guest binary has macOS PTHREAD_ONCE_INIT: first 4 bytes = 0x30B1BCBA.
                // Linux PTHREAD_ONCE_INIT: first 4 bytes = 0.
                // If we see the macOS magic, zero it so Linux treats it as "not yet run".
                unsafe {
                    let raw = once_control as *mut i32;
                    let first_word = raw.read_unaligned();
                    // macOS magic signature in little-endian:
                    if first_word == DARWIN_PTHREAD_ONCE_SIG as i32 {
                        raw.write_unaligned(0);
                    }
                    libc::pthread_once(once_control, f)
                }
            }
        }
        None => libc::EINVAL,
    }
}

// ── Scheduling ──────────────────────────────────────────────────────

pub(super) extern "C" fn shim_sched_yield() -> libc::c_int {
    unsafe { libc::sched_yield() }
}

// ── macOS-specific (with portable fallbacks) ────────────────────────

/// `int pthread_main_np(void)` — returns 1 if current thread is main.
///
/// On macOS: delegates to `libc::pthread_main_np()`.
/// On Linux: compares `gettid() == getpid()`.
pub(super) extern "C" fn shim_pthread_main_np() -> libc::c_int {
    #[cfg(target_os = "macos")]
    {
        unsafe { libc::pthread_main_np() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        // On Linux, the main thread has tid == pid.
        unsafe {
            let tid = libc::syscall(libc::SYS_gettid) as libc::pid_t;
            let pid = libc::getpid();
            if tid == pid { 1 } else { 0 }
        }
    }
}

// ── C++ std::mutex shims (macOS ABI → Linux pthread) ────────────────
//
// Guest binaries compiled for macOS inline-initialise `pthread_mutex_t`
// with the Darwin signature `0x32AAABA7` instead of calling
// `pthread_mutex_init`.  The host's `std::mutex::lock()` (from the
// system libc++) doesn't know about this, so we shim the mangled C++
// symbols to intercept the call at the guest import level.

/// `std::__1::mutex::lock()` — _ZNSt3__15mutex4lockEv
///
/// `this` points to the `std::mutex` object whose first member is
/// a `pthread_mutex_t`.
pub(super) extern "C" fn shim_std_mutex_lock(this: *mut libc::pthread_mutex_t) {
    unsafe { maybe_reinit_darwin_mutex(this) };
    let ret = unsafe { libc::pthread_mutex_lock(this) };
    if ret != 0 {
        log::warn!("std::mutex::lock failed with {ret}");
    }
}

/// `std::__1::mutex::unlock()` — _ZNSt3__15mutex6unlockEv
pub(super) extern "C" fn shim_std_mutex_unlock(this: *mut libc::pthread_mutex_t) {
    let ret = unsafe { libc::pthread_mutex_unlock(this) };
    if ret != 0 {
        log::warn!("std::mutex::unlock failed with {ret}");
    }
}

/// `std::__1::mutex::try_lock()` — _ZNSt3__15mutex8try_lockEv
pub(super) extern "C" fn shim_std_mutex_try_lock(this: *mut libc::pthread_mutex_t) -> bool {
    unsafe { maybe_reinit_darwin_mutex(this) };
    unsafe { libc::pthread_mutex_trylock(this) == 0 }
}
