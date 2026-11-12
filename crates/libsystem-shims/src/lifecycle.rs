//! Process lifecycle shims: atexit, exit, _exit, abort, signal handling.
//!
//! These intercept program termination to give the runtime a chance
//! to report exit status cleanly. In the lab's single-threaded model:
//! - `atexit` registers handlers but we don't call them (process is
//!   about to return from main anyway)
//! - `exit`/`_exit` delegate to the real libc (we're in-process)
//! - `abort` delegates to the real libc
//! - `sigaction`/`signal` are intercepted to prevent guest code from
//!   installing signal handlers that point into guest-mapped memory,
//!   which would crash the host if a signal fires after the guest exits

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Mutex;

use crate::table::ShimTable;

/// Process lifecycle shims.
pub struct LifecycleShims;

/// Registered atexit handlers (LIFO order per spec).
static ATEXIT_HANDLERS: Mutex<Vec<extern "C" fn()>> = Mutex::new(Vec::new());

impl LifecycleShims {
    /// Register lifecycle shims in the table.
    pub fn install(table: &mut ShimTable) {
        table.register(
            "_atexit",
            shim_atexit as *const () as u64,
            "atexit (registered but not called)",
        );
        table.register(
            "___cxa_atexit",
            shim_cxa_atexit as *const () as u64,
            "__cxa_atexit (registered)",
        );
        table.register(
            "__os_feature_enabled_impl",
            shim_os_feature_enabled_impl as *const () as u64,
            "__os_feature_enabled_impl → conservative feature flag false",
        );
        table.register(
            "_exit",
            shim_exit as *const () as u64,
            "exit (clean shutdown)",
        );
        table.register(
            "__exit",
            shim_exit as *const () as u64,
            "_exit (clean shutdown)",
        );
        table.register(
            "_Exit",
            shim_exit as *const () as u64,
            "_Exit (clean shutdown)",
        );
        table.register(
            "__Exit",
            shim_exit as *const () as u64,
            "_Exit (Darwin symbol, clean shutdown)",
        );
        table.register(
            "_sigaction",
            shim_sigaction as *const () as u64,
            "sigaction (guest handlers intercepted)",
        );
        table.register(
            "_signal",
            shim_signal as *const () as u64,
            "signal (guest handlers intercepted)",
        );
        table.register(
            "_sigaltstack",
            shim_sigaltstack as *const () as u64,
            "sigaltstack (intercepted, no guest alt stack)",
        );
        table.register(
            "___register_frame",
            shim_register_frame as *const () as u64,
            "__register_frame (delegate to host for DWARF unwind)",
        );
        table.register(
            "___deregister_frame",
            shim_deregister_frame as *const () as u64,
            "__deregister_frame (delegate to host for DWARF unwind)",
        );
        table.register(
            "_abort",
            shim_abort as *const () as u64,
            "abort (diagnostic wrapper)",
        );
        table.register(
            "_strlen",
            shim_strlen_diagnostic as *const () as u64,
            "strlen (diagnostic wrapper)",
        );
        // C++ guard variable shims — single-threaded, non-blocking.
        // The Linux __cxa_guard_acquire uses futex and can hang if the
        // guard variable contains stale data from the Mach-O binary.
        table.register(
            "___cxa_guard_acquire",
            shim_cxa_guard_acquire as *const () as u64,
            "__cxa_guard_acquire (single-threaded)",
        );
        table.register(
            "___cxa_guard_release",
            shim_cxa_guard_release as *const () as u64,
            "__cxa_guard_release (single-threaded)",
        );
        table.register(
            "___cxa_guard_abort",
            shim_cxa_guard_abort as *const () as u64,
            "__cxa_guard_abort (single-threaded)",
        );
        // Signal mask shims — macOS sigset_t (4 bytes) vs Linux (128 bytes).
        table.register(
            "_pthread_sigmask",
            shim_pthread_sigmask as *const () as u64,
            "pthread_sigmask (sigset_t 4→128 byte translation)",
        );
        table.register(
            "_sigprocmask",
            shim_sigprocmask as *const () as u64,
            "sigprocmask (sigset_t 4→128 byte translation)",
        );
        table.register(
            "_sigwait",
            shim_sigwait as *const () as u64,
            "sigwait (sigset_t 4→128 byte translation)",
        );
        table.register(
            "_kill",
            shim_kill as *const () as u64,
            "kill (diagnostic + signal translation)",
        );

        log::debug!("LifecycleShims: registered 21 lifecycle shims");
    }

    /// Run all registered atexit handlers in LIFO order.
    /// Called by the runtime during controlled shutdown.
    pub fn run_atexit_handlers() {
        let handlers = {
            let mut guard = ATEXIT_HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
            let h = guard.clone();
            guard.clear();
            h
        };
        // LIFO order per C standard
        for handler in handlers.into_iter().rev() {
            handler();
        }
    }

    /// Reset atexit handlers (useful between test runs).
    pub fn reset() {
        let mut guard = ATEXIT_HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }
}

/// `int atexit(void (*function)(void))`
extern "C" fn shim_atexit(function: Option<extern "C" fn()>) -> i32 {
    if let Some(f) = function {
        let mut guard = ATEXIT_HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
        guard.push(f);
        log::trace!("atexit: registered handler ({} total)", guard.len());
        0
    } else {
        -1
    }
}

/// `int __cxa_atexit(void (*function)(void *), void *arg, void *dso_handle)`
///
/// C++ ABI variant. We ignore arg and dso_handle for now.
extern "C" fn shim_cxa_atexit(
    function: Option<extern "C" fn(*mut u8)>,
    _arg: *mut u8,
    _dso_handle: *mut u8,
) -> i32 {
    if function.is_some() {
        // We store a simplified version — the arg semantics are tricky
        // and in practice most __cxa_atexit calls in simple programs pass NULL arg
        log::trace!("__cxa_atexit: registered (arg ignored)");
        0
    } else {
        -1
    }
}

/// Single-threaded __cxa_guard_acquire.
extern "C" fn shim_cxa_guard_acquire(guard: *mut u64) -> i32 {
    let byte0 = guard as *mut u8;
    let val = unsafe { byte0.read_volatile() };
    if val != 0 {
        return 0;
    }
    unsafe { byte0.write_volatile(0x10) };
    1
}
///
/// Mark the static variable as fully initialized. Sets byte 0 to 1.
extern "C" fn shim_cxa_guard_release(guard: *mut u64) {
    let byte0 = guard as *mut u8;
    unsafe { byte0.write_volatile(1) };
}

/// `void __cxa_guard_abort(uint64_t *guard)`
///
/// Initialization failed — reset guard to "not initialized". Sets byte 0 to 0.
extern "C" fn shim_cxa_guard_abort(guard: *mut u64) {
    let byte0 = guard as *mut u8;
    unsafe { byte0.write_volatile(0) };
}

/// `std::mutex::lock()` — handle macOS↔Linux pthread_mutex_t layout mismatch.
///
/// macOS `pthread_mutex_t` is 64 bytes with __sig=0x32AAAB3B for initialized
/// mutexes. Linux aarch64 `pthread_mutex_t` is 48 bytes with a completely
/// different internal layout. We detect uninitialized/macOS mutexes and
/// reinitialize them as Linux mutexes before locking.
extern "C" fn shim_std_mutex_lock(this: *mut libc::c_void) {
    reinit_mutex_if_needed(this);
    unsafe { libc::pthread_mutex_lock(this as *mut libc::pthread_mutex_t) };
}

/// `std::mutex::unlock()` — delegate directly to pthread_mutex_unlock.
extern "C" fn shim_std_mutex_unlock(this: *mut libc::c_void) {
    unsafe { libc::pthread_mutex_unlock(this as *mut libc::pthread_mutex_t) };
}

/// `std::mutex::try_lock()` — delegate directly to pthread_mutex_trylock.
extern "C" fn shim_std_mutex_try_lock(this: *mut libc::c_void) -> bool {
    reinit_mutex_if_needed(this);
    unsafe { libc::pthread_mutex_trylock(this as *mut libc::pthread_mutex_t) == 0 }
}

/// Reinitialize a macOS-layout pthread_mutex_t as a Linux mutex.
///
/// On macOS, an initialized mutex has __sig = 0x32AAAB3B at offset 0.
/// We detect this signature (or any non-zero non-Linux pattern) and
/// zero + reinitialize the mutex for Linux.
fn reinit_mutex_if_needed(ptr: *mut libc::c_void) {
    if (ptr as usize) < 0x10000 {
        log::warn!(
            "std::mutex: refusing to reinit mutex at bogus address {ptr:p} — possible TLS/heap corruption"
        );
        return;
    }
    use std::collections::HashSet;
    use std::sync::Mutex;
    static INITIALIZED: std::sync::OnceLock<Mutex<HashSet<usize>>> = std::sync::OnceLock::new();
    let set = INITIALIZED.get_or_init(|| Mutex::new(HashSet::new()));
    let addr = ptr as usize;
    let mut guard = set.lock().unwrap();
    if guard.insert(addr) {
        // First time seeing this mutex — zero-init as Linux pthread_mutex_t
        let mtx = ptr as *mut libc::pthread_mutex_t;
        unsafe {
            std::ptr::write_bytes(
                mtx as *mut u8,
                0,
                std::mem::size_of::<libc::pthread_mutex_t>(),
            );
            libc::pthread_mutex_init(mtx, std::ptr::null());
        }
        log::trace!("Reinitialized mutex at {ptr:?} for Linux");
    }
}

/// `int pthread_mutex_lock(pthread_mutex_t *mutex)` — with macOS→Linux reinit.
extern "C" fn shim_pthread_mutex_lock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    reinit_mutex_if_needed(mutex as *mut libc::c_void);
    if (mutex as usize) < 0x10000 {
        return libc::EINVAL;
    }
    unsafe { libc::pthread_mutex_lock(mutex) }
}

/// `int pthread_mutex_unlock(pthread_mutex_t *mutex)`
extern "C" fn shim_pthread_mutex_unlock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    if (mutex as usize) < 0x10000 {
        return libc::EINVAL;
    }
    unsafe { libc::pthread_mutex_unlock(mutex) }
}

/// `int pthread_mutex_trylock(pthread_mutex_t *mutex)` — with macOS→Linux reinit.
extern "C" fn shim_pthread_mutex_trylock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    reinit_mutex_if_needed(mutex as *mut libc::c_void);
    if (mutex as usize) < 0x10000 {
        return libc::EINVAL;
    }
    unsafe { libc::pthread_mutex_trylock(mutex) }
}

/// `bool __os_feature_enabled_impl(const char *domain, const char *feature)`
///
/// We conservatively report features as disabled, except for the Foundation
/// `NSPLUtil2` gate used by modern `plutil` to enter its Swift implementation.
extern "C" fn shim_os_feature_enabled_impl(domain: *const c_char, feature: *const c_char) -> i32 {
    let domain_name = if domain.is_null() {
        "<null>".to_string()
    } else {
        unsafe { CStr::from_ptr(domain) }
            .to_string_lossy()
            .into_owned()
    };
    let feature_name = if feature.is_null() {
        "<null>".to_string()
    } else {
        unsafe { CStr::from_ptr(feature) }
            .to_string_lossy()
            .into_owned()
    };

    // NSPLUtil2 gates plutil's Swift implementation. The Swift path requires
    // Swift stdlib metadata and ObjC runtime state that our shim environment
    // does not fully replicate; return false to use the ObjC/C code path
    // which works reliably under the shim runtime.
    let enabled = false;

    log::debug!(
        "__os_feature_enabled_impl({}, {}) -> {}",
        domain_name,
        feature_name,
        enabled
    );
    i32::from(enabled)
}

/// `void exit(int status)`
///
/// Intercept `exit()` to bypass C++ static destructors and atexit handlers
/// that may reference guest-mapped memory. We flush stdio then call `_exit()`
/// to terminate immediately without running cleanup callbacks.
extern "C" fn shim_exit(status: i32) -> ! {
    log::trace!("exit({status}): bypassing atexit handlers, calling _exit");
    // Flush C stdio so guest printf output is not lost
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    // Flush Rust stdio
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { libc::_exit(status) }
}

/// `void abort(void)` — diagnostic wrapper.
///
/// Logs when the guest calls abort, then terminates the process with
/// `_exit(1)`. Using `_exit` rather than the real `abort()` avoids
/// SIGABRT so that tools that intentionally abort (e.g. after printing
/// an error) are treated as normal (non-crash) exits.
extern "C" fn shim_abort() -> ! {
    eprintln!("[DIAGNOSTIC] abort() called by guest code!");
    // Print a simple backtrace for context
    #[cfg(target_arch = "aarch64")]
    {
        let lr: u64;
        unsafe { std::arch::asm!("mov {}, lr", out(reg) lr) };
        eprintln!("[DIAGNOSTIC] abort caller LR = {lr:#x}");
    }
    #[cfg(not(target_arch = "aarch64"))]
    eprintln!(
        "[DIAGNOSTIC] abort caller register unavailable on {}",
        std::env::consts::ARCH
    );
    use std::io::Write;
    let _ = std::io::stderr().flush();
    unsafe { libc::_exit(1) }
}

/// `int sigaction(int sig, const struct sigaction *act, struct sigaction *oact)`
///
/// Intercept guest signal handler installation to prevent guest code from
/// registering handlers that point into guest-mapped memory. If those handlers
/// fire after the guest exits or during host process teardown, the OS would
/// jump to unmapped/invalid addresses, causing a host SIGSEGV.
///
/// We accept the call (return 0) but do NOT pass it to the real `sigaction`.
/// macOS ARM64 `struct sigaction` — only 16 bytes!
/// Linux's `struct sigaction` is 152 bytes.  The guest binary allocates
/// macOS-sized structs on the stack, so we must NEVER read/write using
/// the Linux layout or we'll corrupt the stack.
#[repr(C)]
struct DarwinSigaction {
    sa_sigaction: u64, // union { void(*)(int); void(*)(int,siginfo_t*,void*); }
    sa_mask: u32,      // sigset_t = uint32_t on macOS
    sa_flags: i32,     // int
}

/// Translate macOS signal number to Linux signal number.
/// Most are identical; these are the ones that differ:
///   macOS SIGBUS=10   → Linux SIGBUS=7
///   macOS SIGCHLD=20  → Linux SIGCHLD=17
///   macOS SIGUSR1=30  → Linux SIGUSR1=10
///   macOS SIGUSR2=31  → Linux SIGUSR2=12
fn translate_darwin_signal(darwin_sig: i32) -> i32 {
    match darwin_sig {
        10 => 7,         // SIGBUS
        20 => 17,        // SIGCHLD
        30 => 10,        // SIGUSR1
        31 => 12,        // SIGUSR2
        _ => darwin_sig, // SIGINT(2), SIGABRT(6), SIGSEGV(11), SIGPIPE(13), SIGALRM(14), SIGTERM(15) are same
    }
}

/// Translate Linux signal number back to macOS signal number (reverse of above).
fn translate_linux_signal(linux_sig: i32) -> i32 {
    match linux_sig {
        7 => 10,  // SIGBUS
        17 => 20, // SIGCHLD
        10 => 30, // SIGUSR1
        12 => 31, // SIGUSR2
        _ => linux_sig,
    }
}

/// Signals that are safe to forward to the host:
/// - crash recovery: SIGBUS(10), SIGSEGV(11), SIGABRT(6)
/// - termination/cleanup: SIGTERM(15), SIGINT(2), SIGPIPE(13)
/// - timers: SIGALRM(14)
/// - user-defined: SIGUSR1(30), SIGUSR2(31)
/// - child process: SIGCHLD(20)
fn is_allowed_signal(darwin_sig: i32) -> bool {
    matches!(darwin_sig, 2 | 6 | 10 | 11 | 13 | 14 | 15 | 20 | 30 | 31)
}

fn preserves_host_crash_handler(darwin_sig: i32) -> bool {
    matches!(darwin_sig, 10 | 11)
}

/// If `oact` is non-null we zero it to indicate "no previous handler".
extern "C" fn shim_sigaction(
    sig: i32,
    act: *const DarwinSigaction,
    oact: *mut DarwinSigaction,
) -> i32 {
    if !act.is_null() {
        let handler = unsafe { (*act).sa_sigaction };
        let flags = unsafe { (*act).sa_flags };
        log::debug!("sigaction: intercepted sig={sig} handler={handler:#x} flags={flags:#x}");
        if preserves_host_crash_handler(sig) {
            log::debug!(
                "sigaction: keeping host crash handler for sig={sig}; guest handler change not forwarded"
            );
            if !oact.is_null() {
                unsafe {
                    std::ptr::write_bytes(
                        oact as *mut u8,
                        0,
                        std::mem::size_of::<DarwinSigaction>(),
                    );
                }
            }
            return 0;
        }
        if is_allowed_signal(sig) {
            let linux_sig = translate_darwin_signal(sig);
            log::debug!("sigaction: ALLOWING sig={sig} (linux={linux_sig}) through to host");
            // Build a Linux-sized sigaction from the macOS one
            let mut linux_act: libc::sigaction = unsafe { std::mem::zeroed() };
            linux_act.sa_sigaction = handler as usize;
            linux_act.sa_flags = flags as libc::c_int;
            extern "C" {
                fn sigaction(
                    sig: i32,
                    act: *const libc::sigaction,
                    oact: *mut libc::sigaction,
                ) -> i32;
            }
            let mut linux_oact: libc::sigaction = unsafe { std::mem::zeroed() };
            let oact_ptr = if oact.is_null() {
                std::ptr::null_mut()
            } else {
                &mut linux_oact
            };
            let ret = unsafe { sigaction(linux_sig, &linux_act, oact_ptr) };
            if !oact.is_null() {
                // Convert Linux oact back to macOS layout
                unsafe {
                    (*oact).sa_sigaction = linux_oact.sa_sigaction as u64;
                    (*oact).sa_mask = 0;
                    (*oact).sa_flags = linux_oact.sa_flags as i32;
                }
            }
            return ret;
        }
    } else {
        log::trace!("sigaction: query-only for signal {sig}");
    }
    if !oact.is_null() {
        // Zero out only the macOS-sized struct (16 bytes, not Linux's 152!)
        unsafe {
            std::ptr::write_bytes(oact as *mut u8, 0, std::mem::size_of::<DarwinSigaction>())
        };
    }
    0 // success
}

/// `sig_t signal(int sig, sig_t handler)`
///
/// Intercept the legacy `signal()` call. For SIG_DFL (0) and SIG_IGN (1),
/// pass through to the host so crash handlers can reset properly.
/// For other handlers, return SIG_DFL without installing.
extern "C" fn shim_signal(sig: i32, handler: usize) -> usize {
    // SIG_DFL = 0, SIG_IGN = 1 — pass through to host
    if handler <= 1 {
        let linux_sig = translate_darwin_signal(sig);
        log::trace!("signal: passing through SIG_DFL/SIG_IGN for signal {sig} (linux={linux_sig})");
        extern "C" {
            fn signal(sig: i32, handler: usize) -> usize;
        }
        return unsafe { signal(linux_sig, handler) };
    }
    log::trace!("signal: intercepted guest handler for signal {sig}");
    0 // SIG_DFL
}

/// `int sigaltstack(const stack_t *ss, stack_t *oss)`
///
/// Intercept alternate signal stack setup. Guest code may try to install
/// an alt stack in guest-mapped memory — we must prevent that from
/// reaching the OS since the memory may be invalid when a signal fires.
extern "C" fn shim_sigaltstack(_ss: *const libc::stack_t, oss: *mut libc::stack_t) -> i32 {
    if !oss.is_null() {
        unsafe { std::ptr::write_bytes(oss, 0, 1) };
    }
    log::trace!("sigaltstack: intercepted guest alt-stack setup");
    0 // success
}

/// `void __register_frame(const void *fde)`
///
/// Delegate to the host's DWARF eh_frame registration so that the host
/// unwinder can walk through guest-mapped code during C++ exceptions or
/// signal handling (e.g. LLVM's crash handler).
extern "C" fn shim_register_frame(fde: *const u8) {
    extern "C" {
        fn __register_frame(fde: *const u8);
    }
    log::trace!("__register_frame: delegating to host ({fde:?})");
    unsafe { __register_frame(fde) };
}

/// `void __deregister_frame(const void *fde)`
///
/// Delegate to the host's DWARF eh_frame deregistration.
extern "C" fn shim_deregister_frame(fde: *const u8) {
    extern "C" {
        fn __deregister_frame(fde: *const u8);
    }
    log::trace!("__deregister_frame: delegating to host ({fde:?})");
    unsafe { __deregister_frame(fde) };
}

/// Defensive `strlen` wrapper that returns 0 for unmapped/invalid pointers.
///
/// Some guest code (e.g. LLVM/Clang driver iterating option tables) may
/// encounter bad pointers — small integer values, tagged ObjC pointers, or
/// addresses in unmapped regions. Returning 0 lets the caller skip the
/// invalid entry instead of faulting.
extern "C" fn shim_strlen_diagnostic(s: *const u8) -> usize {
    let addr = s as u64;

    // Very small addresses (below page 0) are never valid C strings.
    // Catches NULL and small integer values like 5.
    if addr < 0x1000 {
        log::warn!("strlen: bad pointer {addr:#x} (below page boundary), returning 0");
        return 0;
    }

    // On arm64, bit 63 set means a tagged ObjC pointer or kernel address —
    // never a valid C string.  On x86_64 this catches canonical-high
    // (kernel) addresses.
    if addr >= 0x8000_0000_0000_0000 {
        log::warn!("strlen: bad pointer {addr:#x} (tagged/kernel), returning 0");
        return 0;
    }

    // Pointers in the GPU carveout / unmapped region are clearly invalid.
    if addr >= 0x10_0000_0000 && addr < 0x70_0000_0000 {
        log::warn!("strlen: bad pointer {addr:#x} (unmapped region), returning 0");
        return 0;
    }

    // Forward to real strlen
    unsafe { strlen_real(s) }
}

unsafe fn strlen_real(s: *const u8) -> usize {
    extern "C" {
        fn strlen(s: *const u8) -> usize;
    }
    unsafe { strlen(s) }
}

// ── Signal mask ABI translation (macOS sigset_t → Linux sigset_t) ───────────
//
// macOS `sigset_t` is `__uint32_t` (4 bytes): a simple bit mask where bit N
// corresponds to signal N.
//
// Linux aarch64 `sigset_t` is 128 bytes (1024 bits), with each `unsigned long`
// (8 bytes) holding 64 signal bits.
//
// When macOS guest code calls pthread_sigmask / sigprocmask / sigwait, it
// passes a pointer to a 4-byte sigset.  Linux's implementation reads 128
// bytes, which means it picks up garbage from the stack.  We translate by
// widening the 4-byte mask into a zeroed 128-byte Linux sigset_t.
//
// Additionally, the `how` parameter values differ:
//   macOS: SIG_BLOCK=1, SIG_UNBLOCK=2, SIG_SETMASK=3
//   Linux: SIG_BLOCK=0, SIG_UNBLOCK=1, SIG_SETMASK=2

/// Translate macOS `how` values (SIG_BLOCK=1, SIG_UNBLOCK=2, SIG_SETMASK=3)
/// to Linux values (SIG_BLOCK=0, SIG_UNBLOCK=1, SIG_SETMASK=2).
#[cfg(target_os = "linux")]
fn translate_sigmask_how(darwin_how: i32) -> i32 {
    match darwin_how {
        1 => libc::SIG_BLOCK,   // macOS SIG_BLOCK=1 → Linux SIG_BLOCK=0
        2 => libc::SIG_UNBLOCK, // macOS SIG_UNBLOCK=2 → Linux SIG_UNBLOCK=1
        3 => libc::SIG_SETMASK, // macOS SIG_SETMASK=3 → Linux SIG_SETMASK=2
        _ => darwin_how,
    }
}

/// Convert a macOS 4-byte sigset_t pointer to a Linux 128-byte sigset_t.
/// Returns a zeroed Linux sigset_t with the low 32 signal bits copied from
/// the macOS mask.  Signal numbers are identical for the first 31 signals
/// on both platforms (the signals nslookup cares about — SIGTERM, SIGINT, etc.).
#[cfg(target_os = "linux")]
fn darwin_sigset_to_linux(darwin_ptr: *const u32) -> libc::sigset_t {
    let mut linux_set: libc::sigset_t = unsafe { std::mem::zeroed() };
    unsafe { libc::sigemptyset(&mut linux_set) };
    if darwin_ptr.is_null() {
        return linux_set;
    }
    let mac_bits = unsafe { darwin_ptr.read_unaligned() };
    // macOS sigset_t: bit (sig-1) = signal sig.  e.g. SIGTERM=15 → bit 14.
    for sig in 1..32 {
        if mac_bits & (1u32 << (sig - 1)) != 0 {
            let linux_sig = translate_darwin_signal(sig as i32);
            unsafe { libc::sigaddset(&mut linux_set, linux_sig) };
        }
    }
    linux_set
}

/// Convert a Linux sigset_t back to a macOS 4-byte sigset_t.
/// Only the first 31 signals are representable.
#[cfg(target_os = "linux")]
fn linux_sigset_to_darwin(linux_set: &libc::sigset_t) -> u32 {
    let mut mac_bits: u32 = 0;
    for sig in 1..32i32 {
        let linux_sig = translate_darwin_signal(sig);
        if unsafe { libc::sigismember(linux_set, linux_sig) } == 1 {
            mac_bits |= 1u32 << (sig as u32 - 1);
        }
    }
    mac_bits
}

/// `int pthread_sigmask(int how, const sigset_t *set, sigset_t *oldset)`
///
/// Translates macOS 4-byte sigset_t to Linux 128-byte sigset_t.
#[cfg(target_os = "linux")]
extern "C" fn shim_pthread_sigmask(how: i32, set: *const u32, oldset: *mut u32) -> i32 {
    let linux_how = translate_sigmask_how(how);
    let linux_set = if set.is_null() {
        None
    } else {
        Some(darwin_sigset_to_linux(set))
    };
    let mut linux_oldset: libc::sigset_t = unsafe { std::mem::zeroed() };
    let old_ptr = if oldset.is_null() {
        std::ptr::null_mut()
    } else {
        &mut linux_oldset
    };

    let ret = unsafe {
        libc::pthread_sigmask(
            linux_how,
            linux_set
                .as_ref()
                .map_or(std::ptr::null(), |s| s as *const _),
            old_ptr,
        )
    };

    // Write back the old signal mask in macOS format.
    if !oldset.is_null() && ret == 0 {
        unsafe { oldset.write_unaligned(linux_sigset_to_darwin(&linux_oldset)) };
    }

    let how_name = match how {
        1 => "BLOCK",
        2 => "UNBLOCK",
        3 => "SETMASK",
        _ => "?",
    };
    log::trace!(
        "pthread_sigmask({how_name}, mac_set={:?}) → {ret}",
        if set.is_null() {
            None
        } else {
            Some(unsafe { set.read_unaligned() })
        }
    );
    ret
}

#[cfg(target_os = "macos")]
extern "C" fn shim_pthread_sigmask(_how: i32, _set: *const u32, _oldset: *mut u32) -> i32 {
    0
}

/// `int sigprocmask(int how, const sigset_t *set, sigset_t *oldset)`
///
/// Same translation as pthread_sigmask.
#[cfg(target_os = "linux")]
extern "C" fn shim_sigprocmask(how: i32, set: *const u32, oldset: *mut u32) -> i32 {
    let linux_how = translate_sigmask_how(how);
    let linux_set = if set.is_null() {
        None
    } else {
        Some(darwin_sigset_to_linux(set))
    };
    let mut linux_oldset: libc::sigset_t = unsafe { std::mem::zeroed() };
    let old_ptr = if oldset.is_null() {
        std::ptr::null_mut()
    } else {
        &mut linux_oldset
    };

    let ret = unsafe {
        libc::sigprocmask(
            linux_how,
            linux_set
                .as_ref()
                .map_or(std::ptr::null(), |s| s as *const _),
            old_ptr,
        )
    };

    if !oldset.is_null() && ret == 0 {
        unsafe { oldset.write_unaligned(linux_sigset_to_darwin(&linux_oldset)) };
    }

    log::debug!("sigprocmask(how={how}) → {ret}");
    ret
}

#[cfg(target_os = "macos")]
extern "C" fn shim_sigprocmask(_how: i32, _set: *const u32, _oldset: *mut u32) -> i32 {
    0
}

/// `int sigwait(const sigset_t *set, int *sig)`
///
/// Translates macOS 4-byte sigset_t to Linux 128-byte sigset_t, then
/// calls the real sigwait.
#[cfg(target_os = "linux")]
extern "C" fn shim_sigwait(set: *const u32, sig: *mut i32) -> i32 {
    let linux_set = darwin_sigset_to_linux(set);
    let mac_mask = if set.is_null() {
        0
    } else {
        unsafe { set.read_unaligned() }
    };
    log::trace!("sigwait: waiting (mac_mask={mac_mask:#x})");
    let ret = unsafe { libc::sigwait(&linux_set, sig) };
    if ret == 0 && !sig.is_null() {
        let caught = unsafe { *sig };
        log::debug!("sigwait: caught signal {caught}");
    }
    ret
}

#[cfg(target_os = "macos")]
extern "C" fn shim_sigwait(_set: *const u32, _sig: *mut i32) -> i32 {
    0
}

/// `int kill(pid_t pid, int sig)`
///
/// Diagnostic wrapper that logs the call and translates macOS signal numbers
/// to Linux signal numbers before calling the real kill(2).
extern "C" fn shim_kill(pid: i32, sig: i32) -> i32 {
    let linux_sig = translate_darwin_signal(sig);
    log::debug!("kill(pid={pid}, sig={sig} → linux {linux_sig})");
    unsafe { libc::kill(pid, linux_sig) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes atexit tests that share global handler state.
    static ATEXIT_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn lifecycle_shims_registered() {
        let mut table = ShimTable::new();
        LifecycleShims::install(&mut table);
        assert!(table.resolve("_atexit").is_some());
        assert!(table.resolve("___cxa_atexit").is_some());
        assert!(table.resolve("__os_feature_enabled_impl").is_some());
        assert!(table.resolve("_exit").is_some());
        assert!(table.resolve("_sigaction").is_some());
        assert!(table.resolve("_signal").is_some());
        assert!(table.resolve("_sigaltstack").is_some());
        assert!(table.resolve("___register_frame").is_some());
        assert!(table.resolve("___deregister_frame").is_some());
        assert!(table.resolve("_abort").is_some());
        assert!(table.resolve("_strlen").is_some());
        assert_eq!(table.len(), 24);
    }

    #[test]
    fn atexit_registers_handler() {
        let _lock = ATEXIT_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        LifecycleShims::reset();

        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);
        extern "C" fn handler() {
            CALLED.store(true, Ordering::Relaxed);
        }

        let result = shim_atexit(Some(handler));
        assert_eq!(result, 0);

        LifecycleShims::run_atexit_handlers();
        assert!(CALLED.load(Ordering::Relaxed));
    }

    #[test]
    fn atexit_lifo_order() {
        let _lock = ATEXIT_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        LifecycleShims::reset();

        use std::sync::Mutex;
        static ORDER: Mutex<Vec<i32>> = Mutex::new(Vec::new());

        extern "C" fn first() {
            ORDER.lock().unwrap_or_else(|e| e.into_inner()).push(1);
        }
        extern "C" fn second() {
            ORDER.lock().unwrap_or_else(|e| e.into_inner()).push(2);
        }
        extern "C" fn third() {
            ORDER.lock().unwrap_or_else(|e| e.into_inner()).push(3);
        }

        shim_atexit(Some(first));
        shim_atexit(Some(second));
        shim_atexit(Some(third));

        LifecycleShims::run_atexit_handlers();

        // LIFO: third registered last → called first
        assert_eq!(
            *ORDER.lock().unwrap_or_else(|e| e.into_inner()),
            vec![3, 2, 1]
        );
    }

    #[test]
    fn atexit_null_returns_error() {
        assert_eq!(shim_atexit(None), -1);
    }

    #[test]
    fn os_feature_enabled_impl_handles_known_and_unknown_flags() {
        let domain = std::ffi::CString::new("Foundation").unwrap();
        let other_feature = std::ffi::CString::new("DefinitelyNotARealFeature").unwrap();
        let feature = std::ffi::CString::new("NSPLUtil2").unwrap();

        // NSPLUtil2 gates plutil's Swift implementation. We return 0 (disabled)
        // so plutil uses the ObjC code path which is reliable under the shim runtime.
        assert_eq!(
            shim_os_feature_enabled_impl(domain.as_ptr(), feature.as_ptr()),
            0
        );
        assert_eq!(
            shim_os_feature_enabled_impl(domain.as_ptr(), other_feature.as_ptr()),
            0
        );
        assert_eq!(
            shim_os_feature_enabled_impl(std::ptr::null(), std::ptr::null()),
            0
        );
    }
}
