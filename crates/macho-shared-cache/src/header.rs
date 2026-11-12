//! dyld_cache_header and dyld_cache_mapping_info parsing.

use crate::error::SharedCacheError;

/// Known shared cache magic strings.
const MAGIC_ARM64E: &[u8; 16] = b"dyld_v1  arm64e\0";
const MAGIC_ARM64: &[u8; 16] = b"dyld_v1   arm64\0";
const MAGIC_X86_64: &[u8; 16] = b"dyld_v1  x86_64\0";
const MAGIC_X86_64H: &[u8; 16] = b"dyld_v1 x86_64h\0";

/// Parsed dyld_cache_header.
///
/// Only the fields needed for image extraction are parsed.
/// The full header has many more fields for slide info, code signatures,
/// branch pools, etc.
#[derive(Debug, Clone)]
pub struct CacheHeader {
    /// The 16-byte magic string (e.g. "dyld_v1  arm64e\0").
    pub magic: [u8; 16],
    /// Number of mapping regions.
    pub mapping_count: u32,
    /// File offset of the first `dyld_cache_mapping_info`.
    pub mapping_offset: u32,
    /// Number of cached images.
    pub images_count: u32,
    /// File offset of the first `dyld_cache_image_info`.
    pub images_offset: u32,
    /// Base address of the cache in virtual memory.
    pub dyld_base_address: u64,
}

impl CacheHeader {
    /// Minimum header size (we read up to images_offset + 4).
    const MIN_SIZE: usize = 48;

    /// Parse the header from the beginning of the cache data.
    pub fn parse(data: &[u8]) -> Result<Self, SharedCacheError> {
        if data.len() < Self::MIN_SIZE {
            return Err(SharedCacheError::TooSmall(data.len()));
        }

        let mut magic = [0u8; 16];
        magic.copy_from_slice(&data[0..16]);

        if !Self::is_valid_magic(&magic) {
            let magic_str = String::from_utf8_lossy(&magic).into_owned();
            return Err(SharedCacheError::InvalidMagic(magic_str));
        }

        let mapping_count = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
        let mapping_offset = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
        let images_count = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
        let images_offset = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);
        let dyld_base_address = u64::from_le_bytes([
            data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
        ]);

        Ok(Self {
            magic,
            mapping_count,
            mapping_offset,
            images_count,
            images_offset,
            dyld_base_address,
        })
    }

    /// Check if magic matches a known shared cache format.
    fn is_valid_magic(magic: &[u8; 16]) -> bool {
        magic == MAGIC_ARM64E
            || magic == MAGIC_ARM64
            || magic == MAGIC_X86_64
            || magic == MAGIC_X86_64H
    }

    /// Human-readable magic string.
    pub fn magic_string(&self) -> &str {
        let end = self.magic.iter().position(|&b| b == 0).unwrap_or(16);
        std::str::from_utf8(&self.magic[..end]).unwrap_or("<invalid>")
    }

    /// Architecture string extracted from magic.
    pub fn arch(&self) -> &str {
        self.magic_string().trim_start_matches("dyld_v1").trim()
    }
}

/// A memory mapping region in the shared cache.
///
/// Corresponds to `dyld_cache_mapping_info`:
/// ```c
/// struct dyld_cache_mapping_info {
///     uint64_t address;
///     uint64_t size;
///     uint64_t fileOffset;
///     uint32_t maxProt;
///     uint32_t initProt;
/// };
/// ```
#[derive(Debug, Clone)]
pub struct MappingInfo {
    /// VM address of the mapping.
    pub address: u64,
    /// Size in bytes.
    pub size: u64,
    /// File offset in the cache file.
    pub file_offset: u64,
    /// Maximum VM protection flags.
    pub max_prot: u32,
    /// Initial VM protection flags.
    pub init_prot: u32,
}

impl MappingInfo {
    /// Size of one `dyld_cache_mapping_info` in bytes.
    const SIZE: usize = 32;

    /// Parse all mapping entries from the cache data.
    pub fn parse_all(data: &[u8], header: &CacheHeader) -> Result<Vec<Self>, SharedCacheError> {
        let offset = header.mapping_offset as usize;
        let count = header.mapping_count as usize;
        let end = offset + count * Self::SIZE;

        if end > data.len() {
            return Err(SharedCacheError::OffsetOutOfBounds {
                offset: end,
                size: data.len(),
            });
        }

        let mut mappings = Vec::with_capacity(count);
        for i in 0..count {
            let base = offset + i * Self::SIZE;
            let d = &data[base..base + Self::SIZE];

            mappings.push(Self {
                address: u64::from_le_bytes([d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]]),
                size: u64::from_le_bytes([d[8], d[9], d[10], d[11], d[12], d[13], d[14], d[15]]),
                file_offset: u64::from_le_bytes([
                    d[16], d[17], d[18], d[19], d[20], d[21], d[22], d[23],
                ]),
                max_prot: u32::from_le_bytes([d[24], d[25], d[26], d[27]]),
                init_prot: u32::from_le_bytes([d[28], d[29], d[30], d[31]]),
            });
        }

        Ok(mappings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_cache() -> Vec<u8> {
        let mut data = vec![0u8; 256];
        // Magic: "dyld_v1   arm64\0"
        data[..16].copy_from_slice(MAGIC_ARM64);
        // mapping_count = 1
        data[16..20].copy_from_slice(&1u32.to_le_bytes());
        // mapping_offset = 48 (right after the header)
        data[20..24].copy_from_slice(&48u32.to_le_bytes());
        // images_count = 1
        data[24..28].copy_from_slice(&1u32.to_le_bytes());
        // images_offset = 80 (after mappings)
        data[28..32].copy_from_slice(&80u32.to_le_bytes());
        // dyld_base_address = 0x180000000
        data[32..40].copy_from_slice(&0x0001_8000_0000u64.to_le_bytes());

        // Mapping at offset 48: addr=0x180000000, size=0x1000, fileOffset=0, maxProt=5, initProt=5
        let m = 48;
        data[m..m + 8].copy_from_slice(&0x0001_8000_0000u64.to_le_bytes());
        data[m + 8..m + 16].copy_from_slice(&0x1000u64.to_le_bytes());
        data[m + 16..m + 24].copy_from_slice(&0u64.to_le_bytes());
        data[m + 24..m + 28].copy_from_slice(&5u32.to_le_bytes());
        data[m + 28..m + 32].copy_from_slice(&5u32.to_le_bytes());

        // Image at offset 80: addr=0x180000000, modTime=0, inode=0, pathFileOffset=112
        let i = 80;
        data[i..i + 8].copy_from_slice(&0x0001_8000_0000u64.to_le_bytes());
        data[i + 8..i + 16].copy_from_slice(&0u64.to_le_bytes()); // modTime
        data[i + 16..i + 24].copy_from_slice(&0u64.to_le_bytes()); // inode
        data[i + 24..i + 28].copy_from_slice(&112u32.to_le_bytes()); // pathFileOffset
                                                                     // pad
        data[i + 28..i + 32].copy_from_slice(&0u32.to_le_bytes());

        // Path at offset 112
        let path = b"/usr/lib/libSystem.B.dylib\0";
        data[112..112 + path.len()].copy_from_slice(path);

        data
    }

    #[test]
    fn parse_valid_header() {
        let data = make_test_cache();
        let header = CacheHeader::parse(&data).unwrap();
        assert_eq!(header.arch(), "arm64");
        assert_eq!(header.mapping_count, 1);
        assert_eq!(header.images_count, 1);
    }

    #[test]
    fn parse_invalid_magic() {
        let data = vec![0u8; 256];
        assert!(CacheHeader::parse(&data).is_err());
    }

    #[test]
    fn parse_too_small() {
        let data = vec![0u8; 10];
        assert!(CacheHeader::parse(&data).is_err());
    }

    #[test]
    fn parse_mappings() {
        let data = make_test_cache();
        let header = CacheHeader::parse(&data).unwrap();
        let mappings = MappingInfo::parse_all(&data, &header).unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].address, 0x0001_8000_0000);
        assert_eq!(mappings[0].size, 0x1000);
    }
}
