//! macOS heap allocation via libc.

use platform_core::alloc::Allocator;

/// Heap allocator backed by the system's `calloc`/`malloc`/`realloc`/`free`.
pub struct DarwinAllocator;

impl Allocator for DarwinAllocator {
    fn calloc(&self, count: usize, size: usize) -> *mut u8 {
        unsafe { libc::calloc(count, size) as *mut u8 }
    }

    fn malloc(&self, size: usize) -> *mut u8 {
        unsafe { libc::malloc(size) as *mut u8 }
    }

    fn realloc(&self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        unsafe { libc::realloc(ptr as *mut libc::c_void, new_size) as *mut u8 }
    }

    fn free(&self, ptr: *mut u8) {
        unsafe { libc::free(ptr as *mut libc::c_void) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malloc_returns_nonnull() {
        let a = DarwinAllocator;
        let ptr = a.malloc(64);
        assert!(!ptr.is_null());
        a.free(ptr);
    }

    #[test]
    fn calloc_zero_initialized() {
        let a = DarwinAllocator;
        let ptr = a.calloc(1, 64);
        assert!(!ptr.is_null());
        let slice = unsafe { std::slice::from_raw_parts(ptr, 64) };
        assert!(slice.iter().all(|&b| b == 0));
        a.free(ptr);
    }

    #[test]
    fn realloc_grows() {
        let a = DarwinAllocator;
        let ptr = a.malloc(16);
        assert!(!ptr.is_null());
        unsafe { *ptr = 0xAB };
        let ptr2 = a.realloc(ptr, 128);
        assert!(!ptr2.is_null());
        assert_eq!(unsafe { *ptr2 }, 0xAB);
        a.free(ptr2);
    }

    #[test]
    fn free_valid_pointer() {
        let a = DarwinAllocator;
        let ptr = a.malloc(32);
        a.free(ptr); // should not crash
    }

    #[test]
    fn malloc_zero_no_crash() {
        let a = DarwinAllocator;
        let ptr = a.malloc(0);
        // May return null or non-null depending on platform
        if !ptr.is_null() {
            a.free(ptr);
        }
    }
}
