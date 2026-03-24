//! Controlled entry: call the mapped binary's entry point.
//!
//! After segments are mapped and fixups applied, we compute the entry
//! address from LC_MAIN's offset relative to __TEXT and call it.
//!
//! # Safety
//!
//! This jumps into mapped code. Only use with known-safe fixture binaries.
//! The caller is responsible for ensuring:
//! - All segments are mapped with correct permissions
//! - All fixups have been applied
//! - The binary is a trivial test fixture (not arbitrary code)

use macho_types::EntryPoint;

use crate::{ExecError, MappedImage};

/// Result of calling a binary's entry point.
#[derive(Debug, Clone)]
pub struct EntryResult {
    /// The return value from main() (or equivalent).
    pub exit_code: i32,
    /// The computed entry address that was called.
    pub entry_address: u64,
}

/// Call the mapped binary's entry point.
///
/// Computes the entry address from the MappedImage's __TEXT base address
/// and the EntryPoint offset, then casts to a C function pointer and calls it.
///
/// For `LC_MAIN` binaries: entry_addr = text_base + entry_offset
/// For `LC_UNIXTHREAD` binaries: not yet supported (would need absolute address mapping).
///
/// # Safety
///
/// This function is inherently unsafe — it executes arbitrary mapped code.
/// Only call with known fixture binaries.
pub fn call_entry(image: &MappedImage, entry: &EntryPoint) -> Result<EntryResult, ExecError> {
    match entry {
        EntryPoint::Main { entry_offset, .. } => {
            let text_base = image.text_base()?;
            let entry_addr = unsafe { text_base.add(*entry_offset as usize) };
            let entry_addr_u64 = entry_addr as u64;

            log::info!(
                "Calling LC_MAIN entry at {entry_addr_u64:#x} (text_base={:#x} + offset={entry_offset:#x})",
                text_base as u64
            );

            // Finalize segment permissions before handing control to the binary.
            // This makes __TEXT RX (not RW) and __DATA_CONST R (not RW).
            image.protect()?;

            // Cast to extern "C" fn() -> i32 and call
            // The simple_cli fixture's main() takes argc/argv but we pass
            // a minimal environment: argc=1, argv=["app", NULL]
            let entry_fn: extern "C" fn(i32, *const *const u8) -> i32 =
                unsafe { std::mem::transmute(entry_addr) };

            let arg0 = c"app".as_ptr().cast::<u8>();
            let argv: [*const u8; 2] = [arg0, std::ptr::null()];

            log::debug!("calling entry at {entry_addr_u64:#x}");

            let exit_code = entry_fn(1, argv.as_ptr());

            log::debug!("entry returned {exit_code}");

            Ok(EntryResult {
                exit_code,
                entry_address: entry_addr_u64,
            })
        }
        EntryPoint::UnixThread { .. } => {
            // LC_UNIXTHREAD requires mapping at absolute addresses,
            // which is not supported in our lab environment.
            Err(ExecError::NoEntryPoint)
        }
    }
}
