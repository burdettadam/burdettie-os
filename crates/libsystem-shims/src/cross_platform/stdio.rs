//! Cross-platform `FILE` struct translation shims.
//!
//! macOS `struct __sFILE` and glibc `FILE` have completely different layouts.
//! macOS binaries use inline `putc`/`getc` macros that read fields at macOS
//! offsets (`_p` at 0, `_r` at 8, `_w` at 12). When these macros operate on
//! a glibc `FILE*`, they read garbage → SIGSEGV or data corruption.
//!
//! ## Strategy: always-empty-buffer shadow
//!
//! We allocate **shadow** `DarwinFile` structs with correct macOS layout.
//! The critical fields `_r` and `_w` are always kept at **0**, so that the
//! inline macros always fall through to `__srget`/`__swbuf`. Those two
//! functions extract the real glibc `FILE*` from the shadow's `_cookie`
//! field (offset 48) and call `fgetc`/`fputc`.
//!
//! All stdio functions that accept `FILE*` (`fread`, `fwrite`, `fclose`,
//! `fflush`, `feof`, `ferror`, `fileno`, `clearerr`, `fseek`, `ftell`,
//! `fseeko`, `ftello`, `rewind`, `fgets`, `fputs`, `fgetc`, `fputc`,
//! `getc`, `putc`, `ungetc`, `flockfile`, `funlockfile`, `setvbuf`,
//! `setbuf`, `setlinebuf`, `getline`, `getdelim`) are shimmed to extract
//! the real glibc `FILE*` and forward. Functions that return `FILE*`
//! (`fopen`, `fdopen`, `freopen`, `tmpfile`, `open_memstream`) wrap the
//! result in a new shadow.
//!
//! ## macOS `struct __sFILE` layout (arm64, 64-bit)
//!
//! | Offset | Field      | Type/Size          | Notes                   |
//! |--------|------------|--------------------|-------------------------|
//! |   0    | _p         | *mut u8 (8)        | current buffer position |
//! |   8    | _r         | i32 (4)            | read space left         |
//! |  12    | _w         | i32 (4)            | write space left        |
//! |  16    | _flags     | i16 (2)            | stream flags            |
//! |  18    | _file      | i16 (2)            | fd, or -1               |
//! |  20    | (pad)      | (4)                |                         |
//! |  24    | _bf._base  | *mut u8 (8)        | buffer base             |
//! |  32    | _bf._size  | i32 (4)            | buffer size             |
//! |  36    | (pad)      | (4)                |                         |
//! |  40    | _lbfsize   | i32 (4)            | 0 or -_bf._size         |
//! |  44    | (pad)      | (4)                |                         |
//! |  48    | _cookie    | *mut void (8)      | **glibc FILE* stored here** |
//! |  56    | _close     | fn ptr (8)         |                         |
//! |  64    | _read      | fn ptr (8)         |                         |
//! |  72    | _seek      | fn ptr (8)         |                         |
//! |  80    | _write     | fn ptr (8)         |                         |
//! |  88    | _ub        | struct __sbuf (16)  | ungetc buffer          |
//! | 104    | _extra     | *mut void (8)      |                         |
//! | 112    | _ur        | i32 (4)            | saved _r during ungetc  |
//! | 116    | _ubuf[3]   | [u8; 3]            |                         |
//! | 119    | _nbuf[1]   | [u8; 1]            |                         |
//! | 120    | _lb        | struct __sbuf (16)  | line buffer            |
//! | 136    | _blksize   | i32 (4)            |                         |
//! | 140    | (pad)      | (4)                |                         |
//! | 144    | _offset    | i64 (8)            | fpos_t                  |
//! | 152    | total size |                    |                         |

use crate::table::ShimTable;

// Functions not exposed by the Rust `libc` crate for the cross-compile target.
#[cfg(not(target_os = "macos"))]
extern "C" {
    fn flockfile(fp: *mut libc::FILE);
    fn funlockfile(fp: *mut libc::FILE);
    fn ftrylockfile(fp: *mut libc::FILE) -> libc::c_int;
    fn setlinebuf(fp: *mut libc::FILE);
    fn getdelim(
        lineptr: *mut *mut libc::c_char,
        n: *mut libc::size_t,
        delim: libc::c_int,
        fp: *mut libc::FILE,
    ) -> libc::ssize_t;
    fn vfprintf(
        fp: *mut libc::FILE,
        fmt: *const libc::c_char,
        ap: *mut libc::c_void,
    ) -> libc::c_int;
    fn fprintf(fp: *mut libc::FILE, fmt: *const libc::c_char, ...) -> libc::c_int;
    fn vfscanf(fp: *mut libc::FILE, fmt: *const libc::c_char, ap: *mut libc::c_void)
        -> libc::c_int;

    // glibc's actual standard stream pointers — must use these instead of
    // fdopen() to avoid creating duplicate FILE objects with separate buffers.
    static mut stdout: *mut libc::FILE;
    static mut stderr: *mut libc::FILE;
    static mut stdin: *mut libc::FILE;
}

pub struct StdioShims;

impl StdioShims {
    pub fn install(table: &mut ShimTable) {
        #[cfg(not(target_os = "macos"))]
        {
            use crate::register_shims;
            register_shims!(
                table,
                // FILE-returning functions — wrap glibc FILE* in DarwinFile shadow
                (
                    "_fopen",
                    shim_fopen as *const () as u64,
                    "fopen → shadow FILE"
                ),
                (
                    "_fopen$DARWIN_EXTSN",
                    shim_fopen as *const () as u64,
                    "fopen$DARWIN_EXTSN → shadow FILE"
                ),
                (
                    "_fdopen",
                    shim_fdopen as *const () as u64,
                    "fdopen → shadow FILE"
                ),
                (
                    "_freopen",
                    shim_freopen as *const () as u64,
                    "freopen → shadow FILE"
                ),
                (
                    "_tmpfile",
                    shim_tmpfile as *const () as u64,
                    "tmpfile → shadow FILE"
                ),
                (
                    "_open_memstream",
                    shim_open_memstream as *const () as u64,
                    "open_memstream → shadow FILE"
                ),
                // FILE-consuming functions — extract glibc FILE* from shadow
                (
                    "_fclose",
                    shim_fclose as *const () as u64,
                    "fclose shadow→glibc"
                ),
                (
                    "_fread",
                    shim_fread as *const () as u64,
                    "fread shadow→glibc"
                ),
                (
                    "_fwrite",
                    shim_fwrite as *const () as u64,
                    "fwrite shadow→glibc"
                ),
                (
                    "_fflush",
                    shim_fflush as *const () as u64,
                    "fflush shadow→glibc"
                ),
                ("_feof", shim_feof as *const () as u64, "feof shadow→glibc"),
                (
                    "_ferror",
                    shim_ferror as *const () as u64,
                    "ferror shadow→glibc"
                ),
                (
                    "_fileno",
                    shim_fileno as *const () as u64,
                    "fileno shadow→glibc"
                ),
                (
                    "_clearerr",
                    shim_clearerr as *const () as u64,
                    "clearerr shadow→glibc"
                ),
                (
                    "_fseek",
                    shim_fseek as *const () as u64,
                    "fseek shadow→glibc"
                ),
                (
                    "_fseeko",
                    shim_fseeko as *const () as u64,
                    "fseeko shadow→glibc"
                ),
                (
                    "_ftell",
                    shim_ftell as *const () as u64,
                    "ftell shadow→glibc"
                ),
                (
                    "_ftello",
                    shim_ftello as *const () as u64,
                    "ftello shadow→glibc"
                ),
                (
                    "_rewind",
                    shim_rewind as *const () as u64,
                    "rewind shadow→glibc"
                ),
                (
                    "_fgetc",
                    shim_fgetc as *const () as u64,
                    "fgetc shadow→glibc"
                ),
                (
                    "_fputc",
                    shim_fputc as *const () as u64,
                    "fputc shadow→glibc"
                ),
                ("_getc", shim_fgetc as *const () as u64, "getc shadow→glibc"),
                ("_putc", shim_fputc as *const () as u64, "putc shadow→glibc"),
                (
                    "_ungetc",
                    shim_ungetc as *const () as u64,
                    "ungetc shadow→glibc"
                ),
                (
                    "_fgets",
                    shim_fgets as *const () as u64,
                    "fgets shadow→glibc"
                ),
                (
                    "_fputs",
                    shim_fputs as *const () as u64,
                    "fputs shadow→glibc"
                ),
                (
                    "_flockfile",
                    shim_flockfile as *const () as u64,
                    "flockfile shadow→glibc"
                ),
                (
                    "_funlockfile",
                    shim_funlockfile as *const () as u64,
                    "funlockfile shadow→glibc"
                ),
                (
                    "_ftrylockfile",
                    shim_ftrylockfile as *const () as u64,
                    "ftrylockfile shadow→glibc"
                ),
                (
                    "_setvbuf",
                    shim_setvbuf as *const () as u64,
                    "setvbuf shadow→glibc"
                ),
                (
                    "_setbuf",
                    shim_setbuf as *const () as u64,
                    "setbuf shadow→glibc"
                ),
                (
                    "_setlinebuf",
                    shim_setlinebuf as *const () as u64,
                    "setlinebuf shadow→glibc"
                ),
                (
                    "_getline",
                    shim_getline as *const () as u64,
                    "getline shadow→glibc"
                ),
                (
                    "_getdelim",
                    shim_getdelim as *const () as u64,
                    "getdelim shadow→glibc"
                ),
                (
                    "_putchar",
                    shim_putchar as *const () as u64,
                    "putchar → stdout"
                ),
                (
                    "_getchar",
                    shim_getchar as *const () as u64,
                    "getchar → stdin"
                ),
                // macOS internal __srget/__swbuf
                (
                    "___srget",
                    shim_srget as *const () as u64,
                    "__srget shadow→fgetc"
                ),
                (
                    "___swbuf",
                    shim_swbuf as *const () as u64,
                    "__swbuf shadow→fputc"
                ),
                // getc_unlocked/putc_unlocked — same as getc/putc
                (
                    "_getc_unlocked",
                    shim_fgetc as *const () as u64,
                    "getc_unlocked shadow→glibc"
                ),
                (
                    "_putc_unlocked",
                    shim_fputc as *const () as u64,
                    "putc_unlocked shadow→glibc"
                ),
                // OpenSSL/LibreSSL BIO functions that receive FILE* from guest code.
                // Native LibreSSL stores the FILE* internally and later calls glibc's
                // fwrite/fread on it — so we must translate shadow → real before handoff.
                (
                    "_BIO_new_fp",
                    shim_bio_new_fp as *const () as u64,
                    "BIO_new_fp shadow→real FILE"
                ),
                (
                    "_BIO_ctrl",
                    shim_bio_ctrl as *const () as u64,
                    "BIO_ctrl intercept FILE_PTR"
                ),
                (
                    "_BIO_dump_fp",
                    shim_bio_dump_fp as *const () as u64,
                    "BIO_dump_fp shadow→real FILE"
                ),
                (
                    "_BIO_dump_indent_fp",
                    shim_bio_dump_indent_fp as *const () as u64,
                    "BIO_dump_indent_fp shadow→real FILE"
                ),
            );
            #[cfg(target_arch = "x86_64")]
            register_shims!(
                table,
                (
                    "_fprintf",
                    shim_fprintf_x86_64 as *const () as u64,
                    "fprintf shadow→glibc"
                ),
            );
            #[cfg(not(target_arch = "aarch64"))]
            register_shims!(
                table,
                (
                    "_vfprintf",
                    shim_vfprintf_non_aarch64 as *const () as u64,
                    "vfprintf shadow→glibc"
                ),
                (
                    "___vfprintf_chk",
                    shim_vfprintf_chk_non_aarch64 as *const () as u64,
                    "vfprintf_chk shadow→glibc"
                ),
            );
        }
        #[cfg(target_os = "macos")]
        let _ = table;
    }
}

// ── DarwinFile shadow struct ────────────────────────────────────────

/// Size of the macOS `struct __sFILE` on arm64.
#[cfg(not(target_os = "macos"))]
const DARWIN_FILE_SIZE: usize = 152;

/// Magic value stored in _flags to identify our shadow FILE structs.
/// Uses an unlikely flag combination that won't appear in real macOS FILEs.
#[cfg(not(target_os = "macos"))]
const SHADOW_MAGIC: i16 = 0x5348; // 'SH' for shadow

/// Offsets into macOS `struct __sFILE` on arm64.
#[cfg(not(target_os = "macos"))]
mod offsets {
    pub const P: usize = 0; // _p: *mut u8
    pub const R: usize = 8; // _r: i32
    pub const W: usize = 12; // _w: i32
    pub const FLAGS: usize = 16; // _flags: i16
    pub const FILE: usize = 18; // _file: i16 (fd)
    pub const COOKIE: usize = 48; // _cookie: *mut void (stores glibc FILE*)
}

/// Registry of active shadow FILE pointers.
/// Maps DarwinFile* address → glibc FILE* for quick validation.
#[cfg(not(target_os = "macos"))]
static FILE_REGISTRY: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<u64>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

/// Allocate a new DarwinFile shadow wrapping a real glibc `FILE*`.
///
/// The shadow has `_r=0` and `_w=0` so inline macros always fall through
/// to `__srget`/`__swbuf`. The real glibc `FILE*` is stored in `_cookie`.
#[cfg(not(target_os = "macos"))]
unsafe fn alloc_shadow(real_fp: *mut libc::FILE) -> *mut u8 {
    let shadow = libc::calloc(1, DARWIN_FILE_SIZE) as *mut u8;
    if shadow.is_null() {
        return std::ptr::null_mut();
    }
    init_shadow(shadow, real_fp);
    if let Ok(mut reg) = FILE_REGISTRY.lock() {
        reg.insert(shadow as u64);
    }
    shadow
}

/// Initialize a shadow FILE struct with fields pointing to the real glibc FILE*.
#[cfg(not(target_os = "macos"))]
unsafe fn init_shadow(shadow: *mut u8, real_fp: *mut libc::FILE) {
    // _p = null (no buffer position)
    (shadow.add(offsets::P) as *mut u64).write(0);
    // _r = 0 (forces __srget on every getc)
    (shadow.add(offsets::R) as *mut i32).write(0);
    // _w = 0 (forces __swbuf on every putc)
    (shadow.add(offsets::W) as *mut i32).write(0);
    // _flags = magic marker
    (shadow.add(offsets::FLAGS) as *mut i16).write(SHADOW_MAGIC);
    // _file = fd from the real FILE
    let fd = libc::fileno(real_fp);
    (shadow.add(offsets::FILE) as *mut i16).write(fd as i16);
    // _cookie = real glibc FILE*
    (shadow.add(offsets::COOKIE) as *mut u64).write(real_fp as u64);
}

/// Extract the real glibc `FILE*` from a shadow `DarwinFile*`.
/// If the pointer is not a registered shadow, returns it as-is
/// (it's already a glibc FILE* from e.g. a direct fopen on the safe-list
/// before our shim was installed).
#[cfg(not(target_os = "macos"))]
pub unsafe fn extract_real_fp(darwin_fp: *mut u8) -> *mut libc::FILE {
    if darwin_fp.is_null() {
        return std::ptr::null_mut();
    }
    if is_shadow(darwin_fp) {
        let real = (darwin_fp.add(offsets::COOKIE) as *const u64).read() as *mut libc::FILE;
        if !real.is_null() {
            return real;
        }
    }
    // Not a shadow — assume it's a raw glibc FILE* (backward compat)
    darwin_fp as *mut libc::FILE
}

/// Check if a pointer is one of our registered shadow FILE structs.
#[cfg(not(target_os = "macos"))]
fn is_shadow(ptr: *const u8) -> bool {
    if let Ok(reg) = FILE_REGISTRY.lock() {
        reg.contains(&(ptr as u64))
    } else {
        false
    }
}

/// Free a shadow FILE struct and remove from registry.
#[cfg(not(target_os = "macos"))]
unsafe fn free_shadow(shadow: *mut u8) {
    if let Ok(mut reg) = FILE_REGISTRY.lock() {
        reg.remove(&(shadow as u64));
    }
    libc::free(shadow as *mut libc::c_void);
}

// ── Stdio shadow pointers for stdin/stdout/stderr ───────────────────

/// Pre-allocated shadow FILE structs for the three standard streams.
/// These are initialized once and their addresses are handed to the
/// guest via `___stdinp` / `___stdoutp` / `___stderrp`.
#[cfg(not(target_os = "macos"))]
static STDIO_SHADOWS_INIT: std::sync::Once = std::sync::Once::new();

#[cfg(not(target_os = "macos"))]
static mut STDIN_SHADOW: *mut u8 = std::ptr::null_mut();
#[cfg(not(target_os = "macos"))]
static mut STDOUT_SHADOW: *mut u8 = std::ptr::null_mut();
#[cfg(not(target_os = "macos"))]
static mut STDERR_SHADOW: *mut u8 = std::ptr::null_mut();

/// Pointers-to-shadow-pointers for the `___std{in,out,err}p` data symbols.
/// macOS exposes these as `FILE**`: the binary loads 8 bytes to get `FILE*`.
#[cfg(not(target_os = "macos"))]
static mut STDIN_SHADOW_PTR: *mut u8 = std::ptr::null_mut();
#[cfg(not(target_os = "macos"))]
static mut STDOUT_SHADOW_PTR: *mut u8 = std::ptr::null_mut();
#[cfg(not(target_os = "macos"))]
static mut STDERR_SHADOW_PTR: *mut u8 = std::ptr::null_mut();

#[cfg(not(target_os = "macos"))]
unsafe fn init_stdio_shadows() {
    STDIO_SHADOWS_INIT.call_once(|| {
        // Use glibc's REAL stdout/stderr/stdin — NOT fdopen().
        // fdopen(fd, mode) creates a NEW FILE object with its own buffer,
        // which conflicts with glibc's internal stdout used by printf/puts.
        // Two FILE objects sharing the same fd → interleaved garbled output.

        // stdin
        STDIN_SHADOW = alloc_shadow(stdin);
        STDIN_SHADOW_PTR = STDIN_SHADOW;

        // stdout
        STDOUT_SHADOW = alloc_shadow(stdout);
        STDOUT_SHADOW_PTR = STDOUT_SHADOW;

        // stderr
        STDERR_SHADOW = alloc_shadow(stderr);
        STDERR_SHADOW_PTR = STDERR_SHADOW;
    });
}

/// Return the address of the `FILE**` cell for a standard stream.
/// Called from `DarwinCompatShims` to register `___std{in,out,err}p`.
#[cfg(not(target_os = "macos"))]
pub fn stdio_shadow_ptr_addr(stream: u8) -> *const () {
    unsafe {
        init_stdio_shadows();
        match stream {
            0 => &STDIN_SHADOW_PTR as *const *mut u8 as *const (),
            1 => &STDOUT_SHADOW_PTR as *const *mut u8 as *const (),
            2 => &STDERR_SHADOW_PTR as *const *mut u8 as *const (),
            _ => std::ptr::null(),
        }
    }
}

// ── FILE-returning shims ────────────────────────────────────────────

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fopen(path: *const libc::c_char, mode: *const libc::c_char) -> *mut u8 {
    let real = unsafe { libc::fopen(path, mode) };
    if real.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { alloc_shadow(real) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fdopen(fd: libc::c_int, mode: *const libc::c_char) -> *mut u8 {
    let real = unsafe { libc::fdopen(fd, mode) };
    if real.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { alloc_shadow(real) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_freopen(
    path: *const libc::c_char,
    mode: *const libc::c_char,
    darwin_fp: *mut u8,
) -> *mut u8 {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    let new_real = unsafe { libc::freopen(path, mode, real_fp) };
    if new_real.is_null() {
        // freopen failed — the original stream is closed
        if is_shadow(darwin_fp) {
            unsafe {
                free_shadow(darwin_fp);
            }
        }
        return std::ptr::null_mut();
    }
    // If it was a shadow, update it in place
    if is_shadow(darwin_fp) {
        unsafe {
            init_shadow(darwin_fp, new_real);
        }
        darwin_fp
    } else {
        unsafe { alloc_shadow(new_real) }
    }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_tmpfile() -> *mut u8 {
    let real = unsafe { libc::tmpfile() };
    if real.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { alloc_shadow(real) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_open_memstream(
    bufp: *mut *mut libc::c_char,
    sizep: *mut libc::size_t,
) -> *mut u8 {
    let real = unsafe { libc::open_memstream(bufp, sizep) };
    if real.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { alloc_shadow(real) }
}

// ── FILE-consuming shims ────────────────────────────────────────────

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fclose(darwin_fp: *mut u8) -> libc::c_int {
    if darwin_fp.is_null() {
        return libc::EOF;
    }
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    let result = unsafe { libc::fclose(real_fp) };
    if is_shadow(darwin_fp) {
        unsafe {
            free_shadow(darwin_fp);
        }
    }
    result
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fread(
    buf: *mut libc::c_void,
    size: libc::size_t,
    count: libc::size_t,
    darwin_fp: *mut u8,
) -> libc::size_t {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fread(buf, size, count, real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fwrite(
    buf: *const libc::c_void,
    size: libc::size_t,
    count: libc::size_t,
    darwin_fp: *mut u8,
) -> libc::size_t {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fwrite(buf, size, count, real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fflush(darwin_fp: *mut u8) -> libc::c_int {
    if darwin_fp.is_null() {
        // fflush(NULL) flushes all streams
        return unsafe { libc::fflush(std::ptr::null_mut()) };
    }
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fflush(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_feof(darwin_fp: *mut u8) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::feof(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_ferror(darwin_fp: *mut u8) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::ferror(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fileno(darwin_fp: *mut u8) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fileno(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_clearerr(darwin_fp: *mut u8) {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::clearerr(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fseek(
    darwin_fp: *mut u8,
    offset: libc::c_long,
    whence: libc::c_int,
) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fseek(real_fp, offset, whence) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fseeko(
    darwin_fp: *mut u8,
    offset: libc::off_t,
    whence: libc::c_int,
) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fseeko(real_fp, offset, whence) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_ftell(darwin_fp: *mut u8) -> libc::c_long {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::ftell(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_ftello(darwin_fp: *mut u8) -> libc::off_t {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::ftello(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_rewind(darwin_fp: *mut u8) {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::rewind(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fgetc(darwin_fp: *mut u8) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fgetc(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fputc(c: libc::c_int, darwin_fp: *mut u8) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fputc(c, real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_ungetc(c: libc::c_int, darwin_fp: *mut u8) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::ungetc(c, real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fgets(
    buf: *mut libc::c_char,
    size: libc::c_int,
    darwin_fp: *mut u8,
) -> *mut libc::c_char {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fgets(buf, size, real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_fputs(s: *const libc::c_char, darwin_fp: *mut u8) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fputs(s, real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_flockfile(darwin_fp: *mut u8) {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { flockfile(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_funlockfile(darwin_fp: *mut u8) {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { funlockfile(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_ftrylockfile(darwin_fp: *mut u8) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { ftrylockfile(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_setvbuf(
    darwin_fp: *mut u8,
    buf: *mut libc::c_char,
    mode: libc::c_int,
    size: libc::size_t,
) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::setvbuf(real_fp, buf, mode, size) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_setbuf(darwin_fp: *mut u8, buf: *mut libc::c_char) {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::setbuf(real_fp, buf) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_setlinebuf(darwin_fp: *mut u8) {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { setlinebuf(real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_getline(
    lineptr: *mut *mut libc::c_char,
    n: *mut libc::size_t,
    darwin_fp: *mut u8,
) -> libc::ssize_t {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::getline(lineptr, n, real_fp) }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_getdelim(
    lineptr: *mut *mut libc::c_char,
    n: *mut libc::size_t,
    delim: libc::c_int,
    darwin_fp: *mut u8,
) -> libc::ssize_t {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { getdelim(lineptr, n, delim, real_fp) }
}

// ── putchar / getchar ───────────────────────────────────────────────

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_putchar(c: libc::c_int) -> libc::c_int {
    unsafe {
        init_stdio_shadows();
        let real_fp = extract_real_fp(STDOUT_SHADOW);
        libc::fputc(c, real_fp)
    }
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_getchar() -> libc::c_int {
    unsafe {
        init_stdio_shadows();
        let real_fp = extract_real_fp(STDIN_SHADOW);
        libc::fgetc(real_fp)
    }
}

// ── macOS internal __srget / __swbuf ────────────────────────────────

/// `int __srget(FILE *fp)` — macOS libc internal that refills a FILE stream
/// buffer and returns the next character. We extract the real glibc FILE*
/// and call fgetc.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_srget(darwin_fp: *mut u8) -> libc::c_int {
    if darwin_fp.is_null() {
        return libc::EOF;
    }
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fgetc(real_fp) }
}

/// `int __swbuf(int c, FILE *fp)` — macOS libc internal that flushes a FILE
/// stream buffer and writes a character. We extract the real glibc FILE*
/// and call fputc.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_swbuf(c: libc::c_int, darwin_fp: *mut u8) -> libc::c_int {
    if darwin_fp.is_null() {
        return libc::EOF;
    }
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { libc::fputc(c, real_fp) }
}

// ── Wrappers for variadic shims (FILE* translation) ─────────────────
//
// The naked assembly variadic shims pass x0 (FILE*) directly to the
// v-function. When x0 is a shadow DarwinFile*, we need to extract the
// real glibc FILE* first. These wrappers are called by the assembly
// shims instead of the raw libc v-functions.

/// `vfprintf` wrapper: translates shadow FILE* → glibc FILE*, then calls
/// the real `vfprintf`.
#[cfg(not(target_os = "macos"))]
#[no_mangle]
pub extern "C" fn vfprintf_shadow(
    darwin_fp: *mut u8,
    fmt: *const libc::c_char,
    ap: *mut libc::c_void, // va_list
) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { vfprintf(real_fp, fmt, ap as _) }
}

/// `vfscanf` wrapper: translates shadow FILE* → glibc FILE*, then calls
/// the real `vfscanf`.
#[cfg(not(target_os = "macos"))]
#[no_mangle]
pub extern "C" fn vfscanf_shadow(
    darwin_fp: *mut u8,
    fmt: *const libc::c_char,
    ap: *mut libc::c_void, // va_list
) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    unsafe { vfscanf(real_fp, fmt, ap as _) }
}

/// `__vfprintf_chk` wrapper: translates shadow FILE* → glibc FILE*, then
/// calls the real `__vfprintf_chk`.
#[cfg(not(target_os = "macos"))]
#[no_mangle]
pub extern "C" fn vfprintf_chk_shadow(
    darwin_fp: *mut u8,
    flag: libc::c_int,
    fmt: *const libc::c_char,
    ap: *mut libc::c_void, // va_list
) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    // __vfprintf_chk is just vfprintf with a flag; fall through to vfprintf
    let _ = flag;
    unsafe { vfprintf(real_fp, fmt, ap as _) }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
extern "C" fn translate_shadow_fp_x86_64(darwin_fp: *mut u8) -> *mut libc::FILE {
    unsafe { extract_real_fp(darwin_fp) }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[unsafe(naked)]
extern "C" fn shim_fprintf_x86_64() -> libc::c_int {
    core::arch::naked_asm!(
        "sub rsp, 184",
        "mov [rsp + 0], rax",
        "mov [rsp + 8], rsi",
        "mov [rsp + 16], rdx",
        "mov [rsp + 24], rcx",
        "mov [rsp + 32], r8",
        "mov [rsp + 40], r9",
        "movdqu [rsp + 48], xmm0",
        "movdqu [rsp + 64], xmm1",
        "movdqu [rsp + 80], xmm2",
        "movdqu [rsp + 96], xmm3",
        "movdqu [rsp + 112], xmm4",
        "movdqu [rsp + 128], xmm5",
        "movdqu [rsp + 144], xmm6",
        "movdqu [rsp + 160], xmm7",
        "call {translate}",
        "mov r11, rax",
        "mov rax, [rsp + 0]",
        "mov rsi, [rsp + 8]",
        "mov rdx, [rsp + 16]",
        "mov rcx, [rsp + 24]",
        "mov r8, [rsp + 32]",
        "mov r9, [rsp + 40]",
        "movdqu xmm0, [rsp + 48]",
        "movdqu xmm1, [rsp + 64]",
        "movdqu xmm2, [rsp + 80]",
        "movdqu xmm3, [rsp + 96]",
        "movdqu xmm4, [rsp + 112]",
        "movdqu xmm5, [rsp + 128]",
        "movdqu xmm6, [rsp + 144]",
        "movdqu xmm7, [rsp + 160]",
        "mov rdi, r11",
        "add rsp, 184",
        "jmp {fprintf_fn}",
        translate = sym translate_shadow_fp_x86_64,
        fprintf_fn = sym fprintf,
    )
}

#[cfg(all(not(target_os = "macos"), not(target_arch = "aarch64")))]
extern "C" fn shim_vfprintf_non_aarch64(
    darwin_fp: *mut u8,
    fmt: *const libc::c_char,
    ap: *mut libc::c_void,
) -> libc::c_int {
    vfprintf_shadow(darwin_fp, fmt, ap)
}

#[cfg(all(not(target_os = "macos"), not(target_arch = "aarch64")))]
extern "C" fn shim_vfprintf_chk_non_aarch64(
    darwin_fp: *mut u8,
    flag: libc::c_int,
    fmt: *const libc::c_char,
    ap: *mut libc::c_void,
) -> libc::c_int {
    vfprintf_chk_shadow(darwin_fp, flag, fmt, ap)
}

// ── BIO_new_fp / BIO_dump_fp shims ──────────────────────────────────
//
// OpenSSL/LibreSSL's BIO layer stores a FILE* and later calls glibc's
// fwrite/fread on it. When the macOS binary passes our shadow DarwinFile*
// to BIO_new_fp, native LibreSSL code can't use it — we must extract the
// real glibc FILE* before handing off.

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_bio_new_fp(darwin_fp: *mut u8, close_flag: libc::c_int) -> *mut libc::c_void {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    // Resolve BIO_new_fp at runtime — it's in the preloaded LibreSSL .so
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"BIO_new_fp\0".as_ptr().cast()) };
    if sym.is_null() {
        return std::ptr::null_mut();
    }
    let func: extern "C" fn(*mut libc::FILE, libc::c_int) -> *mut libc::c_void =
        unsafe { std::mem::transmute(sym) };
    func(real_fp, close_flag)
}

/// Intercept `BIO_ctrl(bio, cmd, larg, parg)`.
/// When cmd == BIO_C_SET_FILE_PTR (106), the 4th arg `parg` is a FILE*
/// from guest code (shadow DarwinFile*) — translate to real glibc FILE*
/// before LibreSSL stores it in the BIO.
/// When cmd == BIO_C_GET_FILE_PTR (107), the caller wants a FILE** written to —
/// we can't translate back, but the returned FILE* will be a real glibc one
/// (set by us), so it's safe.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_bio_ctrl(
    bio: *mut libc::c_void,
    cmd: libc::c_int,
    larg: libc::c_long,
    parg: *mut libc::c_void,
) -> libc::c_long {
    const BIO_C_SET_FILE_PTR: libc::c_int = 106;

    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"BIO_ctrl\0".as_ptr().cast()) };
    if sym.is_null() {
        return -1;
    }
    let real_bio_ctrl: extern "C" fn(
        *mut libc::c_void,
        libc::c_int,
        libc::c_long,
        *mut libc::c_void,
    ) -> libc::c_long = unsafe { std::mem::transmute(sym) };

    if cmd == BIO_C_SET_FILE_PTR && !parg.is_null() {
        // parg is a FILE* (cast to void*) — translate shadow → real
        let real_fp = unsafe { extract_real_fp(parg as *mut u8) };
        return real_bio_ctrl(bio, cmd, larg, real_fp as *mut libc::c_void);
    }

    real_bio_ctrl(bio, cmd, larg, parg)
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_bio_dump_fp(
    darwin_fp: *mut u8,
    s: *const libc::c_char,
    len: libc::c_int,
) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"BIO_dump_fp\0".as_ptr().cast()) };
    if sym.is_null() {
        return -1;
    }
    let func: extern "C" fn(*mut libc::FILE, *const libc::c_char, libc::c_int) -> libc::c_int =
        unsafe { std::mem::transmute(sym) };
    func(real_fp, s, len)
}

#[cfg(not(target_os = "macos"))]
extern "C" fn shim_bio_dump_indent_fp(
    darwin_fp: *mut u8,
    s: *const libc::c_char,
    len: libc::c_int,
    indent: libc::c_int,
) -> libc::c_int {
    let real_fp = unsafe { extract_real_fp(darwin_fp) };
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"BIO_dump_indent_fp\0".as_ptr().cast()) };
    if sym.is_null() {
        return -1;
    }
    let func: extern "C" fn(
        *mut libc::FILE,
        *const libc::c_char,
        libc::c_int,
        libc::c_int,
    ) -> libc::c_int = unsafe { std::mem::transmute(sym) };
    func(real_fp, s, len, indent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(not(target_os = "macos"), not(target_arch = "aarch64")))]
    #[test]
    fn install_registers_non_aarch64_vfprintf_wrappers() {
        let mut table = ShimTable::new();

        StdioShims::install(&mut table);

        #[cfg(target_arch = "x86_64")]
        assert_eq!(
            table.resolve("_fprintf"),
            Some(shim_fprintf_x86_64 as *const () as u64)
        );
        assert_eq!(
            table.resolve("_vfprintf"),
            Some(shim_vfprintf_non_aarch64 as *const () as u64)
        );
        assert_eq!(
            table.resolve("___vfprintf_chk"),
            Some(shim_vfprintf_chk_non_aarch64 as *const () as u64)
        );
    }
}
