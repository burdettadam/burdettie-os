//! Memory model for the ARM64 interpreter.
//!
//! Provides read/write access to the mapped Mach-O segments plus a
//! dedicated interpreter stack. Reads and writes are bounds-checked
//! against the known segment ranges.

use std::collections::BTreeMap;

fn access_last_byte(addr: u64, size: usize) -> Option<u64> {
    addr.checked_add(u64::try_from(size.checked_sub(1)?).ok()?)
}

/// Darwin's mincore reports residency, including successful queries over holes.
/// Walk Mach VM regions instead, rejecting any gap before the last byte.
#[cfg(target_os = "macos")]
fn is_host_range_mapped(addr: u64, size: usize) -> bool {
    // ABI from XNU osfmk/mach/vm_region.h and mach_vm.defs. The packed basic
    // information record occupies nine integer_t slots; its fields are unused.
    const VM_REGION_BASIC_INFO_64: i32 = 9;
    const VM_REGION_BASIC_INFO_COUNT_64: u32 = 9;
    unsafe extern "C" {
        fn mach_vm_region(
            task: u32,
            address: *mut u64,
            size: *mut u64,
            flavor: i32,
            info: *mut i32,
            count: *mut u32,
            object_name: *mut u32,
        ) -> i32;
        fn mach_port_deallocate(task: u32, name: u32) -> i32;
    }

    let Some(last) = access_last_byte(addr, size) else {
        return false;
    };
    // libc deprecates its Mach bindings in favor of an additional wrapper crate.
    #[allow(deprecated)]
    let task = unsafe { libc::mach_task_self() };
    let mut cursor = addr;
    loop {
        let mut region_start = cursor;
        let mut region_size = 0;
        let mut info = [0_i32; VM_REGION_BASIC_INFO_COUNT_64 as usize];
        let mut count = VM_REGION_BASIC_INFO_COUNT_64;
        let mut object_name = 0;
        // SAFETY: All output buffers match the fixed Mach ABI. The query does
        // not dereference guest memory and only inspects this task's mappings.
        let status = unsafe {
            mach_vm_region(
                task,
                &mut region_start,
                &mut region_size,
                VM_REGION_BASIC_INFO_64,
                info.as_mut_ptr(),
                &mut count,
                &mut object_name,
            )
        };
        if object_name != 0 {
            // SAFETY: Release the send right returned by mach_vm_region.
            unsafe { mach_port_deallocate(task, object_name) };
        }
        if status != libc::KERN_SUCCESS || region_start > cursor {
            return false;
        }
        let Some(region_end) = region_start.checked_add(region_size) else {
            return false;
        };
        if region_end <= cursor {
            return false;
        }
        if last < region_end {
            return true;
        }
        cursor = region_end;
    }
}

/// Check that every host page covering an access is mapped.
///
/// This is a mapping check, not a permission or lifetime guarantee. Host
/// passthrough remains limited to trusted pointers whose access is valid.
#[cfg(all(unix, not(target_os = "macos")))]
fn is_host_range_mapped(addr: u64, size: usize) -> bool {
    static PAGE_SIZE: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    let Some(page_size) = *PAGE_SIZE.get_or_init(|| {
        // SAFETY: sysconf reads a process-wide configuration value.
        u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) })
            .ok()
            .filter(|&size| size != 0)
    }) else {
        return false;
    };
    // Reserve enough residency storage for non-Linux Unix granules. Darwin
    // uses the Mach region query above because mincore does not reject holes.
    #[cfg(not(target_os = "linux"))]
    if !(4096..=65536).contains(&page_size) || !page_size.is_power_of_two() {
        return false;
    }
    let Some(last_addr) = access_last_byte(addr, size) else {
        return false;
    };
    let mut page = addr - addr % page_size;
    let last_page = last_addr - last_addr % page_size;

    loop {
        #[cfg(target_os = "linux")]
        let mut residency = [0_u8; 1];
        #[cfg(not(target_os = "linux"))]
        let mut residency = [0_u8; 16];
        // SAFETY: The address is aligned to the actual host page size. A
        // one-byte length produces one residency byte on Linux. Other Unix
        // granules fit in the larger buffer described above.
        // A fixed 16 KiB length with a one-byte buffer overwrites the stack on
        // Linux with 4 KiB pages. mincore does not dereference the queried
        // address; an unmapped page produces an error instead.
        let ret =
            unsafe { libc::mincore(page as *mut libc::c_void, 1, residency.as_mut_ptr().cast()) };
        if ret != 0 {
            return false;
        }
        if page == last_page {
            return true;
        }
        // last_addr was checked above, and another page remains before it.
        page += page_size;
    }
}

#[cfg(windows)]
fn is_host_range_mapped(addr: u64, size: usize) -> bool {
    use windows_sys::Win32::System::Memory::{
        VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS,
    };

    let Some(end) = u64::try_from(size)
        .ok()
        .and_then(|size| addr.checked_add(size))
    else {
        return false;
    };
    let mut cursor = addr;
    while cursor < end {
        let mut info = std::mem::MaybeUninit::<MEMORY_BASIC_INFORMATION>::uninit();
        let queried = unsafe {
            VirtualQuery(
                cursor as *const std::ffi::c_void,
                info.as_mut_ptr(),
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if queried == 0 {
            return false;
        }
        let info = unsafe { info.assume_init() };
        if info.State != MEM_COMMIT || (info.Protect & (PAGE_NOACCESS | PAGE_GUARD)) != 0 {
            return false;
        }
        let Some(region_end) = (info.BaseAddress as u64).checked_add(info.RegionSize as u64) else {
            return false;
        };
        if region_end <= cursor {
            return false;
        }
        cursor = region_end;
    }
    size != 0
}

#[cfg(not(any(unix, windows)))]
fn is_host_range_mapped(_addr: u64, _size: usize) -> bool {
    false
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
/// The stack is allocated at a real virtual address so that guest and host
/// pointers are identical — no address translation is needed at FFI boundaries.
/// When `host_passthrough` is enabled, reads/writes to addresses outside
/// mapped regions are forwarded directly to host memory. This is correct
/// for in-process interpretation where guest pointers may reference
/// host-allocated objects (ObjC class structs, shim allocations, etc.).
pub struct Memory {
    /// Regions sorted by guest_start for binary search.
    regions: BTreeMap<u64, Region>,
    /// Top of the stack (highest address).
    stack_top: u64,
    /// Owned stack storage. `u128` keeps the base at least 16-byte aligned.
    _stack_storage: Box<[u128]>,
    /// When true, unmapped addresses are treated as direct host pointers.
    pub host_passthrough: bool,
}

// SAFETY: Memory owns its stack and holds raw pointers into MappedImage
// segments that outlive it (the caller ensures this).
unsafe impl Send for Memory {}

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

        // Allocate the interpreter stack at a real host virtual address.
        // guest_start == base as u64, so guest pointers ARE host pointers.
        let stack_words = DEFAULT_STACK_SIZE / std::mem::size_of::<u128>();
        let mut stack_storage = vec![0_u128; stack_words].into_boxed_slice();
        let stack_ptr = stack_storage.as_mut_ptr().cast::<u8>();
        let stack_size = stack_storage.len() * std::mem::size_of::<u128>();
        let stack_guest_start = stack_ptr as u64;
        let stack_top = stack_guest_start + stack_size as u64;
        regions.insert(
            stack_guest_start,
            Region {
                base: stack_ptr,
                guest_start: stack_guest_start,
                size: stack_size,
                writable: true,
            },
        );

        Self {
            regions,
            stack_top,
            _stack_storage: stack_storage,
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
        access_last_byte(addr, size)?;
        if let Some((&region_start, region)) = self.regions.range(..=addr).next_back() {
            let offset = usize::try_from(addr - region_start).ok()?;
            if offset <= region.size && size <= region.size - offset {
                return Some(unsafe { region.base.add(offset) as *const u8 });
            }
        }
        None
    }

    /// Host fallback may only cover addresses outside all guest regions.
    fn permits_host_access(&self, addr: u64, size: usize) -> bool {
        if !self.host_passthrough || addr < 0x1000 {
            return false;
        }
        let Some(last) = access_last_byte(addr, size) else {
            return false;
        };
        if usize::try_from(addr).is_err() || usize::try_from(last).is_err() {
            return false;
        }
        // The caller already checked a region containing the starting address.
        // Also reject accesses that begin outside but cross into a later guest
        // region, so passthrough cannot bypass that region's protections.
        if self.regions.range(addr..=last).next().is_some() {
            return false;
        }
        is_host_range_mapped(addr, size)
    }

    /// Translate a guest address to a host pointer for reading.
    fn translate(&self, addr: u64, size: usize) -> Option<*const u8> {
        access_last_byte(addr, size)?;
        // Find the region containing addr (the largest key <= addr).
        if let Some((&region_start, region)) = self.regions.range(..=addr).next_back() {
            let offset = usize::try_from(addr - region_start).ok()?;
            if offset < region.size {
                return (size <= region.size - offset)
                    .then(|| unsafe { region.base.add(offset) as *const u8 });
            }
        }
        // Host passthrough: treat the address as a direct host pointer,
        // but only if the page is actually mapped (to avoid SIGSEGV).
        if self.permits_host_access(addr, size) {
            return Some(addr as *const u8);
        }
        None
    }

    /// Translate a guest address to a host pointer for writing.
    fn translate_mut(&self, addr: u64, size: usize) -> Option<*mut u8> {
        access_last_byte(addr, size)?;
        if let Some((&region_start, region)) = self.regions.range(..=addr).next_back() {
            let offset = usize::try_from(addr - region_start).ok()?;
            if offset < region.size {
                // Host passthrough must not bypass an image's protection or
                // allow an access to run past the registered region boundary.
                return (region.writable && size <= region.size - offset)
                    .then(|| unsafe { region.base.add(offset) });
            }
        }
        // Host passthrough: treat the address as a direct host pointer,
        // but only if the page is actually mapped (to avoid SIGSEGV).
        if self.permits_host_access(addr, size) {
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
        Some(u16::from_le_bytes(unsafe { *(ptr as *const [u8; 2]) }))
    }

    /// Read a u32 from mapped regions only (no host passthrough).
    /// Used for instruction fetch — the interpreter must not execute
    /// bytes from host memory.
    pub fn read_u32_mapped(&self, addr: u64) -> Option<u32> {
        let ptr = self.translate_strict(addr, 4)?;
        Some(u32::from_le_bytes(unsafe { *(ptr as *const [u8; 4]) }))
    }

    /// Read a u32 (little-endian) from guest memory.
    pub fn read_u32(&self, addr: u64) -> Option<u32> {
        let ptr = self.translate(addr, 4)?;
        Some(u32::from_le_bytes(unsafe { *(ptr as *const [u8; 4]) }))
    }

    /// Read a u64 (little-endian) from guest memory.
    pub fn read_u64(&self, addr: u64) -> Option<u64> {
        let ptr = self.translate(addr, 8)?;
        Some(u64::from_le_bytes(unsafe { *(ptr as *const [u8; 8]) }))
    }

    /// Read a u128 (little-endian) from guest memory (for Q register loads).
    pub fn read_u128(&self, addr: u64) -> Option<u128> {
        let ptr = self.translate(addr, 16)?;
        Some(u128::from_le_bytes(unsafe { *(ptr as *const [u8; 16]) }))
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
            addr = addr.checked_add(1)?;
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

    /// Translate a naturally aligned guest access and check the host atomic
    /// alignment too: a registered segment need not use identity mapping.
    fn translate_atomic<T>(&self, addr: u64, writable: bool) -> Option<*const T> {
        let size = std::mem::size_of::<T>();
        if !addr.is_multiple_of(size as u64) {
            return None;
        }
        let ptr = if writable {
            self.translate_mut(addr, size)? as *const u8
        } else {
            self.translate(addr, size)?
        }
        .cast::<T>();
        ptr.is_aligned().then_some(ptr)
    }

    /// Atomic load of a u32 with Acquire ordering.
    /// For LDAR/LDAXR emulation. Identity-mapped addresses allow direct
    /// use of `std::sync::atomic::AtomicU32`.
    pub fn atomic_read_u32(&self, addr: u64) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_atomic::<AtomicU32>(addr, false)?;
        // SAFETY: translation checks the complete readable range and host
        // alignment before a reference is formed. The caller of from_segments
        // is responsible for the backing storage's lifetime and access safety.
        let atomic = unsafe { &*ptr };
        Some(atomic.load(Ordering::Acquire))
    }

    /// Atomic load of a u64 with Acquire ordering.
    pub fn atomic_read_u64(&self, addr: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_atomic::<AtomicU64>(addr, false)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.load(Ordering::Acquire))
    }

    /// Atomic store of a u32 with Release ordering.
    /// For STLR/STLXR emulation.
    pub fn atomic_write_u32(&self, addr: u64, val: u32) -> bool {
        use std::sync::atomic::{AtomicU32, Ordering};
        if let Some(ptr) = self.translate_atomic::<AtomicU32>(addr, true) {
            let atomic = unsafe { &*ptr };
            atomic.store(val, Ordering::Release);
            true
        } else {
            false
        }
    }

    /// Atomic store of a u64 with Release ordering.
    pub fn atomic_write_u64(&self, addr: u64, val: u64) -> bool {
        use std::sync::atomic::{AtomicU64, Ordering};
        if let Some(ptr) = self.translate_atomic::<AtomicU64>(addr, true) {
            let atomic = unsafe { &*ptr };
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
        let ptr = self.translate_atomic::<AtomicU32>(addr, true)?;
        let atomic = unsafe { &*ptr };
        match atomic.compare_exchange(expected, new_val, Ordering::AcqRel, Ordering::Acquire) {
            Ok(old) => Some(old),
            Err(old) => Some(old),
        }
    }

    /// Atomic compare-and-swap (u64). Returns the old value.
    pub fn atomic_cas_u64(&self, addr: u64, expected: u64, new_val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_atomic::<AtomicU64>(addr, true)?;
        let atomic = unsafe { &*ptr };
        match atomic.compare_exchange(expected, new_val, Ordering::AcqRel, Ordering::Acquire) {
            Ok(old) => Some(old),
            Err(old) => Some(old),
        }
    }

    /// Atomic fetch-and-add (u32). Returns the old value.
    pub fn atomic_fetch_add_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_atomic::<AtomicU32>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_add(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-add (u64). Returns the old value.
    pub fn atomic_fetch_add_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_atomic::<AtomicU64>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_add(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-or (u32). Returns the old value.
    pub fn atomic_fetch_or_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_atomic::<AtomicU32>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_or(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-or (u64). Returns the old value.
    pub fn atomic_fetch_or_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_atomic::<AtomicU64>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_or(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-and (u32). Returns the old value.
    pub fn atomic_fetch_and_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_atomic::<AtomicU32>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_and(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-and (u64). Returns the old value.
    pub fn atomic_fetch_and_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_atomic::<AtomicU64>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_and(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-xor (u32). Returns the old value.
    pub fn atomic_fetch_xor_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_atomic::<AtomicU32>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_xor(val, Ordering::AcqRel))
    }

    /// Atomic fetch-and-xor (u64). Returns the old value.
    pub fn atomic_fetch_xor_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_atomic::<AtomicU64>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_xor(val, Ordering::AcqRel))
    }

    /// Atomic swap (u32). Returns the old value.
    pub fn atomic_swap_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_atomic::<AtomicU32>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.swap(val, Ordering::AcqRel))
    }

    /// Atomic swap (u64). Returns the old value.
    pub fn atomic_swap_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_atomic::<AtomicU64>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.swap(val, Ordering::AcqRel))
    }

    /// Atomic fetch-min (signed, u64). Returns the old value.
    pub fn atomic_fetch_smin_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicI64, Ordering};
        let ptr = self.translate_atomic::<AtomicI64>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_min(val as i64, Ordering::AcqRel) as u64)
    }

    /// Atomic fetch-max (signed, u64). Returns the old value.
    pub fn atomic_fetch_smax_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicI64, Ordering};
        let ptr = self.translate_atomic::<AtomicI64>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_max(val as i64, Ordering::AcqRel) as u64)
    }

    /// Atomic fetch-min (unsigned, u64). Returns the old value.
    pub fn atomic_fetch_umin_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_atomic::<AtomicU64>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_min(val, Ordering::AcqRel))
    }

    /// Atomic fetch-max (unsigned, u64). Returns the old value.
    pub fn atomic_fetch_umax_u64(&self, addr: u64, val: u64) -> Option<u64> {
        use std::sync::atomic::{AtomicU64, Ordering};
        let ptr = self.translate_atomic::<AtomicU64>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_max(val, Ordering::AcqRel))
    }

    /// Atomic fetch-min (signed, u32). Returns the old value.
    pub fn atomic_fetch_smin_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicI32, Ordering};
        let ptr = self.translate_atomic::<AtomicI32>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_min(val as i32, Ordering::AcqRel) as u32)
    }

    /// Atomic fetch-max (signed, u32). Returns the old value.
    pub fn atomic_fetch_smax_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicI32, Ordering};
        let ptr = self.translate_atomic::<AtomicI32>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_max(val as i32, Ordering::AcqRel) as u32)
    }

    /// Atomic fetch-min (unsigned, u32). Returns the old value.
    pub fn atomic_fetch_umin_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_atomic::<AtomicU32>(addr, true)?;
        let atomic = unsafe { &*ptr };
        Some(atomic.fetch_min(val, Ordering::AcqRel))
    }

    /// Atomic fetch-max (unsigned, u32). Returns the old value.
    pub fn atomic_fetch_umax_u32(&self, addr: u64, val: u32) -> Option<u32> {
        use std::sync::atomic::{AtomicU32, Ordering};
        let ptr = self.translate_atomic::<AtomicU32>(addr, true)?;
        let atomic = unsafe { &*ptr };
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
    use crate::test_helpers::*;

    #[test]
    fn atomic_regression_rejects_misaligned_guest_or_host_storage() {
        let mut storage = [0x1122_3344_5566_7788_u64; 4];
        let aligned = storage.as_mut_ptr().cast::<u8>();
        for (guest, offset) in [(0x2001, 0), (0x2000, 1), (0x2001, 1)] {
            // SAFETY: storage remains alive and each 16-byte region is within
            // its allocation. Misalignment is deliberately checked by Memory.
            let mem =
                unsafe { super::Memory::from_segments(&[(guest, aligned.add(offset), 16, true)]) };
            assert_eq!(mem.atomic_read_u32(guest), None);
            assert_eq!(mem.atomic_read_u64(guest), None);
            assert!(!mem.atomic_write_u32(guest, 1));
            assert!(!mem.atomic_write_u64(guest, 1));
            assert_eq!(mem.atomic_cas_u32(guest, 0, 1), None);
            assert_eq!(mem.atomic_cas_u64(guest, 0, 1), None);
            assert_eq!(mem.atomic_fetch_add_u32(guest, 1), None);
            assert_eq!(mem.atomic_fetch_add_u64(guest, 1), None);
            assert_eq!(mem.atomic_fetch_or_u32(guest, 1), None);
            assert_eq!(mem.atomic_fetch_or_u64(guest, 1), None);
            assert_eq!(mem.atomic_fetch_and_u32(guest, 1), None);
            assert_eq!(mem.atomic_fetch_and_u64(guest, 1), None);
            assert_eq!(mem.atomic_fetch_xor_u32(guest, 1), None);
            assert_eq!(mem.atomic_fetch_xor_u64(guest, 1), None);
            assert_eq!(mem.atomic_swap_u32(guest, 1), None);
            assert_eq!(mem.atomic_swap_u64(guest, 1), None);
            assert_eq!(mem.atomic_fetch_smin_u32(guest, 1), None);
            assert_eq!(mem.atomic_fetch_smin_u64(guest, 1), None);
            assert_eq!(mem.atomic_fetch_smax_u32(guest, 1), None);
            assert_eq!(mem.atomic_fetch_smax_u64(guest, 1), None);
            assert_eq!(mem.atomic_fetch_umin_u32(guest, 1), None);
            assert_eq!(mem.atomic_fetch_umin_u64(guest, 1), None);
            assert_eq!(mem.atomic_fetch_umax_u32(guest, 1), None);
            assert_eq!(mem.atomic_fetch_umax_u64(guest, 1), None);
            assert_eq!(storage, [0x1122_3344_5566_7788; 4]);
        }
    }

    #[test]
    fn atomic_regression_aligned_nonidentity_mapping_works() {
        let mut storage = 5_u64;
        let ptr = std::ptr::from_mut(&mut storage).cast::<u8>();
        // SAFETY: the aligned, writable storage outlives this mapping.
        let mem = unsafe { super::Memory::from_segments(&[(0x2000, ptr, 8, true)]) };
        assert_eq!(mem.atomic_fetch_add_u64(0x2000, 7), Some(5));
        assert_eq!(mem.atomic_read_u64(0x2000), Some(12));
        assert_eq!(mem.atomic_swap_u32(0x2000, 9), Some(12));
        assert_eq!(mem.atomic_read_u32(0x2000), Some(9));
    }

    #[test]
    fn atomic_regression_signed_minmax_u64() {
        for (is_min, original, source, expected) in [
            (true, 5, (-3_i64) as u64, (-3_i64) as u64),
            (false, (-5_i64) as u64, 3, 3),
            (true, i64::MAX as u64, i64::MIN as u64, i64::MIN as u64),
            (false, i64::MIN as u64, i64::MAX as u64, i64::MAX as u64),
        ] {
            let t = test_interp(&[]);
            let mem = &t.interp.mem;
            assert!(mem.write_u64(t.data_base, original));
            let previous = if is_min {
                mem.atomic_fetch_smin_u64(t.data_base, source)
            } else {
                mem.atomic_fetch_smax_u64(t.data_base, source)
            };
            assert_eq!(previous, Some(original));
            assert_eq!(mem.read_u64(t.data_base), Some(expected));
        }
    }

    #[cfg(unix)]
    #[test]
    fn host_passthrough_reads_and_writes_live_storage() {
        let mut storage = Box::new([0xAAAA_u64, 0x1234, 0xBBBB]);
        let addr = unsafe { storage.as_mut_ptr().add(1) } as u64;
        // SAFETY: No external image regions are supplied; Memory owns its stack.
        let mut mem = unsafe { super::Memory::from_segments(&[]) };
        mem.host_passthrough = true;
        assert_eq!(mem.read_u64(addr), Some(0x1234));
        assert!(mem.write_u64(addr, 0x5678));
        assert_eq!(mem.read_u64(addr), Some(0x5678));
        assert_eq!(*storage, [0xAAAA, 0x5678, 0xBBBB]);
    }

    #[cfg(unix)]
    #[test]
    fn host_passthrough_checks_every_page_of_an_access() {
        // Allocate the interpreter stack before creating a hole so this test's
        // own allocator cannot immediately reuse the unmapped address range.
        let mut mem = unsafe { super::Memory::from_segments(&[]) };
        mem.host_passthrough = true;
        // Leave the second page unmapped so a read crossing the boundary must
        // be rejected before a raw pointer is dereferenced.
        let page_size =
            usize::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).expect("host page size");
        assert!(page_size >= 8);
        let allocation = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                page_size * 2,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        assert_ne!(allocation, libc::MAP_FAILED);
        let first_page = allocation.cast::<u8>();
        assert_eq!(
            unsafe { libc::munmap(first_page.add(page_size).cast(), page_size) },
            0
        );
        // SAFETY: The first page remains mapped and writable.
        unsafe { first_page.add(page_size - 4).cast::<u32>().write(0x1234) };
        let addr = first_page as u64 + (page_size - 4) as u64;
        assert_eq!(mem.read_u32(addr), Some(0x1234));
        assert_eq!(mem.read_u64(addr), None);
        assert!(!mem.write_u64(addr, 0));
        assert_eq!(unsafe { libc::munmap(allocation, page_size) }, 0);
    }

    #[cfg(unix)]
    #[test]
    fn host_page_probe_rejects_overflowing_or_empty_ranges() {
        assert!(!super::is_host_range_mapped(u64::MAX - 3, 8));
        assert!(!super::is_host_range_mapped(0x1000, 0));
        let mut mem = unsafe { super::Memory::from_segments(&[]) };
        mem.host_passthrough = true;
        assert_eq!(mem.read_u64(u64::MAX - 3), None);
        assert!(!mem.write_u64(u64::MAX - 3, 0));
    }

    #[test]
    fn host_passthrough_preserves_registered_region_bounds_and_protection() {
        let mut storage = [0x1234_u64, 0x5678, 0x9ABC];
        let base = storage.as_mut_ptr().cast::<u8>();
        let addr = base as u64 + 8;
        // Only the middle word belongs to this read-only guest region, even
        // though the host allocation makes both neighboring words accessible.
        let mut mem = unsafe { super::Memory::from_segments(&[(addr, base.add(8), 8, false)]) };
        mem.host_passthrough = true;
        assert_eq!(mem.read_u64(addr), Some(0x5678));
        assert!(!mem.write_u64(addr, 0));
        assert_eq!(mem.read_u64(addr - 4), None);
        assert!(!mem.write_u64(addr - 4, 0));
        assert_eq!(mem.read_u64(addr + 4), None);
        assert!(!mem.write_u64(addr + 4, 0));
        assert_eq!(storage, [0x1234, 0x5678, 0x9ABC]);
    }

    #[test]
    fn registered_region_rejects_guest_address_wraparound() {
        let mut storage = [0x1234_u64];
        let base = storage.as_mut_ptr().cast::<u8>();
        let addr = u64::MAX - 3;
        // SAFETY: The host allocation remains live and has eight bytes.
        let mem = unsafe { super::Memory::from_segments(&[(addr, base, 8, true)]) };
        assert_eq!(mem.read_u64(addr), None);
        assert!(!mem.write_u64(addr, 0));
        assert_eq!(mem.read_u32_mapped(u64::MAX - 1), None);
        assert_eq!(storage, [0x1234]);
    }

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
    fn read_cstring_at_guest_address_limit_does_not_wrap() {
        for (mut storage, expected) in [([b'a', b'b'], None), ([b'a', 0], Some("a"))] {
            // SAFETY: these two readable bytes remain alive for the mapping.
            let mem = unsafe {
                super::Memory::from_segments(&[(u64::MAX - 1, storage.as_mut_ptr(), 2, false)])
            };
            assert_eq!(mem.read_cstring(u64::MAX - 1).as_deref(), expected);
        }
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
