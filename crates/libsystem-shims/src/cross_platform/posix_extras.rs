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

            // ── sysconf constant translation ────────────────────
            ("_sysconf", shim_sysconf as *const () as u64, "sysconf (macOS→Linux _SC_ translation)"),

            // ── getopt POSIX mode (BSD default) ─────────────────
            ("_getopt", shim_getopt as *const () as u64, "getopt (POSIX mode — stop at first non-option)"),
            ("_getopt_long", shim_getopt_long as *const () as u64, "getopt_long (POSIX mode)"),
            ("_getopt_long_only", shim_getopt_long_only as *const () as u64, "getopt_long_only (POSIX mode)"),
            ("_optreset", std::ptr::addr_of!(OPTRESET) as *const () as u64, "optreset (BSD compat data symbol)"),
        );

        // ── regex flag translation (Linux-only, glibc regex_t) ──
        #[cfg(target_os = "linux")]
        register_shims!(table,
            ("_regcomp", shim_regcomp as *const () as u64, "regcomp (macOS regex_t bridge + flag translation)"),
            ("_regexec", shim_regexec as *const () as u64, "regexec (macOS regex_t bridge)"),
            ("_regfree", shim_regfree as *const () as u64, "regfree (macOS regex_t bridge)"),
            ("_regerror", shim_regerror as *const () as u64, "regerror (macOS regex_t bridge)"),
        );

        log::debug!("PosixExtrasShims: registered POSIX/libc extra shims");
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
/// Resolves a service name to a synthetic Mach port via the launchd
/// bootstrap service registry. Returns `KERN_SUCCESS` (0) on success,
/// or `BOOTSTRAP_UNKNOWN_SERVICE` (1102) if the name is NULL.
pub extern "C" fn shim_bootstrap_look_up(
    bp: u32,
    service_name: *const c_char,
    sp: *mut u32,
) -> c_int {
    launchd::bootstrap::bootstrap_look_up(bp, service_name, sp) as c_int
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

// ── sysconf constant translation ────────────────────────────────────

/// Translate macOS `_SC_*` constants to Linux equivalents.
/// Every single value differs between the two platforms.
fn translate_sysconf_name(darwin_name: c_int) -> c_int {
    match darwin_name {
        1 => 0,    // _SC_ARG_MAX
        2 => 1,    // _SC_CHILD_MAX
        3 => 2,    // _SC_CLK_TCK
        4 => 3,    // _SC_NGROUPS_MAX
        5 => 4,    // _SC_OPEN_MAX
        6 => 7,    // _SC_JOB_CONTROL
        7 => 8,    // _SC_SAVED_IDS
        8 => 29,   // _SC_VERSION
        9 => 36,   // _SC_BC_BASE_MAX
        10 => 37,  // _SC_BC_DIM_MAX
        11 => 38,  // _SC_BC_SCALE_MAX
        12 => 39,  // _SC_BC_STRING_MAX
        13 => 40,  // _SC_COLL_WEIGHTS_MAX
        14 => 42,  // _SC_EXPR_NEST_MAX
        15 => 43,  // _SC_LINE_MAX
        16 => 44,  // _SC_RE_DUP_MAX
        17 => 46,  // _SC_2_VERSION
        18 => 47,  // _SC_2_C_BIND
        19 => 48,  // _SC_2_C_DEV
        20 => 95,  // _SC_2_CHAR_TERM
        21 => 49,  // _SC_2_FORT_DEV
        22 => 50,  // _SC_2_FORT_RUN
        23 => 52,  // _SC_2_LOCALEDEF
        24 => 51,  // _SC_2_SW_DEV
        25 => 97,  // _SC_2_UPE
        26 => 5,   // _SC_STREAM_MAX
        27 => 6,   // _SC_TZNAME_MAX
        29 => 30,  // _SC_PAGESIZE / _SC_PAGE_SIZE
        56 => 60,  // _SC_IOV_MAX
        57 => 83,  // _SC_NPROCESSORS_CONF
        58 => 84,  // _SC_NPROCESSORS_ONLN
        70 => 69,  // _SC_GETGR_R_SIZE_MAX
        71 => 70,  // _SC_GETPW_R_SIZE_MAX
        72 => 180, // _SC_HOST_NAME_MAX
        73 => 71,  // _SC_LOGIN_NAME_MAX
        101 => 72, // _SC_TTY_NAME_MAX
        120 => 173, // _SC_SYMLOOP_MAX
        200 => 85, // _SC_PHYS_PAGES
        other => {
            log::warn!("sysconf: unknown macOS _SC_ constant {other}, passing through");
            other
        }
    }
}

/// `long sysconf(int name)`
///
/// Translates macOS `_SC_*` constants to Linux equivalents before
/// delegating to the host sysconf.
pub extern "C" fn shim_sysconf(name: c_int) -> c_long {
    let linux_name = translate_sysconf_name(name);
    log::trace!("sysconf: macOS _SC_={name} → Linux _SC_={linux_name}");
    unsafe { libc::sysconf(linux_name as _) }
}

// ── getopt POSIX mode ───────────────────────────────────────────────
//
// macOS (BSD) getopt stops at the first non-option argument (POSIX default).
// Linux glibc getopt *permutes* arguments by default (GNU extension),
// which breaks many macOS tools (e.g., `find /path -maxdepth 1` where
// glibc would parse `-maxdepth` as short `-m` option).
//
// Fix: prepend `+` to optstring to force POSIX mode on glibc.

/// BSD `optreset` global — macOS programs write `optreset = 1` to reset
/// getopt state between multiple parse passes.  On Linux we translate
/// this to `optind = 0` (glibc reset convention) inside the shim.
#[no_mangle]
pub static mut OPTRESET: c_int = 0;

/// Ensure optstring starts with `+` for POSIX behavior.
/// Returns owned CString (caller must keep alive through getopt call).
fn posix_optstring(optstring: *const c_char) -> Option<std::ffi::CString> {
    if optstring.is_null() { return None; }
    let s = unsafe { std::ffi::CStr::from_ptr(optstring) };
    let bytes = s.to_bytes();
    if bytes.first() == Some(&b'+') {
        return None; // already POSIX mode
    }
    let mut buf = Vec::with_capacity(1 + bytes.len() + 1);
    buf.push(b'+');
    buf.extend_from_slice(bytes);
    std::ffi::CString::new(buf).ok()
}

extern "C" {
    static mut optind: c_int;
}

/// Check and consume BSD `optreset` flag.
fn consume_optreset() {
    unsafe {
        if OPTRESET != 0 {
            OPTRESET = 0;
            // glibc resets getopt state when optind is set to 0
            optind = 0;
        }
    }
}

pub extern "C" fn shim_getopt(
    argc: c_int,
    argv: *const *mut c_char,
    optstring: *const c_char,
) -> c_int {
    consume_optreset();
    match posix_optstring(optstring) {
        Some(posix) => unsafe { libc::getopt(argc, argv, posix.as_ptr()) },
        None => unsafe { libc::getopt(argc, argv, optstring) },
    }
}

extern "C" {
    fn getopt_long(
        argc: c_int,
        argv: *const *mut c_char,
        optstring: *const c_char,
        longopts: *const libc::option,
        longindex: *mut c_int,
    ) -> c_int;
    fn getopt_long_only(
        argc: c_int,
        argv: *const *mut c_char,
        optstring: *const c_char,
        longopts: *const libc::option,
        longindex: *mut c_int,
    ) -> c_int;
}

pub extern "C" fn shim_getopt_long(
    argc: c_int,
    argv: *const *mut c_char,
    optstring: *const c_char,
    longopts: *const libc::option,
    longindex: *mut c_int,
) -> c_int {
    consume_optreset();
    match posix_optstring(optstring) {
        Some(posix) => unsafe { getopt_long(argc, argv, posix.as_ptr(), longopts, longindex) },
        None => unsafe { getopt_long(argc, argv, optstring, longopts, longindex) },
    }
}

pub extern "C" fn shim_getopt_long_only(
    argc: c_int,
    argv: *const *mut c_char,
    optstring: *const c_char,
    longopts: *const libc::option,
    longindex: *mut c_int,
) -> c_int {
    consume_optreset();
    match posix_optstring(optstring) {
        Some(posix) => unsafe { getopt_long_only(argc, argv, posix.as_ptr(), longopts, longindex) },
        None => unsafe { getopt_long_only(argc, argv, optstring, longopts, longindex) },
    }
}

// ── regex flag translation ──────────────────────────────────────────

/// Translate macOS `regcomp` cflags to Linux equivalents.
///
/// macOS and Linux swap `REG_NOSUB` and `REG_NEWLINE`:
///
/// | Flag         | macOS | Linux |
/// |--------------|-------|-------|
/// | REG_EXTENDED |   1   |   1   |
/// | REG_ICASE    |   2   |   2   |
/// | REG_NOSUB    |   4   |   8   |
/// | REG_NEWLINE  |   8   |   4   |
#[cfg(target_os = "linux")]
fn translate_regcomp_flags(darwin_cflags: c_int) -> c_int {
    const DARWIN_REG_NOSUB: c_int = 0x0004;
    const DARWIN_REG_NEWLINE: c_int = 0x0008;
    const LINUX_REG_NEWLINE: c_int = 0x0004;
    const LINUX_REG_NOSUB: c_int = 0x0008;

    let mut flags = darwin_cflags;

    // Clear the two bits that differ
    flags &= !(DARWIN_REG_NOSUB | DARWIN_REG_NEWLINE);

    // Re-set them with Linux values
    if darwin_cflags & DARWIN_REG_NOSUB != 0 {
        flags |= LINUX_REG_NOSUB;
    }
    if darwin_cflags & DARWIN_REG_NEWLINE != 0 {
        flags |= LINUX_REG_NEWLINE;
    }

    flags
}

/// macOS `regex_t` struct (32 bytes on 64-bit):
/// ```text
///   int       re_magic;   // offset 0, 4 bytes
///   (4 pad)               // offset 4, 4 bytes
///   size_t    re_nsub;    // offset 8, 8 bytes
///   char     *re_endp;    // offset 16, 8 bytes
///   void     *re_g;       // offset 24, 8 bytes
/// ```
///
/// glibc `regex_t` is ~64 bytes — writing it into a 32-byte macOS buffer
/// corrupts adjacent memory. We allocate glibc-size regex_t on the heap
/// and store its pointer in the macOS struct's `re_g` field (offset 24).
/// On Darwin, `re_nsub` at offset 8 receives the sub-expression count.

#[cfg(target_os = "linux")]
mod regex_shims {
use super::*;
use std::sync::Mutex;
use std::collections::HashMap;

/// Safety: `libc::regex_t` contains raw pointers but we only access
/// the map while holding the mutex, and the pointees live on the heap.
struct RegexBox(Box<libc::regex_t>);
unsafe impl Send for RegexBox {}

static REGEX_MAP: Mutex<Option<HashMap<u64, RegexBox>>> = Mutex::new(None);

fn regex_map_insert(darwin_ptr: u64, linux_re: Box<libc::regex_t>) {
    let mut guard = REGEX_MAP.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(darwin_ptr, RegexBox(linux_re));
}

fn regex_map_get(darwin_ptr: u64) -> Option<*const libc::regex_t> {
    let guard = REGEX_MAP.lock().unwrap();
    guard.as_ref()?.get(&darwin_ptr).map(|b| &*b.0 as *const libc::regex_t)
}

fn regex_map_remove(darwin_ptr: u64) -> Option<Box<libc::regex_t>> {
    let mut guard = REGEX_MAP.lock().unwrap();
    guard.as_mut()?.remove(&darwin_ptr).map(|b| b.0)
}

/// `int regcomp(regex_t *preg, const char *pattern, int cflags)`
///
/// Allocates a glibc-sized `regex_t`, calls glibc `regcomp`, and stores
/// the result. The macOS `regex_t` at `preg` gets `re_nsub` written.
pub extern "C" fn shim_regcomp(
    preg: u64, // macOS regex_t* (32 bytes, can't fit glibc's ~64 bytes)
    pattern: *const c_char,
    cflags: c_int,
) -> c_int {
    let linux_cflags = translate_regcomp_flags(cflags);
    log::trace!("regcomp: preg={preg:#x} flags {cflags:#x} → {linux_cflags:#x}");

    let mut linux_re: Box<libc::regex_t> = Box::new(unsafe { std::mem::zeroed() });
    let ret = unsafe { libc::regcomp(&mut *linux_re, pattern, linux_cflags) };
    if ret == 0 {
        // Write re_nsub into the macOS struct at offset 8
        if preg != 0 {
            unsafe {
                let nsub_ptr = (preg + 8) as *mut u64;
                *nsub_ptr = 0;
            }
        }
        regex_map_insert(preg, linux_re);
    }
    ret
}

/// `int regexec(const regex_t *preg, const char *string, size_t nmatch,
///              regmatch_t *pmatch, int eflags)`
///
/// Looks up the glibc regex_t from our map and calls glibc regexec.
/// macOS `regmatch_t` uses `regoff_t = int64_t` (8 bytes each)→ 16 bytes.
/// Linux  `regmatch_t` uses `regoff_t = int`     (4 bytes each)→  8 bytes.
#[repr(C)]
struct DarwinRegmatch {
    rm_so: i64,
    rm_eo: i64,
}

/// `int regexec(const regex_t *preg, const char *string, size_t nmatch,
///              regmatch_t *pmatch, int eflags)`
///
/// Bridges between macOS 16-byte regmatch_t and Linux 8-byte regmatch_t.
pub extern "C" fn shim_regexec(
    preg: u64,
    string: *const c_char,
    nmatch: u64,
    darwin_pmatch: u64, // actually *mut DarwinRegmatch (16 bytes each)
    eflags: c_int,
) -> c_int {
    if let Some(linux_re) = regex_map_get(preg) {
        let nm = nmatch as usize;
        // Allocate Linux-sized pmatch array
        let mut linux_pmatch: Vec<libc::regmatch_t> = vec![
            libc::regmatch_t { rm_so: -1, rm_eo: -1 }; nm.max(1)
        ];

        // If REG_STARTEND, convert input pmatch[0] from macOS to Linux layout
        if eflags & 4 != 0 && darwin_pmatch != 0 && nm >= 1 {
            let darwin = unsafe { &*(darwin_pmatch as *const DarwinRegmatch) };
            linux_pmatch[0].rm_so = darwin.rm_so as i32;
            linux_pmatch[0].rm_eo = darwin.rm_eo as i32;
        }

        let pmatch_ptr = if darwin_pmatch == 0 {
            std::ptr::null_mut()
        } else {
            linux_pmatch.as_mut_ptr()
        };

        let ret = unsafe { libc::regexec(linux_re, string, nm, pmatch_ptr, eflags) };

        // Convert output pmatch from Linux to macOS layout
        if darwin_pmatch != 0 && ret == 0 {
            for i in 0..nm {
                let darwin = unsafe { &mut *((darwin_pmatch as *mut DarwinRegmatch).add(i)) };
                darwin.rm_so = linux_pmatch[i].rm_so as i64;
                darwin.rm_eo = linux_pmatch[i].rm_eo as i64;
            }
        }

        ret
    } else {
        log::warn!("regexec: no compiled regex found for preg={preg:#x}");
        1 // REG_NOMATCH
    }
}

/// `void regfree(regex_t *preg)`
///
/// Frees the glibc regex_t from our map.
pub extern "C" fn shim_regfree(preg: u64) {
    if let Some(mut linux_re) = regex_map_remove(preg) {
        unsafe { libc::regfree(&mut *linux_re); }
    }
}

/// `size_t regerror(int errcode, const regex_t *preg, char *errbuf, size_t errbuf_size)`
pub extern "C" fn shim_regerror(
    errcode: c_int,
    preg: u64,
    errbuf: *mut c_char,
    errbuf_size: u64,
) -> u64 {
    if let Some(linux_re) = regex_map_get(preg) {
        unsafe { libc::regerror(errcode, linux_re, errbuf, errbuf_size as usize) as u64 }
    } else {
        unsafe { libc::regerror(errcode, std::ptr::null(), errbuf, errbuf_size as usize) as u64 }
    }
}
} // mod regex_shims

#[cfg(target_os = "linux")]
pub use regex_shims::*;

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
        // NULL service name + NULL sp → BOOTSTRAP_BAD_COUNT (1104)
        // (sp is checked first in launchd::bootstrap::bootstrap_look_up)
        assert_eq!(shim_bootstrap_look_up(0, std::ptr::null(), std::ptr::null_mut()), 1104);
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

    // ── sysconf translation tests ───────────────────────────────

    #[test]
    fn test_sysconf_pagesize() {
        assert_eq!(translate_sysconf_name(29), 30);  // _SC_PAGESIZE
    }

    #[test]
    fn test_sysconf_nprocessors() {
        assert_eq!(translate_sysconf_name(57), 83);  // _SC_NPROCESSORS_CONF
        assert_eq!(translate_sysconf_name(58), 84);  // _SC_NPROCESSORS_ONLN
    }

    #[test]
    fn test_sysconf_phys_pages() {
        assert_eq!(translate_sysconf_name(200), 85); // _SC_PHYS_PAGES
    }

    #[test]
    fn test_sysconf_common_values() {
        assert_eq!(translate_sysconf_name(1), 0);    // _SC_ARG_MAX
        assert_eq!(translate_sysconf_name(3), 2);    // _SC_CLK_TCK
        assert_eq!(translate_sysconf_name(5), 4);    // _SC_OPEN_MAX
    }

    #[test]
    fn test_sysconf_unknown_passthrough() {
        assert_eq!(translate_sysconf_name(999), 999);
    }

    // ── regcomp flag translation tests ──────────────────────────

    #[cfg(target_os = "linux")]
    #[test]
    fn test_regcomp_nosub_swap() {
        // macOS REG_NOSUB=4 → Linux REG_NOSUB=8
        assert_eq!(translate_regcomp_flags(0x0004), 0x0008);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_regcomp_newline_swap() {
        // macOS REG_NEWLINE=8 → Linux REG_NEWLINE=4
        assert_eq!(translate_regcomp_flags(0x0008), 0x0004);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_regcomp_both_swapped() {
        // macOS NOSUB(4)|NEWLINE(8) → Linux NOSUB(8)|NEWLINE(4) — same combined value
        assert_eq!(translate_regcomp_flags(0x000C), 0x000C);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_regcomp_extended_unchanged() {
        // REG_EXTENDED=1 is the same on both — should pass through
        assert_eq!(translate_regcomp_flags(0x0001), 0x0001);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_regcomp_combined_flags() {
        // REG_EXTENDED(1) | REG_ICASE(2) | REG_NOSUB(4) → 7
        // Linux: REG_EXTENDED(1) | REG_ICASE(2) | REG_NOSUB(8) → 11
        assert_eq!(translate_regcomp_flags(0x0007), 0x000B);
    }

    // ── getopt POSIX mode tests ─────────────────────────────────

    #[test]
    fn test_posix_optstring_prepends_plus() {
        let result = posix_optstring(b"abc:\0".as_ptr() as *const c_char);
        assert!(result.is_some());
        let cstr = result.unwrap();
        assert_eq!(cstr.to_bytes(), b"+abc:");
    }

    #[test]
    fn test_posix_optstring_already_posix() {
        let result = posix_optstring(b"+abc:\0".as_ptr() as *const c_char);
        assert!(result.is_none()); // already has +, no change needed
    }

    #[test]
    fn test_posix_optstring_null() {
        let result = posix_optstring(std::ptr::null());
        assert!(result.is_none());
    }

    #[test]
    fn test_posix_optstring_empty() {
        let result = posix_optstring(b"\0".as_ptr() as *const c_char);
        assert!(result.is_some());
        let cstr = result.unwrap();
        assert_eq!(cstr.to_bytes(), b"+");
    }
}
