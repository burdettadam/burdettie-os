//! Memory mapping abstraction.

use crate::{PalError, Protection};

/// Platform-independent memory mapping operations.
pub trait MemoryMapper: Send + Sync {
    /// Return the system page size in bytes.
    fn page_size(&self) -> usize;

    /// Allocate an anonymous read-write memory region of at least `size` bytes.
    ///
    /// The region should be zero-filled and page-aligned.
    fn map_anonymous(&self, size: usize) -> Result<*mut u8, PalError>;

    /// Change memory protection on a region.
    fn protect(&self, addr: *mut u8, size: usize, prot: Protection) -> Result<(), PalError>;

    /// Unmap a previously mapped region.
    ///
    /// # Safety
    /// `addr` must be a pointer returned by `map_anonymous` and `size` must
    /// match the original allocation size.
    fn unmap(&self, addr: *mut u8, size: usize) -> Result<(), PalError>;
}
