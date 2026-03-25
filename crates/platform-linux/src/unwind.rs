//! Linux unwind registration via `__register_frame` / `__deregister_frame`.
//!
//! On macOS, `__unw_add_find_dynamic_unwind_sections` provides a callback-based
//! API that the system unwinder invokes lazily.  Linux (libgcc / LLVM libunwind)
//! instead requires explicit `.eh_frame` registration.
//!
//! If a mapped Mach-O only contains `__unwind_info` (compact unwind) and no
//! `__eh_frame`, we convert it on the fly via [`macho_exec::compact_to_ehframe`].

use platform_core::unwind::UnwindRegistry;
use platform_core::PalError;
use std::collections::HashMap;
use std::sync::Mutex;

/// Per-image book-keeping for generated `.eh_frame` buffers.
struct RegisteredImage {
    /// Heap-allocated `.eh_frame` bytes.  Must stay alive while registered.
    _eh_frame_buf: Vec<u8>,
    /// Pointer that was passed to `__register_frame`.
    eh_frame_ptr: *const u8,
}

// SAFETY: the pointer is to a heap allocation we own, and only accessed
// under the mutex.
unsafe impl Send for RegisteredImage {}
unsafe impl Sync for RegisteredImage {}

/// Unwind registry for Linux using `__register_frame` / `__deregister_frame`.
pub struct LinuxUnwindRegistry {
    /// Map from `text_start` → registered image data.
    images: Mutex<HashMap<usize, RegisteredImage>>,
}

impl LinuxUnwindRegistry {
    pub fn new() -> Self {
        Self {
            images: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for LinuxUnwindRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl UnwindRegistry for LinuxUnwindRegistry {
    fn register(
        &self,
        text_start: usize,
        _text_end: usize,
        compact_unwind: Option<(usize, usize)>,
        eh_frame: Option<(usize, usize)>,
    ) -> Result<(), PalError> {
        // Prefer native __eh_frame if available; otherwise convert from compact.
        let buf: Vec<u8> = if let Some((addr, len)) = eh_frame {
            // Copy the existing .eh_frame bytes so we own them.
            let slice = unsafe { std::slice::from_raw_parts(addr as *const u8, len) };
            slice.to_vec()
        } else if let Some((addr, len)) = compact_unwind {
            // Convert compact unwind → .eh_frame.
            let cu_slice = unsafe { std::slice::from_raw_parts(addr as *const u8, len) };
            let entries = macho_exec::compact_to_ehframe::parse_compact_unwind(cu_slice);
            if entries.is_empty() {
                return Ok(());
            }
            macho_exec::compact_to_ehframe::generate_eh_frame(&entries, text_start)
        } else {
            return Ok(());
        };

        if buf.is_empty() {
            return Ok(());
        }

        let ptr = buf.as_ptr();
        register_frame(ptr);

        log::debug!(
            "LinuxUnwindRegistry: registered eh_frame at {:p} ({} bytes) for text {:#x}",
            ptr,
            buf.len(),
            text_start,
        );

        let mut guard = self.images.lock().unwrap_or_else(|e| e.into_inner());
        guard.insert(
            text_start,
            RegisteredImage {
                _eh_frame_buf: buf,
                eh_frame_ptr: ptr,
            },
        );

        Ok(())
    }

    fn unregister(&self, text_start: usize) -> Result<(), PalError> {
        let mut guard = self.images.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(img) = guard.remove(&text_start) {
            deregister_frame(img.eh_frame_ptr);
            log::debug!(
                "LinuxUnwindRegistry: deregistered eh_frame for text {:#x}",
                text_start,
            );
        }
        Ok(())
    }
}

// ── Platform-gated FFI ─────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn register_frame(eh_frame: *const u8) {
    extern "C" {
        fn __register_frame(begin: *const u8);
    }
    unsafe {
        __register_frame(eh_frame);
    }
}

#[cfg(target_os = "linux")]
fn deregister_frame(eh_frame: *const u8) {
    extern "C" {
        fn __deregister_frame(begin: *const u8);
    }
    unsafe {
        __deregister_frame(eh_frame);
    }
}

// On non-Linux (e.g. when compiling on macOS for development), no-op.
#[cfg(not(target_os = "linux"))]
fn register_frame(_eh_frame: *const u8) {}

#[cfg(not(target_os = "linux"))]
fn deregister_frame(_eh_frame: *const u8) {}
