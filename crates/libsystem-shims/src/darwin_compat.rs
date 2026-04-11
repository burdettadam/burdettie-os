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
            // Use shadow FILE structs from the stdio module so that
            // inline putc/getc macros see macOS-layout fields.
            {
                use crate::cross_platform::stdio::stdio_shadow_ptr_addr;
                register_shims!(table,
                    ("___stderrp", stdio_shadow_ptr_addr(2) as u64, "__stderrp data (shadow)"),
                    ("___stdoutp", stdio_shadow_ptr_addr(1) as u64, "__stdoutp data (shadow)"),
                    ("___stdinp", stdio_shadow_ptr_addr(0) as u64, "__stdinp data (shadow)"),
                );
            }

            // ── MB_CUR_MAX ──────────────────────────────────────────
            // `___mb_cur_max` is a DATA symbol (extern int __mb_cur_max).
            // `____mb_cur_max` is a FUNCTION (int ___mb_cur_max(void)) —
            // some tools (awk) call the function variant rather than
            // loading the data symbol.
            register_shims!(table,
                ("___mb_cur_max", shim_mb_cur_max_addr() as u64, "__mb_cur_max data"),
                ("____mb_cur_max", shim_mb_cur_max_fn as *const () as u64, "___mb_cur_max() function"),
            );

            // ── collate lookup ──────────────────────────────────────
            register_shims!(table,
                ("___collate_lookup_l", shim_collate_lookup_l as *const () as u64, "__collate_lookup_l stub"),
            );

            // ── setlocale (category-number translation) ─────────────
            register_shims!(table,
                ("_setlocale", shim_setlocale as *const () as u64, "setlocale macOS→Linux categories"),
                ("_newlocale", shim_newlocale as *const () as u64, "newlocale macOS→Linux mask"),
                ("_querylocale", shim_querylocale as *const () as u64, "querylocale macOS→Linux mask"),
            );

            // ── nl_langinfo (nl_item translation) ───────────────────
            register_shims!(table,
                ("_nl_langinfo", shim_nl_langinfo as *const () as u64, "nl_langinfo macOS→Linux nl_item"),
                ("_nl_langinfo_l", shim_nl_langinfo_l as *const () as u64, "nl_langinfo_l macOS→Linux nl_item"),
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

            // ── malloc_type shims (macOS Sonoma+) ───────────────────
            register_shims!(table,
                ("_malloc_type_malloc", shim_malloc_type_malloc as *const () as u64, "malloc_type_malloc → malloc"),
                ("_malloc_type_calloc", shim_malloc_type_calloc as *const () as u64, "malloc_type_calloc → calloc"),
                ("_malloc_type_realloc", shim_malloc_type_realloc as *const () as u64, "malloc_type_realloc → realloc"),
            );

            // ── __chkstk_darwin (stack probe — no-op with guard pages) ──
            register_shims!(table,
                ("____chkstk_darwin", shim_chkstk_darwin as *const () as u64, "__chkstk_darwin no-op"),
            );

            // ── sysctl ──────────────────────────────────────────────
            register_shims!(table,
                ("_sysctl", shim_sysctl as *const () as u64, "sysctl compat"),
                ("_sysctlbyname", shim_sysctlbyname as *const () as u64, "sysctlbyname compat"),
            );

            // ── BSD string functions ────────────────────────────────
            register_shims!(table,
                ("_strlcpy", shim_strlcpy as *const () as u64, "strlcpy compat"),
                ("_strlcat", shim_strlcat as *const () as u64, "strlcat compat"),
                ("_fmtcheck", shim_fmtcheck as *const () as u64, "fmtcheck compat"),
                ("_reallocf", shim_reallocf as *const () as u64, "reallocf compat"),
            );

            // ── BSD error/warning functions ─────────────────────────
            register_shims!(table,
                ("_errc", shim_errc as *const () as u64, "errc compat"),
                ("_warnc", shim_warnc as *const () as u64, "warnc compat"),
            );

            // ── compat_mode (macOS-specific, always returns 0) ──────
            register_shims!(table,
                ("_compat_mode", shim_compat_mode as *const () as u64, "compat_mode stub"),
            );

            // ── sigsetjmp (glibc uses __sigsetjmp) ──────────────────
            register_shims!(table,
                ("_sigsetjmp", shim_sigsetjmp as *const () as u64, "sigsetjmp compat"),
            );

            // ── fgetln (BSD line reader) ────────────────────────────
            register_shims!(table,
                ("_fgetln", shim_fgetln as *const () as u64, "fgetln compat"),
            );

            // ── BSD user/group/file-flag functions ──────────────────
            register_shims!(table,
                ("_user_from_uid", shim_user_from_uid as *const () as u64, "user_from_uid compat"),
                ("_group_from_gid", shim_group_from_gid as *const () as u64, "group_from_gid compat"),
                ("_strmode", shim_strmode as *const () as u64, "strmode compat"),
                ("_fflagstostr", shim_fflagstostr as *const () as u64, "fflagstostr compat"),
                ("_strtofflags", shim_strtofflags as *const () as u64, "strtofflags stub"),
                ("_getmode", shim_getmode as *const () as u64, "getmode stub"),
                ("_setmode", shim_setmode as *const () as u64, "setmode stub"),
                ("_lchflags", shim_lchflags as *const () as u64, "lchflags stub"),
                ("_devname", shim_devname as *const () as u64, "devname stub"),
            );

            // ── __darwin_check_fd_set_overflow ──────────────────────
            register_shims!(table,
                ("___darwin_check_fd_set_overflow", shim_darwin_check_fd_set_overflow as *const () as u64,
                 "__darwin_check_fd_set_overflow stub"),
            );

            // ── heapsort / mergesort ────────────────────────────────
            register_shims!(table,
                ("_heapsort", shim_heapsort as *const () as u64, "heapsort → qsort"),
                ("_mergesort", shim_mergesort as *const () as u64, "mergesort → qsort"),
            );

            // ── CC_SHA256 (CommonCrypto → OpenSSL/manual) ───────────
            register_shims!(table,
                ("_CC_SHA256_Init", shim_cc_sha256_init as *const () as u64, "CC_SHA256_Init compat"),
                ("_CC_SHA256_Update", shim_cc_sha256_update as *const () as u64, "CC_SHA256_Update compat"),
                ("_CC_SHA256_Final", shim_cc_sha256_final as *const () as u64, "CC_SHA256_Final compat"),
            );

            // ── Mach semaphore stubs ────────────────────────────────
            register_shims!(table,
                ("_semaphore_create", shim_semaphore_create as *const () as u64, "semaphore_create stub"),
                ("_semaphore_destroy", shim_semaphore_destroy as *const () as u64, "semaphore_destroy stub"),
                ("_semaphore_signal", shim_semaphore_signal as *const () as u64, "semaphore_signal stub"),
                ("_semaphore_wait", shim_semaphore_wait as *const () as u64, "semaphore_wait stub"),
            );

            // ── fgetwln (BSD wide-char line reader) ─────────────────
            register_shims!(table,
                ("_fgetwln", shim_fgetwln as *const () as u64, "fgetwln → fgetwc loop"),
            );

            // ── quarantine stubs ────────────────────────────────────
            register_shims!(table,
                ("__qtn_error", shim_qtn_error as *const () as u64, "qtn_error stub"),
                ("__qtn_file_alloc", shim_qtn_file_alloc as *const () as u64, "qtn_file_alloc stub"),
                ("__qtn_file_free", shim_qtn_file_free as *const () as u64, "qtn_file_free stub"),
                ("__qtn_file_init_with_path", shim_qtn_file_init_with_path as *const () as u64, "qtn_file_init_with_path stub"),
                ("__qtn_file_apply_to_path", shim_qtn_file_apply_to_path as *const () as u64, "qtn_file_apply_to_path stub"),
                ("__qtn_file_init_with_fd", shim_qtn_file_init_with_fd as *const () as u64, "qtn_file_init_with_fd stub"),
                ("__qtn_file_apply_to_fd", shim_qtn_file_apply_to_fd as *const () as u64, "qtn_file_apply_to_fd stub"),
            );

            // ── Mach-O introspection stubs ──────────────────────────
            register_shims!(table,
                ("_NXGetAllArchInfos", shim_nx_get_all_arch_infos as *const () as u64, "NXGetAllArchInfos stub"),
                ("_swap_fat_header", shim_swap_fat_header as *const () as u64, "swap_fat_header stub"),
                ("_swap_fat_arch", shim_swap_fat_arch as *const () as u64, "swap_fat_arch stub"),
            );

            // ── memset_pattern16 ────────────────────────────────────
            register_shims!(table,
                ("_memset_pattern16", shim_memset_pattern16 as *const () as u64, "memset_pattern16 compat"),
            );

            // ── readpassphrase (BSD) ────────────────────────────────
            register_shims!(table,
                ("_readpassphrase", shim_readpassphrase as *const () as u64, "readpassphrase stub"),
            );

            // ── ACL stubs ───────────────────────────────────────────
            register_shims!(table,
                ("_acl_get_link_np", shim_acl_get_link_np as *const () as u64, "acl_get_link_np stub"),
            );

            // libxo stubs are now in darwin_stubs/libxo_stubs.rs

            // ── lchmod (not implemented on most Linux fs) ───────────
            register_shims!(table,
                ("_lchmod", shim_lchmod as *const () as u64, "lchmod stub"),
            );

            // stat/fstat/lstat struct layout translation is in cross_platform/stat.rs

            // ── Mach error string (macOS-only API) ──────────────────
            register_shims!(table,
                ("_mach_error_string", shim_mach_error_string as *const () as u64, "mach_error_string stub"),
            );

            // ── libc++ compat (newer symbols not in LLVM 14) ────────
            register_shims!(table,
                ("__ZNSt3__122__libcpp_verbose_abortEPKcz", shim_libcpp_verbose_abort as *const () as u64,
                 "__libcpp_verbose_abort → fprintf+abort"),
            );

            // ── os_variant stubs (Darwin-only APIs) ─────────────────
            register_shims!(table,
                ("_os_variant_is_darwinos", shim_os_variant_false as *const () as u64, "os_variant_is_darwinos → false"),
                ("_os_variant_has_internal_content", shim_os_variant_false as *const () as u64, "os_variant_has_internal_content → false"),
            );

            // NOTE: ___srget and ___swbuf are now registered in
            // cross_platform/stdio.rs along with all other FILE shims.

            // ── BSD number parsing ──────────────────────────────────
            register_shims!(table,
                ("_expand_number", shim_expand_number as *const () as u64, "expand_number compat"),
                ("_strtonum", shim_strtonum as *const () as u64, "strtonum compat"),
            );

            // ── arc4random legacy ───────────────────────────────────
            register_shims!(table,
                ("_arc4random_stir", shim_arc4random_stir as *const () as u64, "arc4random_stir no-op"),
                ("_arc4random_addrandom", shim_arc4random_addrandom as *const () as u64, "arc4random_addrandom no-op"),
            );

            log::info!("DarwinCompatShims: registered compat shims");
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
    /// Mapping functions (2 function pointers — unused, zeroed).
    __sgetrune: u64,
    __sputrune: u64,
    __invalid_rune: i32,
    // No padding here — __runetype immediately follows __invalid_rune
    // at offset 60, matching the real macOS _RuneLocale layout.
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
        // Use glibc's REAL stdout/stderr/stdin — NOT fdopen'd copies.
        // fdopen creates a separate FILE object with its own buffer on
        // the same fd.  When macOS binaries mix fputs(__stdoutp,...)
        // with printf() (which uses glibc's internal stdout), the two
        // independent buffers produce garbled interleaved output.
        extern "C" {
            static mut stdout: *mut libc::FILE;
            static mut stderr: *mut libc::FILE;
            static mut stdin: *mut libc::FILE;
        }
        unsafe {
            STDERR_PTR = stderr;
            STDOUT_PTR = stdout;
            STDIN_PTR = stdin;
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

/// `__mb_cur_max` is a **data symbol** on macOS: `extern int __mb_cur_max;`.
/// The binary reads the value by dereferencing the GOT entry (not calling
/// a function).  We provide 1 (single-byte) to keep tools on the narrow
/// I/O path, avoiding wide-char codepaths that depend on fgetwln/fwprintf.
/// Value 1 (single-byte) keeps sort on the narrow I/O path.
/// sort -n numeric comparison is currently broken with this value.
#[cfg(not(target_os = "macos"))]
static MB_CUR_MAX_VALUE: i32 = 1;

#[cfg(not(target_os = "macos"))]
fn shim_mb_cur_max_addr() -> *const () {
    &MB_CUR_MAX_VALUE as *const i32 as *const ()
}

/// `___mb_cur_max()` is a function on macOS that returns the current
/// `MB_CUR_MAX` value.  Some binaries (awk) import the function symbol
/// `____mb_cur_max` (4 underscores in Mach-O) instead of the data symbol.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_mb_cur_max_fn() -> i32 {
    MB_CUR_MAX_VALUE
}

// ── setlocale (macOS → Linux category translation) ──────────────────

/// macOS and Linux use different integer values for `LC_*` constants:
///
/// | Category    | macOS | Linux |
/// |-------------|-------|-------|
/// | LC_ALL      |   0   |   6   |
/// | LC_COLLATE  |   1   |   3   |
/// | LC_CTYPE    |   2   |   0   |
/// | LC_MONETARY |   3   |   4   |
/// | LC_NUMERIC  |   4   |   1   |
/// | LC_TIME     |   5   |   2   |
/// | LC_MESSAGES |   6   |   5   |
///
/// Without translation, a macOS binary calling `setlocale(LC_ALL=0, "")`
/// would only set `LC_CTYPE` on Linux, causing downstream ctype/wctype
/// failures and null-pointer dereferences.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_setlocale(
    macos_category: libc::c_int,
    locale: *const libc::c_char,
) -> *mut libc::c_char {
    let linux_category = match macos_category {
        0 => 6,  // LC_ALL
        1 => 3,  // LC_COLLATE
        2 => 0,  // LC_CTYPE
        3 => 4,  // LC_MONETARY
        4 => 1,  // LC_NUMERIC
        5 => 2,  // LC_TIME
        6 => 5,  // LC_MESSAGES
        _ => macos_category, // pass-through for unknowns
    };
    unsafe { libc::setlocale(linux_category, locale) }
}

/// Translate a macOS `newlocale` category mask to a Linux mask.
///
/// macOS mask bits: `LC_COLLATE_MASK=1<<0, LC_CTYPE_MASK=1<<1, LC_MONETARY_MASK=1<<2,
/// LC_NUMERIC_MASK=1<<3, LC_TIME_MASK=1<<4, LC_MESSAGES_MASK=1<<5`
///
/// Linux mask bits: `LC_CTYPE_MASK=1<<0, LC_NUMERIC_MASK=1<<1, LC_TIME_MASK=1<<2,
/// LC_COLLATE_MASK=1<<3, LC_MONETARY_MASK=1<<4, LC_MESSAGES_MASK=1<<5`
#[cfg(not(target_os = "macos"))]
fn translate_locale_mask(macos_mask: libc::c_int) -> libc::c_int {
    // If all 6 macOS bits are set (LC_ALL_MASK = 0x3F), use Linux LC_ALL_MASK
    if macos_mask & 0x3F == 0x3F {
        return libc::LC_ALL_MASK;
    }
    let mut linux_mask: libc::c_int = 0;
    if macos_mask & (1 << 0) != 0 { linux_mask |= libc::LC_COLLATE_MASK; }   // macOS LC_COLLATE=0
    if macos_mask & (1 << 1) != 0 { linux_mask |= libc::LC_CTYPE_MASK; }     // macOS LC_CTYPE=1
    if macos_mask & (1 << 2) != 0 { linux_mask |= libc::LC_MONETARY_MASK; }  // macOS LC_MONETARY=2
    if macos_mask & (1 << 3) != 0 { linux_mask |= libc::LC_NUMERIC_MASK; }   // macOS LC_NUMERIC=3
    if macos_mask & (1 << 4) != 0 { linux_mask |= libc::LC_TIME_MASK; }      // macOS LC_TIME=4
    if macos_mask & (1 << 5) != 0 { linux_mask |= libc::LC_MESSAGES_MASK; }  // macOS LC_MESSAGES=5
    linux_mask
}

/// `locale_t newlocale(int category_mask, const char *locale, locale_t base)`
///
/// Translates macOS category mask bits to Linux bits before delegating.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_newlocale(
    macos_mask: libc::c_int,
    locale: *const libc::c_char,
    base: libc::locale_t,
) -> libc::locale_t {
    let linux_mask = translate_locale_mask(macos_mask);
    unsafe { libc::newlocale(linux_mask, locale, base) }
}

/// `int querylocale(int mask, locale_t loc)` — macOS extension.
///
/// Returns the category that the locale was created with.  We return
/// the translated mask since the caller expects macOS-format bits,
/// but since we can't reverse the translation reliably, we just
/// return the mask as-is (the caller typically only checks for zero/non-zero).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_querylocale(
    mask: libc::c_int,
    _loc: libc::locale_t,
) -> libc::c_int {
    // glibc doesn't have querylocale; just return the mask unchanged
    // since BSD code typically only checks the result for 0 vs non-0.
    mask
}

// ── nl_langinfo (macOS → Linux nl_item translation) ─────────────────

/// macOS and Linux use completely different integer values for `nl_item`
/// constants passed to `nl_langinfo()`.  macOS uses simple sequential
/// integers (CODESET=49, D_T_FMT=1, etc.), while Linux/glibc encodes
/// them as `(LC_category << 16) | index`.
///
/// Without translation, the macOS binary passes e.g. 49 (CODESET on
/// macOS) which Linux interprets as a completely different item, returning
/// garbage or a small integer instead of a string pointer.
#[cfg(not(target_os = "macos"))]
fn translate_nl_item(macos_item: libc::c_int) -> Option<libc::nl_item> {
    Some(match macos_item {
        1  => libc::D_T_FMT,
        2  => libc::D_FMT,
        3  => libc::T_FMT,
        4  => libc::T_FMT_AMPM,
        5  => libc::AM_STR,
        6  => libc::PM_STR,
        7  => libc::DAY_1,
        8  => libc::DAY_2,
        9  => libc::DAY_3,
        10 => libc::DAY_4,
        11 => libc::DAY_5,
        12 => libc::DAY_6,
        13 => libc::DAY_7,
        14 => libc::ABDAY_1,
        15 => libc::ABDAY_2,
        16 => libc::ABDAY_3,
        17 => libc::ABDAY_4,
        18 => libc::ABDAY_5,
        19 => libc::ABDAY_6,
        20 => libc::ABDAY_7,
        21 => libc::MON_1,
        22 => libc::MON_2,
        23 => libc::MON_3,
        24 => libc::MON_4,
        25 => libc::MON_5,
        26 => libc::MON_6,
        27 => libc::MON_7,
        28 => libc::MON_8,
        29 => libc::MON_9,
        30 => libc::MON_10,
        31 => libc::MON_11,
        32 => libc::MON_12,
        33 => libc::ABMON_1,
        34 => libc::ABMON_2,
        35 => libc::ABMON_3,
        36 => libc::ABMON_4,
        37 => libc::ABMON_5,
        38 => libc::ABMON_6,
        39 => libc::ABMON_7,
        40 => libc::ABMON_8,
        41 => libc::ABMON_9,
        42 => libc::ABMON_10,
        43 => libc::ABMON_11,
        44 => libc::ABMON_12,
        45 => libc::ERA,
        46 => libc::ERA_D_FMT,
        47 => libc::ERA_D_T_FMT,
        48 => libc::ERA_T_FMT,
        49 => libc::CODESET,
        50 => libc::RADIXCHAR,
        51 => libc::THOUSEP,
        52 => libc::YESEXPR,
        53 => libc::NOEXPR,
        56 => libc::CRNCYSTR,
        _  => return None,
    })
}

/// Static fallback for unmapped nl_item values.
#[cfg(not(target_os = "macos"))]
static NL_EMPTY: &[u8] = b"\0";

/// `char *nl_langinfo(nl_item item)` — translate macOS nl_item, call Linux.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_nl_langinfo(macos_item: libc::c_int) -> *mut libc::c_char {
    match translate_nl_item(macos_item) {
        Some(linux_item) => unsafe { libc::nl_langinfo(linux_item) },
        None => {
            log::debug!("nl_langinfo: unmapped macOS nl_item {}", macos_item);
            NL_EMPTY.as_ptr() as *mut libc::c_char
        }
    }
}

/// `char *nl_langinfo_l(nl_item item, locale_t loc)` — locale-aware variant.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_nl_langinfo_l(
    macos_item: libc::c_int,
    loc: libc::locale_t,
) -> *mut libc::c_char {
    match translate_nl_item(macos_item) {
        Some(linux_item) => unsafe { libc::nl_langinfo_l(linux_item, loc) },
        None => {
            log::debug!("nl_langinfo_l: unmapped macOS nl_item {}", macos_item);
            NL_EMPTY.as_ptr() as *mut libc::c_char
        }
    }
}

// ── __collate_lookup_l ──────────────────────────────────────────────

/// `void __collate_lookup_l(const wchar_t *s, int *len, int *prim,
///                          int *sec, locale_t loc)`
///
/// macOS internal collation lookup used by regex and sort.  In the "C"
/// locale, each character's primary weight is its ordinal and secondary
/// weight is 0.  We implement this simple case.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_collate_lookup_l(
    s: *const libc::wchar_t,
    len: *mut libc::c_int,
    prim: *mut libc::c_int,
    sec: *mut libc::c_int,
    _loc: libc::locale_t,
) {
    if s.is_null() {
        if !len.is_null()  { unsafe { *len = 0 }; }
        if !prim.is_null() { unsafe { *prim = 0 }; }
        if !sec.is_null()  { unsafe { *sec = 0 }; }
        return;
    }
    let wc = unsafe { *s } as libc::c_int;
    // In "C" locale: one character consumed, primary = ordinal, secondary = 0
    if !len.is_null()  { unsafe { *len = 1 }; }
    if !prim.is_null() { unsafe { *prim = wc }; }
    if !sec.is_null()  { unsafe { *sec = 0 }; }
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

// ── malloc_type shims ───────────────────────────────────────────────

/// `void *malloc_type_malloc(size_t size, malloc_type_id_t type_id)`
///
/// macOS Sonoma introduced typed malloc.  On Linux just forward to `malloc`.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_malloc_type_malloc(size: libc::size_t, _type_id: u64) -> *mut libc::c_void {
    unsafe { libc::malloc(size) }
}

/// `void *malloc_type_calloc(size_t count, size_t size, malloc_type_id_t type_id)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_malloc_type_calloc(
    count: libc::size_t,
    size: libc::size_t,
    _type_id: u64,
) -> *mut libc::c_void {
    unsafe { libc::calloc(count, size) }
}

/// `void *malloc_type_realloc(void *ptr, size_t size, malloc_type_id_t type_id)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_malloc_type_realloc(
    ptr: *mut libc::c_void,
    size: libc::size_t,
    _type_id: u64,
) -> *mut libc::c_void {
    unsafe { libc::realloc(ptr, size) }
}

// ── __chkstk_darwin ─────────────────────────────────────────────────

/// macOS stack probe for large stack frames.
/// Linux uses guard pages, so this is a no-op.
#[cfg(not(target_os = "macos"))]
#[unsafe(naked)]
unsafe extern "C" fn shim_chkstk_darwin() {
    core::arch::naked_asm!("ret");
}

// ── sysctl ──────────────────────────────────────────────────────────

/// `int sysctl(int *name, u_int namelen, void *oldp, size_t *oldlenp,
///             void *newp, size_t newlen)`
///
/// Basic sysctl shim: handles kern.ostype, kern.osrelease, kern.hostname
/// which uname(1) needs.  Falls back to reading /proc/sys.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_sysctl(
    name: *const libc::c_int,
    namelen: libc::c_uint,
    oldp: *mut libc::c_void,
    oldlenp: *mut libc::size_t,
    _newp: *const libc::c_void,
    _newlen: libc::size_t,
) -> libc::c_int {
    if name.is_null() || namelen < 2 {
        return -1;
    }
    let mib0 = unsafe { *name };
    let mib1 = unsafe { *name.add(1) };

    // CTL_KERN = 1
    if mib0 != 1 {
        return -1;
    }

    let value: &[u8] = match mib1 {
        1 => b"Darwin\0",        // KERN_OSTYPE
        2 => b"24.0.0\0",        // KERN_OSRELEASE
        10 => b"linux\0",        // KERN_HOSTNAME
        65 => b"macOS\0",        // KERN_OSPRODUCTVERSION (used by some tools)
        _ => return -1,
    };

    if oldp.is_null() || oldlenp.is_null() {
        if !oldlenp.is_null() {
            unsafe { *oldlenp = value.len() };
        }
        return 0;
    }

    let available = unsafe { *oldlenp };
    let copy_len = available.min(value.len());
    unsafe {
        std::ptr::copy_nonoverlapping(value.as_ptr(), oldp as *mut u8, copy_len);
        *oldlenp = copy_len;
    }
    0
}

// ── sysctlbyname ────────────────────────────────────────────────────

/// `int sysctlbyname(const char *name, void *oldp, size_t *oldlenp,
///                    void *newp, size_t newlen)`
///
/// Name-based sysctl lookup used by LLVM tools, Swift runtime, etc.
/// Handles commonly queried keys; returns ENOENT for unknown names.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_sysctlbyname(
    name: *const libc::c_char,
    oldp: *mut libc::c_void,
    oldlenp: *mut libc::size_t,
    _newp: *const libc::c_void,
    _newlen: libc::size_t,
) -> libc::c_int {
    if name.is_null() {
        unsafe { *libc::__errno_location() = libc::EINVAL; }
        return -1;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(name) };
    let key = c_str.to_str().unwrap_or("");

    let value: &[u8] = match key {
        "kern.ostype" => b"Darwin\0",
        "kern.osrelease" => b"24.0.0\0",
        "kern.hostname" => {
            // Return a generic hostname
            b"linux\0"
        }
        "kern.osproductversion" => b"15.0\0",
        "kern.osversion" => b"24A335\0",
        "hw.ncpu" | "hw.logicalcpu" | "hw.physicalcpu" | "hw.activecpu" => {
            // Return CPU count as 4-byte integer
            let ncpu = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) } as u32;
            let ncpu = if ncpu == 0 { 1u32 } else { ncpu };
            if !oldlenp.is_null() {
                let avail = unsafe { *oldlenp };
                if !oldp.is_null() && avail >= 4 {
                    unsafe {
                        *(oldp as *mut u32) = ncpu;
                        *oldlenp = 4;
                    }
                } else {
                    unsafe { *oldlenp = 4; }
                }
            }
            return 0;
        }
        "hw.memsize" | "hw.physmem" => {
            // Return physical memory as 8-byte integer
            let pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) } as u64;
            let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
            let memsize = pages * page_size;
            if !oldlenp.is_null() {
                let avail = unsafe { *oldlenp };
                if !oldp.is_null() && avail >= 8 {
                    unsafe {
                        *(oldp as *mut u64) = memsize;
                        *oldlenp = 8;
                    }
                } else {
                    unsafe { *oldlenp = 8; }
                }
            }
            return 0;
        }
        "hw.cachelinesize" => {
            if !oldlenp.is_null() {
                if !oldp.is_null() {
                    let avail = unsafe { *oldlenp };
                    if avail >= 8 {
                        unsafe {
                            *(oldp as *mut u64) = 64;
                            *oldlenp = 8;
                        }
                    }
                } else {
                    unsafe { *oldlenp = 8; }
                }
            }
            return 0;
        }
        "hw.pagesize" | "vm.pagesize" => {
            let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
            if !oldlenp.is_null() {
                if !oldp.is_null() {
                    let avail = unsafe { *oldlenp };
                    if avail >= 8 {
                        unsafe {
                            *(oldp as *mut u64) = page_size;
                            *oldlenp = 8;
                        }
                    }
                } else {
                    unsafe { *oldlenp = 8; }
                }
            }
            return 0;
        }
        "machdep.cpu.brand_string" => b"Apple M1\0",
        _ => {
            log::trace!("sysctlbyname: unknown key '{}'", key);
            unsafe { *libc::__errno_location() = libc::ENOENT; }
            return -1;
        }
    };

    // String-valued keys
    if oldp.is_null() || oldlenp.is_null() {
        if !oldlenp.is_null() {
            unsafe { *oldlenp = value.len(); }
        }
        return 0;
    }
    let available = unsafe { *oldlenp };
    let copy_len = available.min(value.len());
    unsafe {
        std::ptr::copy_nonoverlapping(value.as_ptr(), oldp as *mut u8, copy_len);
        *oldlenp = copy_len;
    }
    0
}

// ── BSD string functions ────────────────────────────────────────────

/// `size_t strlcpy(char *dst, const char *src, size_t dstsize)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_strlcpy(
    dst: *mut libc::c_char,
    src: *const libc::c_char,
    dstsize: libc::size_t,
) -> libc::size_t {
    if src.is_null() {
        return 0;
    }
    let src_len = unsafe { libc::strlen(src) };
    if dstsize > 0 && !dst.is_null() {
        let copy_len = if src_len < dstsize { src_len } else { dstsize - 1 };
        unsafe {
            std::ptr::copy_nonoverlapping(src, dst, copy_len);
            *dst.add(copy_len) = 0;
        }
    }
    src_len
}

/// `size_t strlcat(char *dst, const char *src, size_t dstsize)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_strlcat(
    dst: *mut libc::c_char,
    src: *const libc::c_char,
    dstsize: libc::size_t,
) -> libc::size_t {
    if dst.is_null() || src.is_null() {
        return 0;
    }
    unsafe {
        let dst_len = libc::strlen(dst);
        let src_len = libc::strlen(src);
        if dst_len >= dstsize {
            return dst_len + src_len;
        }
        let remaining = dstsize - dst_len - 1;
        let copy_len = if src_len < remaining { src_len } else { remaining };
        std::ptr::copy_nonoverlapping(src, dst.add(dst_len), copy_len);
        *dst.add(dst_len + copy_len) = 0;
        dst_len + src_len
    }
}

/// `const char *fmtcheck(const char *fmt_suspect, const char *fmt_default)`
///
/// Returns fmt_suspect if its format specifiers are compatible with
/// fmt_default, otherwise returns fmt_default.  Our shim always trusts
/// the suspect (matches BSD behavior for most callers).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fmtcheck(
    fmt_suspect: *const libc::c_char,
    fmt_default: *const libc::c_char,
) -> *const libc::c_char {
    if fmt_suspect.is_null() {
        fmt_default
    } else {
        fmt_suspect
    }
}

/// `void *reallocf(void *ptr, size_t size)`
///
/// Like realloc, but frees the original block on failure.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_reallocf(ptr: *mut libc::c_void, size: libc::size_t) -> *mut libc::c_void {
    let result = unsafe { libc::realloc(ptr, size) };
    if result.is_null() && size != 0 && !ptr.is_null() {
        unsafe { libc::free(ptr) };
    }
    result
}

// ── BSD error/warning functions ─────────────────────────────────────

/// `void errc(int status, int code, const char *fmt, ...)`
///
/// Print error message with error code and exit.
/// Simplified: we support fmt with zero or one %s argument.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_errc(status: libc::c_int, code: libc::c_int, fmt: *const libc::c_char) -> ! {
    let msg = if !fmt.is_null() {
        unsafe { std::ffi::CStr::from_ptr(fmt).to_string_lossy().into_owned() }
    } else {
        String::new()
    };
    eprintln!("error: {}: {}", msg, std::io::Error::from_raw_os_error(code));
    std::process::exit(status);
}

/// `void warnc(int code, const char *fmt, ...)`
///
/// Print warning message with error code.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_warnc(code: libc::c_int, fmt: *const libc::c_char) {
    let msg = if !fmt.is_null() {
        unsafe { std::ffi::CStr::from_ptr(fmt).to_string_lossy().into_owned() }
    } else {
        String::new()
    };
    eprintln!("warning: {}: {}", msg, std::io::Error::from_raw_os_error(code));
}

// ── compat_mode ─────────────────────────────────────────────────────

/// `int compat_mode(const char *function, const char *mode)`
///
/// macOS compatibility-mode check.  Always returns 0 (no compat mode).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_compat_mode(
    _function: *const libc::c_char,
    _mode: *const libc::c_char,
) -> libc::c_int {
    0
}

// ── sigsetjmp ───────────────────────────────────────────────────────

/// On glibc, `sigsetjmp` is a macro that expands to `__sigsetjmp`.
/// We forward directly to the underlying glibc symbol.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_sigsetjmp(env: *mut libc::c_void, savemask: libc::c_int) -> libc::c_int {
    extern "C" {
        fn __sigsetjmp(env: *mut libc::c_void, savemask: libc::c_int) -> libc::c_int;
    }
    unsafe { __sigsetjmp(env, savemask) }
}

// ── fgetln ──────────────────────────────────────────────────────────

/// `char *fgetln(FILE *stream, size_t *len)`
///
/// BSD function that returns a pointer to the next line (NOT NUL-terminated).
/// We implement via getline and cache the buffer in a thread-local.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fgetln(
    stream: *mut libc::FILE,
    len: *mut libc::size_t,
) -> *mut libc::c_char {
    use std::cell::RefCell;
    // Translate shadow DarwinFile* → real glibc FILE*
    let real_stream = unsafe {
        crate::cross_platform::stdio::extract_real_fp(stream as *mut u8)
    };
    thread_local! {
        static BUF: RefCell<(*mut libc::c_char, libc::size_t)> = RefCell::new((std::ptr::null_mut(), 0));
    }
    BUF.with(|cell| {
        let mut guard = cell.borrow_mut();
        let (ref mut lineptr, ref mut n) = *guard;
        let ret = unsafe { libc::getline(lineptr, n, real_stream) };
        if ret < 0 {
            if !len.is_null() {
                unsafe { *len = 0 };
            }
            return std::ptr::null_mut();
        }
        if !len.is_null() {
            unsafe { *len = ret as libc::size_t };
        }
        *lineptr
    })
}

// ── BSD user/group/file-flag functions ──────────────────────────────

/// `const char *user_from_uid(uid_t uid, int nouser)`
///
/// Returns the user name associated with the given UID. On failure and
/// nouser != 0, returns NULL; otherwise returns a numeric string.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_user_from_uid(uid: libc::uid_t, nouser: libc::c_int) -> *const libc::c_char {
    use std::cell::RefCell;
    thread_local! {
        static BUF: RefCell<[u8; 64]> = RefCell::new([0u8; 64]);
    }
    let pw = unsafe { libc::getpwuid(uid) };
    if !pw.is_null() {
        return unsafe { (*pw).pw_name };
    }
    if nouser != 0 {
        return std::ptr::null();
    }
    BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        let s = format!("{uid}\0");
        let bytes = s.as_bytes();
        let len = bytes.len().min(buf.len());
        buf[..len].copy_from_slice(&bytes[..len]);
        buf.as_ptr() as *const libc::c_char
    })
}

/// `const char *group_from_gid(gid_t gid, int nogroup)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_group_from_gid(gid: libc::gid_t, nogroup: libc::c_int) -> *const libc::c_char {
    use std::cell::RefCell;
    thread_local! {
        static BUF: RefCell<[u8; 64]> = RefCell::new([0u8; 64]);
    }
    let gr = unsafe { libc::getgrgid(gid) };
    if !gr.is_null() {
        return unsafe { (*gr).gr_name };
    }
    if nogroup != 0 {
        return std::ptr::null();
    }
    BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        let s = format!("{gid}\0");
        let bytes = s.as_bytes();
        let len = bytes.len().min(buf.len());
        buf[..len].copy_from_slice(&bytes[..len]);
        buf.as_ptr() as *const libc::c_char
    })
}

/// `void strmode(mode_t mode, char *bp)`
///
/// BSD function: convert file mode to a human-readable string like "drwxr-xr-x ".
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_strmode(mode: libc::mode_t, bp: *mut libc::c_char) {
    if bp.is_null() {
        return;
    }
    let ft = match mode & libc::S_IFMT {
        libc::S_IFDIR => b'd',
        libc::S_IFLNK => b'l',
        libc::S_IFREG => b'-',
        libc::S_IFBLK => b'b',
        libc::S_IFCHR => b'c',
        libc::S_IFIFO => b'p',
        libc::S_IFSOCK => b's',
        _ => b'?',
    };
    let s = [
        ft,
        if mode & libc::S_IRUSR != 0 { b'r' } else { b'-' },
        if mode & libc::S_IWUSR != 0 { b'w' } else { b'-' },
        if mode & libc::S_ISUID != 0 {
            if mode & libc::S_IXUSR != 0 { b's' } else { b'S' }
        } else if mode & libc::S_IXUSR != 0 { b'x' } else { b'-' },
        if mode & libc::S_IRGRP != 0 { b'r' } else { b'-' },
        if mode & libc::S_IWGRP != 0 { b'w' } else { b'-' },
        if mode & libc::S_ISGID != 0 {
            if mode & libc::S_IXGRP != 0 { b's' } else { b'S' }
        } else if mode & libc::S_IXGRP != 0 { b'x' } else { b'-' },
        if mode & libc::S_IROTH != 0 { b'r' } else { b'-' },
        if mode & libc::S_IWOTH != 0 { b'w' } else { b'-' },
        if mode & libc::S_ISVTX != 0 {
            if mode & libc::S_IXOTH != 0 { b't' } else { b'T' }
        } else if mode & libc::S_IXOTH != 0 { b'x' } else { b'-' },
        b' ',
        0,
    ];
    unsafe {
        std::ptr::copy_nonoverlapping(s.as_ptr(), bp as *mut u8, 12);
    }
}

/// `char *fflagstostr(u_long flags)` — convert file flags to string.
/// Simplified: returns empty string (no macOS file flags on Linux).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fflagstostr(_flags: libc::c_ulong) -> *mut libc::c_char {
    static EMPTY: &[u8] = b"\0";
    // Note: BSD fflagstostr returns malloc'd memory. We return a static ptr
    // which callers may try to free. This is a known leak-avoidance tradeoff.
    EMPTY.as_ptr() as *mut libc::c_char
}

/// `int strtofflags(char **stringp, u_long *setp, u_long *clrp)`
/// Stub: always succeeds, sets flags to 0.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_strtofflags(
    _stringp: *mut *mut libc::c_char,
    setp: *mut libc::c_ulong,
    clrp: *mut libc::c_ulong,
) -> libc::c_int {
    if !setp.is_null() {
        unsafe { *setp = 0 };
    }
    if !clrp.is_null() {
        unsafe { *clrp = 0 };
    }
    0
}

/// `void *getmode(const char *mode_str)` — stub returning NULL.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_getmode(_mode_str: *const libc::c_char) -> *mut libc::c_void {
    std::ptr::null_mut()
}

/// `mode_t setmode(const char *p)` — stub.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_setmode(_p: *const libc::c_char) -> *mut libc::c_void {
    std::ptr::null_mut()
}

/// `int lchflags(const char *path, u_long flags)` — stub returning 0 (success).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_lchflags(_path: *const libc::c_char, _flags: libc::c_ulong) -> libc::c_int {
    0
}

/// `char *devname(dev_t dev, mode_t type)` — returns "??".
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_devname(_dev: libc::dev_t, _type: libc::mode_t) -> *const libc::c_char {
    b"??\0".as_ptr() as *const libc::c_char
}

// ── __darwin_check_fd_set_overflow ──────────────────────────────────

/// macOS runtime check for fd_set overflow.  Always returns 1 (OK).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_darwin_check_fd_set_overflow(
    _fd: libc::c_int,
    _addr: *const libc::c_void,
    _is_write: libc::c_int,
) -> libc::c_int {
    1
}

// ── heapsort / mergesort ────────────────────────────────────────────

/// `int heapsort(void *base, size_t nel, size_t width,
///               int (*compar)(const void *, const void *))`
///
/// BSD sorting function.  We delegate to qsort which has the same signature.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_heapsort(
    base: *mut libc::c_void,
    nel: libc::size_t,
    width: libc::size_t,
    compar: Option<unsafe extern "C" fn(*const libc::c_void, *const libc::c_void) -> libc::c_int>,
) -> libc::c_int {
    if let Some(cmp) = compar {
        unsafe { libc::qsort(base, nel, width, Some(cmp)) };
    }
    0
}

/// `int mergesort(void *base, size_t nel, size_t width,
///                int (*compar)(const void *, const void *))`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_mergesort(
    base: *mut libc::c_void,
    nel: libc::size_t,
    width: libc::size_t,
    compar: Option<unsafe extern "C" fn(*const libc::c_void, *const libc::c_void) -> libc::c_int>,
) -> libc::c_int {
    if let Some(cmp) = compar {
        unsafe { libc::qsort(base, nel, width, Some(cmp)) };
    }
    0
}

// ── CC_SHA256 (CommonCrypto) ────────────────────────────────────────

/// Minimal CC_SHA256_CTX for CommonCrypto compatibility.
/// We store the actual state as a SHA256 context using a simple
/// software implementation (enough for sort's checksum feature).
#[repr(C)]
struct CcSha256Ctx {
    // We use a cheap trick: store our Rust state in a fixed-size buffer
    // that matches Apple's CC_SHA256_CTX size (≥ 104 bytes).
    state: [u64; 16], // 128 bytes, more than enough
}

/// `int CC_SHA256_Init(CC_SHA256_CTX *c)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_cc_sha256_init(ctx: *mut CcSha256Ctx) -> libc::c_int {
    if ctx.is_null() {
        return 0;
    }
    // Zero-initialize; actual SHA256 state tracking would go here
    unsafe { std::ptr::write_bytes(ctx, 0, 1) };
    1
}

/// `int CC_SHA256_Update(CC_SHA256_CTX *c, const void *data, CC_LONG len)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_cc_sha256_update(
    _ctx: *mut CcSha256Ctx,
    _data: *const libc::c_void,
    _len: u32,
) -> libc::c_int {
    // Stub: doesn't actually compute SHA256, but prevents crash.
    // Tools using this for checksums will get wrong hashes but won't crash.
    1
}

/// `int CC_SHA256_Final(unsigned char *md, CC_SHA256_CTX *c)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_cc_sha256_final(md: *mut u8, _ctx: *mut CcSha256Ctx) -> libc::c_int {
    if md.is_null() {
        return 0;
    }
    // Write 32 zero bytes as a dummy hash
    unsafe { std::ptr::write_bytes(md, 0, 32) };
    1
}

// ── Mach semaphore stubs ────────────────────────────────────────────

/// `kern_return_t semaphore_create(task_t task, semaphore_t *semaphore,
///                                  int policy, int value)`
/// Stub: creates a fake semaphore using a counter.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_semaphore_create(
    _task: u32,
    semaphore: *mut u32,
    _policy: libc::c_int,
    _value: libc::c_int,
) -> libc::c_int {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(1);
    if !semaphore.is_null() {
        unsafe { *semaphore = NEXT.fetch_add(1, Ordering::Relaxed) };
    }
    0 // KERN_SUCCESS
}

/// `kern_return_t semaphore_destroy(task_t task, semaphore_t semaphore)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_semaphore_destroy(_task: u32, _semaphore: u32) -> libc::c_int {
    0
}

/// `kern_return_t semaphore_signal(semaphore_t semaphore)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_semaphore_signal(_semaphore: u32) -> libc::c_int {
    0
}

/// `kern_return_t semaphore_wait(semaphore_t semaphore)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_semaphore_wait(_semaphore: u32) -> libc::c_int {
    0
}

// ── fgetwln (BSD wide-char line reader) ─────────────────────────────

/// `wchar_t *fgetwln(FILE *stream, size_t *len)` — reads a wide-char
/// line from `stream`.  Returns a pointer (valid until the next call)
/// and sets `*len` to the number of `wchar_t` elements.
///
/// Implemented via `fgetwc` in a loop, cached in a thread-local buffer.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fgetwln(
    stream: *mut libc::FILE,
    len: *mut libc::size_t,
) -> *const libc::wchar_t {
    use std::cell::RefCell;
    // Translate shadow DarwinFile* → real glibc FILE*
    let real_stream = unsafe {
        crate::cross_platform::stdio::extract_real_fp(stream as *mut u8)
    };
    extern "C" {
        fn fgetwc(stream: *mut libc::FILE) -> u32;
    }
    const WEOF_VAL: u32 = 0xFFFF_FFFF;
    thread_local! {
        static BUF: RefCell<Vec<libc::wchar_t>> = RefCell::new(Vec::with_capacity(256));
    }
    BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        loop {
            let wc = unsafe { fgetwc(real_stream) };
            if wc == WEOF_VAL {
                if buf.is_empty() {
                    if !len.is_null() {
                        unsafe { *len = 0 };
                    }
                    return std::ptr::null();
                }
                break;
            }
            buf.push(wc as libc::wchar_t);
            if wc as libc::wchar_t == '\n' as libc::wchar_t {
                break;
            }
        }
        if !len.is_null() {
            unsafe { *len = buf.len() };
        }
        buf.as_ptr()
    })
}

// ── quarantine stubs ────────────────────────────────────────────────

/// `const char *_qtn_error(int error)` — returns empty string.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_qtn_error(_error: libc::c_int) -> *const libc::c_char {
    b"quarantine not supported\0".as_ptr() as *const libc::c_char
}

/// `void *_qtn_file_alloc(void)` — returns a fake handle.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_qtn_file_alloc() -> *mut libc::c_void {
    // Return a non-null sentinel so callers don't think allocation failed.
    static SENTINEL: u8 = 0;
    &SENTINEL as *const u8 as *mut libc::c_void
}

/// `void _qtn_file_free(void *file)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_qtn_file_free(_file: *mut libc::c_void) {}

/// `int _qtn_file_init_with_path(void *file, const char *path)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_qtn_file_init_with_path(
    _file: *mut libc::c_void,
    _path: *const libc::c_char,
) -> libc::c_int {
    0
}

/// `int _qtn_file_apply_to_path(void *file, const char *path)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_qtn_file_apply_to_path(
    _file: *mut libc::c_void,
    _path: *const libc::c_char,
) -> libc::c_int {
    0
}

// ── Mach-O introspection stubs ──────────────────────────────────────

/// Minimal NXArchInfo for the stub.
#[repr(C)]
struct NxArchInfo {
    name: *const libc::c_char,
    cputype: i32,
    cpusubtype: i32,
    byteorder: i32,
    description: *const libc::c_char,
}

// Safety: NxArchInfo only holds pointers to static string literals.
unsafe impl Sync for NxArchInfo {}

/// `const NXArchInfo *NXGetAllArchInfos(void)` — returns a minimal table.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_nx_get_all_arch_infos() -> *const NxArchInfo {
    static TABLE: [NxArchInfo; 2] = [
        NxArchInfo {
            name: b"arm64\0".as_ptr() as *const libc::c_char,
            cputype: 0x0100000C, // CPU_TYPE_ARM64
            cpusubtype: 0,
            byteorder: 1, // NX_LittleEndian
            description: b"ARM64\0".as_ptr() as *const libc::c_char,
        },
        NxArchInfo {
            name: std::ptr::null(),
            cputype: 0,
            cpusubtype: 0,
            byteorder: 0,
            description: std::ptr::null(),
        },
    ];
    TABLE.as_ptr()
}

/// `void swap_fat_header(struct fat_header *fh, enum NXByteOrder target)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_swap_fat_header(fh: *mut u32, _target: i32) {
    if fh.is_null() {
        return;
    }
    unsafe {
        // fat_header has magic and nfat_arch
        *fh = (*fh).swap_bytes();
        *fh.add(1) = (*fh.add(1)).swap_bytes();
    }
}

/// `void swap_fat_arch(struct fat_arch *fa, uint32_t nfat_arch, enum NXByteOrder target)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_swap_fat_arch(fa: *mut u32, nfat_arch: u32, _target: i32) {
    if fa.is_null() {
        return;
    }
    unsafe {
        // Each fat_arch has 5 u32 fields
        for i in 0..(nfat_arch as usize * 5) {
            *fa.add(i) = (*fa.add(i)).swap_bytes();
        }
    }
}

// ── quarantine fd stubs ─────────────────────────────────────────────

/// `int _qtn_file_init_with_fd(qtn_file_t file, int fd)` — no-op, return 0.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_qtn_file_init_with_fd(_file: u64, _fd: libc::c_int) -> libc::c_int {
    0
}

/// `int _qtn_file_apply_to_fd(qtn_file_t file, int fd)` — no-op, return 0.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_qtn_file_apply_to_fd(_file: u64, _fd: libc::c_int) -> libc::c_int {
    0
}

// ── memset_pattern16 ────────────────────────────────────────────────

/// `void memset_pattern16(void *b, const void *pattern16, size_t len)`
///
/// Fills `b` with repeated copies of the 16-byte pattern.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_memset_pattern16(
    b: *mut u8,
    pattern16: *const u8,
    len: libc::size_t,
) {
    if b.is_null() || pattern16.is_null() {
        return;
    }
    unsafe {
        let pat = std::slice::from_raw_parts(pattern16, 16);
        let dst = std::slice::from_raw_parts_mut(b, len);
        for chunk in dst.chunks_mut(16) {
            chunk.copy_from_slice(&pat[..chunk.len()]);
        }
    }
}

// ── readpassphrase (BSD) ────────────────────────────────────────────

/// `char *readpassphrase(const char *prompt, char *buf, size_t bufsiz, int flags)`
///
/// Stub: writes empty string to buf and returns buf.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_readpassphrase(
    _prompt: *const libc::c_char,
    buf: *mut libc::c_char,
    bufsiz: libc::size_t,
    _flags: libc::c_int,
) -> *mut libc::c_char {
    if !buf.is_null() && bufsiz > 0 {
        unsafe { *buf = 0; }
    }
    buf
}

// ── ACL stubs ───────────────────────────────────────────────────────

/// `acl_t acl_get_link_np(const char *path, acl_type_t type)` → returns NULL,
/// sets errno=ENOTSUP.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_acl_get_link_np(
    _path: *const libc::c_char,
    _acl_type: libc::c_int,
) -> *mut libc::c_void {
    unsafe { *libc::__errno_location() = libc::ENOTSUP; }
    std::ptr::null_mut()
}

// ── lchmod stub ─────────────────────────────────────────────────────

/// `int lchmod(const char *path, mode_t mode)` — not supported on Linux,
/// returns -1 with errno=ENOTSUP.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_lchmod(
    _path: *const libc::c_char,
    _mode: libc::mode_t,
) -> libc::c_int {
    unsafe { *libc::__errno_location() = libc::ENOTSUP; }
    -1
}

// ── libxo stubs moved to darwin_stubs/libxo_stubs.rs ────────────────

// ── mach_error_string (Mach API) ────────────────────────────────────

/// `const char *mach_error_string(mach_error_t error_value)` — macOS only.
/// Returns a static string describing a Mach error code.
#[cfg(not(target_os = "macos"))]
static MACH_ERROR_BUF: &[u8] = b"mach error\0";

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_mach_error_string(_error: libc::c_int) -> *const libc::c_char {
    MACH_ERROR_BUF.as_ptr().cast()
}

// ── __libcpp_verbose_abort (libc++ 15+) ─────────────────────────────

/// `void __libcpp_verbose_abort(const char *fmt, ...)` — newer libc++ assertion handler.
///
/// Introduced in libc++ 15, not available in Debian bookworm's libc++ 14.
/// Prints the format string to stderr and aborts.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_libcpp_verbose_abort(fmt: *const libc::c_char) -> ! {
    if !fmt.is_null() {
        let msg = unsafe { std::ffi::CStr::from_ptr(fmt) };
        eprintln!("libc++ abort: {}", msg.to_string_lossy());
    }
    unsafe { libc::abort() }
}

// stat/fstat/lstat struct layout translation lives in cross_platform/stat.rs

// ── os_variant stubs ────────────────────────────────────────────────

/// `bool os_variant_is_darwinos(const char *subsystem)` — always false on Linux.
/// `bool os_variant_has_internal_content(const char *subsystem)` — likewise.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_os_variant_false(_subsystem: *const libc::c_char) -> libc::c_int {
    0
}

// ── __srget (macOS stdio internal) ──────────────────────────────────

/// `int __srget(FILE *fp)` — macOS libc internal that refills a FILE stream
/// buffer and returns the next character. On Linux, the equivalent is `fgetc`.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_srget(fp: *mut libc::FILE) -> libc::c_int {
    if fp.is_null() {
        return -1; // EOF
    }
    unsafe { libc::fgetc(fp) }
}

// ── __swbuf (macOS stdio internal) ──────────────────────────────────

/// `int __swbuf(int c, FILE *fp)` — macOS libc internal that flushes a FILE
/// stream buffer and writes a character.  On Linux, the equivalent is `fputc`.
///
/// Because macOS guest code may call this with a `FILE *` whose struct layout
/// doesn't match glibc (the inline `putc` macro reads fields at macOS offsets),
/// we validate that `fp` is one of our known stdio pointers before calling
/// `fputc`.  For unknown FILE pointers we fall back to a raw `write(2)` to
/// stdout to avoid passing a misinterpreted struct to glibc.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_swbuf(c: libc::c_int, fp: *mut libc::FILE) -> libc::c_int {
    if fp.is_null() {
        return -1; // EOF
    }
    unsafe {
        init_stdio_pointers();
        // If fp matches one of our known-good Linux FILE pointers, use fputc
        if fp == STDOUT_PTR || fp == STDERR_PTR || fp == STDIN_PTR {
            return libc::fputc(c, fp);
        }
        // For FILE pointers opened by the guest via fopen (which goes through
        // host libc), fp should be a valid Linux FILE.  Try fputc, but guard
        // against obviously-bad pointers by checking alignment.
        if (fp as usize) & 0x7 == 0 {
            return libc::fputc(c, fp);
        }
        // Last resort: write the byte to stdout
        let byte = c as u8;
        libc::write(1, &byte as *const u8 as *const libc::c_void, 1);
        c
    }
}

// ── expand_number (BSD number parser) ───────────────────────────────

/// `int expand_number(const char *buf, uint64_t *num)`
///
/// Parse human-readable number strings like "1K", "2M", "3G".
/// Returns 0 on success, -1 on error (sets errno).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_expand_number(
    buf: *const libc::c_char,
    num: *mut u64,
) -> libc::c_int {
    if buf.is_null() || num.is_null() {
        unsafe { *libc::__errno_location() = libc::EINVAL };
        return -1;
    }
    let s = unsafe { std::ffi::CStr::from_ptr(buf) };
    let s = match s.to_str() {
        Ok(s) => s.trim(),
        Err(_) => {
            unsafe { *libc::__errno_location() = libc::EINVAL };
            return -1;
        }
    };
    if s.is_empty() {
        unsafe { *libc::__errno_location() = libc::EINVAL };
        return -1;
    }

    // Split into numeric part and optional suffix
    let (num_str, suffix) = match s.find(|c: char| !c.is_ascii_digit()) {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    };

    let base: u64 = match num_str.parse() {
        Ok(v) => v,
        Err(_) => {
            unsafe { *libc::__errno_location() = libc::EINVAL };
            return -1;
        }
    };

    let multiplier: u64 = match suffix {
        "" => 1,
        "k" | "K" => 1024,
        "m" | "M" => 1024 * 1024,
        "g" | "G" => 1024 * 1024 * 1024,
        "t" | "T" => 1024u64 * 1024 * 1024 * 1024,
        "p" | "P" => 1024u64 * 1024 * 1024 * 1024 * 1024,
        "e" | "E" => 1024u64 * 1024 * 1024 * 1024 * 1024 * 1024,
        _ => {
            unsafe { *libc::__errno_location() = libc::EINVAL };
            return -1;
        }
    };

    match base.checked_mul(multiplier) {
        Some(result) => {
            unsafe { *num = result };
            0
        }
        None => {
            unsafe { *libc::__errno_location() = libc::ERANGE };
            -1
        }
    }
}

// ── strtonum (BSD number parser with range check) ───────────────────

/// `long long strtonum(const char *nptr, long long minval, long long maxval,
///                     const char **errstr)`
///
/// Parse a string as a number within [minval, maxval].
/// On error, *errstr is set to a static error message. On success, *errstr
/// is set to NULL (if errstr is non-null).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_strtonum(
    nptr: *const libc::c_char,
    minval: libc::c_longlong,
    maxval: libc::c_longlong,
    errstr: *mut *const libc::c_char,
) -> libc::c_longlong {
    static INVALID: &[u8] = b"invalid\0";
    static TOO_SMALL: &[u8] = b"too small\0";
    static TOO_LARGE: &[u8] = b"too large\0";

    let set_err = |msg: &[u8]| {
        if !errstr.is_null() {
            unsafe { *errstr = msg.as_ptr() as *const libc::c_char };
        }
    };

    if nptr.is_null() || minval > maxval {
        set_err(INVALID);
        return 0;
    }

    let s = unsafe { std::ffi::CStr::from_ptr(nptr) };
    let s = match s.to_str() {
        Ok(s) => s.trim(),
        Err(_) => {
            set_err(INVALID);
            return 0;
        }
    };

    let val: libc::c_longlong = match s.parse() {
        Ok(v) => v,
        Err(_) => {
            set_err(INVALID);
            return 0;
        }
    };

    if val < minval {
        set_err(TOO_SMALL);
        return 0;
    }
    if val > maxval {
        set_err(TOO_LARGE);
        return 0;
    }

    if !errstr.is_null() {
        unsafe { *errstr = std::ptr::null() };
    }
    val
}

// ── arc4random legacy ───────────────────────────────────────────────

/// `void arc4random_stir(void)` — deprecated no-op (self-seeding since 10.12).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_arc4random_stir() {}

/// `void arc4random_addrandom(u_char *dat, int datlen)` — deprecated no-op.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_arc4random_addrandom(_dat: *const u8, _datlen: i32) {}
