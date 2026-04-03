//! Shared cache integration for the runtime load path.
//!
//! When a shared cache file is available (e.g. extracted from macOS), the
//! runtime can resolve dylib dependencies directly from the cache instead
//! of requiring individual .dylib files on disk.
//!
//! # Usage
//!
//! ```no_run
//! use macho_runtime::shared_cache::SharedCacheLoader;
//! use std::path::Path;
//!
//! let loader = SharedCacheLoader::open(Path::new("/path/to/dyld_shared_cache_arm64e"))
//!     .expect("failed to open shared cache");
//!
//! if let Some(data) = loader.resolve_dylib("/usr/lib/libSystem.B.dylib") {
//!     println!("Found libSystem: {} bytes", data.len());
//! }
//!
//! println!("Cache contains {} images", loader.image_count());
//! ```

use std::collections::HashMap;
use std::path::Path;

use macho_shared_cache::SharedCache;

/// Errors from shared cache loading.
#[derive(Debug, thiserror::Error)]
pub enum SharedCacheError {
    /// Failed to open or stat the cache file.
    #[error("failed to open shared cache: {0}")]
    Io(#[from] std::io::Error),

    /// The file is empty.
    #[error("shared cache file is empty")]
    EmptyFile,

    /// Memory-mapping failed.
    #[error("mmap failed: {0}")]
    MmapFailed(std::io::Error),

    /// The cache data could not be parsed.
    #[error("failed to parse shared cache: {0}")]
    Parse(#[from] macho_shared_cache::SharedCacheError),
}

/// A memory-mapped shared cache with dylib resolution.
pub struct SharedCacheLoader {
    /// Memory-mapped cache data. Held as a raw mmap to avoid copying.
    _mmap: Mmap,
    /// Pre-extracted image metadata (owned strings, no lifetime dependency).
    images: Vec<CachedImage>,
    /// O(1) path → image-index lookup.
    path_index: HashMap<String, usize>,
    /// Total file size for diagnostics.
    file_size: usize,
}

/// Pre-extracted image metadata (owned, no lifetime dependency on cache).
struct CachedImage {
    /// Install path (e.g. "/usr/lib/libSystem.B.dylib").
    path: String,
    /// File offset of the Mach-O header in the cache.
    file_offset: usize,
    /// Size of the Mach-O header + load commands region.
    header_size: usize,
}

/// RAII wrapper around a raw mmap.
struct Mmap {
    ptr: *mut u8,
    len: usize,
}

// Safety: The mmap is read-only and lives for the lifetime of SharedCacheLoader.
unsafe impl Send for Mmap {}
unsafe impl Sync for Mmap {}

impl Mmap {
    fn as_slice(&self) -> &[u8] {
        // Safety: ptr was obtained from a successful mmap call with length `len`,
        // mapped as PROT_READ. The slice is valid for the lifetime of `self`.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        // Safety: ptr and len come from a successful mmap call.
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.len);
        }
    }
}

impl SharedCacheLoader {
    /// Open and memory-map a shared cache file, then parse its image catalog.
    pub fn open(path: &Path) -> Result<Self, SharedCacheError> {
        use std::os::unix::io::AsRawFd;

        let file = std::fs::File::open(path)?;
        let file_size = file.metadata()?.len() as usize;
        if file_size == 0 {
            return Err(SharedCacheError::EmptyFile);
        }

        // Safety: we mmap the file as read-only (PROT_READ, MAP_PRIVATE).
        // The FD can be closed immediately after mmap returns.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                file_size,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                file.as_raw_fd(),
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(SharedCacheError::MmapFailed(std::io::Error::last_os_error()));
        }

        let mmap = Mmap {
            ptr: ptr as *mut u8,
            len: file_size,
        };

        // Parse the cache structure from the mapped data.
        let cache = SharedCache::parse(mmap.as_slice())?;

        // Pre-extract image metadata so we don't hold a borrow on the cache.
        let mut images = Vec::with_capacity(cache.images().len());
        for img in cache.images() {
            // Resolve VM address → file offset via the cache mappings.
            let file_offset = cache
                .mappings()
                .iter()
                .find_map(|m| {
                    let addr = img.address();
                    if addr >= m.address && addr < m.address + m.size {
                        Some((m.file_offset + (addr - m.address)) as usize)
                    } else {
                        None
                    }
                });

            let file_offset = match file_offset {
                Some(off) => off,
                None => continue, // skip unmappable images
            };

            // Read the Mach-O header to get the header+load commands size.
            let header_size = if file_offset + 32 <= file_size {
                let data = mmap.as_slice();
                let sizeofcmds = u32::from_le_bytes([
                    data[file_offset + 20],
                    data[file_offset + 21],
                    data[file_offset + 22],
                    data[file_offset + 23],
                ]) as usize;
                (32 + sizeofcmds).min(file_size - file_offset)
            } else {
                continue; // truncated
            };

            images.push(CachedImage {
                path: img.path().to_string(),
                file_offset,
                header_size,
            });
        }

        log::debug!(
            "SharedCacheLoader: opened {} ({} images, {:.1} MB)",
            path.display(),
            images.len(),
            file_size as f64 / (1024.0 * 1024.0),
        );

        // Build O(1) path index.
        let path_index: HashMap<String, usize> = images
            .iter()
            .enumerate()
            .map(|(i, img)| (img.path.clone(), i))
            .collect();

        Ok(Self {
            _mmap: mmap,
            images,
            path_index,
            file_size,
        })
    }

    /// Look up a dylib by install name and return a slice of its Mach-O data.
    ///
    /// The returned slice points directly into the memory-mapped cache (zero-copy).
    /// Uses O(1) HashMap lookup instead of linear scan.
    pub fn resolve_dylib(&self, install_name: &str) -> Option<&[u8]> {
        let &idx = self.path_index.get(install_name)?;
        let img = &self.images[idx];
        let data = self._mmap.as_slice();
        let end = img.file_offset + img.header_size;
        if end <= data.len() {
            Some(&data[img.file_offset..end])
        } else {
            None
        }
    }

    /// Access the raw underlying memory-mapped cache data.
    ///
    /// Used by export trie parsing which needs to read __LINKEDIT data
    /// at arbitrary file offsets within the cache.
    pub fn raw_data(&self) -> &[u8] {
        self._mmap.as_slice()
    }

    /// Build a symbol → address map by parsing export tries from the
    /// specified dylibs. This is the "eager loading" optimization that
    /// pre-resolves symbols at startup.
    ///
    /// Returns a HashMap where keys are symbol names (with leading underscore,
    /// e.g. `_strlen`) and values are VM addresses from the export trie.
    pub fn eager_export_map(&self, dylib_paths: &[&str]) -> HashMap<String, u64> {
        let raw = self._mmap.as_slice();
        let mut map = HashMap::new();

        for &path in dylib_paths {
            let Some(&idx) = self.path_index.get(path) else {
                log::debug!("eager_export_map: image not found: {}", path);
                continue;
            };
            let img = &self.images[idx];
            let end = img.file_offset + img.header_size;
            if end > raw.len() {
                continue;
            }
            let header_data = &raw[img.file_offset..end];

            // Parse the Mach-O header+load commands to find the export trie.
            let Ok(parsed) = macho_parse::parse(header_data) else {
                log::debug!("eager_export_map: failed to parse {}", path);
                continue;
            };

            // Extract export trie — the data_off in the load command is
            // relative to the image's file offset within the cache.
            // However, in the shared cache the __LINKEDIT is shared and
            // the offsets are cache-absolute for LC_DYLD_EXPORTS_TRIE.
            //
            // Strategy: try cache-absolute offset first (shared cache),
            // then image-relative offset (standalone Mach-O).
            let exports = Self::extract_exports_from_cache(&parsed, raw, img.file_offset);
            let count = exports.len();

            for sym in exports {
                if !sym.name.is_empty() && !sym.is_reexport() {
                    map.insert(sym.name, sym.address);
                }
            }

            log::debug!(
                "eager_export_map: {} → {} exports",
                path, count
            );
        }

        log::info!(
            "eager_export_map: pre-resolved {} symbols from {} dylibs",
            map.len(),
            dylib_paths.len()
        );
        map
    }

    /// Extract exports from a cached image, handling cache-absolute offsets.
    fn extract_exports_from_cache(
        parsed: &macho_parse::ParsedMachO,
        cache_data: &[u8],
        image_file_offset: usize,
    ) -> Vec<macho_parse::ExportedSymbol> {
        use macho_types::load_command::{LoadCommandPayload, LoadCommandType};

        // Try LC_DYLD_EXPORTS_TRIE first (modern)
        for cmd in &parsed.commands {
            if cmd.cmd == LoadCommandType::DyldExportsTrie {
                if let LoadCommandPayload::LinkEdit { data_off, data_size } = &cmd.payload {
                    let off = *data_off as usize;
                    let size = *data_size as usize;
                    if size == 0 {
                        continue;
                    }
                    // Try cache-absolute offset
                    if off + size <= cache_data.len() {
                        return macho_parse::parse_export_trie(&cache_data[off..off + size]);
                    }
                    // Try image-relative offset
                    let abs = image_file_offset + off;
                    if abs + size <= cache_data.len() {
                        return macho_parse::parse_export_trie(&cache_data[abs..abs + size]);
                    }
                }
            }
        }

        // Fall back to LC_DYLD_INFO(_ONLY)
        for cmd in &parsed.commands {
            if cmd.cmd == LoadCommandType::DyldInfoOnly || cmd.cmd == LoadCommandType::DyldInfo {
                if let LoadCommandPayload::DyldInfo { export_off, export_size, .. } = &cmd.payload {
                    let off = *export_off as usize;
                    let size = *export_size as usize;
                    if size == 0 {
                        continue;
                    }
                    if off + size <= cache_data.len() {
                        return macho_parse::parse_export_trie(&cache_data[off..off + size]);
                    }
                    let abs = image_file_offset + off;
                    if abs + size <= cache_data.len() {
                        return macho_parse::parse_export_trie(&cache_data[abs..abs + size]);
                    }
                }
            }
        }

        Vec::new()
    }

    /// The number of images in the cache.
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Iterate over install paths of all cached images.
    pub fn image_paths(&self) -> impl Iterator<Item = &str> {
        self.images.iter().map(|i| i.path.as_str())
    }

    /// Total size of the memory-mapped cache file in bytes.
    pub fn file_size(&self) -> usize {
        self.file_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_nonexistent_cache_returns_error() {
        let result = SharedCacheLoader::open(Path::new("/nonexistent/cache"));
        assert!(result.is_err());
    }
}
