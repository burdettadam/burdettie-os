//! libsystem_malloc zone API shims.
//!
//! On macOS, zone calls resolve via dlsym to real malloc. On Linux these
//! symbols don't exist, so we provide functional wrappers around standard
//! malloc/free.

use crate::table::ShimTable;

/// Sentinel byte used as the single default malloc zone pointer.
static DEFAULT_ZONE: u8 = 0xAA;

/// Returns the address of our static sentinel as the "default zone".
#[inline]
fn default_zone_ptr() -> u64 {
    &DEFAULT_ZONE as *const u8 as u64
}

pub struct MallocZoneStubs;

impl MallocZoneStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_malloc_default_zone", shim_malloc_default_zone as *const () as u64, "malloc_default_zone → default zone pointer"),
            ("_malloc_create_zone", shim_malloc_create_zone as *const () as u64, "malloc_create_zone → default zone pointer"),
            ("_malloc_destroy_zone", shim_malloc_destroy_zone as *const () as u64, "malloc_destroy_zone (no-op)"),
            ("_malloc_zone_malloc", shim_malloc_zone_malloc as *const () as u64, "malloc_zone_malloc → libc::malloc"),
            ("_malloc_zone_calloc", shim_malloc_zone_calloc as *const () as u64, "malloc_zone_calloc → libc::calloc"),
            ("_malloc_zone_realloc", shim_malloc_zone_realloc as *const () as u64, "malloc_zone_realloc → libc::realloc"),
            ("_malloc_zone_free", shim_malloc_zone_free as *const () as u64, "malloc_zone_free → libc::free"),
            ("_malloc_zone_memalign", shim_malloc_zone_memalign as *const () as u64, "malloc_zone_memalign → libc::posix_memalign"),
            ("_malloc_size", shim_malloc_size as *const () as u64, "malloc_size → 0 (portable fallback)"),
            ("_malloc_good_size", shim_malloc_good_size as *const () as u64, "malloc_good_size → next 16-byte alignment"),
            ("_malloc_zone_statistics", shim_malloc_zone_statistics as *const () as u64, "malloc_zone_statistics → zeroed stats"),
            ("_malloc_zone_batch_malloc", shim_malloc_zone_batch_malloc as *const () as u64, "malloc_zone_batch_malloc → individual mallocs"),
            ("_malloc_zone_batch_free", shim_malloc_zone_batch_free as *const () as u64, "malloc_zone_batch_free → individual frees"),
            ("_malloc_set_zone_name", shim_malloc_set_zone_name as *const () as u64, "malloc_set_zone_name (no-op)"),
            ("_malloc_zone_from_ptr", shim_malloc_zone_from_ptr as *const () as u64, "malloc_zone_from_ptr → default zone")
        );
        log::debug!("MallocZoneStubs: registered 15 stubs");
    }
}

/// Returns a fixed non-null pointer representing the default malloc zone.
pub extern "C" fn shim_malloc_default_zone() -> u64 {
    default_zone_ptr()
}

/// Creates a "new" zone — we return the same default zone (single-zone model).
pub extern "C" fn shim_malloc_create_zone(_vm_size: u64, _flags: u32) -> u64 {
    default_zone_ptr()
}

/// Destroys a zone — no-op (we use a single static zone).
pub extern "C" fn shim_malloc_destroy_zone(_zone: u64) {}

/// Allocates `size` bytes via standard malloc, ignoring the zone.
pub extern "C" fn shim_malloc_zone_malloc(_zone: u64, size: u64) -> u64 {
    unsafe { libc::malloc(size as libc::size_t) as u64 }
}

/// Allocates zeroed memory for `count * size` bytes via standard calloc.
pub extern "C" fn shim_malloc_zone_calloc(_zone: u64, count: u64, size: u64) -> u64 {
    unsafe { libc::calloc(count as libc::size_t, size as libc::size_t) as u64 }
}

/// Reallocates `ptr` to `size` bytes via standard realloc.
pub extern "C" fn shim_malloc_zone_realloc(_zone: u64, ptr: u64, size: u64) -> u64 {
    unsafe { libc::realloc(ptr as *mut libc::c_void, size as libc::size_t) as u64 }
}

/// Frees `ptr` via standard free, ignoring the zone.
pub extern "C" fn shim_malloc_zone_free(_zone: u64, ptr: u64) {
    unsafe { libc::free(ptr as *mut libc::c_void) }
}

/// Allocates `size` bytes with the given alignment via posix_memalign.
pub extern "C" fn shim_malloc_zone_memalign(_zone: u64, alignment: u64, size: u64) -> u64 {
    let mut out: *mut libc::c_void = core::ptr::null_mut();
    let ret = unsafe {
        libc::posix_memalign(&mut out, alignment as libc::size_t, size as libc::size_t)
    };
    if ret == 0 {
        out as u64
    } else {
        0
    }
}

/// Returns 0 — can't query allocation size portably; 0 is valid per spec.
pub extern "C" fn shim_malloc_size(_ptr: u64) -> u64 {
    0
}

/// Rounds `size` up to the next 16-byte alignment boundary.
pub extern "C" fn shim_malloc_good_size(size: u64) -> u64 {
    (size.wrapping_add(15)) & !15
}

/// Zeroes the stats struct. The macOS `malloc_statistics_t` is 32 bytes.
pub extern "C" fn shim_malloc_zone_statistics(_zone: u64, stats: u64) {
    if stats != 0 {
        unsafe {
            core::ptr::write_bytes(stats as *mut u8, 0, 32);
        }
    }
}

/// Allocates `count` individual blocks of `size` bytes, writes pointers into
/// `results`, and returns the number actually allocated.
pub extern "C" fn shim_malloc_zone_batch_malloc(
    _zone: u64,
    size: u64,
    results: u64,
    count: u32,
) -> u32 {
    if results == 0 || count == 0 {
        return 0;
    }
    let out = results as *mut u64;
    let mut allocated = 0u32;
    for i in 0..count {
        let ptr = unsafe { libc::malloc(size as libc::size_t) };
        if ptr.is_null() {
            break;
        }
        unsafe { *out.add(i as usize) = ptr as u64 };
        allocated += 1;
    }
    allocated
}

/// Frees `count` pointers from the `ptrs` array.
pub extern "C" fn shim_malloc_zone_batch_free(_zone: u64, ptrs: u64, count: u32) {
    if ptrs == 0 {
        return;
    }
    let arr = ptrs as *const u64;
    for i in 0..count {
        let ptr = unsafe { *arr.add(i as usize) };
        if ptr != 0 {
            unsafe { libc::free(ptr as *mut libc::c_void) };
        }
    }
}

/// Sets the zone name — no-op.
pub extern "C" fn shim_malloc_set_zone_name(_zone: u64, _name: u64) {}

/// Returns the zone that `ptr` was allocated from — always the default zone.
pub extern "C" fn shim_malloc_zone_from_ptr(_ptr: u64) -> u64 {
    default_zone_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::ShimTable;

    #[test]
    fn test_default_zone_non_null() {
        let zone = shim_malloc_default_zone();
        assert_ne!(zone, 0, "default zone must be non-null");
    }

    #[test]
    fn test_zone_malloc_free() {
        let zone = shim_malloc_default_zone();
        let ptr = shim_malloc_zone_malloc(zone, 64);
        assert_ne!(ptr, 0);
        // Write to the allocation to verify it's usable.
        unsafe { *(ptr as *mut u8) = 0xAB };
        assert_eq!(unsafe { *(ptr as *const u8) }, 0xAB);
        shim_malloc_zone_free(zone, ptr);
    }

    #[test]
    fn test_zone_calloc_zeroed() {
        let zone = shim_malloc_default_zone();
        let ptr = shim_malloc_zone_calloc(zone, 4, 32);
        assert_ne!(ptr, 0);
        // calloc must return zeroed memory.
        let slice = unsafe { core::slice::from_raw_parts(ptr as *const u8, 128) };
        assert!(slice.iter().all(|&b| b == 0), "calloc memory must be zeroed");
        shim_malloc_zone_free(zone, ptr);
    }

    #[test]
    fn test_zone_realloc() {
        let zone = shim_malloc_default_zone();
        let ptr = shim_malloc_zone_malloc(zone, 16);
        assert_ne!(ptr, 0);
        // Write a marker byte.
        unsafe { *(ptr as *mut u8) = 0xCD };
        // Realloc to a larger size — data must be preserved.
        let ptr2 = shim_malloc_zone_realloc(zone, ptr, 256);
        assert_ne!(ptr2, 0);
        assert_eq!(unsafe { *(ptr2 as *const u8) }, 0xCD);
        shim_malloc_zone_free(zone, ptr2);
    }

    #[test]
    fn test_good_size_rounds_up() {
        assert!(shim_malloc_good_size(1) >= 16);
        assert!(shim_malloc_good_size(16) >= 16);
        assert!(shim_malloc_good_size(17) >= 32);
        assert_eq!(shim_malloc_good_size(0), 0);
        assert_eq!(shim_malloc_good_size(32), 32);
    }

    #[test]
    fn test_batch_malloc_free() {
        let zone = shim_malloc_default_zone();
        const N: u32 = 8;
        let mut results = [0u64; N as usize];
        let allocated = shim_malloc_zone_batch_malloc(
            zone,
            64,
            results.as_mut_ptr() as u64,
            N,
        );
        assert_eq!(allocated, N);
        for &ptr in &results {
            assert_ne!(ptr, 0, "each batch-allocated pointer must be non-null");
        }
        shim_malloc_zone_batch_free(zone, results.as_ptr() as u64, N);
    }

    #[test]
    fn malloc_zone_stubs_registered() {
        let mut table = ShimTable::new();
        MallocZoneStubs::install(&mut table);
        assert!(table.resolve("_malloc_default_zone").is_some());
        assert!(table.resolve("_malloc_zone_malloc").is_some());
        assert!(table.resolve("_malloc_good_size").is_some());
        assert!(table.resolve("_malloc_zone_batch_malloc").is_some());
        assert!(table.resolve("_malloc_set_zone_name").is_some());
    }
}
