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
