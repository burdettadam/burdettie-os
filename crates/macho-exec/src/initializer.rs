//! Constructor execution: call C/C++ initializers before main().
//!
//! After segments are mapped and fixups applied, constructors from
//! `__mod_init_func` or `__init_offsets` must be called before the
//! binary's `main()` entry point. This matches dyld's initialization
//! sequence.
//!
//! # Safety
//!
//! Constructors are called as `extern "C" fn()` — they execute arbitrary
//! mapped code. Only use with known fixture binaries.

use macho_objc::InitializerInfo;

use crate::ptrauth;
use crate::{ExecError, MappedImage};

/// Call all constructors for a mapped image.
///
/// Each constructor address from `InitializerInfo` is slide-adjusted
/// and called as `extern "C" fn()`. Constructors are called in the
/// order they appear in the section (matching dyld behavior).
///
/// Returns the number of constructors called.
pub fn call_constructors(image: &MappedImage, inits: &InitializerInfo) -> Result<usize, ExecError> {
    let slide = image.slide;
    let mut count = 0;

    for ctor in &inits.constructors {
        // The ConstructorEntry.address is the virtual address of the
        // constructor function. Apply slide to get the in-process address.
        let in_process_addr = (ctor.address as i64 + slide) as u64;

        log::info!(
            "Calling constructor at {in_process_addr:#x} (vmaddr={:#x}, slide={slide:#x}, source={:?})",
            ctor.address, ctor.source
        );

        let stripped = ptrauth::ptrauth_strip(in_process_addr);
        let ctor_fn: extern "C" fn() = unsafe { std::mem::transmute(stripped) };
        ctor_fn();
        count += 1;
    }

    Ok(count)
}
