//! NSLog shim: format ObjC objects via fprintf(stderr).
//!
//! Real NSLog uses CFStringCreateWithFormat + asl_log. Our shim
//! handles the common case: `NSLog(@"string literal", ...)` where
//! the format is a compile-time `__CFConstantStringClassReference`.
//!
//! On ARM64 macOS, NSLog is variadic — named param (`format`) is in x0,
//! but variadic args are on the stack. The assembly trampoline in
//! `nslog_trampoline.s` reads stack args into registers before calling
//! `shim_nslog_inner`.

use std::sync::Mutex;

// ── ARM64 assembly trampoline linkage ──────────────────────────────

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
extern "C" {
    fn shim_nslog_asm();
}

// ── x86_64 assembly trampoline linkage ─────────────────────────────

#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
extern "C" {
    fn shim_nslog_x86_64_asm();
}

/// Returns the address of the NSLog variadic trampoline.
///
/// On aarch64-apple, this is the assembly trampoline that reads
/// variadic args from the caller's stack before calling `shim_nslog_inner`.
/// On x86_64-apple, variadic args are already in registers, so
/// the trampoline is a simple tail-call.
/// On other platforms, returns the Rust function directly.
pub fn nslog_trampoline_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_nslog_asm as u64
    }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    {
        shim_nslog_x86_64_asm as u64
    }
    #[cfg(not(all(any(target_arch = "aarch64", target_arch = "x86_64"), target_os = "macos")))]
    {
        shim_nslog_inner as u64
    }
}

/// Capture buffer for NSLog output. Tests can read this to verify
/// NSLog produced correct output without parsing stderr.
static NSLOG_CAPTURE: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Read and clear all captured NSLog output lines.
pub fn drain_captured_nslog() -> Vec<String> {
    let mut buf = NSLOG_CAPTURE.lock().unwrap_or_else(|e| e.into_inner());
    buf.drain(..).collect()
}

/// Inner NSLog implementation, called by the ARM64 trampoline.
///
/// The trampoline reads stack-passed variadic args and places them
/// in a1-a3 before calling this function with a normal C ABI.
#[no_mangle]
pub extern "C" fn shim_nslog_inner(format: u64, a1: u64, a2: u64, a3: u64) {
    if format == 0 {
        return;
    }

    // Try to extract C string from CFConstantString layout
    let c_str = crate::cfstring::extract_cfstring_cstr(format);

    match c_str {
        Some(s) => {
            let expanded = expand_format(s, &[a1, a2, a3]);
            // Capture for test verification
            let mut buf = NSLOG_CAPTURE.lock().unwrap_or_else(|e| e.into_inner());
            buf.push(expanded.clone());
            eprintln!("{}", expanded);
        }
        None => {
            eprintln!("<NSLog: unreadable format at {:#x}>", format);
        }
    }
}

/// Simple format string expansion — delegates to format module.
pub(crate) use crate::format::expand_format;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn drain_initially_empty() {
        let _ = drain_captured_nslog(); // flush any prior output
        let lines = drain_captured_nslog();
        assert!(lines.is_empty());
    }

    #[test]
    fn nil_format_is_safe() {
        // shim_nslog_inner with format=0 must not panic
        shim_nslog_inner(0, 0, 0, 0);
    }

    #[test]
    fn captures_log_output() {
        let _ = drain_captured_nslog();
        let fmt = make_string("cf_lite_nslog_sentinel_xyz789");
        shim_nslog_inner(fmt, 0, 0, 0);
        let lines = drain_captured_nslog();
        assert!(
            lines.iter().any(|l| l == "cf_lite_nslog_sentinel_xyz789"),
            "Expected sentinel in NSLog capture, got: {lines:?}"
        );
    }

    #[test]
    fn trampoline_addr_is_nonzero() {
        assert_ne!(nslog_trampoline_addr(), 0);
    }
}
