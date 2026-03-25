//! Unwind frame registration for mapped Mach-O images.
//!
//! Parses Mach-O segment metadata to extract `__unwind_info` (compact unwind)
//! and `__eh_frame` (DWARF) section addresses, then delegates to the
//! platform's `UnwindRegistry` implementation. The actual libunwind callback
//! management lives in `platform-darwin-host`.

use macho_types::segment::Segment;
use platform_core::unwind::UnwindRegistry;

/// Register a mapped image's unwind sections via the platform registry.
///
/// Parses the Mach-O segments to find `__unwind_info` and `__eh_frame`
/// sections, then calls `registry.register()`.
pub fn register_unwind_info(
    segments: &[Segment],
    slide: i64,
    registry: &dyn UnwindRegistry,
) {
    let text_seg = segments.iter().find(|s| s.name == "__TEXT");
    let text_exec_seg = segments.iter().find(|s| s.name == "__TEXT_EXEC");

    let Some(primary) = text_seg.or(text_exec_seg) else {
        return;
    };

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
        return;
    }

    log::debug!(
        "unwind: registering image at {:#x}..{:#x} (compact={:?}, eh_frame={:?})",
        text_start,
        text_end,
        compact_unwind,
        eh_frame,
    );

    if let Err(e) = registry.register(text_start, text_end, compact_unwind, eh_frame) {
        log::warn!("unwind: failed to register image at {:#x}: {e}", text_start);
    }
}

/// Remove a mapped image from the unwind registry.
#[allow(dead_code)]
pub fn unregister_unwind_info(segments: &[Segment], slide: i64, registry: &dyn UnwindRegistry) {
    let Some(text_seg) = segments.iter().find(|s| s.name == "__TEXT") else {
        return;
    };

    let text_start = (text_seg.vmaddr as i64 + slide) as usize;
    if let Err(e) = registry.unregister(text_start) {
        log::warn!("unwind: failed to unregister image at {:#x}: {e}", text_start);
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
        registered: Mutex<Vec<(usize, usize)>>,
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
            _compact_unwind: Option<(usize, usize)>,
            _eh_frame: Option<(usize, usize)>,
        ) -> Result<(), platform_core::PalError> {
            self.registered.lock().unwrap_or_else(|e| e.into_inner()).push((text_start, text_end));
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

        register_unwind_info(&segments, 0, &reg);
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

        register_unwind_info(&segments, 0, &reg);
        assert!(reg.registered.lock().unwrap_or_else(|e| e.into_inner()).is_empty());
    }

    #[test]
    fn noop_registry_does_not_panic() {
        let noop = NoopUnwindRegistry;
        let seg = make_text_segment(0x3_0000_0000, 0x2000, true, true);
        let segments = vec![seg];

        register_unwind_info(&segments, 0, &noop);
        unregister_unwind_info(&segments, 0, &noop);
    }
}
