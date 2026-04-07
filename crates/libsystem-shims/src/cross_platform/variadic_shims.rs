//! Variadic function ABI translation shims (Apple → Linux ARM64).
//!
//! On ARM64, Apple and Linux use different calling conventions for
//! variadic (ellipsis) arguments:
//!
//! - **Apple ABI**: variadic arguments are placed entirely on the **stack**.
//! - **Linux AAPCS64**: variadic arguments go in registers (`x1`‒`x7`,
//!   `q0`‒`q7`) first, then overflow to the stack.
//!
//! When a macOS Mach-O binary calls a Linux variadic function (resolved
//! via `dlsym`), Linux's `va_arg` reads registers that contain stale
//! values — not the actual arguments.  This causes crashes (e.g.
//! `strlen(0x2)` when `%s` reads garbage from `x2` instead of the
//! string pointer on the stack).
//!
//! ## Fix
//!
//! For each variadic C function, we install a **naked assembly shim** that:
//!
//! 1. Captures the original stack pointer (where Apple placed variadic args).
//! 2. Constructs a Linux `va_list` struct with `__stack` pointing there and
//!    `__gr_offs = __vr_offs = 0` (tells `va_arg` to skip registers).
//! 3. Tail-calls the `v`-variant (e.g. `vfprintf`) with this `va_list`.
//!
//! ### Linux aarch64 `va_list` layout (32 bytes):
//!
//! | Offset | Field       | Size | Value we set           |
//! |--------|-------------|------|------------------------|
//! | 0      | `__stack`   | 8    | original SP (Apple args) |
//! | 8      | `__gr_top`  | 8    | 0 (unused)             |
//! | 16     | `__vr_top`  | 8    | 0 (unused)             |
//! | 24     | `__gr_offs` | 4    | 0 (no GP regs left)    |
//! | 28     | `__vr_offs` | 4    | 0 (no FP regs left)    |

use crate::table::ShimTable;

/// Install variadic ABI translation shims.
///
/// These override the StubResolver safe-list entries for printf-family
/// and err-family functions, ensuring the Apple-ABI variadic args are
/// correctly forwarded to Linux's v-variant implementations.
pub struct VariadicShims;

impl VariadicShims {
    pub fn install(table: &mut ShimTable) {
        log::info!("VariadicShims::install called (pre-cfg)");
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            use crate::register_shims;
            register_shims!(table,
                // ── printf family ───────────────────────────────
                ("_fprintf",  shim_fprintf  as *const () as u64, "fprintf (Apple variadic ABI)"),
                ("_printf",   shim_printf   as *const () as u64, "printf (Apple variadic ABI)"),
                ("_snprintf", shim_snprintf as *const () as u64, "snprintf (Apple variadic ABI)"),
                ("_sprintf",  shim_sprintf  as *const () as u64, "sprintf (Apple variadic ABI)"),
                ("_dprintf",  shim_dprintf  as *const () as u64, "dprintf (Apple variadic ABI)"),
                ("_asprintf", shim_asprintf as *const () as u64, "asprintf (Apple variadic ABI)"),
                // ── scanf family ────────────────────────────────
                ("_sscanf",   shim_sscanf   as *const () as u64, "sscanf (Apple variadic ABI)"),
                ("_fscanf",   shim_fscanf   as *const () as u64, "fscanf (Apple variadic ABI)"),
                ("_scanf",    shim_scanf    as *const () as u64, "scanf (Apple variadic ABI)"),
                // ── err / warn family ───────────────────────────
                ("_err",      shim_err      as *const () as u64, "err (Apple variadic ABI)"),
                ("_errx",     shim_errx     as *const () as u64, "errx (Apple variadic ABI)"),
                ("_errc",     shim_errc     as *const () as u64, "errc (Apple variadic ABI)"),
                ("_warn",     shim_warn     as *const () as u64, "warn (Apple variadic ABI)"),
                ("_warnx",    shim_warnx    as *const () as u64, "warnx (Apple variadic ABI)"),
                // ── syslog ──────────────────────────────────────
                ("_syslog",   shim_syslog   as *const () as u64, "syslog (Apple variadic ABI)"),
                // ── v-variant family (Apple va_list → Linux va_list) ──
                ("_vfprintf",  shim_vfprintf  as *const () as u64, "vfprintf (Apple va_list→Linux)"),
                ("_vprintf",   shim_vprintf   as *const () as u64, "vprintf (Apple va_list→Linux)"),
                ("_vsnprintf", shim_vsnprintf as *const () as u64, "vsnprintf (Apple va_list→Linux)"),
                ("_vsprintf",  shim_vsprintf  as *const () as u64, "vsprintf (Apple va_list→Linux)"),
                ("_vdprintf",  shim_vdprintf  as *const () as u64, "vdprintf (Apple va_list→Linux)"),
                ("_vasprintf", shim_vasprintf as *const () as u64, "vasprintf (Apple va_list→Linux)"),
                ("_vsscanf",   shim_vsscanf   as *const () as u64, "vsscanf (Apple va_list→Linux)"),
                ("_vfscanf",   shim_vfscanf   as *const () as u64, "vfscanf (Apple va_list→Linux)"),
                ("_vscanf",    shim_vscanf    as *const () as u64, "vscanf (Apple va_list→Linux)"),
                ("_vsyslog",   shim_vsyslog   as *const () as u64, "vsyslog (Apple va_list→Linux)"),
                // ── fortified (_chk) variadic family ────────────
                ("___snprintf_chk", shim_snprintf_chk as *const () as u64, "snprintf_chk (Apple variadic ABI)"),
                ("___sprintf_chk",  shim_sprintf_chk  as *const () as u64, "sprintf_chk (Apple variadic ABI)"),
                ("___fprintf_chk",  shim_fprintf_chk  as *const () as u64, "fprintf_chk (Apple variadic ABI)"),
                ("___printf_chk",   shim_printf_chk   as *const () as u64, "printf_chk (Apple variadic ABI)"),
                // ── fortified v-variant family ──────────────────
                ("___vsnprintf_chk", shim_vsnprintf_chk as *const () as u64, "vsnprintf_chk (Apple va_list→Linux)"),
                ("___vsprintf_chk",  shim_vsprintf_chk  as *const () as u64, "vsprintf_chk (Apple va_list→Linux)"),
                ("___vfprintf_chk",  shim_vfprintf_chk  as *const () as u64, "vfprintf_chk (Apple va_list→Linux)"),
                ("___vprintf_chk",   shim_vprintf_chk   as *const () as u64, "vprintf_chk (Apple va_list→Linux)"),
            );

            log::info!("VariadicShims: registered 33 Apple→Linux ABI shims");
        }
    }
}

// ── Implementation (Linux aarch64 only) ────────────────────────────

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
mod inner {
    // Re-export the shim functions at module level.
    pub use shims::*;

    /// Extern declarations for the Linux v-variant functions.
    /// The `ap` parameter is `va_list` which on aarch64 decays to a
    /// pointer to the 32-byte struct.
    extern "C" {
        fn vfprintf(
            stream: *mut libc::FILE,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;

        // Shadow wrappers that translate DarwinFile* → glibc FILE* before
        // calling the real v-function. Defined in cross_platform/stdio.rs.
        fn vfprintf_shadow(
            darwin_fp: *mut u8,
            fmt: *const libc::c_char,
            ap: *mut libc::c_void,
        ) -> libc::c_int;
        fn vfscanf_shadow(
            darwin_fp: *mut u8,
            fmt: *const libc::c_char,
            ap: *mut libc::c_void,
        ) -> libc::c_int;
        fn vfprintf_chk_shadow(
            darwin_fp: *mut u8,
            flag: libc::c_int,
            fmt: *const libc::c_char,
            ap: *mut libc::c_void,
        ) -> libc::c_int;

        fn vprintf(format: *const libc::c_char, ap: *const u8) -> libc::c_int;
        fn vsnprintf(
            buf: *mut libc::c_char,
            size: usize,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;
        fn vsprintf(
            buf: *mut libc::c_char,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;
        fn vdprintf(
            fd: libc::c_int,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;
        fn vasprintf(
            ret: *mut *mut libc::c_char,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;
        fn vsscanf(
            s: *const libc::c_char,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;
        fn vfscanf(
            stream: *mut libc::FILE,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;
        fn vscanf(format: *const libc::c_char, ap: *const u8) -> libc::c_int;
        fn vsyslog(priority: libc::c_int, format: *const libc::c_char, ap: *const u8);

        // Fortified (_chk) v-variants — these take a buffer-size arg
        // but are otherwise identical to their non-chk counterparts.
        #[link_name = "__vfprintf_chk"]
        fn vfprintf_chk(
            stream: *mut libc::FILE,
            flag: libc::c_int,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;
        #[link_name = "__vprintf_chk"]
        fn vprintf_chk(
            flag: libc::c_int,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;
        #[link_name = "__vsnprintf_chk"]
        fn vsnprintf_chk(
            buf: *mut libc::c_char,
            maxlen: usize,
            flag: libc::c_int,
            slen: usize,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;
        #[link_name = "__vsprintf_chk"]
        fn vsprintf_chk(
            buf: *mut libc::c_char,
            flag: libc::c_int,
            slen: usize,
            format: *const libc::c_char,
            ap: *const u8,
        ) -> libc::c_int;
    }

    // ── Build-a-va_list assembly macro ──────────────────────────
    //
    // Every shim follows the same pattern:
    //   1. Push frame (16 bytes) + va_list (32 bytes) + padding = 64 bytes
    //   2. Compute original SP = FP + 64
    //   3. Fill va_list at [sp+16]
    //   4. Put &va_list into the correct argument register
    //   5. BL to the v-variant
    //   6. Pop frame, RET
    //
    // The `va_list_arg` parameter is the register for &va_list:
    //   - 1 named arg  → x1  (e.g. printf, scanf, warn, warnx)
    //   - 2 named args → x2  (e.g. fprintf, sprintf, err, errx, sscanf, ...)
    //   - 3 named args → x3  (e.g. snprintf, errc)

    mod shims {
        use super::*;

        // ── printf family ──────────────────────────────────

        /// `int fprintf(FILE *stream, const char *fmt, ...)`
        ///   → `vfprintf_shadow(stream, fmt, &va_list)`
        /// Named args: x0=stream, x1=fmt  ⇒ va_list in x2
        #[unsafe(naked)]
        pub extern "C" fn shim_fprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",          // x8 = original SP
                "str  x8, [sp, #16]",          // __stack
                "stp  xzr, xzr, [sp, #24]",   // __gr_top, __vr_top
                "str  xzr, [sp, #40]",         // __gr_offs=0, __vr_offs=0
                "add  x2, sp, #16",            // x2 = &va_list
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vfprintf_shadow,
            )
        }

        /// `int printf(const char *fmt, ...)`
        ///   → `vprintf(fmt, &va_list)`
        /// Named args: x0=fmt  ⇒ va_list in x1
        #[unsafe(naked)]
        pub extern "C" fn shim_printf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x1, sp, #16",            // x1 = &va_list
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vprintf,
            )
        }

        /// `int snprintf(char *buf, size_t size, const char *fmt, ...)`
        ///   → `vsnprintf(buf, size, fmt, &va_list)`
        /// Named args: x0=buf, x1=size, x2=fmt  ⇒ va_list in x3
        #[unsafe(naked)]
        pub extern "C" fn shim_snprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x3, sp, #16",            // x3 = &va_list
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsnprintf,
            )
        }

        /// `int sprintf(char *buf, const char *fmt, ...)`
        ///   → `vsprintf(buf, fmt, &va_list)`
        /// Named args: x0=buf, x1=fmt  ⇒ va_list in x2
        #[unsafe(naked)]
        pub extern "C" fn shim_sprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsprintf,
            )
        }

        /// `int dprintf(int fd, const char *fmt, ...)`
        ///   → `vdprintf(fd, fmt, &va_list)`
        /// Named args: x0=fd, x1=fmt  ⇒ va_list in x2
        #[unsafe(naked)]
        pub extern "C" fn shim_dprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vdprintf,
            )
        }

        /// `int asprintf(char **ret, const char *fmt, ...)`
        ///   → `vasprintf(ret, fmt, &va_list)`
        /// Named args: x0=ret, x1=fmt  ⇒ va_list in x2
        #[unsafe(naked)]
        pub extern "C" fn shim_asprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vasprintf,
            )
        }

        // ── scanf family ───────────────────────────────────

        /// `int sscanf(const char *s, const char *fmt, ...)`
        ///   → `vsscanf(s, fmt, &va_list)`
        /// Named args: x0=s, x1=fmt  ⇒ va_list in x2
        #[unsafe(naked)]
        pub extern "C" fn shim_sscanf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsscanf,
            )
        }

        /// `int fscanf(FILE *stream, const char *fmt, ...)`
        ///   → `vfscanf_shadow(stream, fmt, &va_list)`
        /// Named args: x0=stream, x1=fmt  ⇒ va_list in x2
        #[unsafe(naked)]
        pub extern "C" fn shim_fscanf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vfscanf_shadow,
            )
        }

        /// `int scanf(const char *fmt, ...)`
        ///   → `vscanf(fmt, &va_list)`
        /// Named args: x0=fmt  ⇒ va_list in x1
        #[unsafe(naked)]
        pub extern "C" fn shim_scanf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x1, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vscanf,
            )
        }

        // ── syslog ─────────────────────────────────────────

        /// `void syslog(int priority, const char *fmt, ...)`
        ///   → `vsyslog(priority, fmt, &va_list)`
        /// Named args: x0=priority, x1=fmt  ⇒ va_list in x2
        #[unsafe(naked)]
        pub extern "C" fn shim_syslog() {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsyslog,
            )
        }

        // ── v-variant shims (Apple va_list → Linux va_list) ──
        //
        // On Apple ARM64, va_list is `char *` (8-byte pointer to
        // the variadic args on the stack).  On Linux aarch64, va_list
        // is a 32-byte struct.  These shims take the Apple va_list
        // from the appropriate register, build a Linux va_list with
        // __stack = apple_va_list, and call the real glibc function.

        /// `int vfprintf(FILE *stream, const char *fmt, va_list ap)`
        /// Apple: x2 = char* (Apple va_list)
        /// Linux: x2 = __va_list* (pointer to 32-byte struct)
        #[unsafe(naked)]
        pub extern "C" fn shim_vfprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x2, [sp, #16]",          // __stack = Apple va_list
                "stp  xzr, xzr, [sp, #24]",   // __gr_top = 0, __vr_top = 0
                "str  xzr, [sp, #40]",         // __gr_offs = 0, __vr_offs = 0
                "add  x2, sp, #16",            // x2 = &linux_va_list
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vfprintf_shadow,
            )
        }

        /// `int vprintf(const char *fmt, va_list ap)`
        /// Apple: x1 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x1, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x1, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vprintf,
            )
        }

        /// `int vsnprintf(char *buf, size_t size, const char *fmt, va_list ap)`
        /// Apple: x3 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vsnprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x3, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x3, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsnprintf,
            )
        }

        /// `int vsprintf(char *buf, const char *fmt, va_list ap)`
        /// Apple: x2 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vsprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x2, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsprintf,
            )
        }

        /// `int vdprintf(int fd, const char *fmt, va_list ap)`
        /// Apple: x2 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vdprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x2, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vdprintf,
            )
        }

        /// `int vasprintf(char **ret, const char *fmt, va_list ap)`
        /// Apple: x2 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vasprintf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x2, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vasprintf,
            )
        }

        /// `int vsscanf(const char *s, const char *fmt, va_list ap)`
        /// Apple: x2 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vsscanf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x2, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsscanf,
            )
        }

        /// `int vfscanf(FILE *stream, const char *fmt, va_list ap)`
        /// Apple: x2 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vfscanf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x2, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vfscanf_shadow,
            )
        }

        /// `int vscanf(const char *fmt, va_list ap)`
        /// Apple: x1 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vscanf() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x1, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x1, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vscanf,
            )
        }

        /// `void vsyslog(int priority, const char *fmt, va_list ap)`
        /// Apple: x2 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vsyslog() {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x2, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsyslog,
            )
        }

        // ── fortified (_chk) variadic shims ────────────────
        //
        // Apple's libSystem uses `__snprintf_chk`, `__sprintf_chk`,
        // `__fprintf_chk`, `__printf_chk` as hardened printf wrappers.
        // These are variadic — Apple ABI puts varargs on stack.
        // We forward to the glibc `__v*_chk` variants with a va_list.

        /// `int __snprintf_chk(buf, maxlen, flag, slen, fmt, ...)`
        /// Named args: x0=buf, x1=maxlen, x2=flag, x3=slen, x4=fmt ⇒ va in x5
        /// → `__vsnprintf_chk(buf, maxlen, flag, slen, fmt, &va_list)`
        #[unsafe(naked)]
        pub extern "C" fn shim_snprintf_chk() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",          // x8 = original SP (Apple varargs)
                "str  x8, [sp, #16]",          // __stack
                "stp  xzr, xzr, [sp, #24]",   // __gr_top, __vr_top
                "str  xzr, [sp, #40]",         // __gr_offs=0, __vr_offs=0
                "add  x5, sp, #16",            // x5 = &va_list
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsnprintf_chk,
            )
        }

        /// `int __sprintf_chk(buf, flag, slen, fmt, ...)`
        /// Named args: x0=buf, x1=flag, x2=slen, x3=fmt ⇒ va in x4
        /// → `__vsprintf_chk(buf, flag, slen, fmt, &va_list)`
        #[unsafe(naked)]
        pub extern "C" fn shim_sprintf_chk() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x4, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsprintf_chk,
            )
        }

        /// `int __fprintf_chk(stream, flag, fmt, ...)`
        /// Named args: x0=stream, x1=flag, x2=fmt ⇒ va in x3
        /// → `vfprintf_chk_shadow(stream, flag, fmt, &va_list)`
        #[unsafe(naked)]
        pub extern "C" fn shim_fprintf_chk() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x3, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vfprintf_chk_shadow,
            )
        }

        /// `int __printf_chk(flag, fmt, ...)`
        /// Named args: x0=flag, x1=fmt ⇒ va in x2
        /// → `__vprintf_chk(flag, fmt, &va_list)`
        #[unsafe(naked)]
        pub extern "C" fn shim_printf_chk() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vprintf_chk,
            )
        }

        // ── fortified v-variant shims (Apple va_list → Linux va_list) ──

        /// `int __vsnprintf_chk(buf, maxlen, flag, slen, fmt, va_list ap)`
        /// Apple: x5 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vsnprintf_chk() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x5, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x5, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsnprintf_chk,
            )
        }

        /// `int __vsprintf_chk(buf, flag, slen, fmt, va_list ap)`
        /// Apple: x4 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vsprintf_chk() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x4, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x4, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vsprintf_chk,
            )
        }

        /// `int __vfprintf_chk(stream, flag, fmt, va_list ap)`
        /// Apple: x3 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vfprintf_chk() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x3, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x3, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vfprintf_chk_shadow,
            )
        }

        /// `int __vprintf_chk(flag, fmt, va_list ap)`
        /// Apple: x2 = Apple va_list
        #[unsafe(naked)]
        pub extern "C" fn shim_vprintf_chk() -> libc::c_int {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "str  x2, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {v_func}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                v_func = sym vprintf_chk,
            )
        }
        //
        // These can't just forward to Linux's err/errx/warn/warnx
        // because those internally use `program_invocation_short_name`
        // (the host binary name).  We reimplement them using our
        // `getprogname()` shim for the correct macOS tool name.

        /// `void err(int eval, const char *fmt, ...)`
        /// Named args: x0=eval, x1=fmt  ⇒ va_list in x2
        ///
        /// Constructs va_list, then calls `err_inner(eval, fmt, &va_list)`.
        #[unsafe(naked)]
        pub extern "C" fn shim_err() -> ! {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {inner}",
                // err_inner never returns, but for safety:
                "brk  #1",
                inner = sym err_inner,
            )
        }

        /// `void errx(int eval, const char *fmt, ...)`
        /// Named args: x0=eval, x1=fmt  ⇒ va_list in x2
        #[unsafe(naked)]
        pub extern "C" fn shim_errx() -> ! {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x2, sp, #16",
                "bl   {inner}",
                "brk  #1",
                inner = sym errx_inner,
            )
        }

        /// `void errc(int eval, int code, const char *fmt, ...)`
        /// Named args: x0=eval, x1=code, x2=fmt  ⇒ va_list in x3
        #[unsafe(naked)]
        pub extern "C" fn shim_errc() -> ! {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x3, sp, #16",
                "bl   {inner}",
                "brk  #1",
                inner = sym errc_inner,
            )
        }

        /// `void warn(const char *fmt, ...)`
        /// Named args: x0=fmt  ⇒ va_list in x1
        #[unsafe(naked)]
        pub extern "C" fn shim_warn() {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x1, sp, #16",
                "bl   {inner}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                inner = sym warn_inner,
            )
        }

        /// `void warnx(const char *fmt, ...)`
        /// Named args: x0=fmt  ⇒ va_list in x1
        #[unsafe(naked)]
        pub extern "C" fn shim_warnx() {
            core::arch::naked_asm!(
                "stp  x29, x30, [sp, #-64]!",
                "mov  x29, sp",
                "add  x8, x29, #64",
                "str  x8, [sp, #16]",
                "stp  xzr, xzr, [sp, #24]",
                "str  xzr, [sp, #40]",
                "add  x1, sp, #16",
                "bl   {inner}",
                "ldp  x29, x30, [sp], #64",
                "ret",
                inner = sym warnx_inner,
            )
        }

        // ── err/warn inner implementations ─────────────────

        /// Helper: get stderr FILE* for our error output.
        /// We use libc::fdopen-based approach since libc::stderr isn't
        /// directly accessible as a symbol on all platforms.
        fn host_stderr() -> *mut libc::FILE {
            use std::sync::OnceLock;
            static STDERR: OnceLock<usize> = OnceLock::new();
            let addr = *STDERR.get_or_init(|| unsafe {
                // dlsym for the Linux "stderr" global
                let ptr = libc::dlsym(libc::RTLD_DEFAULT, b"stderr\0".as_ptr().cast());
                if ptr.is_null() {
                    // Fallback: fdopen(2)
                    libc::fdopen(2, b"w\0".as_ptr().cast()) as usize
                } else {
                    // stderr is a FILE*, and dlsym returns &stderr, so deref
                    *(ptr as *const *mut libc::FILE) as usize
                }
            });
            addr as *mut libc::FILE
        }

        /// Helper: get the current process name as a C string.
        fn progname_cstr() -> *const libc::c_char {
            use std::sync::OnceLock;
            // Cache the CString so the pointer remains valid.
            static CACHE: OnceLock<std::ffi::CString> = OnceLock::new();
            CACHE
                .get_or_init(|| {
                    let name = crate::process::process_name().unwrap_or("?");
                    std::ffi::CString::new(name).unwrap_or_else(|_| {
                        std::ffi::CString::new("?").unwrap()
                    })
                })
                .as_ptr()
        }

        extern "C" {
            fn fprintf(
                stream: *mut libc::FILE,
                format: *const libc::c_char,
                ...
            ) -> libc::c_int;
        }

        /// `err(eval, fmt, ap)` — print "progname: <msg>: <errno>\n" and exit.
        extern "C" fn err_inner(eval: libc::c_int, fmt: *const libc::c_char, ap: *const u8) -> ! {
            unsafe {
                let se = host_stderr();
                let pn = progname_cstr();
                fprintf(se, b"%s: \0".as_ptr().cast(), pn);
                if !fmt.is_null() {
                    vfprintf(se, fmt, ap);
                    fprintf(se, b": \0".as_ptr().cast());
                }
                let errno_str = libc::strerror(*libc::__errno_location());
                fprintf(se, b"%s\n\0".as_ptr().cast(), errno_str);
            }
            std::process::exit(eval);
        }

        /// `errx(eval, fmt, ap)` — print "progname: <msg>\n" and exit.
        extern "C" fn errx_inner(
            eval: libc::c_int,
            fmt: *const libc::c_char,
            ap: *const u8,
        ) -> ! {
            unsafe {
                let se = host_stderr();
                let pn = progname_cstr();
                fprintf(se, b"%s: \0".as_ptr().cast(), pn);
                if !fmt.is_null() {
                    vfprintf(se, fmt, ap);
                }
                fprintf(se, b"\n\0".as_ptr().cast());
            }
            std::process::exit(eval);
        }

        /// `errc(eval, code, fmt, ap)` — print "progname: <msg>: strerror(code)\n" and exit.
        extern "C" fn errc_inner(
            eval: libc::c_int,
            code: libc::c_int,
            fmt: *const libc::c_char,
            ap: *const u8,
        ) -> ! {
            unsafe {
                let se = host_stderr();
                let pn = progname_cstr();
                fprintf(se, b"%s: \0".as_ptr().cast(), pn);
                if !fmt.is_null() {
                    vfprintf(se, fmt, ap);
                    fprintf(se, b": \0".as_ptr().cast());
                }
                let errno_str = libc::strerror(code);
                fprintf(se, b"%s\n\0".as_ptr().cast(), errno_str);
            }
            std::process::exit(eval);
        }

        /// `warn(fmt, ap)` — print "progname: <msg>: <errno>\n".
        extern "C" fn warn_inner(fmt: *const libc::c_char, ap: *const u8) {
            unsafe {
                let se = host_stderr();
                let pn = progname_cstr();
                fprintf(se, b"%s: \0".as_ptr().cast(), pn);
                if !fmt.is_null() {
                    vfprintf(se, fmt, ap);
                    fprintf(se, b": \0".as_ptr().cast());
                }
                let errno_str = libc::strerror(*libc::__errno_location());
                fprintf(se, b"%s\n\0".as_ptr().cast(), errno_str);
            }
        }

        /// `warnx(fmt, ap)` — print "progname: <msg>\n".
        extern "C" fn warnx_inner(fmt: *const libc::c_char, ap: *const u8) {
            unsafe {
                let se = host_stderr();
                let pn = progname_cstr();
                fprintf(se, b"%s: \0".as_ptr().cast(), pn);
                if !fmt.is_null() {
                    vfprintf(se, fmt, ap);
                }
                fprintf(se, b"\n\0".as_ptr().cast());
            }
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub use inner::*;
