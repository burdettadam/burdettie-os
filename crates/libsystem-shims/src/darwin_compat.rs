//! macOS → Linux compatibility shims.
//!
//! These shims provide Linux-compatible implementations of macOS-specific
//! C library symbols.  On macOS the symbols are resolved natively via
//! `dlsym`, so these are only registered on non-macOS platforms.
//!
//! Key translations:
//!
//! | macOS symbol         | Linux equivalent / shim strategy            |
//! |----------------------|---------------------------------------------|
//! | `___error`           | wrapper around `__errno_location`            |
//! | `___assert_rtn`      | wrapper around `__assert_fail` (reordered)  |
//! | `___maskrune`        | locale-aware char classification             |
//! | `___tolower`         | delegates to `tolower(3)`                    |
//! | `___toupper`         | delegates to `toupper(3)`                    |
//! | `__stderrp`          | pointer to host `stderr`                     |
//! | `__stdoutp`          | pointer to host `stdout`                     |
//! | `__stdinp`           | pointer to host `stdin`                      |
//! | `_DefaultRuneLocale` | synthetic `_RuneLocale` struct               |
//! | `___mb_cur_max`      | delegates to `MB_CUR_MAX`                    |
//! | `___strlcpy_chk`     | delegates to `strlcpy` / manual impl         |
//! | `___strlcat_chk`     | delegates to `strlcat` / manual impl         |
//! | `_clock_gettime_nsec_np` | wrapper around `clock_gettime`           |

use crate::table::ShimTable;

pub struct DarwinCompatShims;

impl DarwinCompatShims {
    /// Register macOS compatibility shims.  Only active on non-macOS.
    pub fn install(table: &mut ShimTable) {
        // On macOS these symbols exist natively — nothing to register.
        #[cfg(target_os = "macos")]
        let _ = table;

        #[cfg(not(target_os = "macos"))]
        {
            use crate::register_shims;

            log::info!("DarwinCompatShims: registering Linux compat shims");

            // ── errno ───────────────────────────────────────────────
            register_shims!(table,
                ("___error", shim_error as *const () as u64, "__error → errno location"),
            );

            // ── assert ──────────────────────────────────────────────
            register_shims!(table,
                ("___assert_rtn", shim_assert_rtn as *const () as u64, "__assert_rtn compat"),
            );

            // ── locale / ctype ──────────────────────────────────────
            register_shims!(table,
                ("___maskrune", shim_maskrune as *const () as u64, "__maskrune compat"),
                ("___tolower", shim_tolower as *const () as u64, "__tolower compat"),
                ("___toupper", shim_toupper as *const () as u64, "__toupper compat"),
                ("__DefaultRuneLocale", shim_default_rune_locale_addr() as u64, "DefaultRuneLocale data"),
            );

            // ── stdio file pointers ─────────────────────────────────
            init_stdio_pointers();
            register_shims!(table,
                ("___stderrp", stdio_ptr_addr(StdioStream::Stderr) as u64, "__stderrp data"),
                ("___stdoutp", stdio_ptr_addr(StdioStream::Stdout) as u64, "__stdoutp data"),
                ("___stdinp", stdio_ptr_addr(StdioStream::Stdin) as u64, "__stdinp data"),
            );

            // ── MB_CUR_MAX ──────────────────────────────────────────
            register_shims!(table,
                ("___mb_cur_max", shim_mb_cur_max as *const () as u64, "__mb_cur_max compat"),
            );

            // ── fortified string functions ──────────────────────────
            register_shims!(table,
                ("___strlcpy_chk", shim_strlcpy_chk as *const () as u64, "__strlcpy_chk compat"),
                ("___strlcat_chk", shim_strlcat_chk as *const () as u64, "__strlcat_chk compat"),
            );

            // ── time ────────────────────────────────────────────────
            register_shims!(table,
                ("_clock_gettime_nsec_np", shim_clock_gettime_nsec_np as *const () as u64,
                 "clock_gettime_nsec_np compat"),
            );

            log::info!("DarwinCompatShims: registered {} compat shims", 13);
        }
    }
}

// ── Implementation (Linux only) ─────────────────────────────────────

#[cfg(not(target_os = "macos"))]
use std::sync::Once;

// ── errno ───────────────────────────────────────────────────────────

/// `int *__error(void)` — returns a pointer to the thread-local errno.
/// On Linux this wraps `__errno_location()`.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_error() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

// ── assert ──────────────────────────────────────────────────────────

/// `void __assert_rtn(const char *func, const char *file, int line,
///                     const char *expr)`
///
/// macOS signature differs from glibc's `__assert_fail(expr, file, line, func)`.
/// We reorder the arguments and delegate.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_assert_rtn(
    func: *const libc::c_char,
    file: *const libc::c_char,
    line: libc::c_int,
    expr: *const libc::c_char,
) -> ! {
    extern "C" {
        fn __assert_fail(
            assertion: *const libc::c_char,
            file: *const libc::c_char,
            line: libc::c_uint,
            function: *const libc::c_char,
        ) -> !;
    }
    unsafe {
        __assert_fail(expr, file, line as libc::c_uint, func)
    }
}

// ── locale / ctype ──────────────────────────────────────────────────

/// `unsigned long __maskrune(__darwin_ct_rune_t c, unsigned long mask)`
///
/// On macOS this looks up a flags word in `_DefaultRuneLocale` and ANDs
/// with `mask`.  We use the synthetic rune table below.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_maskrune(c: i32, mask: libc::c_ulong) -> libc::c_ulong {
    // For characters in the ASCII range, compute classification flags
    // compatible with the macOS _RuneLocale bitmask layout.
    let flags = ascii_rune_flags(c);
    flags & mask
}

/// macOS _RuneLocale character classification bitmask constants.
/// These match <ctype.h> / <RuneLocale.h> on macOS.
#[cfg(not(target_os = "macos"))]
mod rune_flags {
    pub const _CTYPE_A: u32 = 0x0000_0100; // alpha
    pub const _CTYPE_C: u32 = 0x0000_0200; // control
    pub const _CTYPE_D: u32 = 0x0000_0400; // digit
    pub const _CTYPE_G: u32 = 0x0000_0800; // graph (visible)
    pub const _CTYPE_L: u32 = 0x0000_1000; // lower
    pub const _CTYPE_P: u32 = 0x0000_2000; // punct
    pub const _CTYPE_S: u32 = 0x0000_4000; // space
    pub const _CTYPE_U: u32 = 0x0000_8000; // upper
    pub const _CTYPE_X: u32 = 0x0001_0000; // hex digit
    pub const _CTYPE_B: u32 = 0x0002_0000; // blank (space or tab)
    pub const _CTYPE_R: u32 = 0x0004_0000; // print (visible + space)
}

/// Compute macOS-compatible character classification flags for a codepoint.
#[cfg(not(target_os = "macos"))]
fn ascii_rune_flags(c: i32) -> libc::c_ulong {
    use rune_flags::*;

    if c < 0 || c > 127 {
        return 0;
    }
    let c = c as u8;
    let mut f: u32 = 0;

    // Control characters: 0x00–0x1F and 0x7F
    if c <= 0x1F || c == 0x7F {
        f |= _CTYPE_C;
    }

    // Digits
    if c.is_ascii_digit() {
        f |= _CTYPE_D | _CTYPE_G | _CTYPE_R | _CTYPE_X;
    }

    // Uppercase
    if c.is_ascii_uppercase() {
        f |= _CTYPE_A | _CTYPE_U | _CTYPE_G | _CTYPE_R;
        if matches!(c, b'A'..=b'F') {
            f |= _CTYPE_X;
        }
    }

    // Lowercase
    if c.is_ascii_lowercase() {
        f |= _CTYPE_A | _CTYPE_L | _CTYPE_G | _CTYPE_R;
        if matches!(c, b'a'..=b'f') {
            f |= _CTYPE_X;
        }
    }

    // Punctuation (printable, not alnum, not space)
    if c.is_ascii_punctuation() {
        f |= _CTYPE_P | _CTYPE_G | _CTYPE_R;
    }

    // Space characters
    if c.is_ascii_whitespace() {
        f |= _CTYPE_S;
        if c == b' ' || c == b'\t' {
            f |= _CTYPE_B;
        }
    }

    // Print = graph + space
    if c == b' ' {
        f |= _CTYPE_R;
    }

    f as libc::c_ulong
}

/// `int __tolower(int c)` — macOS internal ctype function.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_tolower(c: libc::c_int) -> libc::c_int {
    unsafe { libc::tolower(c) }
}

/// `int __toupper(int c)` — macOS internal ctype function.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_toupper(c: libc::c_int) -> libc::c_int {
    unsafe { libc::toupper(c) }
}

// ── DefaultRuneLocale ───────────────────────────────────────────────

/// Minimal synthetic `_RuneLocale` matching the macOS layout.
///
/// Most macOS binaries only read the `__runetype[256]` array to
/// classify characters.  We pre-fill ASCII entries and zero the rest.
#[cfg(not(target_os = "macos"))]
#[repr(C)]
struct RuneLocale {
    /// Magic: "RuneMagi" = [0x52, 0x75, 0x6E, 0x65, 0x4D, 0x61, 0x67, 0x69]
    __magic: [u8; 8],
    /// Encoding name (null-terminated, 32 bytes on macOS).
    __encoding: [u8; 32],
    /// Mapping functions (6 function pointers — unused, zeroed).
    __sgetrune: u64,
    __sputrune: u64,
    __invalid_rune: i32,
    _pad0: u32,
    /// Rune flags for codepoints 0..=255.
    __runetype: [u32; 256],
    /// Case-mapping for 0..=255.
    __maplower: [i32; 256],
    __mapupper: [i32; 256],
}

/// Global synthetic `_DefaultRuneLocale`.
#[cfg(not(target_os = "macos"))]
static mut DEFAULT_RUNE_LOCALE: RuneLocale = RuneLocale {
    __magic: [0x52, 0x75, 0x6E, 0x65, 0x4D, 0x61, 0x67, 0x69],
    __encoding: [0; 32],
    __sgetrune: 0,
    __sputrune: 0,
    __invalid_rune: 0xFFFD_u32 as i32,
    _pad0: 0,
    __runetype: [0; 256],
    __maplower: [0; 256],
    __mapupper: [0; 256],
};

#[cfg(not(target_os = "macos"))]
static INIT_RUNE_LOCALE: Once = Once::new();

/// Initialise the synthetic rune locale with ASCII classification data.
#[cfg(not(target_os = "macos"))]
fn ensure_rune_locale() {
    INIT_RUNE_LOCALE.call_once(|| {
        // Safety: single-threaded init via Once.
        unsafe {
            for i in 0..=255i32 {
                DEFAULT_RUNE_LOCALE.__runetype[i as usize] =
                    ascii_rune_flags(i) as u32;
                DEFAULT_RUNE_LOCALE.__maplower[i as usize] =
                    if (i as u8).is_ascii_uppercase() {
                        i + 32
                    } else {
                        i
                    };
                DEFAULT_RUNE_LOCALE.__mapupper[i as usize] =
                    if (i as u8).is_ascii_lowercase() {
                        i - 32
                    } else {
                        i
                    };
            }
        }
    });
}

/// Return the address of the synthetic `_DefaultRuneLocale`.
#[cfg(not(target_os = "macos"))]
fn shim_default_rune_locale_addr() -> *const () {
    ensure_rune_locale();
    // Safety: address of a static — stable for the process lifetime.
    unsafe { &DEFAULT_RUNE_LOCALE as *const RuneLocale as *const () }
}

// ── stdio file pointers ─────────────────────────────────────────────

/// macOS exposes `__stderrp`, `__stdoutp`, `__stdinp` as `FILE **` —
/// pointers to the actual `FILE *` values.  On Linux we store the host
/// `FILE *` values in static cells and hand out their addresses.
#[cfg(not(target_os = "macos"))]
static mut STDERR_PTR: *mut libc::FILE = std::ptr::null_mut();
#[cfg(not(target_os = "macos"))]
static mut STDOUT_PTR: *mut libc::FILE = std::ptr::null_mut();
#[cfg(not(target_os = "macos"))]
static mut STDIN_PTR: *mut libc::FILE = std::ptr::null_mut();

#[cfg(not(target_os = "macos"))]
static INIT_STDIO: Once = Once::new();

#[cfg(not(target_os = "macos"))]
enum StdioStream {
    Stderr,
    Stdout,
    Stdin,
}

#[cfg(not(target_os = "macos"))]
fn init_stdio_pointers() {
    INIT_STDIO.call_once(|| {
        unsafe {
            STDERR_PTR = libc::fdopen(2, b"w\0".as_ptr().cast());
            STDOUT_PTR = libc::fdopen(1, b"w\0".as_ptr().cast());
            STDIN_PTR = libc::fdopen(0, b"r\0".as_ptr().cast());
        }
    });
}

/// Return the address of one of the stdio FILE** cells.
#[cfg(not(target_os = "macos"))]
fn stdio_ptr_addr(stream: StdioStream) -> *const () {
    unsafe {
        match stream {
            StdioStream::Stderr => &STDERR_PTR as *const *mut libc::FILE as *const (),
            StdioStream::Stdout => &STDOUT_PTR as *const *mut libc::FILE as *const (),
            StdioStream::Stdin => &STDIN_PTR as *const *mut libc::FILE as *const (),
        }
    }
}

// ── MB_CUR_MAX ──────────────────────────────────────────────────────

/// `int __mb_cur_max(void)` — returns the max bytes per character in
/// the current locale.  On Linux, `MB_CUR_MAX` is a macro that calls
/// an internal glibc function.  We default to 4 (UTF-8).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_mb_cur_max() -> libc::c_int {
    // UTF-8 locales use up to 4 bytes; this is a safe default.
    4
}

// ── fortified string functions ──────────────────────────────────────

/// `size_t __strlcpy_chk(char *dst, const char *src, size_t len,
///                        size_t dstlen)`
///
/// macOS's fortified strlcpy.  On Linux we implement strlcpy directly
/// since glibc doesn't provide it (or the fortified variant).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_strlcpy_chk(
    dst: *mut libc::c_char,
    src: *const libc::c_char,
    len: libc::size_t,
    _dstlen: libc::size_t,
) -> libc::size_t {
    if dst.is_null() || src.is_null() {
        return 0;
    }
    unsafe {
        let src_len = libc::strlen(src);
        if len > 0 {
            let copy_len = if src_len < len { src_len } else { len - 1 };
            std::ptr::copy_nonoverlapping(src, dst, copy_len);
            *dst.add(copy_len) = 0;
        }
        src_len
    }
}

/// `size_t __strlcat_chk(char *dst, const char *src, size_t len,
///                        size_t dstlen)`
///
/// macOS's fortified strlcat.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_strlcat_chk(
    dst: *mut libc::c_char,
    src: *const libc::c_char,
    len: libc::size_t,
    _dstlen: libc::size_t,
) -> libc::size_t {
    if dst.is_null() || src.is_null() {
        return 0;
    }
    unsafe {
        let dst_len = libc::strlen(dst);
        let src_len = libc::strlen(src);
        if dst_len >= len {
            return dst_len + src_len;
        }
        let remaining = len - dst_len - 1;
        let copy_len = if src_len < remaining {
            src_len
        } else {
            remaining
        };
        std::ptr::copy_nonoverlapping(src, dst.add(dst_len), copy_len);
        *dst.add(dst_len + copy_len) = 0;
        dst_len + src_len
    }
}

// ── time ────────────────────────────────────────────────────────────

/// `uint64_t clock_gettime_nsec_np(clockid_t clock_id)`
///
/// macOS-only convenience that returns nanoseconds directly.
/// We implement via `clock_gettime(2)`.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_clock_gettime_nsec_np(clock_id: libc::clockid_t) -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let ret = unsafe { libc::clock_gettime(clock_id, &mut ts) };
    if ret != 0 {
        return 0;
    }
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}
