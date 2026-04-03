//! dyld_cache_image_info parsing.

use crate::error::SharedCacheError;
use crate::header::CacheHeader;

/// A cached image entry.
///
/// Corresponds to `dyld_cache_image_info`:
/// ```c
/// struct dyld_cache_image_info {
///     uint64_t address;
///     uint64_t modTime;
///     uint64_t inode;
///     uint32_t pathFileOffset;
///     uint32_t pad;
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// VM address of the image's Mach-O header.
    address: u64,
    /// Modification time of the original dylib.
    mod_time: u64,
    /// Inode of the original dylib.
    inode: u64,
    /// The install path (e.g. "/usr/lib/libSystem.B.dylib").
    path: String,
}

impl ImageInfo {
    /// Size of one `dyld_cache_image_info` in bytes.
    const SIZE: usize = 32;

    /// Parse all image entries from the cache data.
    pub fn parse_all(data: &[u8], header: &CacheHeader) -> Result<Vec<Self>, SharedCacheError> {
        let offset = header.images_offset as usize;
        let count = header.images_count as usize;
        let end = offset + count * Self::SIZE;

        if end > data.len() {
            return Err(SharedCacheError::OffsetOutOfBounds {
                offset: end,
                size: data.len(),
            });
        }

        let mut images = Vec::with_capacity(count);
        for i in 0..count {
            let base = offset + i * Self::SIZE;
            let d = &data[base..base + Self::SIZE];

            let address  = u64::from_le_bytes([d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]]);
            let mod_time = u64::from_le_bytes([d[8], d[9], d[10], d[11], d[12], d[13], d[14], d[15]]);
            let inode    = u64::from_le_bytes([d[16], d[17], d[18], d[19], d[20], d[21], d[22], d[23]]);
            let path_offset = u32::from_le_bytes([d[24], d[25], d[26], d[27]]) as usize;

            let path = Self::read_path(data, path_offset)?;

            images.push(Self { address, mod_time, inode, path });
        }

        Ok(images)
    }

    /// Read a null-terminated path string from the cache data.
    fn read_path(data: &[u8], offset: usize) -> Result<String, SharedCacheError> {
        if offset >= data.len() {
            return Err(SharedCacheError::OffsetOutOfBounds {
                offset,
                size: data.len(),
            });
        }

        let remaining = &data[offset..];
        let end = remaining.iter().position(|&b| b == 0).unwrap_or(remaining.len());
        let bytes = &remaining[..end];

        String::from_utf8(bytes.to_vec())
            .map_err(|_| SharedCacheError::InvalidPath(offset))
    }

    /// The VM address of this image's Mach-O header.
    pub fn address(&self) -> u64 {
        self.address
    }

    /// The install path of this dylib.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The modification time of the original dylib file.
    pub fn mod_time(&self) -> u64 {
        self.mod_time
    }

    /// The inode of the original dylib file.
    pub fn inode(&self) -> u64 {
        self.inode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_image_from_test_cache() {
        // Build the same test cache as in header tests
        let mut data = vec![0u8; 256];
        data[..16].copy_from_slice(b"dyld_v1   arm64\0");
        data[16..20].copy_from_slice(&1u32.to_le_bytes());
        data[20..24].copy_from_slice(&48u32.to_le_bytes());
        data[24..28].copy_from_slice(&1u32.to_le_bytes());
        data[28..32].copy_from_slice(&80u32.to_le_bytes());
        data[32..40].copy_from_slice(&0x0001_8000_0000u64.to_le_bytes());

        // Image at offset 80
        let i = 80;
        data[i..i+8].copy_from_slice(&0x0001_8000_0000u64.to_le_bytes());
        data[i+8..i+16].copy_from_slice(&0u64.to_le_bytes());
        data[i+16..i+24].copy_from_slice(&0u64.to_le_bytes());
        data[i+24..i+28].copy_from_slice(&112u32.to_le_bytes());
        data[i+28..i+32].copy_from_slice(&0u32.to_le_bytes());

        // Path at 112
        let path = b"/usr/lib/libSystem.B.dylib\0";
        data[112..112+path.len()].copy_from_slice(path);

        let header = CacheHeader::parse(&data).unwrap();
        let images = ImageInfo::parse_all(&data, &header).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].path(), "/usr/lib/libSystem.B.dylib");
        assert_eq!(images[0].address(), 0x0001_8000_0000);
    }
}
