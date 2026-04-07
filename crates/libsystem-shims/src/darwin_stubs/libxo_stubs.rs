//! Stubs for FreeBSD/Apple's libxo structured-output library.
//!
//! Delegates to the `libxo` crate which provides a text-mode
//! reimplementation.  Variadic entry points (`xo_emit`, `xo_emit_h`)
//! use naked assembly trampolines on Linux aarch64 to bridge the
//! Apple → Linux variadic ABI and build a `va_list`.

use crate::table::ShimTable;

/// Get the stderr FILE* in a cross-platform way.
/// On macOS, `libc::stderr` is a constant; on Linux, it's not available
/// directly in the `libc` crate.
#[cfg(not(target_os = "macos"))]
unsafe fn get_stderr() -> *mut libc::FILE {
    extern "C" {
        // glibc exports `stderr` as a global `FILE*`
        static stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

// ═══════════════════════════════════════════════════════════════════
//  Non-variadic C-ABI shim functions
// ═══════════════════════════════════════════════════════════════════

/// `int xo_parse_args(int argc, char **argv)` — strip `--libxo` options.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_parse_args(
    argc: libc::c_int,
    argv: *mut *mut libc::c_char,
) -> libc::c_int {
    unsafe { libxo::parse_args(argc, argv) }
}

/// `xo_handle_t *xo_create_to_file(FILE *fp, unsigned style, const char *info)`
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_create_to_file(
    fp: *mut libc::c_void,
    style: libc::c_uint,
    _info: *const libc::c_char,
) -> *mut libc::c_void {
    let fp = if fp.is_null() {
        // Default: stdout
        unsafe { libc::fdopen(1, b"w\0".as_ptr() as *const libc::c_char) }
    } else {
        fp as *mut libc::FILE
    };
    libxo::create_handle(fp, style) as *mut libc::c_void
}

/// Container/list/instance open/close — all no-ops returning 0.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_open_container(_name: *const libc::c_char) -> libc::c_int { 0 }
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_close_container(_name: *const libc::c_char) -> libc::c_int { 0 }
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_open_list(_name: *const libc::c_char) -> libc::c_int { 0 }
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_close_list(_name: *const libc::c_char) -> libc::c_int { 0 }
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_open_instance(_name: *const libc::c_char) -> libc::c_int { 0 }
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_close_instance(_name: *const libc::c_char) -> libc::c_int { 0 }

/// Container/list/instance with handle — also no-ops.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_open_container_h(_h: *mut libc::c_void, _n: *const libc::c_char) -> libc::c_int { 0 }
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_close_container_h(_h: *mut libc::c_void, _n: *const libc::c_char) -> libc::c_int { 0 }
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_open_list_h(_h: *mut libc::c_void, _n: *const libc::c_char) -> libc::c_int { 0 }
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_close_list_h(_h: *mut libc::c_void, _n: *const libc::c_char) -> libc::c_int { 0 }
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_open_instance_h(_h: *mut libc::c_void, _n: *const libc::c_char) -> libc::c_int { 0 }
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_close_instance_h(_h: *mut libc::c_void, _n: *const libc::c_char) -> libc::c_int { 0 }

/// `int xo_emit_hv(xo_handle_t *handle, const char *fmt, va_list ap)`
///
/// The non-variadic emit function.  The macOS binary passes an Apple-ABI
/// va_list (a plain `char *` on arm64) which we wrap into a Linux va_list
/// struct before forwarding to the real implementation.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_emit_hv(
    handle: *mut libc::c_void,
    fmt: *const libc::c_char,
    apple_ap: *mut libc::c_void,
) -> libc::c_int {
    // On aarch64-linux, build a Linux va_list struct from the Apple va_list.
    // Apple's va_list is just a pointer to the remaining stack args.
    #[cfg(target_arch = "aarch64")]
    {
        #[repr(C)]
        struct LinuxVaList {
            __stack: *mut libc::c_void,
            __gr_top: *mut libc::c_void,
            __vr_top: *mut libc::c_void,
            __gr_offs: i32,
            __vr_offs: i32,
        }

        let mut va = LinuxVaList {
            __stack: apple_ap,
            __gr_top: std::ptr::null_mut(),
            __vr_top: std::ptr::null_mut(),
            __gr_offs: 0,
            __vr_offs: 0,
        };

        let fp = unsafe { libxo::handle_to_fp(handle as *mut libxo::XoHandle) };
        unsafe { libxo::xo_emit_text(fp, fmt, &mut va as *mut LinuxVaList as *mut libc::c_void) }
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        // On x86_64 or other arches, va_list is also just a pointer.
        // Forward directly.
        let fp = unsafe { libxo::handle_to_fp(handle as *mut libxo::XoHandle) };
        unsafe { libxo::xo_emit_text(fp, fmt, apple_ap) }
    }
}

/// `void xo_finish(void)` / `void xo_finish_h(xo_handle_t *h)` — flush.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_finish() {
    unsafe { libc::fflush(std::ptr::null_mut()); }
}

/// `void xo_finish_h(xo_handle_t *h)` — flush handle.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_finish_h(handle: *mut libc::c_void) {
    if !handle.is_null() {
        let fp = unsafe { libxo::handle_to_fp(handle as *mut libxo::XoHandle) };
        unsafe { libc::fflush(fp); }
    } else {
        unsafe { libc::fflush(std::ptr::null_mut()); }
    }
}

/// `void xo_err(int eval, const char *fmt, ...)` — error + exit.
///
/// Note: The fmt here is an xo format string but for error messages
/// it is typically a simple printf format.  We use fprintf as a
/// best-effort since we cannot forward varargs from Rust.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_err(eval: libc::c_int, fmt: *const libc::c_char) {
    if !fmt.is_null() {
        unsafe {
            libc::fprintf(get_stderr(), fmt);
            libc::fprintf(get_stderr(), b"\n\0".as_ptr() as *const libc::c_char);
        }
    }
    unsafe { libc::exit(eval); }
}

/// `void xo_errx(int eval, const char *fmt, ...)` — error (no errno) + exit.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_errx(eval: libc::c_int, fmt: *const libc::c_char) {
    if !fmt.is_null() {
        unsafe {
            libc::fprintf(get_stderr(), fmt);
            libc::fprintf(get_stderr(), b"\n\0".as_ptr() as *const libc::c_char);
        }
    }
    unsafe { libc::exit(eval); }
}

/// `void xo_error(const char *fmt, ...)` — error to stderr.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_error(fmt: *const libc::c_char) {
    if !fmt.is_null() {
        unsafe { libc::fprintf(get_stderr(), fmt); }
    }
}

/// `void xo_warn(const char *fmt, ...)` — warning with errno.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_warn(fmt: *const libc::c_char) {
    if !fmt.is_null() {
        unsafe {
            libc::fprintf(get_stderr(), fmt);
            libc::fprintf(get_stderr(), b"\n\0".as_ptr() as *const libc::c_char);
        }
    }
}

/// `void xo_warnx(const char *fmt, ...)` — warning (no errno).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_warnx(fmt: *const libc::c_char) {
    if !fmt.is_null() {
        unsafe {
            libc::fprintf(get_stderr(), fmt);
            libc::fprintf(get_stderr(), b"\n\0".as_ptr() as *const libc::c_char);
        }
    }
}

/// `void xo_message(xo_handle_t *, int, const char *fmt, ...)` — no-op.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_message(
    _handle: *mut libc::c_void,
    _code: libc::c_int,
    _fmt: *const libc::c_char,
) {}

/// `void xo_set_style(xo_handle_t *, int)` — no-op (text mode only).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_set_style(_handle: *mut libc::c_void, _style: libc::c_int) {}

/// `void xo_set_flags(xo_handle_t *, int)` — no-op.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_set_flags(_handle: *mut libc::c_void, _flags: libc::c_int) {}

/// `int xo_set_options(xo_handle_t *, const char *)` — no-op, returns 0.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_set_options(
    _handle: *mut libc::c_void,
    _opts: *const libc::c_char,
) -> libc::c_int { 0 }

/// `void xo_no_setlocale(void)` — no-op.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_no_setlocale() {}

/// Generic no-op returning 0 for minor functions.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_noop() -> libc::c_int { 0 }

/// `void xo_flush(void)` / `void xo_flush_h(xo_handle_t *)` — flush.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_flush() {
    unsafe { libc::fflush(std::ptr::null_mut()); }
}

/// `int xo_attr(const char *, const char *, ...)` — no-op (text mode).
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_attr(
    _name: *const libc::c_char,
    _fmt: *const libc::c_char,
) -> libc::c_int { 0 }

/// `int xo_attr_h(xo_handle_t *, const char *, const char *, ...)` — no-op.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_attr_h(
    _handle: *mut libc::c_void,
    _name: *const libc::c_char,
    _fmt: *const libc::c_char,
) -> libc::c_int { 0 }

/// `void xo_destroy(xo_handle_t *)` — free handle.
#[cfg(not(target_os = "macos"))]
extern "C" fn shim_xo_destroy(handle: *mut libc::c_void) {
    unsafe { libxo::destroy_handle(handle as *mut libxo::XoHandle); }
}

// ═══════════════════════════════════════════════════════════════════
//  Variadic assembly shims (Linux aarch64 only)
//
//  xo_emit(fmt, ...)   → xo_emit_text(stdout, fmt, &va_list)
//  xo_emit_h(h, fmt, ...) → xo_emit_text_h(h, fmt, &va_list)
//
//  These naked functions capture Apple-ABI stack varargs into a
//  Linux va_list struct, then call the Rust implementation.
// ═══════════════════════════════════════════════════════════════════

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
mod asm_shims {
    /// The Rust-side trampoline called by the assembly shims.
    /// Dispatches to libxo's emit function.
    #[no_mangle]
    pub extern "C" fn xo_emit_text_trampoline(
        fp: *mut libc::FILE,
        fmt: *const libc::c_char,
        ap: *mut libc::c_void,
    ) -> libc::c_int {
        unsafe { libxo::xo_emit_text(fp, fmt, ap) }
    }

    /// Trampoline for handle-based emit.
    #[no_mangle]
    pub extern "C" fn xo_emit_text_h_trampoline(
        handle: *mut libc::c_void,
        fmt: *const libc::c_char,
        ap: *mut libc::c_void,
    ) -> libc::c_int {
        unsafe { libxo::xo_emit_text_h(handle as *mut libxo::XoHandle, fmt, ap) }
    }

    /// `int xo_emit(const char *fmt, ...)`
    ///
    /// Named args: x0 = fmt
    /// Variadic args start on Apple stack at original SP.
    /// Build va_list, get stdout fp, call xo_emit_text_trampoline(fp, fmt, &va_list).
    #[unsafe(naked)]
    pub extern "C" fn shim_xo_emit() -> libc::c_int {
        core::arch::naked_asm!(
            // Frame: 64 bytes = 16 (FP/LR) + 32 (va_list) + 16 (pad)
            "stp  x29, x30, [sp, #-64]!",
            "mov  x29, sp",
            // Build Linux va_list at [sp+16]
            "add  x8, x29, #64",          // x8 = original SP (Apple varargs)
            "str  x8, [sp, #16]",          // __stack = original SP
            "stp  xzr, xzr, [sp, #24]",   // __gr_top = 0, __vr_top = 0
            "str  xzr, [sp, #40]",         // __gr_offs = 0, __vr_offs = 0
            // Set up call: xo_emit_text_trampoline(NULL, fmt, &va_list)
            // x0 already has fmt; shuffle args:
            //   x0 = fp (NULL = default stdout)
            //   x1 = fmt
            //   x2 = &va_list
            "mov  x1, x0",                // x1 = fmt (was in x0)
            "mov  x0, xzr",               // x0 = NULL (default handle → stdout)
            "add  x2, sp, #16",           // x2 = &va_list
            "bl   {trampoline}",
            "ldp  x29, x30, [sp], #64",
            "ret",
            trampoline = sym xo_emit_text_trampoline,
        )
    }

    /// `int xo_emit_h(xo_handle_t *handle, const char *fmt, ...)`
    ///
    /// Named args: x0 = handle, x1 = fmt
    /// Build va_list, call xo_emit_text_h_trampoline(handle, fmt, &va_list).
    #[unsafe(naked)]
    pub extern "C" fn shim_xo_emit_h() -> libc::c_int {
        core::arch::naked_asm!(
            "stp  x29, x30, [sp, #-64]!",
            "mov  x29, sp",
            "add  x8, x29, #64",
            "str  x8, [sp, #16]",
            "stp  xzr, xzr, [sp, #24]",
            "str  xzr, [sp, #40]",
            // x0 = handle (stays), x1 = fmt (stays), x2 = &va_list
            "add  x2, sp, #16",
            "bl   {trampoline}",
            "ldp  x29, x30, [sp], #64",
            "ret",
            trampoline = sym xo_emit_text_h_trampoline,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Non-Linux / non-aarch64 fallback stubs for xo_emit / xo_emit_h
//
//  On macOS (host), these are never called at runtime (native libxo
//  is used).  But we need them to compile.
// ═══════════════════════════════════════════════════════════════════

#[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
mod asm_shims {
    /// Fallback xo_emit — cannot forward varargs, returns 0.
    #[cfg(not(target_os = "macos"))]
    pub extern "C" fn shim_xo_emit(
        _fmt: *const libc::c_char,
    ) -> libc::c_int { 0 }

    /// Fallback xo_emit_h — cannot forward varargs, returns 0.
    #[cfg(not(target_os = "macos"))]
    pub extern "C" fn shim_xo_emit_h(
        _handle: *mut libc::c_void,
        _fmt: *const libc::c_char,
    ) -> libc::c_int { 0 }
}

// ═══════════════════════════════════════════════════════════════════
//  Registration
// ═══════════════════════════════════════════════════════════════════

pub struct LibxoStubs;

impl LibxoStubs {
    pub fn install(table: &mut ShimTable) {
        #[cfg(not(target_os = "macos"))]
        {
            crate::register_shims!(table,
                // ── lifecycle ────────────────────────────────────
                ("_xo_parse_args", shim_xo_parse_args as *const () as u64, "xo_parse_args (libxo)"),
                ("_xo_create_to_file", shim_xo_create_to_file as *const () as u64, "xo_create_to_file (libxo)"),
                ("_xo_destroy", shim_xo_destroy as *const () as u64, "xo_destroy (libxo)"),
                ("_xo_finish", shim_xo_finish as *const () as u64, "xo_finish (libxo)"),
                ("_xo_finish_h", shim_xo_finish_h as *const () as u64, "xo_finish_h (libxo)"),
                ("_xo_flush", shim_xo_flush as *const () as u64, "xo_flush (libxo)"),
                ("_xo_flush_h", shim_xo_flush as *const () as u64, "xo_flush_h (libxo)"),
                // ── emit (the critical functions) ───────────────
                ("_xo_emit", asm_shims::shim_xo_emit as *const () as u64, "xo_emit (libxo text)"),
                ("_xo_emit_h", asm_shims::shim_xo_emit_h as *const () as u64, "xo_emit_h (libxo text)"),
                ("_xo_emit_hv", shim_xo_emit_hv as *const () as u64, "xo_emit_hv (libxo text)"),
                // ── container / list / instance ─────────────────
                ("_xo_open_container", shim_xo_open_container as *const () as u64, "xo_open_container (libxo)"),
                ("_xo_close_container", shim_xo_close_container as *const () as u64, "xo_close_container (libxo)"),
                ("_xo_open_list", shim_xo_open_list as *const () as u64, "xo_open_list (libxo)"),
                ("_xo_close_list", shim_xo_close_list as *const () as u64, "xo_close_list (libxo)"),
                ("_xo_open_instance", shim_xo_open_instance as *const () as u64, "xo_open_instance (libxo)"),
                ("_xo_close_instance", shim_xo_close_instance as *const () as u64, "xo_close_instance (libxo)"),
                ("_xo_open_container_h", shim_xo_open_container_h as *const () as u64, "xo_open_container_h (libxo)"),
                ("_xo_close_container_h", shim_xo_close_container_h as *const () as u64, "xo_close_container_h (libxo)"),
                ("_xo_open_list_h", shim_xo_open_list_h as *const () as u64, "xo_open_list_h (libxo)"),
                ("_xo_close_list_h", shim_xo_close_list_h as *const () as u64, "xo_close_list_h (libxo)"),
                ("_xo_open_instance_h", shim_xo_open_instance_h as *const () as u64, "xo_open_instance_h (libxo)"),
                ("_xo_close_instance_h", shim_xo_close_instance_h as *const () as u64, "xo_close_instance_h (libxo)"),
                // ── error / warning ─────────────────────────────
                ("_xo_err", shim_xo_err as *const () as u64, "xo_err (libxo)"),
                ("_xo_errx", shim_xo_errx as *const () as u64, "xo_errx (libxo)"),
                ("_xo_error", shim_xo_error as *const () as u64, "xo_error (libxo)"),
                ("_xo_warn", shim_xo_warn as *const () as u64, "xo_warn (libxo)"),
                ("_xo_warnx", shim_xo_warnx as *const () as u64, "xo_warnx (libxo)"),
                ("_xo_message", shim_xo_message as *const () as u64, "xo_message (libxo)"),
                // ── configuration ───────────────────────────────
                ("_xo_set_style", shim_xo_set_style as *const () as u64, "xo_set_style (libxo)"),
                ("_xo_set_flags", shim_xo_set_flags as *const () as u64, "xo_set_flags (libxo)"),
                ("_xo_set_options", shim_xo_set_options as *const () as u64, "xo_set_options (libxo)"),
                ("_xo_no_setlocale", shim_xo_no_setlocale as *const () as u64, "xo_no_setlocale (libxo)"),
                ("_xo_set_allocator", shim_xo_noop as *const () as u64, "xo_set_allocator (libxo)"),
                ("_xo_set_writer", shim_xo_noop as *const () as u64, "xo_set_writer (libxo)"),
                ("_xo_attr", shim_xo_attr as *const () as u64, "xo_attr (libxo)"),
                ("_xo_attr_h", shim_xo_attr_h as *const () as u64, "xo_attr_h (libxo)"),
            );

            log::info!("libxo_stubs: registered {} libxo text-mode shims", 35);
        }
    }
}
