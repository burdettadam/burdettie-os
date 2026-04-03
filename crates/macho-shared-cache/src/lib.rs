//! Parser for Apple's dyld_shared_cache format.
//!
//! The dyld shared cache is a single file containing all system dylibs
//! pre-linked and optimized. On macOS it lives at:
//! - ARM64: `/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_arm64e`
//! - x86_64: `/var/db/dyld/dyld_shared_cache_x86_64`
//!
//! On iOS, the shared cache is in the firmware.
//!
//! # Apple Stack Mapping
//!
//! ```text
//! Apple:  dyld shared cache → pre-linked system frameworks
//! Us:     macho-shared-cache → parse + extract individual dylibs from cache
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use macho_shared_cache::SharedCache;
//!
//! let data = std::fs::read("/path/to/dyld_shared_cache_arm64e").unwrap();
//! let cache = SharedCache::parse(&data).unwrap();
//!
//! println!("Format: {}", cache.header().magic_string());
//! println!("Images: {}", cache.images().len());
//!
//! for img in cache.images() {
//!     println!("  {} @ {:#x}", img.path(), img.address());
//! }
//!
//! // Extract a specific dylib
//! if let Some(img) = cache.find_image("/usr/lib/libSystem.B.dylib") {
//!     let mach_o_data = cache.extract_image(img).unwrap();
//!     // mach_o_data is now parseable by macho-parse
//! }
//! ```

mod header;
mod error;
mod image;

pub use error::SharedCacheError;
pub use header::{CacheHeader, MappingInfo};
pub use image::ImageInfo;

/// Parsed dyld shared cache.
pub struct SharedCache<'a> {
    data: &'a [u8],
    header: CacheHeader,
    mappings: Vec<MappingInfo>,
    images: Vec<ImageInfo>,
}

impl<'a> SharedCache<'a> {
    /// Parse a dyld shared cache from raw bytes.
    pub fn parse(data: &'a [u8]) -> Result<Self, SharedCacheError> {
        let header = CacheHeader::parse(data)?;
        let mappings = MappingInfo::parse_all(data, &header)?;
        let images = ImageInfo::parse_all(data, &header)?;

        log::debug!(
            "SharedCache: parsed {} mappings, {} images",
            mappings.len(),
            images.len()
        );

        Ok(Self { data, header, mappings, images })
    }

    /// Access the cache header.
    pub fn header(&self) -> &CacheHeader {
        &self.header
    }

    /// Access all memory mappings.
    pub fn mappings(&self) -> &[MappingInfo] {
        &self.mappings
    }

    /// Access all cached images (dylibs).
    pub fn images(&self) -> &[ImageInfo] {
        &self.images
    }

    /// Find a cached image by install path.
    pub fn find_image(&self, path: &str) -> Option<&ImageInfo> {
        self.images.iter().find(|img| img.path() == path)
    }

    /// Extract raw Mach-O data for a cached image.
    ///
    /// The returned data starts at the image's Mach-O header and extends
    /// to include all its mapped content. This data can be passed to
    /// `macho-parse` for further analysis.
    pub fn extract_image(&self, image: &ImageInfo) -> Result<&'a [u8], SharedCacheError> {
        let file_offset = self.vm_addr_to_file_offset(image.address())
            .ok_or(SharedCacheError::AddressNotMapped(image.address()))?;

        if file_offset >= self.data.len() {
            return Err(SharedCacheError::OffsetOutOfBounds {
                offset: file_offset,
                size: self.data.len(),
            });
        }

        // Read the Mach-O header to determine the full size
        // We need at least the mach_header_64 to get sizeofcmds
        let remaining = &self.data[file_offset..];
        if remaining.len() < 32 {
            return Err(SharedCacheError::TruncatedImage(image.path().to_string()));
        }

        // Read ncmds and sizeofcmds from the Mach-O header
        // mach_header_64: magic(4) + cpu(4) + sub(4) + filetype(4) + ncmds(4) + sizeofcmds(4) + flags(4) + reserved(4) = 32
        let sizeofcmds = u32::from_le_bytes([
            remaining[20], remaining[21], remaining[22], remaining[23],
        ]) as usize;
        let header_plus_cmds = 32 + sizeofcmds;

        let end = file_offset + header_plus_cmds.min(remaining.len());
        Ok(&self.data[file_offset..end])
    }

    /// Convert a VM address to a file offset using the mapping table.
    fn vm_addr_to_file_offset(&self, addr: u64) -> Option<usize> {
        for mapping in &self.mappings {
            if addr >= mapping.address && addr < mapping.address + mapping.size {
                let delta = addr - mapping.address;
                return Some((mapping.file_offset + delta) as usize);
            }
        }
        None
    }
}
