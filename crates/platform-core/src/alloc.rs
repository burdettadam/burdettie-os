//! Heap allocation abstraction.

/// Platform heap allocator for ObjC/Foundation object memory.
///
/// All methods mirror the C standard library semantics:
/// - `calloc`: allocate zero-initialized memory
/// - `malloc`: allocate uninitialized memory
/// - `realloc`: resize a previous allocation
/// - `free`: release a previous allocation
///
/// # Safety
///
/// Pointers returned by `calloc`/`malloc`/`realloc` must only be freed
/// by `free` on the same allocator.  All pointer arguments to `realloc`
/// and `free` must have been previously returned by this allocator.
pub trait Allocator: Send + Sync {
    /// Allocate `count * size` bytes, zero-initialized.
    /// Returns null on failure.
    fn calloc(&self, count: usize, size: usize) -> *mut u8;

    /// Allocate `size` bytes (contents undefined).
    /// Returns null on failure.
    fn malloc(&self, size: usize) -> *mut u8;

    /// Resize a previous allocation to `new_size` bytes.
    ///
    /// `ptr` must have been returned by a prior `malloc`, `calloc`, or
    /// `realloc` on this allocator (or be null, in which case this
    /// behaves like `malloc`).
    /// Returns null on failure (the original allocation is unchanged).
    fn realloc(&self, ptr: *mut u8, new_size: usize) -> *mut u8;

    /// Free a previous allocation.
    ///
    /// `ptr` must have been returned by a prior `malloc`, `calloc`, or
    /// `realloc` on this allocator (or be null, which is a no-op).
    fn free(&self, ptr: *mut u8);
}
