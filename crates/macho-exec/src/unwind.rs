//! Unwind frame registration for mapped Mach-O images.
//!
//! Parses Mach-O segment metadata to extract `__unwind_info` (compact unwind)
//! and `__eh_frame` (DWARF) section addresses, then delegates to the
//! platform's `UnwindRegistry` implementation. The actual libunwind callback
//! management lives in `platform-darwin-host`.

use macho_types::segment::Segment;
use platform_core::unwind::UnwindRegistry;

#[derive(Clone, Copy)]
struct UnwindSections {
    text_start: usize,
    text_end: usize,
    compact_unwind: Option<(usize, usize)>,
    eh_frame: Option<(usize, usize)>,
}

fn collect_unwind_sections(segments: &[Segment], slide: i64) -> Option<UnwindSections> {
    let text_seg = segments.iter().find(|s| s.name == "__TEXT");
    let text_exec_seg = segments.iter().find(|s| s.name == "__TEXT_EXEC");

    let primary = text_seg.or(text_exec_seg)?;
    let text_start = (primary.vmaddr as i64 + slide) as usize;
    let text_end = text_start + primary.vmsize as usize;

    let mut compact_unwind = None;
    let mut eh_frame = None;

    let seg_chain: Vec<&Segment> = segments
        .iter()
        .filter(|s| s.name == "__TEXT" || s.name == "__TEXT_EXEC")
        .collect();

    for seg in seg_chain {
        for section in &seg.sections {
            let sect_addr = (section.addr as i64 + slide) as usize;
            let sect_len = section.size as usize;

            match section.sectname.as_str() {
                "__unwind_info" if compact_unwind.is_none() => {
                    compact_unwind = Some((sect_addr, sect_len));
                }
                "__eh_frame" if eh_frame.is_none() => {
                    eh_frame = Some((sect_addr, sect_len));
                }
                _ => {}
            }
        }
    }

    if compact_unwind.is_none() && eh_frame.is_none() {
        return None;
    }

    Some(UnwindSections {
        text_start,
        text_end,
        compact_unwind,
        eh_frame,
    })
}

/// Generate a runtime-ready `.eh_frame` buffer for a mapped image when only
/// compact unwind info is available.
pub fn synthesize_runtime_eh_frame(segments: &[Segment], slide: i64) -> Option<Vec<u8>> {
    let sections = collect_unwind_sections(segments, slide)?;
    if sections.eh_frame.is_some() {
        return None;
    }

    let (addr, len) = sections.compact_unwind?;
    let cu_slice = unsafe { std::slice::from_raw_parts(addr as *const u8, len) };
    let entries = crate::compact_to_ehframe::parse_compact_unwind(cu_slice);
    if entries.is_empty() {
        return None;
    }

    let eh = crate::compact_to_ehframe::generate_eh_frame_runtime(&entries, sections.text_start);
    (!eh.is_empty()).then_some(eh)
}

/// Register a mapped image's unwind sections via the platform registry.
///
/// Parses the Mach-O segments to find `__unwind_info` and `__eh_frame`
/// sections, then calls `registry.register()`.
pub fn register_unwind_info(
    segments: &[Segment],
    slide: i64,
    registry: &dyn UnwindRegistry,
    generated_eh_frame: Option<&[u8]>,
) {
    let Some(sections) = collect_unwind_sections(segments, slide) else {
        return;
    };

    let eh_frame = generated_eh_frame
        .map(|buf| (buf.as_ptr() as usize, buf.len()))
        .or(sections.eh_frame);
    let compact_unwind = sections.compact_unwind;

    log::debug!(
        "unwind: registering image at {:#x}..{:#x} (compact={:?}, eh_frame={:?})",
        sections.text_start,
        sections.text_end,
        compact_unwind,
        eh_frame,
    );

    if let Err(e) = registry.register(sections.text_start, sections.text_end, compact_unwind, eh_frame) {
        log::warn!("unwind: failed to register image at {:#x}: {e}", sections.text_start);
    }
}

/// Remove a mapped image from the unwind registry.
#[allow(dead_code)]
pub fn unregister_unwind_info(segments: &[Segment], slide: i64, registry: &dyn UnwindRegistry) {
    let Some(sections) = collect_unwind_sections(segments, slide) else {
        return;
    };

    if let Err(e) = registry.unregister(sections.text_start) {
        log::warn!("unwind: failed to unregister image at {:#x}: {e}", sections.text_start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macho_types::segment::{Section, Segment, VmProt};
    use platform_core::unwind::NoopUnwindRegistry;
    use std::sync::Mutex;

    /// Test registry that records register/unregister calls.
    struct RecordingRegistry {
        registered: Mutex<Vec<(usize, usize, Option<(usize, usize)>, Option<(usize, usize)>)>>,
        unregistered: Mutex<Vec<usize>>,
    }

    impl RecordingRegistry {
        fn new() -> Self {
            Self {
                registered: Mutex::new(Vec::new()),
                unregistered: Mutex::new(Vec::new()),
            }
        }
    }

    impl UnwindRegistry for RecordingRegistry {
        fn register(
            &self,
            text_start: usize,
            text_end: usize,
            compact_unwind: Option<(usize, usize)>,
            eh_frame: Option<(usize, usize)>,
        ) -> Result<(), platform_core::PalError> {
            self.registered
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((text_start, text_end, compact_unwind, eh_frame));
            Ok(())
        }

        fn unregister(&self, text_start: usize) -> Result<(), platform_core::PalError> {
            self.unregistered.lock().unwrap_or_else(|e| e.into_inner()).push(text_start);
            Ok(())
        }
    }

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
        let reg = RecordingRegistry::new();
        let seg = make_text_segment(0x1_0000_0000, 0x1000, true, false);
        let segments = vec![seg];

        register_unwind_info(&segments, 0, &reg, None);
        assert_eq!(reg.registered.lock().unwrap_or_else(|e| e.into_inner()).len(), 1);
        assert_eq!(reg.registered.lock().unwrap_or_else(|e| e.into_inner())[0].0, 0x1_0000_0000);

        unregister_unwind_info(&segments, 0, &reg);
        assert_eq!(reg.unregistered.lock().unwrap_or_else(|e| e.into_inner()).len(), 1);
        assert_eq!(reg.unregistered.lock().unwrap_or_else(|e| e.into_inner())[0], 0x1_0000_0000);
    }

    #[test]
    fn no_unwind_sections_skips_registration() {
        let reg = RecordingRegistry::new();
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

        register_unwind_info(&segments, 0, &reg, None);
        assert!(reg.registered.lock().unwrap_or_else(|e| e.into_inner()).is_empty());
    }

    #[test]
    fn noop_registry_does_not_panic() {
        let noop = NoopUnwindRegistry;
        let seg = make_text_segment(0x3_0000_0000, 0x2000, true, true);
        let segments = vec![seg];

        register_unwind_info(&segments, 0, &noop, None);
        unregister_unwind_info(&segments, 0, &noop);
    }

    #[test]
    fn unregister_without_text_segment_is_noop() {
        let reg = RecordingRegistry::new();
        let seg = Segment {
            name: "__DATA".to_string(),
            vmaddr: 0x4_0000_0000,
            vmsize: 0x1000,
            fileoff: 0,
            filesize: 0x1000,
            maxprot: VmProt::READ | VmProt::WRITE,
            initprot: VmProt::READ | VmProt::WRITE,
            nsects: 0,
            flags: 0,
            sections: vec![],
        };
        let segments = vec![seg];
        unregister_unwind_info(&segments, 0, &reg);
        assert!(reg.unregistered.lock().unwrap_or_else(|e| e.into_inner()).is_empty());
    }

    #[test]
    fn register_with_slide_applies_offset() {
        let reg = RecordingRegistry::new();
        let seg = make_text_segment(0x1_0000_0000, 0x1000, true, false);
        let segments = vec![seg];

        let slide: i64 = 0x1000;
        register_unwind_info(&segments, slide, &reg, None);

        let registered = reg.registered.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(registered.len(), 1);
        // text_start = vmaddr + slide
        assert_eq!(registered[0].0, 0x1_0000_0000 + 0x1000);
    }

    #[test]
    fn generated_eh_frame_keeps_compact_unwind_registered() {
        let reg = RecordingRegistry::new();
        let seg = make_text_segment(0x5_0000_0000, 0x1000, true, false);
        let segments = vec![seg];
        let generated = vec![1u8, 2, 3, 4];

        register_unwind_info(&segments, 0, &reg, Some(&generated));

        let registered = reg.registered.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0].0, 0x5_0000_0000);
        assert_eq!(registered[0].2, Some((0x5_0000_0100, 0x60)));
        assert_eq!(registered[0].3, Some((generated.as_ptr() as usize, generated.len())));
    }
}
