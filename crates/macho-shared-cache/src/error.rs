//! Error types for shared cache parsing.

/// Errors that can occur when parsing a dyld shared cache.
#[derive(Debug, thiserror::Error)]
pub enum SharedCacheError {
    /// The data is too small to contain a valid cache header.
    #[error("data too small for shared cache header: {0} bytes")]
    TooSmall(usize),

    /// The magic bytes don't match any known shared cache format.
    #[error("invalid shared cache magic: {0:?}")]
    InvalidMagic(String),

    /// A file offset is beyond the end of the data.
    #[error("offset {offset:#x} out of bounds (file size: {size:#x})")]
    OffsetOutOfBounds { offset: usize, size: usize },

    /// A VM address doesn't fall within any mapped region.
    #[error("VM address {0:#x} not covered by any mapping")]
    AddressNotMapped(u64),

    /// An image's Mach-O data is truncated.
    #[error("truncated image: {0}")]
    TruncatedImage(String),

    /// A string in the cache is not valid UTF-8.
    #[error("invalid UTF-8 in image path at offset {0:#x}")]
    InvalidPath(usize),
}
