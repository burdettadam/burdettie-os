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
pub mod interpose;
mod mapped;
pub mod compact_to_ehframe;
pub mod patch;
pub mod ptrauth;
mod registry;
mod security;
mod tlv;
mod unwind;

pub use entry::{call_entry, call_entry_with_args, call_entry_interpreted, EntryArgs, EntryResult};
pub use fixup::{apply_binds, apply_chained_fixups, apply_rebases, StubResolver, SymbolResolver};
pub use fixup::data_symbols::DataSymbolStore;
pub use initializer::call_constructors;
pub use mapped::{MappedImage, MappedSegment};
pub use tlv::patch_tlv_thunks;
pub use registry::{ImageRegistry, LoadedImage, RegistryResolver, normalize_install_name};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_error_display_mmap() {
        let err = ExecError::MmapFailed {
            segment: "__TEXT".into(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        let msg = err.to_string();
        assert!(msg.contains("__TEXT"), "message: {msg}");
        assert!(msg.contains("denied"), "message: {msg}");
    }

    #[test]
    fn exec_error_display_mprotect() {
        let err = ExecError::MprotectFailed {
            segment: "__DATA".into(),
            source: std::io::Error::new(std::io::ErrorKind::Other, "bad perm"),
        };
        assert!(err.to_string().contains("__DATA"));
    }

    #[test]
    fn exec_error_display_segment_oob() {
        let err = ExecError::SegmentOutOfBounds {
            segment: "__TEXT".into(),
            fileoff: 0x1000,
            filesize: 0x2000,
            binary_len: 0x1500,
        };
        let msg = err.to_string();
        assert!(msg.contains("__TEXT"));
        assert!(msg.contains("0x1000"));
    }

    #[test]
    fn exec_error_display_fixup_segment_range() {
        let err = ExecError::FixupSegmentOutOfRange {
            index: 5,
            count: 3,
        };
        let msg = err.to_string();
        assert!(msg.contains("5"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn exec_error_display_fixup_offset_oob() {
        let err = ExecError::FixupOffsetOutOfBounds {
            segment: "__DATA".into(),
            offset: 0x9000,
            size: 0x4000,
        };
        let msg = err.to_string();
        assert!(msg.contains("__DATA"));
        assert!(msg.contains("0x9000"));
    }

    #[test]
    fn exec_error_display_security_denied() {
        let err = ExecError::SecurityDenied {
            reason: "quarantine flag set".into(),
        };
        assert!(err.to_string().contains("quarantine flag set"));
    }

    #[test]
    fn exec_error_display_encrypted() {
        let err = ExecError::EncryptedBinary { crypt_id: 42 };
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn exec_error_display_no_text() {
        let err = ExecError::NoTextSegment;
        assert!(err.to_string().contains("__TEXT"));
    }

    #[test]
    fn exec_error_display_no_entry() {
        let err = ExecError::NoEntryPoint;
        assert!(err.to_string().contains("entry point"));
    }
}
