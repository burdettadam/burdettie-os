//! macOS unwind registration via libunwind.

use platform_core::unwind::UnwindRegistry;
use platform_core::PalError;
use std::sync::Mutex;

/// Per-image unwind info stored for the libunwind callback.
struct UnwindImageInfo {
    text_start: usize,
    text_end: usize,
    compact_unwind: Option<(usize, usize)>,
    eh_frame: Option<(usize, usize)>,
}

static UNWIND_IMAGES: Mutex<Vec<UnwindImageInfo>> = Mutex::new(Vec::new());
static CALLBACK_REGISTERED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[repr(C)]
struct UnwDynamicUnwindSections {
    dso_base: usize,
    dwarf_section: usize,
    dwarf_section_length: usize,
    compact_unwind_section: usize,
    compact_unwind_section_length: usize,
}

type FindDynamicUnwindSectionsFn =
    unsafe extern "C" fn(usize, *mut UnwDynamicUnwindSections) -> i32;

extern "C" {
    fn __unw_add_find_dynamic_unwind_sections(callback: FindDynamicUnwindSectionsFn);
    fn __unw_remove_find_dynamic_unwind_sections(callback: FindDynamicUnwindSectionsFn);
}

unsafe extern "C" fn find_unwind_sections(
    target_addr: usize,
    info: *mut UnwDynamicUnwindSections,
) -> i32 {
    let guard = match UNWIND_IMAGES.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };

    for img in guard.iter() {
        if target_addr >= img.text_start && target_addr < img.text_end {
            let out = unsafe { &mut *info };
            out.dso_base = img.text_start;

            if let Some((addr, len)) = img.eh_frame {
                out.dwarf_section = addr;
                out.dwarf_section_length = len;
            } else {
                out.dwarf_section = 0;
                out.dwarf_section_length = 0;
            }

            if let Some((addr, len)) = img.compact_unwind {
                out.compact_unwind_section = addr;
                out.compact_unwind_section_length = len;
            } else {
                out.compact_unwind_section = 0;
                out.compact_unwind_section_length = 0;
            }

            return 1;
        }
    }

    0
}

pub struct DarwinUnwindRegistry;

impl UnwindRegistry for DarwinUnwindRegistry {
    fn register(
        &self,
        text_start: usize,
        text_end: usize,
        compact_unwind: Option<(usize, usize)>,
        eh_frame: Option<(usize, usize)>,
    ) -> Result<(), PalError> {
        if !CALLBACK_REGISTERED.swap(true, std::sync::atomic::Ordering::AcqRel) {
            unsafe {
                __unw_add_find_dynamic_unwind_sections(find_unwind_sections);
            }
            log::debug!("DarwinUnwindRegistry: registered callback with libunwind");
        }

        let mut guard = UNWIND_IMAGES.lock().unwrap_or_else(|e| e.into_inner());
        guard.push(UnwindImageInfo {
            text_start,
            text_end,
            compact_unwind,
            eh_frame,
        });
        Ok(())
    }

    fn unregister(&self, text_start: usize) -> Result<(), PalError> {
        let mut guard = UNWIND_IMAGES.lock().unwrap_or_else(|e| e.into_inner());
        guard.retain(|img| img.text_start != text_start);
        Ok(())
    }
}
