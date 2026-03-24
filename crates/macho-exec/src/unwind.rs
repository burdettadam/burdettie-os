//! Unwind frame registration for mapped Mach-O images.
//!
//! When we map a Mach-O image via mmap (bypassing dyld), the system unwinder
//! (libunwind) has no knowledge of the image's unwind tables. This means
//! `@try`/`@catch`/`@finally` and C++ exceptions will fail because
//! `_Unwind_RaiseException` cannot find landing pads in our mapped code.
//!
//! We solve this by calling `__unw_add_find_dynamic_unwind_sections` to
//! register a callback. When libunwind encounters a PC it doesn't recognize,
//! it invokes registered callbacks. Our callback checks whether the PC falls
//! within any of our mapped images and, if so, returns the addresses of
//! `__TEXT.__unwind_info` (compact unwind) and/or `__TEXT.__eh_frame` (DWARF)
//! sections.
//!
//! On drop, all registrations are cleaned up via
//! `__unw_remove_find_dynamic_unwind_sections`.

use std::sync::Mutex;

use macho_types::segment::Segment;

/// Describes one mapped image's unwind sections for the callback.
#[derive(Debug)]
struct UnwindImageInfo {
    /// Start of the mapped image (dso_base for libunwind).
    text_start: usize,
    /// End of the mapped __TEXT segment.
    text_end: usize,
    /// Address and length of `__TEXT.__unwind_info` (compact unwind), if present.
    compact_unwind: Option<(usize, usize)>,
    /// Address and length of `__TEXT.__eh_frame` (DWARF), if present.
    eh_frame: Option<(usize, usize)>,
}

/// Global registry of mapped images with unwind info.
/// The callback from libunwind has no user-data parameter, so we must
/// use a global to find our image info.
static UNWIND_IMAGES: Mutex<Vec<UnwindImageInfo>> = Mutex::new(Vec::new());

/// Whether our callback has been registered with libunwind.
static CALLBACK_REGISTERED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

// ── libunwind FFI ──────────────────────────────────────────────────

/// Layout matching `struct unw_dynamic_unwind_sections` from libunwind.
#[repr(C)]
struct UnwDynamicUnwindSections {
    dso_base: usize,
    dwarf_section: usize,
    dwarf_section_length: usize,
    compact_unwind_section: usize,
    compact_unwind_section_length: usize,
}

/// Callback type for `__unw_add_find_dynamic_unwind_sections`.
type FindDynamicUnwindSectionsFn =
    unsafe extern "C" fn(usize, *mut UnwDynamicUnwindSections) -> i32;

extern "C" {
    fn __unw_add_find_dynamic_unwind_sections(callback: FindDynamicUnwindSectionsFn);
    fn __unw_remove_find_dynamic_unwind_sections(callback: FindDynamicUnwindSectionsFn);
}

/// The callback invoked by libunwind when searching for unwind info.
///
/// `target_addr` is the PC libunwind is trying to unwind from.
/// If it falls within one of our mapped images, fill `info` and return 1.
/// Otherwise return 0.
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
            log::debug!(
                "unwind callback: HIT addr={:#x} in image {:#x}..{:#x}",
                target_addr,
                img.text_start,
                img.text_end
            );
            let out = &mut *info;
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

// ── Public API ─────────────────────────────────────────────────────

/// Register a mapped image's unwind sections with the system unwinder.
///
/// After this call, `@try`/`@catch`/`@finally` and C++ exceptions will
/// work for code in this image because libunwind can find the landing
/// pads via our callback.
///
/// # Arguments
///
/// * `segments` — the parsed Mach-O segments (need sections for unwind info lookup)
/// * `slide` — ASLR slide for the mapped image
///
/// # Safety
///
/// The mapped image must remain at the same address for the lifetime of
/// the registration. Call `unregister_unwind_info` before unmapping.
pub fn register_unwind_info(segments: &[Segment], slide: i64) {
    // Find __TEXT segment and its __unwind_info / __eh_frame sections
    let Some(text_seg) = segments.iter().find(|s| s.name == "__TEXT") else {
        return; // No __TEXT → nothing to register
    };

    let text_start = (text_seg.vmaddr as i64 + slide) as usize;
    let text_end = text_start + text_seg.vmsize as usize;

    let mut compact_unwind = None;
    let mut eh_frame = None;

    for section in &text_seg.sections {
        let sect_addr = (section.addr as i64 + slide) as usize;
        let sect_len = section.size as usize;

        match section.sectname.as_str() {
            "__unwind_info" => {
                compact_unwind = Some((sect_addr, sect_len));
            }
            "__eh_frame" => {
                eh_frame = Some((sect_addr, sect_len));
            }
            _ => {}
        }
    }

    // Only register if there's actual unwind data
    if compact_unwind.is_none() && eh_frame.is_none() {
        return;
    }

    log::debug!(
        "unwind: registering image at {:#x}..{:#x} (compact={:?}, eh_frame={:?})",
        text_start,
        text_end,
        compact_unwind,
        eh_frame,
    );

    let info = UnwindImageInfo {
        text_start,
        text_end,
        compact_unwind,
        eh_frame,
    };

    {
        let mut guard = UNWIND_IMAGES.lock().unwrap_or_else(|e| e.into_inner());
        guard.push(info);
    }

    // Register callback once (idempotent — libunwind only adds it once)
    if !CALLBACK_REGISTERED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        unsafe {
            __unw_add_find_dynamic_unwind_sections(find_unwind_sections);
        }
        log::debug!("unwind: registered find_dynamic_unwind_sections callback");
    }
}

/// Remove a mapped image from the unwind registry.
///
/// Must be called before unmapping the image to avoid dangling references.
pub fn unregister_unwind_info(segments: &[Segment], slide: i64) {
    let Some(text_seg) = segments.iter().find(|s| s.name == "__TEXT") else {
        return;
    };

    let text_start = (text_seg.vmaddr as i64 + slide) as usize;
    unregister_unwind_info_by_addr(text_start);
}

/// Unregister unwind info for an image whose __TEXT starts at `text_start`.
///
/// Called from `MappedImage::drop()` to clean up before munmap.
pub fn unregister_unwind_info_by_addr(text_start: usize) {
    let mut guard = UNWIND_IMAGES.lock().unwrap_or_else(|e| e.into_inner());
    guard.retain(|img| img.text_start != text_start);

    // If no more images, deregister the callback
    if guard.is_empty() && CALLBACK_REGISTERED.swap(false, std::sync::atomic::Ordering::SeqCst) {
        unsafe {
            __unw_remove_find_dynamic_unwind_sections(find_unwind_sections);
        }
        log::debug!("unwind: deregistered find_dynamic_unwind_sections callback");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macho_types::segment::{Section, Segment, VmProt};

    fn make_text_segment(vmaddr: u64, vmsize: u64, unwind_info: bool, eh_frame: bool) -> Segment {
        let mut sections = Vec::new();
        if unwind_info {
            sections.push(Section {
                sectname: "__unwind_info".to_string(),
                segname: "__TEXT".to_string(),
                addr: vmaddr + 0x100,
                size: 0x60,
                offset: 0,
                align: 0,
                reloff: 0,
                nreloc: 0,
                flags: 0,
                reserved1: 0,
                reserved2: 0,
            });
        }
        if eh_frame {
            sections.push(Section {
                sectname: "__eh_frame".to_string(),
                segname: "__TEXT".to_string(),
                addr: vmaddr + 0x200,
                size: 0x80,
                offset: 0,
                align: 0,
                reloff: 0,
                nreloc: 0,
                flags: 0,
                reserved1: 0,
                reserved2: 0,
            });
        }
        Segment {
            name: "__TEXT".to_string(),
            vmaddr,
            vmsize,
            fileoff: 0,
            filesize: vmsize,
            maxprot: VmProt::READ | VmProt::EXECUTE,
            initprot: VmProt::READ | VmProt::EXECUTE,
            nsects: sections.len() as u32,
            flags: 0,
            sections,
        }
    }

    #[test]
    fn register_and_unregister_unwind_info() {
        let seg = make_text_segment(0x1_0000_0000, 0x1000, true, false);
        let segments = vec![seg];
        let slide = 0i64;

        register_unwind_info(&segments, slide);

        {
            let guard = UNWIND_IMAGES.lock().unwrap();
            assert!(guard.iter().any(|i| i.text_start == 0x1_0000_0000));
        }

        unregister_unwind_info(&segments, slide);

        {
            let guard = UNWIND_IMAGES.lock().unwrap();
            assert!(!guard.iter().any(|i| i.text_start == 0x1_0000_0000));
        }
    }

    #[test]
    fn no_unwind_sections_skips_registration() {
        let seg = Segment {
            name: "__TEXT".to_string(),
            vmaddr: 0x2_0000_0000,
            vmsize: 0x1000,
            fileoff: 0,
            filesize: 0x1000,
            maxprot: VmProt::READ | VmProt::EXECUTE,
            initprot: VmProt::READ | VmProt::EXECUTE,
            nsects: 0,
            flags: 0,
            sections: vec![],
        };
        let segments = vec![seg];

        register_unwind_info(&segments, 0);

        let guard = UNWIND_IMAGES.lock().unwrap();
        assert!(!guard.iter().any(|i| i.text_start == 0x2_0000_0000));
    }

    #[test]
    fn callback_finds_image() {
        let seg = make_text_segment(0x3_0000_0000, 0x2000, true, true);
        let segments = vec![seg];
        register_unwind_info(&segments, 0);

        // Target inside image range
        let mut info = UnwDynamicUnwindSections {
            dso_base: 0,
            dwarf_section: 0,
            dwarf_section_length: 0,
            compact_unwind_section: 0,
            compact_unwind_section_length: 0,
        };
        let result = unsafe { find_unwind_sections(0x3_0000_0500, &mut info) };
        assert_eq!(result, 1);
        assert_eq!(info.dso_base, 0x3_0000_0000);
        assert_eq!(info.compact_unwind_section, 0x3_0000_0100);
        assert_eq!(info.compact_unwind_section_length, 0x60);
        assert_eq!(info.dwarf_section, 0x3_0000_0200);
        assert_eq!(info.dwarf_section_length, 0x80);

        // Target outside image range
        let result2 = unsafe { find_unwind_sections(0x4_0000_0000, &mut info) };
        assert_eq!(result2, 0);

        unregister_unwind_info(&segments, 0);
    }
}
