//! In-process segment mapping via MemoryMapper trait.
//!
//! Maps Mach-O segments into the current process address space using
//! the platform's MemoryMapper implementation. All segments (excluding
//! __PAGEZERO) are placed within a single contiguous allocation that
//! preserves their relative virtual address offsets. This is essential for PC-relative instructions
//! (e.g., `adrp` in ARM64 stubs) that reference data in other segments.
//!
//! Strategy:
//! 1. Compute the virtual address range [min_vmaddr, max_vmaddr+vmsize)
//!    across all non-PAGEZERO segments
//! 2. mmap one large anonymous RW region of that total size
//! 3. Copy each segment's file-backed data to the correct offset
//! 4. After fixups, call `protect()` to tighten per-segment permissions
//!    via `mprotect` sub-regions

use std::ptr;
use std::sync::Arc;

use macho_map::{MapRegion, MappingPlan};
use macho_types::segment::VmProt;
use platform_core::memory::MemoryMapper;
use platform_core::unwind::UnwindRegistry;
use platform_core::Protection;

use crate::ExecError;

/// A single mapped segment within the contiguous image region.
#[derive(Debug)]
pub struct MappedSegment {
    /// Segment name (e.g., "__TEXT").
    pub name: String,
    /// Base address of this segment within the contiguous region.
    pub base: *mut u8,
    /// Size of this segment (page-aligned).
    pub size: usize,
    /// Original vmaddr from the Mach-O.
    pub vmaddr: u64,
    /// Declared protection from the segment command.
    pub protection: VmProt,
    /// File offset in the original binary.
    pub fileoff: u64,
    /// File size copied into the mapping.
    pub filesize: u64,
}

/// A fully mapped Mach-O image in the current process.
///
/// All segments share a single contiguous mmap allocation, preserving
/// their relative virtual address layout. This ensures PC-relative
/// instructions (adrp, b, bl) between segments work correctly.
///
/// The image is unmapped on drop.
pub struct MappedImage {
    /// Mapped segments (in order from MappingPlan, excluding PAGEZERO).
    pub segments: Vec<MappedSegment>,
    /// ASLR slide: actual base minus the binary's lowest vmaddr.
    pub slide: i64,
    /// Base of the contiguous allocation (for unmap on drop).
    alloc_base: *mut u8,
    /// Total size of the contiguous allocation (for unmap on drop).
    alloc_size: usize,
    /// Memory mapper for cleanup on drop.
    mapper: Arc<dyn MemoryMapper>,
    /// Unwind registry for cleanup on drop.
    unwind: Arc<dyn UnwindRegistry>,
}

// MappedImage contains a raw pointer but is single-owner and thread-confined.
// The pointer is always valid between construction and drop.
unsafe impl Send for MappedImage {}

impl MappedImage {
    /// Map all segments from a MappingPlan into the current process.
    ///
    /// Creates a single contiguous anonymous mapping that covers the
    /// entire virtual address range of the binary (excluding __PAGEZERO).
    /// Each segment is placed at its correct relative offset within this
    /// allocation, preserving PC-relative references between segments.
    ///
    /// All memory is initially RW to allow fixup application.
    /// Call `protect()` after fixups to tighten permissions.
    pub fn map(
        plan: &MappingPlan,
        binary_data: &[u8],
        mapper: Arc<dyn MemoryMapper>,
        unwind: Arc<dyn UnwindRegistry>,
    ) -> Result<Self, ExecError> {
        let page_size = mapper.page_size();

        // Collect non-PAGEZERO regions
        let regions: Vec<&MapRegion> = plan
            .regions
            .iter()
            .filter(|r| r.segment_name != "__PAGEZERO")
            .collect();

        if regions.is_empty() {
            return Ok(Self {
                segments: Vec::new(),
                slide: 0,
                alloc_base: ptr::null_mut(),
                alloc_size: 0,
                mapper,
                unwind,
            });
        }

        // Validate file bounds for all regions upfront
        for region in &regions {
            if region.filesize > 0 {
                let end = region.fileoff + region.filesize;
                if end > binary_data.len() as u64 {
                    return Err(ExecError::SegmentOutOfBounds {
                        segment: region.segment_name.clone(),
                        fileoff: region.fileoff,
                        filesize: region.filesize,
                        binary_len: binary_data.len() as u64,
                    });
                }
            }
        }

        // Compute the virtual address range
        let min_vmaddr = regions.iter().map(|r| r.vmaddr).min()
            .ok_or_else(|| ExecError::MmapFailed {
                segment: "contiguous_region".to_string(),
                source: std::io::Error::other("no loadable segments found"),
            })?;
        let max_vm_end = regions.iter().map(|r| r.vmaddr + r.vmsize).max()
            .ok_or_else(|| ExecError::MmapFailed {
                segment: "contiguous_region".to_string(),
                source: std::io::Error::other("no loadable segments found"),
            })?;
        let total_vm_span = (max_vm_end - min_vmaddr) as usize;

        // Round up to page boundary
        let alloc_size = (total_vm_span + page_size - 1) & !(page_size - 1);

        // Allocate one contiguous anonymous RW region
        let alloc_base = mapper
            .map_anonymous(alloc_size)
            .map_err(|e| ExecError::MmapFailed {
                segment: "contiguous_region".to_string(),
                source: std::io::Error::other(e.to_string()),
            })?;
        let slide = alloc_base as i64 - min_vmaddr as i64;

        // Place each segment at its correct relative offset
        let mut segments = Vec::with_capacity(regions.len());
        for region in &regions {
            let offset_in_alloc = (region.vmaddr - min_vmaddr) as usize;
            let seg_base = unsafe { alloc_base.add(offset_in_alloc) };
            let seg_size = (region.vmsize as usize + page_size - 1) & !(page_size - 1);

            // Copy file-backed data
            if region.filesize > 0 {
                let src = &binary_data
                    [region.fileoff as usize..(region.fileoff + region.filesize) as usize];
                unsafe {
                    ptr::copy_nonoverlapping(src.as_ptr(), seg_base, src.len());
                }
            }
            // Remainder is already zero-filled by MAP_ANON

            segments.push(MappedSegment {
                name: region.segment_name.clone(),
                base: seg_base,
                size: seg_size,
                vmaddr: region.vmaddr,
                protection: region.protection,
                fileoff: region.fileoff,
                filesize: region.filesize,
            });
        }

        Ok(Self {
            segments,
            slide,
            alloc_base,
            alloc_size,
            mapper,
            unwind,
        })
    }

    /// Find the mapped segment by name.
    pub fn segment(&self, name: &str) -> Option<&MappedSegment> {
        self.segments.iter().find(|s| s.name == name)
    }

    /// Get a mutable pointer into a segment at the given offset.
    ///
    /// Returns None if the segment is not found or offset is out of bounds.
    pub fn segment_ptr_mut(&self, segment_name: &str, offset: u64) -> Option<*mut u8> {
        let seg = self.segment(segment_name)?;
        if offset >= seg.size as u64 {
            return None;
        }
        Some(unsafe { seg.base.add(offset as usize) })
    }

    /// Tighten memory protections to match declared segment permissions.
    ///
    /// Must be called after all fixups have been applied.
    /// Uses mprotect on each segment's sub-region within the contiguous
    /// allocation.
    pub fn protect(&self) -> Result<(), ExecError> {
        for seg in &self.segments {
            let prot = vmprot_to_protection(seg.protection);
            self.mapper.protect(seg.base, seg.size, prot).map_err(|e| {
                ExecError::MprotectFailed {
                    segment: seg.name.clone(),
                    source: std::io::Error::other(e.to_string()),
                }
            })?;
        }
        Ok(())
    }

    /// Get the base address of __TEXT for computing entry point addresses.
    pub fn text_base(&self) -> Result<*const u8, ExecError> {
        self.segment("__TEXT")
            .map(|s| s.base as *const u8)
            .ok_or(ExecError::NoTextSegment)
    }

    /// Total mapped size (the contiguous allocation size).
    pub fn total_mapped_size(&self) -> usize {
        self.alloc_size
    }
}

impl Drop for MappedImage {
    fn drop(&mut self) {
        // Unregister any unwind sections for this image before unmapping
        if let Some(text_seg) = self.segments.iter().find(|s| s.name == "__TEXT") {
            if let Err(e) = self.unwind.unregister(text_seg.base as usize) {
                log::warn!("MappedImage Drop: failed to unregister unwind info: {}", e);
            }
        }

        if !self.alloc_base.is_null() && self.alloc_size > 0 {
            if let Err(e) = self.mapper.unmap(self.alloc_base, self.alloc_size) {
                log::warn!("MappedImage Drop: failed to unmap {:#x} ({} bytes): {}",
                    self.alloc_base as usize, self.alloc_size, e);
            }
        }
    }
}

/// Convert VmProt flags to platform Protection.
fn vmprot_to_protection(prot: VmProt) -> Protection {
    Protection {
        read: prot.contains(VmProt::READ),
        write: prot.contains(VmProt::WRITE),
        execute: prot.contains(VmProt::EXECUTE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vmprot_to_protection_readonly() {
        let p = vmprot_to_protection(VmProt::READ);
        assert!(p.read);
        assert!(!p.write);
        assert!(!p.execute);
    }

    #[test]
    fn vmprot_to_protection_rx() {
        let p = vmprot_to_protection(VmProt::READ | VmProt::EXECUTE);
        assert!(p.read);
        assert!(!p.write);
        assert!(p.execute);
    }

    #[test]
    fn vmprot_to_protection_rw() {
        let p = vmprot_to_protection(VmProt::READ | VmProt::WRITE);
        assert!(p.read);
        assert!(p.write);
        assert!(!p.execute);
    }

    #[test]
    fn vmprot_to_protection_none() {
        let p = vmprot_to_protection(VmProt::empty());
        assert!(!p.read);
        assert!(!p.write);
        assert!(!p.execute);
    }
}
