//! Shared test utilities for Foundation shim unit tests.
//!
//! Provides ergonomic helpers for object allocation, string creation,
//! and cleanup — eliminating boilerplate across per-class test modules.
//!
//! # Usage from a shim module's `#[cfg(test)]` block:
//!
//! ```rust,ignore
//! #[cfg(test)]
//! mod tests {
//!     use super::*;
//!     use crate::shim_test_helpers::*;
//!
//!     #[test]
//!     fn my_test() {
//!         let obj = alloc_obj(24, nsobj::nsdata_class_addr());
//!         // ... test shim functions ...
//!         free_obj(obj);
//!     }
//! }
//! ```
//!
//! # Compromises (for future maintainers)
//!
//! - Fake ISA pointers are suitable only when a test never dispatches ObjC
//!   methods. Dispatch tests need allocated objects with valid class addresses.
//! - No retain/release tracking — tests do manual `free_obj` calls, not
//!   autorelease semantics.
//! - String equality is byte-level via `extract_cfstring_cstr`, not locale-aware.
//! - Allocation-only tests can use the `pal_alloc()` fallback. Tests that use
//!   platform services such as clocks must explicitly install a provider.
//! - The test harness runs tests in parallel. Tests sharing mutable global
//!   state must coordinate access; tests requiring fresh process-wide identity
//!   use `with_isolated_test_process` instead of resetting production state.

/// Run an exact test in a fresh process, so its process-wide `OnceLock` state
/// does not depend on which other test worker initialized the runtime first.
/// The per-test environment guard lets the child execute the body once.
pub fn with_isolated_test_process(test_name: &str, body: impl FnOnce()) {
    const GUARD: &str = "BURDETTIE_CF_ISOLATED_TEST";
    let completion = format!("isolated test completed: {test_name}");
    if std::env::var(GUARD).ok().as_deref() == Some(test_name) {
        body();
        eprintln!("{completion}");
        return;
    }

    let output = std::process::Command::new(std::env::current_exe().expect("current test binary"))
        .args(["--exact", test_name, "--nocapture"])
        .env(GUARD, test_name)
        .output()
        .expect("run isolated test process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stderr.contains(&completion),
        "isolated test {test_name} did not complete successfully: {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status
    );
}

/// Allocate a zeroed object of `size` bytes with the given ISA pointer.
///
/// Returns the object address as u64. The caller must eventually call
/// `free_obj` to avoid leaks in tests.
pub fn alloc_obj(size: usize, isa: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, size) as u64;
    assert_ne!(obj, 0, "calloc failed in test");
    unsafe {
        *(obj as *mut u64) = isa;
    }
    obj
}

/// Allocate a zeroed object with a fake ISA (`0xDEAD`).
///
/// Use this when the test doesn't care about the ISA value.
pub fn alloc_obj_fake(size: usize) -> u64 {
    alloc_obj(size, 0xDEAD)
}

/// Free an object previously allocated with `alloc_obj` / `alloc_obj_fake`.
pub fn free_obj(obj: u64) {
    if obj != 0 {
        crate::pal_alloc().free(obj as *mut u8);
    }
}

/// Create a CFString/NSString from a Rust `&str`.
///
/// The returned object is heap-allocated and must be freed by the caller
/// (or allowed to leak in short-lived tests).
pub fn make_string(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap();
    crate::cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

/// Extract the string content of a CFString/NSString as a Rust `String`.
///
/// Panics if the object is not a valid CFString.
pub fn read_string(obj: u64) -> String {
    crate::cfstring::extract_cfstring_cstr(obj)
        .unwrap_or_else(|| panic!("read_string: not a valid CFString at {obj:#x}"))
        .to_string()
}

/// Assert that a CFString/NSString object contains exactly `expected`.
pub fn assert_string_eq(obj: u64, expected: &str) {
    let actual = read_string(obj);
    assert_eq!(actual, expected, "CFString content mismatch");
}

/// Write a u64 value at a byte offset from an object base address.
///
/// # Safety
/// Caller must ensure `obj + offset` is within the allocated region.
pub fn write_field(obj: u64, offset: usize, value: u64) {
    unsafe {
        *((obj as usize + offset) as *mut u64) = value;
    }
}

/// Read a u64 value at a byte offset from an object base address.
///
/// # Safety
/// Caller must ensure `obj + offset` is within the allocated region.
pub fn read_field(obj: u64, offset: usize) -> u64 {
    unsafe { *((obj as usize + offset) as *const u64) }
}

/// Read an i64 value at a byte offset from an object base address.
pub fn read_field_i64(obj: u64, offset: usize) -> i64 {
    unsafe { *((obj as usize + offset) as *const i64) }
}

/// Allocate a buffer of `count` u64 slots and return its address.
///
/// Useful for creating element arrays for NSArray, NSSet, etc.
pub fn alloc_u64_buf(count: usize) -> u64 {
    crate::pal_alloc().calloc(count, 8) as u64
}

/// Free a buffer previously allocated with `alloc_u64_buf`.
pub fn free_buf(ptr: u64) {
    if ptr != 0 {
        crate::pal_alloc().free(ptr as *mut u8);
    }
}

/// Copy a slice of u64 values into a freshly allocated buffer.
///
/// Returns the buffer address. Caller must free with `free_buf`.
pub fn make_u64_array(items: &[u64]) -> u64 {
    let buf = alloc_u64_buf(items.len());
    if buf != 0 && !items.is_empty() {
        unsafe {
            std::ptr::copy_nonoverlapping(
                items.as_ptr() as *const u8,
                buf as *mut u8,
                items.len() * 8,
            );
        }
    }
    buf
}
