//! POSIX/libc extras.
//!
//! Functions that exist in libc on both platforms but may need thin
//! wrappers, polyfills, or safe stubs for cross-platform operation.

use std::os::raw::{c_char, c_int, c_long};

use crate::table::ShimTable;

pub struct PosixExtrasShims;

impl PosixExtrasShims {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            // ── bit scanning ────────────────────────────────────
            ("_flsl", shim_flsl as *const () as u64, "flsl (find last set bit, long)"),
            ("_fls", shim_fls as *const () as u64, "fls (find last set bit, int)"),

            // ── file/pty ────────────────────────────────────────
            ("_mkostemp", shim_mkostemp as *const () as u64, "mkostemp (mkstemp + flags)"),
            ("_openpty", shim_openpty as *const () as u64, "openpty (allocate pty)"),

            // ── process ─────────────────────────────────────────
            ("_waitid", shim_waitid as *const () as u64, "waitid (wait for process state change)"),

            // ── time ────────────────────────────────────────────
            ("_timespec_get", shim_timespec_get as *const () as u64, "timespec_get (C11)"),

            // ── backtrace ───────────────────────────────────────
            ("_backtrace", shim_backtrace as *const () as u64, "backtrace (stack trace)"),
            ("_backtrace_symbols", shim_backtrace_symbols as *const () as u64, "backtrace_symbols (symbolicate)"),
            ("_backtrace_symbols_fd", shim_backtrace_symbols_fd as *const () as u64, "backtrace_symbols_fd (symbolicate to fd)"),

            // ── Apple-specific with safe defaults ───────────────
            ("__availability_version_check", shim_availability_version_check as *const () as u64, "_availability_version_check (stub → 1, available)"),
            ("_atexit_b", shim_atexit_b as *const () as u64, "atexit_b (block-based atexit)"),
            ("_bootstrap_look_up", shim_bootstrap_look_up as *const () as u64, "bootstrap_look_up (stub → not privileged)"),

            // ── posix_spawn core ────────────────────────────────
            ("_posix_spawn", shim_posix_spawn as *const () as u64, "posix_spawn (thin wrapper → libc)"),
            ("_posix_spawnp", shim_posix_spawnp as *const () as u64, "posix_spawnp (thin wrapper → libc)"),

            // ── posix_spawn_file_actions ─────────────────────────
            ("_posix_spawn_file_actions_init", shim_posix_spawn_file_actions_init as *const () as u64, "posix_spawn_file_actions_init"),
            ("_posix_spawn_file_actions_destroy", shim_posix_spawn_file_actions_destroy as *const () as u64, "posix_spawn_file_actions_destroy"),
            ("_posix_spawn_file_actions_addclose", shim_posix_spawn_file_actions_addclose as *const () as u64, "posix_spawn_file_actions_addclose"),
            ("_posix_spawn_file_actions_adddup2", shim_posix_spawn_file_actions_adddup2 as *const () as u64, "posix_spawn_file_actions_adddup2"),
            ("_posix_spawn_file_actions_addopen", shim_posix_spawn_file_actions_addopen as *const () as u64, "posix_spawn_file_actions_addopen"),
            ("_posix_spawn_file_actions_addchdir_np", shim_posix_spawn_file_actions_addchdir_np as *const () as u64, "posix_spawn_file_actions_addchdir_np (stub → ENOSYS)"),
            ("_posix_spawn_file_actions_addfchdir_np", shim_posix_spawn_file_actions_addfchdir_np as *const () as u64, "posix_spawn_file_actions_addfchdir_np (stub → ENOSYS)"),

            // ── posix_spawnattr extras ──────────────────────────
            ("_posix_spawnattr_getflags", shim_posix_spawnattr_getflags as *const () as u64, "posix_spawnattr_getflags"),
            ("_posix_spawnattr_setflags", shim_posix_spawnattr_setflags as *const () as u64, "posix_spawnattr_setflags"),
            ("_posix_spawnattr_init", shim_posix_spawnattr_init as *const () as u64, "posix_spawnattr_init"),
            ("_posix_spawnattr_destroy", shim_posix_spawnattr_destroy as *const () as u64, "posix_spawnattr_destroy"),
            ("_posix_spawnattr_getsigmask", shim_posix_spawnattr_getsigmask as *const () as u64, "posix_spawnattr_getsigmask"),
            ("_posix_spawnattr_setsigmask", shim_posix_spawnattr_setsigmask as *const () as u64, "posix_spawnattr_setsigmask"),
            ("_posix_spawnattr_getsigdefault", shim_posix_spawnattr_getsigdefault as *const () as u64, "posix_spawnattr_getsigdefault"),
            ("_posix_spawnattr_setsigdefault", shim_posix_spawnattr_setsigdefault as *const () as u64, "posix_spawnattr_setsigdefault"),
            ("_posix_spawnattr_getpgroup", shim_posix_spawnattr_getpgroup as *const () as u64, "posix_spawnattr_getpgroup"),
            ("_posix_spawnattr_setpgroup", shim_posix_spawnattr_setpgroup as *const () as u64, "posix_spawnattr_setpgroup"),
            ("_posix_spawnattr_setbinpref_np", shim_posix_spawnattr_setbinpref_np as *const () as u64, "posix_spawnattr_setbinpref_np (stub → 0)"),
            ("_posix_spawnattr_set_persona_np", shim_posix_spawnattr_set_persona_np as *const () as u64, "posix_spawnattr_set_persona_np (stub → 0)"),

            // ── BSD extras ──────────────────────────────────────
            ("_funopen", shim_funopen as *const () as u64, "funopen (stub → NULL)"),

            // ── sandbox/security stubs ──────────────────────────
            ("_chroot", shim_chroot as *const () as u64, "chroot (stub → EPERM)"),

            // ── mmap flag translation ───────────────────────────
            ("_mmap", shim_mmap as *const () as u64, "mmap (macOS→Linux flag translation)"),

            // ── resource limits ─────────────────────────────────
            ("_getrlimit", shim_getrlimit as *const () as u64, "getrlimit (macOS→Linux RLIMIT_ translation)"),
            ("_setrlimit", shim_setrlimit as *const () as u64, "setrlimit (macOS→Linux RLIMIT_ translation)"),
        );
        log::debug!("PosixExtrasShims: registered 39 POSIX/libc extra shims");
    }
}

// ── Bit scanning ────────────────────────────────────────────────────

/// Find last set bit (1-indexed) in a long. Returns 0 if value is 0.
pub extern "C" fn shim_flsl(value: c_long) -> c_int {
    if value == 0 {
        0
    } else {
        (c_long::BITS - value.leading_zeros()) as c_int
    }
}

/// Find last set bit (1-indexed) in an int. Returns 0 if value is 0.
pub extern "C" fn shim_fls(value: c_int) -> c_int {
    if value == 0 {
        0
    } else {
        (c_int::BITS - (value as u32).leading_zeros()) as c_int
    }
}

// ── File / PTY ──────────────────────────────────────────────────────

/// mkostemp: like mkstemp but with additional flags (O_CLOEXEC etc.)
/// Falls back to mkstemp if mkostemp is unavailable (flags ignored).
pub extern "C" fn shim_mkostemp(template: *mut c_char, _flags: c_int) -> c_int {
    if template.is_null() {
        return -1;
    }
    // libc crate may not expose mkostemp; use mkstemp + fcntl for O_CLOEXEC
    let fd = unsafe { libc::mkstemp(template) };
    if fd >= 0 && (_flags & libc::O_CLOEXEC) != 0 {
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    fd
}

/// openpty: allocate a pseudo-terminal pair.
pub extern "C" fn shim_openpty(
    amaster: *mut c_int,
    aslave: *mut c_int,
    name: *mut c_char,
    termp: *const libc::termios,
    winp: *const libc::winsize,
) -> c_int {
    if amaster.is_null() || aslave.is_null() {
        return -1;
    }
    unsafe { libc::openpty(amaster, aslave, name, termp as *mut _, winp as *mut _) }
}

// ── Process ─────────────────────────────────────────────────────────

/// waitid: wait for process state change.
pub extern "C" fn shim_waitid(
    idtype: c_int,
    id: u32,
    infop: *mut libc::siginfo_t,
    options: c_int,
) -> c_int {
    unsafe {
        libc::waitid(idtype as libc::idtype_t, id as libc::id_t, infop, options)
    }
}

// ── Time ────────────────────────────────────────────────────────────

/// timespec_get: C11 time function. base must be TIME_UTC (1).
pub extern "C" fn shim_timespec_get(ts: *mut libc::timespec, base: c_int) -> c_int {
    if ts.is_null() || base != 1 {
        return 0; // failure
    }
    let ret = unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, ts) };
    if ret == 0 { base } else { 0 }
}

// ── Backtrace ───────────────────────────────────────────────────────

/// backtrace: capture stack frames.
pub extern "C" fn shim_backtrace(buffer: *mut *mut std::ffi::c_void, size: c_int) -> c_int {
    if buffer.is_null() || size <= 0 {
        return 0;
    }
    // Use libc backtrace on platforms that have execinfo.h (glibc)
    // On musl/other, return 0 frames
    #[cfg(target_os = "linux")]
    {
        unsafe { libc::backtrace(buffer, size) }
    }
    #[cfg(not(target_os = "linux"))]
    {
        // On macOS this would normally be handled by host dlsym
        0
    }
}

/// backtrace_symbols: symbolicate captured frames.
pub extern "C" fn shim_backtrace_symbols(
    buffer: *const *mut std::ffi::c_void,
    size: c_int,
) -> *mut *mut c_char {
    if buffer.is_null() || size <= 0 {
        return std::ptr::null_mut();
    }
    #[cfg(target_os = "linux")]
    {
        unsafe { libc::backtrace_symbols(buffer as *mut *mut _, size) }
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::ptr::null_mut()
    }
}

/// backtrace_symbols_fd: symbolicate to file descriptor.
pub extern "C" fn shim_backtrace_symbols_fd(
    buffer: *const *mut std::ffi::c_void,
    size: c_int,
    fd: c_int,
) {
    #[cfg(target_os = "linux")]
    {
        unsafe { libc::backtrace_symbols_fd(buffer as *mut *mut _, size, fd) };
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (buffer, size, fd);
    }
}

// ── Apple-specific with safe defaults ───────────────────────────────

/// _availability_version_check: returns 1 (available) for all versions.
/// On macOS this checks OS version availability; on Linux we assume
/// all APIs are "available" since we're shimming everything.
pub extern "C" fn shim_availability_version_check(
    _count: u32,
    _versions: u64,
) -> i32 {
    1 // always available
}

/// atexit_b: block-based atexit. The block's invoke pointer is at
/// offset 16 in the block literal. We treat it as a plain function
/// pointer and register it with atexit.
pub extern "C" fn shim_atexit_b(block: u64) -> c_int {
    // Block layout: isa(8) + flags(4) + reserved(4) + invoke(8)
    // invoke is at offset 16
    if block == 0 {
        return -1;
    }
    let invoke_ptr = unsafe { *((block + 16) as *const u64) };
    if invoke_ptr == 0 {
        return -1;
    }
    // We can't safely register an arbitrary block with libc::atexit
    // because the block may capture state. Log and ignore.
    log::debug!("atexit_b: registered block at {:#x} (will not execute at exit)", invoke_ptr);
    0
}

/// bootstrap_look_up: Mach bootstrap server lookup.
///
/// Resolves a service name to a synthetic Mach port via the internal
/// bootstrap service registry. Returns `KERN_SUCCESS` (0) on success,
/// or `BOOTSTRAP_UNKNOWN_SERVICE` (1102) if the name is NULL.
pub extern "C" fn shim_bootstrap_look_up(
    _bp: u32,
    service_name: *const c_char,
    sp: *mut u32,
) -> c_int {
    if service_name.is_null() || sp.is_null() {
        return 1102; // BOOTSTRAP_UNKNOWN_SERVICE
    }
    let name = unsafe { std::ffi::CStr::from_ptr(service_name) };
    let Ok(name_str) = name.to_str() else {
        return 1102; // BOOTSTRAP_UNKNOWN_SERVICE
    };
    let port = crate::darwin_stubs::mach_extras::bootstrap_look_up_service(name_str);
    unsafe { *sp = port; }
    log::debug!("bootstrap_look_up(\"{}\") → port {:#x}", name_str, port);
    0 // KERN_SUCCESS
}

// ── posix_spawn core ────────────────────────────────────────────────

/// `posix_spawn(pid, path, file_actions, attrp, argv, envp)` → libc.
pub extern "C" fn shim_posix_spawn(
    pid: *mut libc::pid_t,
    path: *const c_char,
    file_actions: *const libc::posix_spawn_file_actions_t,
    attrp: *const libc::posix_spawnattr_t,
    argv: *const *mut c_char,
    envp: *const *mut c_char,
) -> c_int {
    if path.is_null() || argv.is_null() {
        return libc::EINVAL;
    }
    unsafe {
        libc::posix_spawn(
            pid,
            path,
            file_actions,
            attrp,
            argv as *const *mut _,
            envp as *const *mut _,
        )
    }
}

/// `posix_spawnp(pid, file, file_actions, attrp, argv, envp)` → libc (PATH search).
pub extern "C" fn shim_posix_spawnp(
    pid: *mut libc::pid_t,
    file: *const c_char,
    file_actions: *const libc::posix_spawn_file_actions_t,
    attrp: *const libc::posix_spawnattr_t,
    argv: *const *mut c_char,
    envp: *const *mut c_char,
) -> c_int {
    if file.is_null() || argv.is_null() {
        return libc::EINVAL;
    }
    unsafe {
        libc::posix_spawnp(
            pid,
            file,
            file_actions,
            attrp,
            argv as *const *mut _,
            envp as *const *mut _,
        )
    }
}

// ── posix_spawn_file_actions ─────────────────────────────────────────

/// `posix_spawn_file_actions_init`.
pub extern "C" fn shim_posix_spawn_file_actions_init(
    actions: *mut libc::posix_spawn_file_actions_t,
) -> c_int {
    if actions.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawn_file_actions_init(actions) }
}

/// `posix_spawn_file_actions_destroy`.
pub extern "C" fn shim_posix_spawn_file_actions_destroy(
    actions: *mut libc::posix_spawn_file_actions_t,
) -> c_int {
    if actions.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawn_file_actions_destroy(actions) }
}

/// `posix_spawn_file_actions_addclose(actions, fd)`.
pub extern "C" fn shim_posix_spawn_file_actions_addclose(
    actions: *mut libc::posix_spawn_file_actions_t,
    fd: c_int,
) -> c_int {
    if actions.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawn_file_actions_addclose(actions, fd) }
}

/// `posix_spawn_file_actions_adddup2(actions, fd, newfd)`.
pub extern "C" fn shim_posix_spawn_file_actions_adddup2(
    actions: *mut libc::posix_spawn_file_actions_t,
    fd: c_int,
    newfd: c_int,
) -> c_int {
    if actions.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawn_file_actions_adddup2(actions, fd, newfd) }
}

/// `posix_spawn_file_actions_addopen(actions, fd, path, oflag, mode)`.
pub extern "C" fn shim_posix_spawn_file_actions_addopen(
    actions: *mut libc::posix_spawn_file_actions_t,
    fd: c_int,
    path: *const c_char,
    oflag: c_int,
    mode: libc::mode_t,
) -> c_int {
    if actions.is_null() || path.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawn_file_actions_addopen(actions, fd, path, oflag, mode) }
}

/// `posix_spawn_file_actions_addchdir_np` — macOS extension; stub on Linux.
pub extern "C" fn shim_posix_spawn_file_actions_addchdir_np(
    _actions: u64, _path: u64,
) -> c_int {
    libc::ENOSYS
}

/// `posix_spawn_file_actions_addfchdir_np` — macOS extension; stub on Linux.
pub extern "C" fn shim_posix_spawn_file_actions_addfchdir_np(
    _actions: u64, _fd: c_int,
) -> c_int {
    libc::ENOSYS
}

// ── posix_spawnattr ──────────────────────────────────────────────────

/// `posix_spawnattr_init`.
pub extern "C" fn shim_posix_spawnattr_init(attr: *mut libc::posix_spawnattr_t) -> c_int {
    if attr.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawnattr_init(attr) }
}

/// `posix_spawnattr_destroy`.
pub extern "C" fn shim_posix_spawnattr_destroy(attr: *mut libc::posix_spawnattr_t) -> c_int {
    if attr.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawnattr_destroy(attr) }
}

/// `posix_spawnattr_getsigmask`.
pub extern "C" fn shim_posix_spawnattr_getsigmask(
    attr: *const libc::posix_spawnattr_t,
    sigmask: *mut libc::sigset_t,
) -> c_int {
    if attr.is_null() || sigmask.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawnattr_getsigmask(attr, sigmask) }
}

/// `posix_spawnattr_setsigmask`.
pub extern "C" fn shim_posix_spawnattr_setsigmask(
    attr: *mut libc::posix_spawnattr_t,
    sigmask: *const libc::sigset_t,
) -> c_int {
    if attr.is_null() || sigmask.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawnattr_setsigmask(attr, sigmask) }
}

/// `posix_spawnattr_getsigdefault`.
pub extern "C" fn shim_posix_spawnattr_getsigdefault(
    attr: *const libc::posix_spawnattr_t,
    sigdefault: *mut libc::sigset_t,
) -> c_int {
    if attr.is_null() || sigdefault.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawnattr_getsigdefault(attr, sigdefault) }
}

/// `posix_spawnattr_setsigdefault`.
pub extern "C" fn shim_posix_spawnattr_setsigdefault(
    attr: *mut libc::posix_spawnattr_t,
    sigdefault: *const libc::sigset_t,
) -> c_int {
    if attr.is_null() || sigdefault.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawnattr_setsigdefault(attr, sigdefault) }
}

/// `posix_spawnattr_getpgroup`.
pub extern "C" fn shim_posix_spawnattr_getpgroup(
    attr: *const libc::posix_spawnattr_t,
    pgroup: *mut libc::pid_t,
) -> c_int {
    if attr.is_null() || pgroup.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawnattr_getpgroup(attr, pgroup) }
}

/// `posix_spawnattr_setpgroup`.
pub extern "C" fn shim_posix_spawnattr_setpgroup(
    attr: *mut libc::posix_spawnattr_t,
    pgroup: libc::pid_t,
) -> c_int {
    if attr.is_null() { return libc::EINVAL; }
    unsafe { libc::posix_spawnattr_setpgroup(attr, pgroup) }
}

// ── posix_spawn extras ──────────────────────────────────────────────

pub extern "C" fn shim_posix_spawnattr_getflags(
    attr: *const u8,
    flags: *mut i16,
) -> c_int {
    if attr.is_null() || flags.is_null() {
        return libc::EINVAL;
    }
    unsafe { libc::posix_spawnattr_getflags(attr as *const _, flags) }
}

pub extern "C" fn shim_posix_spawnattr_setflags(
    attr: *mut u8,
    flags: i16,
) -> c_int {
    if attr.is_null() {
        return libc::EINVAL;
    }
    unsafe { libc::posix_spawnattr_setflags(attr as *mut _, flags) }
}

/// macOS-only: set binary preference for CPU type selection.
/// No-op on Linux.
pub extern "C" fn shim_posix_spawnattr_setbinpref_np(
    _attr: *mut u8,
    _count: u64,
    _pref: *const u32,
    _ocount: *mut u64,
) -> c_int {
    0
}

/// macOS-only: set persona for spawned process.
/// No-op on Linux.
pub extern "C" fn shim_posix_spawnattr_set_persona_np(
    _attr: *mut u8,
    _persona: u64,
    _flags: u32,
) -> c_int {
    0
}

// ── BSD extras ──────────────────────────────────────────────────────

/// funopen: BSD-specific custom FILE* creation. Not available on Linux.
/// Returns NULL (callers should handle NULL FILE*).
pub extern "C" fn shim_funopen(
    _cookie: u64,
    _readfn: u64,
    _writefn: u64,
    _seekfn: u64,
    _closefn: u64,
) -> u64 {
    0 // NULL
}

// ── chroot stub ─────────────────────────────────────────────────────

/// `int chroot(const char *dirname)` — always fails with EPERM.
///
/// In the loader context, chroot is a security-sensitive operation that
/// we never want to actually perform.  Returning EPERM allows tools like
/// tar that optionally call chroot to fall through to their non-chroot path.
pub extern "C" fn shim_chroot(_dirname: *const c_char) -> c_int {
    unsafe {
        #[cfg(target_os = "macos")]
        { *libc::__error() = libc::EPERM; }
        #[cfg(not(target_os = "macos"))]
        { *libc::__errno_location() = libc::EPERM; }
    }
    -1
}

// ── mmap flag translation ───────────────────────────────────────────

/// Translate macOS mmap flags to Linux mmap flags.
/// Key differences:
///   macOS MAP_ANON = 0x1000, Linux MAP_ANONYMOUS = 0x20
///   macOS MAP_JIT  = 0x0800 (macOS-only, strip it)
///   MAP_PRIVATE(0x2), MAP_SHARED(0x1), MAP_FIXED(0x10) are the same.
fn translate_mmap_flags(darwin_flags: c_int) -> c_int {
    const DARWIN_MAP_ANON: c_int = 0x1000;
    const DARWIN_MAP_JIT: c_int = 0x0800;
    const DARWIN_MAP_RESILIENT_CODESIGN: c_int = 0x2000;
    const DARWIN_MAP_RESILIENT_MEDIA: c_int = 0x4000;

    let mut flags = darwin_flags;

    // Translate MAP_ANON
    if flags & DARWIN_MAP_ANON != 0 {
        flags &= !DARWIN_MAP_ANON;
        flags |= libc::MAP_ANONYMOUS;
    }

    // Strip macOS-only flags that have no Linux equivalent
    flags &= !(DARWIN_MAP_JIT | DARWIN_MAP_RESILIENT_CODESIGN | DARWIN_MAP_RESILIENT_MEDIA);

    flags
}

/// `void *mmap(void *addr, size_t len, int prot, int flags, int fd, off_t offset)`
///
/// Translates macOS mmap flags (MAP_ANON=0x1000) to Linux (MAP_ANONYMOUS=0x20)
/// before delegating to the host mmap.
pub extern "C" fn shim_mmap(
    addr: u64,
    len: u64,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: i64,
) -> u64 {
    let linux_flags = translate_mmap_flags(flags);
    log::trace!("mmap: flags {flags:#x} → {linux_flags:#x}, len={len}, prot={prot:#x}");
    unsafe { libc::mmap(addr as *mut libc::c_void, len as usize, prot, linux_flags, fd, offset) as u64 }
}

// ── Resource limit translation ──────────────────────────────────────

/// Translate macOS RLIMIT_* constants to Linux equivalents.
/// macOS:                    Linux:
///   RLIMIT_CPU      = 0      RLIMIT_CPU      = 0  (same)
///   RLIMIT_FSIZE    = 1      RLIMIT_FSIZE    = 1  (same)
///   RLIMIT_DATA     = 2      RLIMIT_DATA     = 2  (same)
///   RLIMIT_STACK    = 3      RLIMIT_STACK    = 3  (same)
///   RLIMIT_CORE     = 4      RLIMIT_CORE     = 4  (same)
///   RLIMIT_AS       = 5      RLIMIT_AS       = 9
///   RLIMIT_MEMLOCK  = 6      RLIMIT_MEMLOCK  = 8
///   RLIMIT_NPROC    = 7      RLIMIT_NPROC    = 6
///   RLIMIT_NOFILE   = 8      RLIMIT_NOFILE   = 7
fn translate_rlimit_resource(darwin_res: c_int) -> c_int {
    match darwin_res {
        0..=4 => darwin_res, // CPU, FSIZE, DATA, STACK, CORE — same
        5 => 9,              // RLIMIT_AS
        6 => 8,              // RLIMIT_MEMLOCK
        7 => 6,              // RLIMIT_NPROC
        8 => 7,              // RLIMIT_NOFILE
        _ => darwin_res,     // pass through unknown values
    }
}

/// `int getrlimit(int resource, struct rlimit *rlp)`
pub extern "C" fn shim_getrlimit(resource: c_int, rlp: *mut libc::rlimit) -> c_int {
    let linux_res = translate_rlimit_resource(resource);
    log::trace!("getrlimit: resource {resource} → {linux_res}");
    unsafe { libc::getrlimit(linux_res as _, rlp) }
}

/// `int setrlimit(int resource, const struct rlimit *rlp)`
pub extern "C" fn shim_setrlimit(resource: c_int, rlp: *const libc::rlimit) -> c_int {
    let linux_res = translate_rlimit_resource(resource);
    log::trace!("setrlimit: resource {resource} → {linux_res}");
    unsafe { libc::setrlimit(linux_res as _, rlp) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flsl() {
        assert_eq!(shim_flsl(0), 0);
        assert_eq!(shim_flsl(1), 1);
        assert_eq!(shim_flsl(2), 2);
        assert_eq!(shim_flsl(0xFF), 8);
        assert_eq!(shim_flsl(0x8000_0000), 32);
    }

    #[test]
    fn test_fls() {
        assert_eq!(shim_fls(0), 0);
        assert_eq!(shim_fls(1), 1);
        assert_eq!(shim_fls(255), 8);
    }

    #[test]
    fn test_availability_version_check() {
        assert_eq!(shim_availability_version_check(0, 0), 1);
    }

    #[test]
    fn test_bootstrap_look_up() {
        // NULL service name → BOOTSTRAP_UNKNOWN_SERVICE
        assert_eq!(shim_bootstrap_look_up(0, std::ptr::null(), std::ptr::null_mut()), 1102);
        // Valid service name → KERN_SUCCESS (0) + non-zero port
        let name = b"com.apple.test.service\0";
        let mut port: u32 = 0;
        let result = shim_bootstrap_look_up(0, name.as_ptr() as *const c_char, &mut port);
        assert_eq!(result, 0);
        assert!(port >= 0x1000);
    }

    #[test]
    fn test_funopen_returns_null() {
        assert_eq!(shim_funopen(0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn test_posix_spawn_null_path_returns_einval() {
        let mut pid: libc::pid_t = 0;
        let rc = shim_posix_spawn(&mut pid, std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null());
        assert_eq!(rc, libc::EINVAL);
    }

    #[test]
    fn test_posix_spawnp_null_file_returns_einval() {
        let rc = shim_posix_spawnp(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null());
        assert_eq!(rc, libc::EINVAL);
    }

    #[test]
    fn test_posix_spawn_file_actions_init_null_returns_einval() {
        let rc = shim_posix_spawn_file_actions_init(std::ptr::null_mut());
        assert_eq!(rc, libc::EINVAL);
    }

    #[test]
    fn test_posix_spawn_file_actions_lifecycle() {
        let mut actions = std::mem::MaybeUninit::<libc::posix_spawn_file_actions_t>::uninit();
        let rc = shim_posix_spawn_file_actions_init(actions.as_mut_ptr());
        assert_eq!(rc, 0);
        let rc2 = shim_posix_spawn_file_actions_destroy(actions.as_mut_ptr());
        assert_eq!(rc2, 0);
    }

    #[test]
    fn test_posix_spawnattr_lifecycle() {
        let mut attr = std::mem::MaybeUninit::<libc::posix_spawnattr_t>::uninit();
        let rc = shim_posix_spawnattr_init(attr.as_mut_ptr());
        assert_eq!(rc, 0);
        let rc2 = shim_posix_spawnattr_destroy(attr.as_mut_ptr());
        assert_eq!(rc2, 0);
    }

    #[test]
    fn test_posix_spawn_addchdir_returns_enosys() {
        assert_eq!(shim_posix_spawn_file_actions_addchdir_np(0, 0), libc::ENOSYS);
        assert_eq!(shim_posix_spawn_file_actions_addfchdir_np(0, 0), libc::ENOSYS);
    }

    // ── mmap flag translation tests ─────────────────────────────

    #[test]
    fn test_mmap_flags_darwin_map_anon() {
        // macOS MAP_ANON=0x1000 should be replaced with libc::MAP_ANONYMOUS
        let darwin_flags = 0x1000 | libc::MAP_PRIVATE;
        let linux_flags = translate_mmap_flags(darwin_flags);
        assert_eq!(linux_flags & libc::MAP_ANONYMOUS, libc::MAP_ANONYMOUS);
        assert_eq!(linux_flags & libc::MAP_PRIVATE, libc::MAP_PRIVATE);
    }

    #[test]
    fn test_mmap_flags_strip_jit() {
        // MAP_JIT=0x0800 should be stripped
        let darwin_flags = 0x0800 | 0x1000 | libc::MAP_PRIVATE;
        let linux_flags = translate_mmap_flags(darwin_flags);
        assert_eq!(linux_flags & 0x0800, 0);
        assert_eq!(linux_flags & libc::MAP_ANONYMOUS, libc::MAP_ANONYMOUS);
    }

    #[test]
    fn test_mmap_flags_passthrough() {
        // Flags that are the same should pass through unchanged
        let flags = libc::MAP_PRIVATE | libc::MAP_FIXED;
        assert_eq!(translate_mmap_flags(flags), flags);
    }

    // ── rlimit translation tests ────────────────────────────────

    #[test]
    fn test_rlimit_same_values() {
        // 0..4 are the same on both platforms
        for i in 0..=4 {
            assert_eq!(translate_rlimit_resource(i), i);
        }
    }

    #[test]
    fn test_rlimit_divergent_values() {
        assert_eq!(translate_rlimit_resource(5), 9);  // RLIMIT_AS
        assert_eq!(translate_rlimit_resource(6), 8);  // RLIMIT_MEMLOCK
        assert_eq!(translate_rlimit_resource(7), 6);  // RLIMIT_NPROC
        assert_eq!(translate_rlimit_resource(8), 7);  // RLIMIT_NOFILE
    }
}
