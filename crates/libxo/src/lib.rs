//! libxo — text-mode reimplementation of FreeBSD's structured-output library.
//!
//! libxo is the FreeBSD/macOS library used by system tools (`wc`, `sort`,
//! `df`, `ls`, `ps`, etc.) to produce structured output in TEXT, XML,
//! JSON, or HTML.  This crate reimplements the **text-mode** path: it
//! parses libxo format strings — `{:field/%fmt}` — extracts the printf
//! specifiers, and rewrite them into a plain printf format string that
//! `vfprintf` can handle.
//!
//! # Architecture
//!
//! ```text
//! macOS binary calls xo_emit(fmt, ...)
//!         │
//!         ▼
//! [naked asm shim]  captures Apple-ABI varargs → Linux va_list
//!         │
//!         ▼
//! xo_emit_text(fp, xo_fmt, &va_list)
//!         │
//!    parse_xo_format(xo_fmt) → segments
//!         │
//!    rewrite to printf fmt    → "%7llu %7llu %s\n"
//!         │
//!    vfprintf(fp, printf_fmt, va_list)
//! ```

mod format;

pub use format::{parse_xo_format, Segment};

// ── Handle management ───────────────────────────────────────────────

/// Opaque libxo handle (C: `xo_handle_t *`).
///
/// For text-mode output the handle is just a thin wrapper around a
/// FILE*.  NULL means "use the default handle" (stdout).
#[repr(C)]
pub struct XoHandle {
    fp: *mut libc::FILE,
    style: u32,
    flags: u32,
}

/// Output styles (only TEXT is supported).
pub const XO_STYLE_TEXT: u32 = 0;
pub const XO_STYLE_XML: u32 = 1;
pub const XO_STYLE_JSON: u32 = 2;
pub const XO_STYLE_HTML: u32 = 3;

/// XO flags.
pub const XOF_WARN: u32 = 0x200;

impl XoHandle {
    /// Create a new handle writing to the given FILE*.
    pub fn new(fp: *mut libc::FILE, style: u32) -> Self {
        Self { fp, style, flags: 0 }
    }

    pub fn file_ptr(&self) -> *mut libc::FILE {
        self.fp
    }
}

/// Create a handle, returning a heap-allocated opaque pointer.
pub fn create_handle(fp: *mut libc::FILE, style: u32) -> *mut XoHandle {
    Box::into_raw(Box::new(XoHandle::new(fp, style)))
}

/// Destroy a handle previously created by [`create_handle`].
///
/// # Safety
/// `handle` must be a valid pointer returned by `create_handle`, or null.
pub unsafe fn destroy_handle(handle: *mut XoHandle) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

/// Extract the FILE* from a handle, falling back to stdout.
///
/// # Safety
/// If `handle` is non-null it must point to a valid `XoHandle`.
pub unsafe fn handle_to_fp(handle: *mut XoHandle) -> *mut libc::FILE {
    if handle.is_null() {
        default_fp()
    } else {
        let h = unsafe { &*handle };
        h.fp
    }
}

fn default_fp() -> *mut libc::FILE {
    // Use file descriptor 1 (stdout) via libc.
    // We open with fdopen so we get buffered I/O.
    use std::sync::OnceLock;
    static FP: OnceLock<usize> = OnceLock::new();
    let ptr = *FP.get_or_init(|| unsafe {
        libc::fdopen(1, b"w\0".as_ptr() as *const libc::c_char) as usize
    });
    ptr as *mut libc::FILE
}

// ── Core emit function ──────────────────────────────────────────────

extern "C" {
    fn vfprintf(
        stream: *mut libc::FILE,
        format: *const libc::c_char,
        ap: *mut libc::c_void,
    ) -> libc::c_int;
}

/// Rewrite an xo format string into an equivalent printf format string,
/// then call `vfprintf` once with the caller-supplied `va_list`.
///
/// # Safety
///
/// * `fp` must be a valid `FILE*`.
/// * `xo_fmt` must be a valid C string.
/// * `ap` must point to a valid platform-native `va_list` struct whose
///   remaining arguments match the printf specifiers embedded in the xo
///   format string.
pub unsafe fn xo_emit_text(
    fp: *mut libc::FILE,
    xo_fmt: *const libc::c_char,
    ap: *mut libc::c_void,
) -> libc::c_int {
    if fp.is_null() || xo_fmt.is_null() {
        return -1;
    }

    let fmt_cstr = unsafe { std::ffi::CStr::from_ptr(xo_fmt) };
    let fmt_str = match fmt_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let rewritten = format::rewrite_to_printf(fmt_str);

    let c_fmt = match std::ffi::CString::new(rewritten) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    unsafe { vfprintf(fp, c_fmt.as_ptr(), ap) }
}

/// Convenience: emit with a handle (or NULL for default).
///
/// # Safety
/// Same requirements as [`xo_emit_text`], plus `handle` must be null or
/// a valid `XoHandle` pointer.
pub unsafe fn xo_emit_text_h(
    handle: *mut XoHandle,
    xo_fmt: *const libc::c_char,
    ap: *mut libc::c_void,
) -> libc::c_int {
    let fp = unsafe { handle_to_fp(handle) };
    unsafe { xo_emit_text(fp, xo_fmt, ap) }
}

// ── Parse-args helper ───────────────────────────────────────────────

/// Strip `--libxo` arguments from argv, returning the new argc.
///
/// # Safety
/// `argv` must be a valid pointer to `argc` C strings.
pub unsafe fn parse_args(
    argc: libc::c_int,
    argv: *mut *mut libc::c_char,
) -> libc::c_int {
    if argv.is_null() || argc <= 0 {
        return argc;
    }

    let mut read = 0usize;
    let mut write = 0usize;
    let n = argc as usize;

    while read < n {
        let arg = unsafe { *argv.add(read) };
        if arg.is_null() {
            read += 1;
            continue;
        }
        let s = unsafe { std::ffi::CStr::from_ptr(arg) };
        if let Ok(s) = s.to_str() {
            if s == "--libxo" || s.starts_with("--libxo=") || s.starts_with("--libxo:") {
                read += 1;
                // If it's bare `--libxo` (no =), the next arg is the option value
                if s == "--libxo" && read < n {
                    read += 1; // skip the value too
                }
                continue;
            }
        }
        unsafe { *argv.add(write) = arg; }
        write += 1;
        read += 1;
    }

    write as libc::c_int
}
