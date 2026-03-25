//! In-process Mach-O segment mapping, fixup application, and controlled entry.
//!
//! This crate crosses the boundary from read-only binary analysis into
//! actual execution. It takes a parsed Mach-O and its mapping plan,
//! maps segments into the current process using `mmap`, applies fixups
//! (rebases + symbol binds), and provides controlled entry to `LC_MAIN`.
//!
//! # Safety
//!
//! This crate uses `unsafe` for:
//! - `mmap` / `mprotect` / `munmap` (POSIX memory mapping)
//! - Casting mapped memory to a function pointer for entry
//!
//! Only tested fixture binaries should be executed. This is a laboratory
//! tool, not a production loader.

mod entry;
mod fixup;
mod initializer;
mod mapped;
pub mod compact_to_ehframe;
pub mod ptrauth;
mod registry;
mod security;
mod unwind;

pub use entry::{call_entry, call_entry_with_args, EntryArgs, EntryResult};
pub use fixup::{apply_binds, apply_chained_fixups, apply_rebases, StubResolver, SymbolResolver};
pub use initializer::call_constructors;
pub use mapped::{MappedImage, MappedSegment};
pub use registry::{ImageRegistry, LoadedImage, RegistryResolver};
#[doc(hidden)]
pub use security::checked_entry;
pub use unwind::register_unwind_info;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecError {
    #[error("mmap failed for segment {segment}: {source}")]
    MmapFailed {
        segment: String,
        source: std::io::Error,
    },

    #[error("mprotect failed for segment {segment}: {source}")]
    MprotectFailed {
        segment: String,
        source: std::io::Error,
    },

    #[error("segment {segment} fileoff {fileoff:#x} + filesize {filesize:#x} exceeds binary size {binary_len:#x}")]
    SegmentOutOfBounds {
        segment: String,
        fileoff: u64,
        filesize: u64,
        binary_len: u64,
    },

    #[error("no __TEXT segment found — cannot compute entry address")]
    NoTextSegment,

    #[error("no entry point found in binary")]
    NoEntryPoint,

    #[error("fixup target segment index {index} out of range (have {count} segments)")]
    FixupSegmentOutOfRange { index: usize, count: usize },

    #[error("fixup offset {offset:#x} out of bounds for segment {segment} (size {size:#x})")]
    FixupOffsetOutOfBounds {
        segment: String,
        offset: u64,
        size: u64,
    },

    #[error("security policy denied execution: {reason}")]
    SecurityDenied { reason: String },

    #[error("binary is encrypted (cryptid={crypt_id}), cannot execute")]
    EncryptedBinary { crypt_id: u32 },
}
