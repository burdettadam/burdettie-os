//! Memory model for the ARM64 interpreter.
//!
//! Provides read/write access to the mapped Mach-O segments plus a
//! dedicated interpreter stack. Reads and writes are bounds-checked
//! against the known segment ranges.

use std::collections::BTreeMap;

/// Check if the memory page containing `addr` is mapped in the process.
/// Uses `mincore()` on macOS/Linux — it returns ENOMEM for unmapped pages.
fn is_page_mapped(addr: u64) -> bool {
    let page_size = 0x4000u64; // 16 KB on arm64 macOS (safe default)
    let page_start = addr & !(page_size - 1);
    let mut vec: u8 = 0;
    // SAFETY: mincore checks page residency; it won't fault even on
    // unmapped addresses — it returns -1 with errno=ENOMEM instead.
    let ret = unsafe {
        libc::mincore(
            page_start as *mut libc::c_void,
            page_size as usize,
            &mut vec as *mut u8 as *mut libc::c_char,
        )
    };
    ret == 0
}

/// A memory region accessible to the interpreter.
#[derive(Debug)]
struct Region {
    /// Host pointer to the start of this region.
    base: *mut u8,
    /// Guest (virtual) start address — identity-mapped, kept for Debug output.
    #[allow(dead_code)]
    guest_start: u64,
    /// Size in bytes.
    size: usize,
    /// Writable?
    writable: bool,
}

/// Memory model: maps guest (ARM64 virtual) addresses to host memory.
///
/// Backed by the MappedImage segments plus an interpreter-owned stack.
/// The stack is `mmap`'d at a real virtual address so that guest and host
/// pointers are identical — no address translation is needed at FFI boundaries.
/// When `host_passthrough` is enabled, reads/writes to addresses outside
/// mapped regions are forwarded directly to host memory. This is correct
/// for in-process interpretation where guest pointers may reference
/// host-allocated objects (ObjC class structs, shim allocations, etc.).
pub struct Memory {
    /// Regions sorted by guest_start for binary search.
    regions: BTreeMap<u64, Region>,
    /// mmap'd stack allocation (host pointer == guest address).
    stack_ptr: *mut u8,
    /// Size of the mmap'd stack.
    stack_size: usize,
    /// Top of the stack (highest address).
    stack_top: u64,
    /// When true, unmapped addresses are treated as direct host pointers.
    pub host_passthrough: bool,
}

// SAFETY: Memory owns its stack and holds raw pointers into MappedImage
// segments that outlive it (the caller ensures this).
unsafe impl Send for Memory {}

impl Drop for Memory {
    fn drop(&mut self) {
        if !self.stack_ptr.is_null() {
            unsafe {
                libc::munmap(self.stack_ptr as *mut libc::c_void, self.stack_size);
            }
        }
    }
}

/// Default interpreter stack size: 1 MB.
const DEFAULT_STACK_SIZE: usize = 1024 * 1024;

impl Memory {
    /// Create a new memory model from mapped segments.
    ///
    /// # Safety
    ///
    /// The caller must ensure the MappedImage segments remain valid
    /// (not unmapped) for the lifetime of this Memory.
    pub unsafe fn from_segments(segments: &[(u64, *mut u8, usize, bool)]) -> Self {
        let mut regions = BTreeMap::new();
        for &(guest_start, base, size, writable) in segments {
            regions.insert(
                guest_start,
                Region {
                    base,
                    guest_start,
                    size,
                    writable,
                },
            );
        }

        // mmap the interpreter stack at a real virtual address.
        // guest_start == base as u64, so guest pointers ARE host pointers.
        let stack_ptr = libc::mmap(
            std::ptr::null_mut(),
            DEFAULT_STACK_SIZE,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        ) as *mut u8;
        assert!(
            stack_ptr != libc::MAP_FAILED as *mut u8,
            "mmap failed for interpreter stack"
        );
        let stack_guest_start = stack_ptr as u64;
        let stack_top = stack_guest_start + DEFAULT_STACK_SIZE as u64;
        regions.insert(
            stack_guest_start,
            Region {
                base: stack_ptr,
                guest_start: stack_guest_start,
                size: DEFAULT_STACK_SIZE,
                writable: true,
            },
        );

        Self {
            regions,
            stack_ptr,
            stack_size: DEFAULT_STACK_SIZE,
            stack_top,
            host_passthrough: false,
        }
    }

    /// Initial stack pointer value (top of stack, 16-byte aligned).
    /// Leaves headroom above SP for the 128-byte host stack copy used by
    /// variadic FFI calls (call_host_capturing).
    pub fn initial_sp(&self) -> u64 {
        self.stack_top - 256
    }

    /// Translate a guest address to a host pointer using only mapped regions
    /// (ignoring host_passthrough). Used for instruction fetch.
    fn translate_strict(&self, addr: u64, size: usize) -> Option<*const u8> {
        if let Some((&region_start, region)) = self.regions.range(..=addr).next_back() {
            let offset = (addr - region_start) as usize;
            if offset + size <= region.size {
                return Some(unsafe { region.base.add(offset) as *const u8 });
            }
        }
        None
    }

    /// Translate a guest address to a host pointer for reading.
    fn translate(&self, addr: u64, size: usize) -> Option<*const u8> {
        // Find the region containing addr (the largest key <= addr).
        if let Some((&region_start, region)) = self.regions.range(..=addr).next_back() {
            let offset = (addr - region_start) as usize;
            if offset + size <= region.size {
                return Some(unsafe { region.base.add(offset) as *const u8 });
            }
        }
        // Host passthrough: treat the address as a direct host pointer,
        // but only if the page is actually mapped (to avoid SIGSEGV).
        if self.host_passthrough && addr >= 0x1000 && is_page_mapped(addr) {
            return Some(addr as *const u8);
        }
        None
    }

    /// Translate a guest address to a host pointer for writing.
    fn translate_mut(&self, addr: u64, size: usize) -> Option<*mut u8> {
        if let Some((&region_start, region)) = self.regions.range(..=addr).next_back() {
            if region.writable {
                let offset = (addr - region_start) as usize;
                if offset + size <= region.size {
                    return Some(unsafe { region.base.add(offset) });
                }
            }
        }
        // Host passthrough: treat the address as a direct host pointer,
        // but only if the page is actually mapped (to avoid SIGSEGV).
        if self.host_passthrough && addr >= 0x1000 && is_page_mapped(addr) {
            return Some(addr as *mut u8);
        }
        None
    }

    /// Read a u8 from guest memory.
    pub fn read_u8(&self, addr: u64) -> Option<u8> {
        let ptr = self.translate(addr, 1)?;
        Some(unsafe { ptr.read() })
    }

    /// Read a u16 (little-endian) from guest memory.
    pub fn read_u16(&self, addr: u64) -> Option<u16> {
        let ptr = self.translate(addr, 2)?;
        Some(u16::from_le_bytes(unsafe {
            *(ptr as *const [u8; 2])
        }))
    }

    /// Read a u32 from mapped regions only (no host passthrough).
    /// Used for instruction fetch — the interpreter must not execute
    /// bytes from host memory.
    pub fn read_u32_mapped(&self, addr: u64) -> Option<u32> {
        let ptr = self.translate_strict(addr, 4)?;
        Some(u32::from_le_bytes(unsafe {
            *(ptr as *const [u8; 4])
        }))
    }

    /// Read a u32 (little-endian) from guest memory.
    pub fn read_u32(&self, addr: u64) -> Option<u32> {
        let ptr = self.translate(addr, 4)?;
        Some(u32::from_le_bytes(unsafe {
            *(ptr as *const [u8; 4])
        }))
    }

    /// Read a u64 (little-endian) from guest memory.
    pub fn read_u64(&self, addr: u64) -> Option<u64> {
        let ptr = self.translate(addr, 8)?;
        Some(u64::from_le_bytes(unsafe {
            *(ptr as *const [u8; 8])
        }))
    }

    /// Read a u128 (little-endian) from guest memory (for Q register loads).
    pub fn read_u128(&self, addr: u64) -> Option<u128> {
        let ptr = self.translate(addr, 16)?;
        Some(u128::from_le_bytes(unsafe {
            *(ptr as *const [u8; 16])
        }))
    }

    /// Write a u8 to guest memory.
    pub fn write_u8(&self, addr: u64, val: u8) -> bool {
        if let Some(ptr) = self.translate_mut(addr, 1) {
            unsafe { ptr.write(val) };
            true
        } else {
            false
        }
    }

    /// Write a u16 (little-endian) to guest memory.
    pub fn write_u16(&self, addr: u64, val: u16) -> bool {
        if let Some(ptr) = self.translate_mut(addr, 2) {
            unsafe { (ptr as *mut [u8; 2]).write(val.to_le_bytes()) };
            true
        } else {
            false
        }
    }

    /// Write a u32 (little-endian) to guest memory.
    pub fn write_u32(&self, addr: u64, val: u32) -> bool {
        if let Some(ptr) = self.translate_mut(addr, 4) {
            unsafe { (ptr as *mut [u8; 4]).write(val.to_le_bytes()) };
            true
        } else {
            false
        }
    }

    /// Write a u64 (little-endian) to guest memory.
    pub fn write_u64(&self, addr: u64, val: u64) -> bool {
        if let Some(ptr) = self.translate_mut(addr, 8) {
            unsafe { (ptr as *mut [u8; 8]).write(val.to_le_bytes()) };
            true
        } else {
            false
        }
    }

    /// Write a u128 (little-endian) to guest memory (for Q register stores).
    pub fn write_u128(&self, addr: u64, val: u128) -> bool {
        if let Some(ptr) = self.translate_mut(addr, 16) {
            unsafe { (ptr as *mut [u8; 16]).write(val.to_le_bytes()) };
            true
        } else {
            false
        }
    }

    /// Read a NUL-terminated C string from guest memory.
    pub fn read_cstring(&self, mut addr: u64) -> Option<String> {
        let mut bytes = Vec::new();
        loop {
            let b = self.read_u8(addr)?;
            if b == 0 {
                break;
            }
            bytes.push(b);
            addr += 1;
            if bytes.len() > 4096 {
                break; // safety limit
            }
        }
        String::from_utf8(bytes).ok()
    }

    /// Check whether a guest address falls within a mapped region.
    /// Uses strict checking (no host passthrough) — this is used for
    /// branch target validation to detect FFI calls.
    pub fn is_valid(&self, addr: u64) -> bool {
        self.translate_strict(addr, 1).is_some()
    }

    // ── Atomic operations ──────────────────────────────────────

    /// Atomic load of a u32 with Acquire ordering.
    /// For LDAR/LDAXR emulation. Identity-mapped addresses allow direct
    /// use of `std::sync::atomic::AtomicU32`.
    pub fn atomic_read_u32(&self, addr: u64) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate(addr, 4)?;
        // SAFETY: ptr is a valid, aligned (or unaligned — AtomicU32 requires
        // alignment) host pointer. Identity-mapped addresses are naturally
        // aligned for Mach-O segments.
        let atomic = unsafe { &*(ptr as *const AtomicU32) };
        Some(atomic.load(Ordering::Acquire))
    }

    /// Atomic load of a u64 with Acquire ordering.
    pub fn atomic_read_u64(&self, addr: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate(addr, 8)?;
        let atomic = unsafe { &*(ptr as *const AtomicU64) };
        Some(atomic.load(Ordering::Acquire))
    }

    /// Atomic store of a u32 with Release ordering.
    /// For STLR/STLXR emulation.
    pub fn atomic_write_u32(&self, addr: u64, val: u32) -> bool {
        use std::sync::atomic::{AtomicU32, Ordering};
        if let Some(ptr) = self.translate_mut(addr, 4) {
            let atomic = unsafe { &*(ptr as *const AtomicU32) };
            atomic.store(val, Ordering::Release);
            true
        } else {
            false
        }
    }

    /// Atomic store of a u64 with Release ordering.
    pub fn atomic_write_u64(&self, addr: u64, val: u64) -> bool {
        use std::sync::atomic::{AtomicU64, Ordering};
        if let Some(ptr) = self.translate_mut(addr, 8) {
            let atomic = unsafe { &*(ptr as *const AtomicU64) };
            atomic.store(val, Ordering::Release);
            true
        } else {
            false
        }
    }

    /// Atomic compare-and-swap (u32). Returns the old value.
    /// For CAS/CASA/CASAL emulation (LSE atomics).
    pub fn atomic_cas_u32(&self, addr: u64, expected: u32, new_val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_mut(addr, 4)?;
        let atomic = unsafe { &*(ptr as *const AtomicU32) };
        match atomic.compare_exchange(expected, new_val, Ordering::AcqRel, Ordering::Acquire) {
            Ok(old) => Some(old),
            Err(old) => Some(old),
        }
    }

    /// Atomic compare-and-swap (u64). Returns the old value.
    pub fn atomic_cas_u64(&self, addr: u64, expected: u64, new_val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_mut(addr, 8)?;
        let atomic = unsafe { &*(ptr as *const AtomicU64) };
        match atomic.compare_exchange(expected, new_val, Ordering::AcqRel, Ordering::Acquire) {
            Ok(old) => Some(old),
            Err(old) => Some(old),
        }
    }

    /// Atomic fetch-and-add (u32). Returns the old value.
    pub fn atomic_fetch_add_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_mut(addr, 4)?;
        let atomic = unsafe { &*(ptr as *const AtomicU32) };
        Some(atomic.fetch_add(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-add (u64). Returns the old value.
    pub fn atomic_fetch_add_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_mut(addr, 8)?;
        let atomic = unsafe { &*(ptr as *const AtomicU64) };
        Some(atomic.fetch_add(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-or (u32). Returns the old value.
    pub fn atomic_fetch_or_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_mut(addr, 4)?;
        let atomic = unsafe { &*(ptr as *const AtomicU32) };
        Some(atomic.fetch_or(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-or (u64). Returns the old value.
    pub fn atomic_fetch_or_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_mut(addr, 8)?;
        let atomic = unsafe { &*(ptr as *const AtomicU64) };
        Some(atomic.fetch_or(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-and (u32). Returns the old value.
    pub fn atomic_fetch_and_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_mut(addr, 4)?;
        let atomic = unsafe { &*(ptr as *const AtomicU32) };
        Some(atomic.fetch_and(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-and (u64). Returns the old value.
    pub fn atomic_fetch_and_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_mut(addr, 8)?;
        let atomic = unsafe { &*(ptr as *const AtomicU64) };
        Some(atomic.fetch_and(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-xor (u32). Returns the old value.
    pub fn atomic_fetch_xor_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_mut(addr, 4)?;
        let atomic = unsafe { &*(ptr as *const AtomicU32) };
        Some(atomic.fetch_xor(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-xor (u64). Returns the old value.
    pub fn atomic_fetch_xor_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_mut(addr, 8)?;
        let atomic = unsafe { &*(ptr as *const AtomicU64) };
        Some(atomic.fetch_xor(val, Ordering::AcqRel))
    }

    /// Atomic swap (u32). Returns the old value.
    pub fn atomic_swap_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_mut(addr, 4)?;
        let atomic = unsafe { &*(ptr as *const AtomicU32) };
        Some(atomic.swap(val, Ordering::AcqRel))
    }

    /// Atomic swap (u64). Returns the old value.
    pub fn atomic_swap_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_mut(addr, 8)?;
        let atomic = unsafe { &*(ptr as *const AtomicU64) };
        Some(atomic.swap(val, Ordering::AcqRel))
    }

    /// Atomic fetch-min (signed, u64). Returns the old value.
    pub fn atomic_fetch_smin_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_mut(addr, 8)?;
        let atomic = unsafe { &*(ptr as *const AtomicU64) };
        Some(atomic.fetch_min(val, Ordering::AcqRel))
    }

    /// Atomic fetch-max (signed, u64). Returns the old value.
    pub fn atomic_fetch_smax_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_mut(addr, 8)?;
        let atomic = unsafe { &*(ptr as *const AtomicU64) };
        Some(atomic.fetch_max(val, Ordering::AcqRel))
    }
}

// SAFETY: Memory is safe to share across threads once constructed.
// Regions are read-only metadata; reads/writes go through raw host pointers
// which are identity-mapped and valid for the process lifetime. Atomic
// operations use std::sync::atomic for proper ordering.
unsafe impl Sync for Memory {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[test]
    fn test_write_read_u8() {
        let mut t = test_interp(&[]);
        t.write_data_u8(0, 0xAB);
        assert_eq!(t.read_data_u8(0), 0xAB);
    }

    #[test]
    fn test_write_read_u16() {
        let mut t = test_interp(&[]);
        t.write_data_u16(0, 0xBEEF);
        assert_eq!(t.read_data_u16(0), 0xBEEF);
    }

    #[test]
    fn test_write_read_u32() {
        let mut t = test_interp(&[]);
        t.write_data_u32(0, 0xDEAD_BEEF);
        assert_eq!(t.read_data_u32(0), 0xDEAD_BEEF);
    }

    #[test]
    fn test_write_read_u64() {
        let mut t = test_interp(&[]);
        t.write_data_u64(0, 0xCAFE_BABE_DEAD_BEEF);
        assert_eq!(t.read_data_u64(0), 0xCAFE_BABE_DEAD_BEEF);
    }

    #[test]
    fn test_write_read_u128() {
        let mut t = test_interp(&[]);
        let val: u128 = 0x0123_4567_89AB_CDEF_FEDC_BA98_7654_3210;
        t.write_data_u128(0, val);
        assert_eq!(t.read_data_u128(0), val);
    }

    #[test]
    fn test_read_u32_mapped_code_region() {
        let t = test_interp(&[enc_nop()]);
        // The NOP instruction should be readable from the code region via read_u32_mapped.
        let val = t.interp.mem.read_u32_mapped(t.code_base);
        assert_eq!(val, Some(enc_nop()));
    }

    #[test]
    fn test_is_valid_mapped() {
        let t = test_interp(&[]);
        // Code region is mapped.
        assert!(t.interp.mem.is_valid(t.code_base));
        // Data region is mapped (same 8K allocation).
        assert!(t.interp.mem.is_valid(t.data_base));
    }

    #[test]
    fn test_is_valid_unmapped() {
        let t = test_interp(&[]);
        // Address 0x1 is not in any mapped region.
        assert!(!t.interp.mem.is_valid(0x1));
    }

    #[test]
    fn test_initial_sp_in_stack() {
        let t = test_interp(&[]);
        let sp = t.interp.mem.initial_sp();
        // SP should be within the stack region (stack_top - 256).
        assert_eq!(sp, t.interp.mem.stack_top - 256);
        // SP should be in a valid mapped region.
        assert!(t.interp.mem.is_valid(sp));
    }

    #[test]
    fn test_write_multiple_read_back() {
        let mut t = test_interp(&[]);
        t.write_data_u32(0, 0xAAAA_AAAA);
        t.write_data_u32(4, 0xBBBB_BBBB);
        t.write_data_u32(8, 0xCCCC_CCCC);
        // Read in order.
        assert_eq!(t.read_data_u32(0), 0xAAAA_AAAA);
        assert_eq!(t.read_data_u32(4), 0xBBBB_BBBB);
        assert_eq!(t.read_data_u32(8), 0xCCCC_CCCC);
        // Read in reverse order.
        assert_eq!(t.read_data_u32(8), 0xCCCC_CCCC);
        assert_eq!(t.read_data_u32(4), 0xBBBB_BBBB);
        assert_eq!(t.read_data_u32(0), 0xAAAA_AAAA);
    }

    #[test]
    fn test_read_cstring() {
        let mut t = test_interp(&[]);
        let msg = b"hello\0";
        for (i, &b) in msg.iter().enumerate() {
            t.write_data_u8(i as u64, b);
        }
        let s = t.interp.mem.read_cstring(t.data_base).unwrap();
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_read_cstring_truncation() {
        let mut t = test_interp(&[]);
        // Write "abc\0xyz" — read_cstring should stop at the NUL.
        for (i, &b) in b"abc\0xyz".iter().enumerate() {
            t.write_data_u8(i as u64, b);
        }
        let s = t.interp.mem.read_cstring(t.data_base).unwrap();
        assert_eq!(s, "abc");
    }

    #[test]
    fn test_read_unmapped_returns_none() {
        let t = test_interp(&[]);
        // host_passthrough is false by default, so 0x1 should return None.
        assert!(t.interp.mem.read_u64(0x1).is_none());
    }

    #[test]
    fn test_write_unmapped_returns_false() {
        let t = test_interp(&[]);
        // Writing to an unmapped address should return false.
        assert!(!t.interp.mem.write_u64(0x1, 0x1234));
    }

    #[test]
    fn test_endianness_little_endian() {
        let mut t = test_interp(&[]);
        // Write 0xDEADBEEF as u32, then read byte at same address → least significant byte.
        t.write_data_u32(0, 0xDEAD_BEEF);
        assert_eq!(t.read_data_u8(0), 0xEF); // LSB first (little-endian)
        assert_eq!(t.read_data_u8(1), 0xBE);
        assert_eq!(t.read_data_u8(2), 0xAD);
        assert_eq!(t.read_data_u8(3), 0xDE);
    }
}
