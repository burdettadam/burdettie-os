//! Unwind frame registration abstraction.

use crate::PalError;

/// Register / deregister mapped images with the system unwinder so that
/// `@try`/`@catch`, C++ exceptions, etc. work for dynamically loaded code.
pub trait UnwindRegistry: Send + Sync {
    /// Register unwind sections for a mapped image.
    ///
    /// `text_start` / `text_end`: bounds of the __TEXT segment.
    /// `compact_unwind`: address and length of `__TEXT.__unwind_info`, if any.
    /// `eh_frame`: address and length of `__TEXT.__eh_frame`, if any.
    fn register(
        &self,
        text_start: usize,
        text_end: usize,
        compact_unwind: Option<(usize, usize)>,
        eh_frame: Option<(usize, usize)>,
    ) -> Result<(), PalError>;

    /// Deregister unwind sections for an image whose __TEXT starts at `text_start`.
    fn unregister(&self, text_start: usize) -> Result<(), PalError>;
}
