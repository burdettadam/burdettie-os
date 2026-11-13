//! libedit / readline compatibility stubs.
//!
//! macOS ships `libedit.dylib` which provides both the native editline API
//! (`el_init`, `el_gets`, etc.) and a GNU readline compatibility layer
//! (`readline`, `add_history`, etc.). On Linux these symbols may not exist
//! or come from a different library.
//!
//! This module provides minimal stubs so that Mach-O binaries which import
//! libedit symbols can resolve them without crashing. The stubs return
//! empty/null results — they don't provide real line-editing functionality.

use crate::table::ShimTable;
use std::sync::Mutex;

pub struct LibeditStubs;

impl LibeditStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── EditLine (native libedit API) ───────────────────────
        register_shims!(
            table,
            (
                "_el_init",
                shim_el_init as *const () as u64,
                "el_init (stub → opaque handle)"
            ),
            (
                "_el_end",
                shim_el_end as *const () as u64,
                "el_end (stub → void)"
            ),
            (
                "_el_gets",
                shim_el_gets as *const () as u64,
                "el_gets (stub → reads from stdin)"
            ),
            (
                "_el_set",
                shim_el_set as *const () as u64,
                "el_set (stub → 0)"
            ),
            (
                "_el_get",
                shim_el_get as *const () as u64,
                "el_get (stub → -1)"
            ),
            (
                "_el_source",
                shim_el_source as *const () as u64,
                "el_source (stub → 0)"
            ),
            (
                "_el_line",
                shim_el_line as *const () as u64,
                "el_line (stub → empty LineInfo)"
            ),
            (
                "_el_parse",
                shim_el_parse as *const () as u64,
                "el_parse (stub → -1)"
            ),
            (
                "_el_resize",
                shim_el_resize as *const () as u64,
                "el_resize (stub → void)"
            ),
            (
                "_el_push",
                shim_el_push as *const () as u64,
                "el_push (stub → void)"
            ),
            (
                "_el_insertstr",
                shim_el_insertstr as *const () as u64,
                "el_insertstr (stub → -1)"
            ),
            (
                "_el_deletestr",
                shim_el_deletestr as *const () as u64,
                "el_deletestr (stub → void)"
            )
        );

        // ── History API ─────────────────────────────────────────
        register_shims!(
            table,
            (
                "_history_init",
                shim_history_init as *const () as u64,
                "history_init (stub → opaque handle)"
            ),
            (
                "_history_end",
                shim_history_end as *const () as u64,
                "history_end (stub → void)"
            ),
            (
                "_history",
                shim_history as *const () as u64,
                "history (stub → 0)"
            )
        );

        // ── GNU Readline compatibility layer ────────────────────
        register_shims!(
            table,
            (
                "_readline",
                shim_readline as *const () as u64,
                "readline (stub → reads from stdin)"
            ),
            (
                "_add_history",
                shim_add_history as *const () as u64,
                "add_history (stub → void)"
            ),
            (
                "_rl_completion_matches",
                shim_rl_completion_matches as *const () as u64,
                "rl_completion_matches (stub → NULL)"
            )
        );

        // ── Readline global variables (data symbols) ────────────
        register_shims!(
            table,
            (
                "_rl_attempted_completion_function",
                &RL_ATTEMPTED_COMPLETION_FN as *const u64 as u64,
                "rl_attempted_completion_function data symbol"
            ),
            (
                "_rl_line_buffer",
                &RL_LINE_BUFFER as *const u64 as u64,
                "rl_line_buffer data symbol"
            ),
            (
                "_rl_point",
                &RL_POINT as *const i32 as u64,
                "rl_point data symbol"
            ),
            (
                "_rl_readline_name",
                &RL_READLINE_NAME as *const u64 as u64,
                "rl_readline_name data symbol"
            ),
            (
                "_rl_end",
                &RL_END as *const i32 as u64,
                "rl_end data symbol"
            ),
            (
                "_rl_prompt",
                &RL_PROMPT as *const u64 as u64,
                "rl_prompt data symbol"
            )
        );

        log::debug!("LibeditStubs: registered libedit/readline stubs");
    }
}

// ── Global data symbols ─────────────────────────────────────────

/// rl_attempted_completion_function — function pointer, initially NULL.
static RL_ATTEMPTED_COMPLETION_FN: u64 = 0;
/// rl_line_buffer — pointer to current line, initially NULL.
static RL_LINE_BUFFER: u64 = 0;
/// rl_point — cursor position in rl_line_buffer.
static RL_POINT: i32 = 0;
/// rl_end — length of current line.
static RL_END: i32 = 0;
/// rl_readline_name — application name, initially NULL.
static RL_READLINE_NAME: u64 = 0;
/// rl_prompt — current prompt string, initially NULL.
static RL_PROMPT: u64 = 0;

// ── EditLine handle sentinel ────────────────────────────────────

static EL_SENTINEL: u64 = 0x454C_4954_0001; // "ELIT" + editline
static HIST_SENTINEL: u64 = 0x4849_5354_0001; // "HIST" + history

/// Shared buffer for el_gets / readline results.
/// Only one thread reads from stdin at a time, guarded by mutex.
static READLINE_BUF: Mutex<Option<Vec<u8>>> = Mutex::new(None);

/// Static LineInfo struct returned by el_line.
/// Layout: [buffer_ptr:8][cursor_ptr:8][lastchar_ptr:8]
static LINE_INFO: [u64; 3] = [0, 0, 0];

// ── EditLine function stubs ─────────────────────────────────────

/// el_init(prog, fin, fout, ferr) → EditLine*
pub extern "C" fn shim_el_init(_prog: *const u8, _fin: u64, _fout: u64, _ferr: u64) -> u64 {
    &EL_SENTINEL as *const u64 as u64
}

/// el_end(el) → void
pub extern "C" fn shim_el_end(_el: u64) {}

/// el_gets(el, &count) → const char*
///
/// Minimal implementation: reads one line from stdin via libc fgets.
pub extern "C" fn shim_el_gets(_el: u64, count_out: *mut i32) -> *const u8 {
    let mut guard = READLINE_BUF.lock().unwrap_or_else(|e| e.into_inner());
    let buf = guard.get_or_insert_with(|| vec![0u8; 1024]);
    buf.fill(0);
    let ptr = buf.as_mut_ptr();
    let result = unsafe { libc::fgets(ptr as *mut libc::c_char, 1024, libc_stdin()) };
    if result.is_null() {
        if !count_out.is_null() {
            unsafe {
                *count_out = 0;
            }
        }
        return std::ptr::null();
    }
    let len = unsafe { libc::strlen(ptr as *const libc::c_char) } as i32;
    if !count_out.is_null() {
        unsafe {
            *count_out = len;
        }
    }
    ptr
}

/// el_set(el, op, ...) → int  (variadic — we accept up to 4 extra args)
pub extern "C" fn shim_el_set(_el: u64, _op: i32, _a: u64, _b: u64, _c: u64, _d: u64) -> i32 {
    0 // success
}

/// el_get(el, op, ...) → int
pub extern "C" fn shim_el_get(_el: u64, _op: i32, _a: u64) -> i32 {
    -1 // not supported
}

/// el_source(el, file) → int
pub extern "C" fn shim_el_source(_el: u64, _file: *const u8) -> i32 {
    0 // success (no-op)
}

/// el_line(el) → const LineInfo*
pub extern "C" fn shim_el_line(_el: u64) -> *const u64 {
    LINE_INFO.as_ptr()
}

/// el_parse(el, argc, argv) → int
pub extern "C" fn shim_el_parse(_el: u64, _argc: i32, _argv: u64) -> i32 {
    -1 // not recognized
}

/// el_resize(el) → void
pub extern "C" fn shim_el_resize(_el: u64) {}

/// el_push(el, str) → void
pub extern "C" fn shim_el_push(_el: u64, _str: *const u8) {}

/// el_insertstr(el, str) → int
pub extern "C" fn shim_el_insertstr(_el: u64, _str: *const u8) -> i32 {
    -1 // fail
}

/// el_deletestr(el, count) → void
pub extern "C" fn shim_el_deletestr(_el: u64, _count: i32) {}

// ── History function stubs ──────────────────────────────────────

/// history_init() → History*
pub extern "C" fn shim_history_init() -> u64 {
    &HIST_SENTINEL as *const u64 as u64
}

/// history_end(h) → void
pub extern "C" fn shim_history_end(_h: u64) {}

/// history(h, ev, op, ...) → int
pub extern "C" fn shim_history(_h: u64, _ev: u64, _op: i32, _a: u64) -> i32 {
    0 // success (no-op)
}

// ── GNU Readline compatibility stubs ────────────────────────────

/// readline(prompt) → char*  (malloc'd string, caller must free)
///
/// Minimal: prints prompt to stdout, reads a line from stdin.
pub extern "C" fn shim_readline(prompt: *const u8) -> *mut u8 {
    // Print prompt
    if !prompt.is_null() {
        unsafe {
            libc::fputs(prompt as *const libc::c_char, libc_stdout());
            libc::fflush(libc_stdout());
        }
    }

    let mut buf = vec![0u8; 1024];
    let result = unsafe { libc::fgets(buf.as_mut_ptr() as *mut libc::c_char, 1024, libc_stdin()) };
    if result.is_null() {
        return std::ptr::null_mut();
    }

    // Strip trailing newline
    let len = unsafe { libc::strlen(buf.as_ptr() as *const libc::c_char) };
    if len > 0 && buf[len - 1] == b'\n' {
        buf[len - 1] = 0;
    }

    // Return a malloc'd copy (caller frees with free())
    let out_len = unsafe { libc::strlen(buf.as_ptr() as *const libc::c_char) } + 1;
    let out = unsafe { libc::malloc(out_len) as *mut u8 };
    if !out.is_null() {
        unsafe {
            std::ptr::copy_nonoverlapping(buf.as_ptr(), out, out_len);
        }
    }
    out
}

/// add_history(line) → void
pub extern "C" fn shim_add_history(_line: *const u8) {
    // No-op: we don't maintain a history list
}

/// rl_completion_matches(text, generator) → char**
pub extern "C" fn shim_rl_completion_matches(_text: *const u8, _gen: u64) -> u64 {
    0 // NULL — no completions
}

// ── Helpers ─────────────────────────────────────────────────────

fn libc_stdin() -> *mut libc::FILE {
    unsafe { libc::fdopen(0, c"r".as_ptr()) }
}

fn libc_stdout() -> *mut libc::FILE {
    unsafe { libc::fdopen(1, c"w".as_ptr()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::ShimTable;

    #[test]
    fn libedit_stubs_registered() {
        let mut table = ShimTable::new();
        LibeditStubs::install(&mut table);
        // EditLine
        assert!(table.resolve("_el_init").is_some());
        assert!(table.resolve("_el_end").is_some());
        assert!(table.resolve("_el_gets").is_some());
        assert!(table.resolve("_el_set").is_some());
        assert!(table.resolve("_el_source").is_some());
        assert!(table.resolve("_el_line").is_some());
        // History
        assert!(table.resolve("_history_init").is_some());
        assert!(table.resolve("_history_end").is_some());
        assert!(table.resolve("_history").is_some());
        // Readline compat
        assert!(table.resolve("_readline").is_some());
        assert!(table.resolve("_add_history").is_some());
        assert!(table.resolve("_rl_completion_matches").is_some());
        // Data symbols
        assert!(table.resolve("_rl_attempted_completion_function").is_some());
        assert!(table.resolve("_rl_line_buffer").is_some());
        assert!(table.resolve("_rl_point").is_some());
        assert!(table.resolve("_rl_readline_name").is_some());
    }

    #[test]
    fn el_init_returns_sentinel() {
        let handle = shim_el_init(std::ptr::null(), 0, 0, 0);
        assert_ne!(handle, 0);
        // el_end should not panic
        shim_el_end(handle);
    }

    #[test]
    fn history_init_returns_sentinel() {
        let handle = shim_history_init();
        assert_ne!(handle, 0);
        shim_history_end(handle);
    }

    #[test]
    fn el_set_returns_success() {
        let el = shim_el_init(std::ptr::null(), 0, 0, 0);
        assert_eq!(shim_el_set(el, 0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn el_get_returns_error() {
        let el = shim_el_init(std::ptr::null(), 0, 0, 0);
        assert_eq!(shim_el_get(el, 0, 0), -1);
    }

    #[test]
    fn el_source_returns_success() {
        let el = shim_el_init(std::ptr::null(), 0, 0, 0);
        assert_eq!(shim_el_source(el, std::ptr::null()), 0);
    }

    #[test]
    fn el_line_returns_non_null() {
        let el = shim_el_init(std::ptr::null(), 0, 0, 0);
        let info = shim_el_line(el);
        assert!(!info.is_null());
    }

    #[test]
    fn el_parse_returns_error() {
        let el = shim_el_init(std::ptr::null(), 0, 0, 0);
        assert_eq!(shim_el_parse(el, 0, 0), -1);
    }

    #[test]
    fn rl_completion_matches_returns_null() {
        assert_eq!(shim_rl_completion_matches(std::ptr::null(), 0), 0);
    }

    #[test]
    fn add_history_does_not_crash() {
        let line = b"test command\0";
        shim_add_history(line.as_ptr());
    }

    #[test]
    fn data_symbols_have_stable_addresses() {
        let a1 = &RL_ATTEMPTED_COMPLETION_FN as *const u64 as u64;
        let a2 = &RL_LINE_BUFFER as *const u64 as u64;
        let a3 = &RL_POINT as *const i32 as u64;
        let a4 = &RL_READLINE_NAME as *const u64 as u64;
        // All should be different non-zero addresses
        assert_ne!(a1, 0);
        assert_ne!(a2, 0);
        assert_ne!(a3, 0);
        assert_ne!(a4, 0);
        assert_ne!(a1, a2);
        assert_ne!(a1, a3);
    }

    #[test]
    fn history_op_returns_success() {
        let h = shim_history_init();
        assert_eq!(shim_history(h, 0, 0, 0), 0);
    }

    #[test]
    fn el_insertstr_returns_error() {
        let el = shim_el_init(std::ptr::null(), 0, 0, 0);
        assert_eq!(shim_el_insertstr(el, std::ptr::null()), -1);
    }

    #[test]
    fn el_resize_does_not_crash() {
        let el = shim_el_init(std::ptr::null(), 0, 0, 0);
        shim_el_resize(el);
    }

    #[test]
    fn el_push_does_not_crash() {
        let el = shim_el_init(std::ptr::null(), 0, 0, 0);
        shim_el_push(el, std::ptr::null());
    }

    #[test]
    fn el_deletestr_does_not_crash() {
        let el = shim_el_init(std::ptr::null(), 0, 0, 0);
        shim_el_deletestr(el, 5);
    }
}
