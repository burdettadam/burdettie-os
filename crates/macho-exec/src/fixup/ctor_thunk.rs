//! Constructor/destructor ABI wrapper thunks.
//!
//! Apple's ARM64 ABI specifies that C++ constructors and destructors
//! return `this` in x0 upon return.  Linux's AAPCS64 ABI does not
//! guarantee this — constructors return void.
//!
//! When macOS Mach-O binaries call constructors resolved from Linux
//! shared libraries (e.g. libc++.so.1), the caller expects x0 to
//! still contain the `this` pointer after the BL returns.  If the
//! Linux constructor clobbers x0, the caller dereferences a stale
//! or zero value → SIGSEGV.
//!
//! This module generates small ARM64 thunks that:
//!   1. Save x0 (`this`) on the stack
//!   2. Call the real Linux constructor/destructor
//!   3. Restore x0 from the stack
//!   4. Return to the caller
//!
//! The thunks live in mmap'd executable pages and are generated on demand.
//!
//! ## Thunk layout (36 bytes per entry, 40 aligned):
//! ```text
//! [0]  STP x29, x30, [sp, #-32]!   ; save frame + LR
//! [4]  STR x0, [sp, #16]           ; save this
//! [8]  LDR x9, [pc, #20]           ; load real address
//! [12] BLR x9                      ; call real
//! [16] LDR x0, [sp, #16]           ; restore this
//! [20] LDP x29, x30, [sp], #32     ; restore frame + LR
//! [24] RET
//! [28] <real_address: u64>          ; 8 bytes of data
//! ```

#[cfg(all(target_arch = "aarch64", not(target_os = "macos")))]
use std::sync::Mutex;

/// Size of one thunk entry (instructions + data, aligned to 8).
#[cfg(target_arch = "aarch64")]
const THUNK_SIZE: usize = 40; // 36 rounded up to 8-byte alignment

/// Number of thunks per 4 KiB page.
#[cfg(all(target_arch = "aarch64", not(target_os = "macos")))]
const THUNKS_PER_PAGE: usize = 4096 / THUNK_SIZE; // 102

/// Machine code template for the thunk (7 instructions = 28 bytes).
#[cfg(target_arch = "aarch64")]
static THUNK_TEMPLATE: [u32; 7] = [
    0xA9BE_7BFD, // STP x29, x30, [sp, #-32]!
    0xF900_0BE0, // STR x0, [sp, #16]
    0x5800_00A9, // LDR x9, [pc, #20]
    0xD63F_0120, // BLR x9
    0xF940_0BE0, // LDR x0, [sp, #16]
    0xA8C2_7BFD, // LDP x29, x30, [sp], #32
    0xD65F_03C0, // RET
];

#[cfg(all(target_arch = "aarch64", not(target_os = "macos")))]
struct ThunkPage {
    base: *mut u8,
    used: usize,
}

// SAFETY: ThunkPage pointers are to mmap'd memory that outlives the process.
#[cfg(all(target_arch = "aarch64", not(target_os = "macos")))]
unsafe impl Send for ThunkPage {}

#[cfg(all(target_arch = "aarch64", not(target_os = "macos")))]
static THUNK_PAGES: Mutex<Vec<ThunkPage>> = Mutex::new(Vec::new());

/// Allocate a new executable page for thunks.
#[cfg(all(target_arch = "aarch64", not(target_os = "macos")))]
fn alloc_page() -> Option<ThunkPage> {
    let page = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            4096,
            libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
            libc::MAP_ANONYMOUS | libc::MAP_PRIVATE,
            -1,
            0,
        )
    };
    if page == libc::MAP_FAILED || page.is_null() {
        log::error!("ctor_thunk: failed to mmap executable page");
        return None;
    }
    Some(ThunkPage {
        base: page as *mut u8,
        used: 0,
    })
}

/// Generate a constructor/destructor wrapper thunk that preserves x0.
///
/// Returns the address of the thunk, or `None` if allocation fails.
#[cfg(all(target_arch = "aarch64", not(target_os = "macos")))]
pub fn wrap_ctor(real_addr: u64) -> Option<u64> {
    let mut pages = THUNK_PAGES.lock().ok()?;

    // Find a page with space, or allocate a new one.
    let page = if let Some(p) = pages.last_mut().filter(|p| p.used < THUNKS_PER_PAGE) {
        p
    } else {
        pages.push(alloc_page()?);
        pages.last_mut().unwrap()
    };

    let offset = page.used * THUNK_SIZE;
    let thunk_ptr = unsafe { page.base.add(offset) };

    // Write the instruction template (28 bytes).
    unsafe {
        let instr_ptr = thunk_ptr as *mut u32;
        for (i, &instr) in THUNK_TEMPLATE.iter().enumerate() {
            instr_ptr.add(i).write_unaligned(instr);
        }
        // Write the real address at offset 28.
        let addr_ptr = thunk_ptr.add(28) as *mut u64;
        addr_ptr.write_unaligned(real_addr);
    }

    // Clear instruction cache for the thunk.
    #[cfg(target_os = "linux")]
    unsafe {
        extern "C" {
            fn __clear_cache(beg: *mut u8, end: *mut u8);
        }
        __clear_cache(thunk_ptr, thunk_ptr.add(THUNK_SIZE));
    }

    page.used += 1;
    let thunk_addr = thunk_ptr as u64;
    let slot_idx = page.used - 1;
    let page_idx = pages.len() - 1;
    log::trace!(
        "ctor_thunk: wrapped {real_addr:#x} at thunk {thunk_addr:#x} (page {page_idx}, slot {slot_idx})",
    );
    Some(thunk_addr)
}

/// Check if a symbol name (Mach-O mangled, with leading `_`) is a C++
/// constructor or destructor that needs a `this`-preserving wrapper.
///
/// Itanium mangling: constructors are `C1`/`C2`/`CI1`/`CI2`,
/// destructors are `D0`/`D1`/`D2`.  The `E` after the ctor/dtor
/// marker separates it from the parameter types.
pub fn is_ctor_or_dtor(symbol: &str) -> bool {
    // Strip Mach-O leading underscore(s) to get the Itanium mangled name.
    let name = symbol
        .strip_prefix("__Z")
        .or_else(|| symbol.strip_prefix("_Z"));
    let Some(name) = name else { return false };

    // Look for constructor/destructor markers followed by 'E' (end of nested name).
    // The markers appear right after the class/namespace encoding.
    has_ctor_dtor_marker(name)
}

/// Scan a demangled fragment for C1E, C2E, D0E, D1E, D2E patterns.
fn has_ctor_dtor_marker(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 3 {
        return false;
    }
    for i in 0..bytes.len() - 2 {
        match (bytes[i], bytes[i + 1], bytes[i + 2]) {
            (b'C', b'1', b'E') | (b'C', b'2', b'E') => return true,
            (b'D', b'0', b'E') | (b'D', b'1', b'E') | (b'D', b'2', b'E') => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_constructors() {
        assert!(is_ctor_or_dtor("__ZNSt3__115recursive_mutexC1Ev"));
        assert!(is_ctor_or_dtor("__ZNSt3__16localeC1Ev"));
        assert!(is_ctor_or_dtor("__ZNSt3__16localeC2Ev"));
        assert!(is_ctor_or_dtor("__ZNSt3__114error_categoryD2Ev"));
        assert!(is_ctor_or_dtor("__ZNSt3__119__shared_weak_countD2Ev"));
    }

    #[test]
    fn reject_non_constructors() {
        assert!(!is_ctor_or_dtor("__ZNSt3__15mutex4lockEv"));
        assert!(!is_ctor_or_dtor("__ZNSt3__19to_stringEi"));
        assert!(!is_ctor_or_dtor("_printf"));
        assert!(!is_ctor_or_dtor("_malloc"));
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn thunk_template_size() {
        assert_eq!(THUNK_TEMPLATE.len() * 4, 28);
        const { assert!(THUNK_SIZE >= 28 + 8) }; // instructions + address
    }
}
