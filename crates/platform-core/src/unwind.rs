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

/// No-op unwind registry for use in tests and non-macOS platforms.
pub struct NoopUnwindRegistry;

impl UnwindRegistry for NoopUnwindRegistry {
    fn register(
        &self,
        _text_start: usize,
        _text_end: usize,
        _compact_unwind: Option<(usize, usize)>,
        _eh_frame: Option<(usize, usize)>,
    ) -> Result<(), PalError> {
        Ok(())
    }

    fn unregister(&self, _text_start: usize) -> Result<(), PalError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_register_returns_ok() {
        let reg = NoopUnwindRegistry;
        assert!(reg.register(0x1000, 0x2000, None, None).is_ok());
    }

    #[test]
    fn noop_unregister_returns_ok() {
        let reg = NoopUnwindRegistry;
        assert!(reg.unregister(0x1000).is_ok());
    }

    #[test]
    fn noop_register_multiple_times() {
        let reg = NoopUnwindRegistry;
        for i in 0..5 {
            assert!(reg
                .register(i * 0x1000, (i + 1) * 0x1000, None, None)
                .is_ok());
        }
    }

    #[test]
    fn noop_unregister_without_register() {
        let reg = NoopUnwindRegistry;
        assert!(reg.unregister(0xDEAD).is_ok());
    }

    #[test]
    fn noop_register_with_all_sections() {
        let reg = NoopUnwindRegistry;
        let result = reg.register(0x1000, 0x5000, Some((0x2000, 0x100)), Some((0x3000, 0x200)));
        assert!(result.is_ok());
    }
}
