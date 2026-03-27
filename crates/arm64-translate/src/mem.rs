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
    /// Guest (virtual) start address.
    guest_start: u64,
    /// Size in bytes.
    size: usize,
    /// Writable?
    writable: bool,
}

/// Memory model: maps guest (ARM64 virtual) addresses to host memory.
///
/// Backed by the MappedImage segments plus an interpreter-owned stack.
/// When `host_passthrough` is enabled, reads/writes to addresses outside
/// mapped regions are forwarded directly to host memory. This is correct
/// for in-process interpretation where guest pointers may reference
/// host-allocated objects (ObjC class structs, shim allocations, etc.).
pub struct Memory {
    /// Regions sorted by guest_start for binary search.
    regions: BTreeMap<u64, Region>,
    /// Interpreter-owned stack allocation.
    stack: Vec<u8>,
    /// Guest address of the stack base (bottom, highest address).
    stack_guest_base: u64,
    /// When true, unmapped addresses are treated as direct host pointers.
    pub host_passthrough: bool,
}

// SAFETY: Memory owns its stack and holds raw pointers into MappedImage
// segments that outlive it (the caller ensures this).
unsafe impl Send for Memory {}

/// Default interpreter stack size: 1 MB.
const DEFAULT_STACK_SIZE: usize = 1024 * 1024;

/// Guest address for the interpreter stack (well above typical Mach-O load addresses).
const STACK_GUEST_BASE: u64 = 0x7000_0000_0000;

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

        let mut stack = vec![0u8; DEFAULT_STACK_SIZE];
        let stack_guest_start = STACK_GUEST_BASE - DEFAULT_STACK_SIZE as u64;
        let stack_ptr = stack.as_mut_ptr();
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
            stack,
            stack_guest_base: STACK_GUEST_BASE,
            host_passthrough: false,
        }
    }

    /// Initial stack pointer value (top of stack, 16-byte aligned).
    pub fn initial_sp(&self) -> u64 {
        self.stack_guest_base - 16
    }

    /// Translate a guest address to a host pointer (read-only).
    /// Returns 0 if the address cannot be mapped.
    pub fn guest_to_host(&self, addr: u64, size: usize) -> u64 {
        // Use strict region lookup — never fall through to host_passthrough,
        // because guest stack addresses are NOT valid host pointers.
        self.translate_strict(addr, size).map(|p| p as u64).unwrap_or(0)
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
}
