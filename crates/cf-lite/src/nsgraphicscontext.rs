//! NSGraphicsContext shim: drawing context wrapper for AppKit.
//!
//! Object layout:
//!
//!   offset  0: isa           (u64)
//!   offset  8: cg_context    (u64 — CGContextRef handle from cg-shims bridge)
//!
//! Total: 16 bytes.
//!
//! Thread-local current context tracks the active drawing context
//! for `drawRect:` dispatch.

use std::cell::Cell;

const CG_CONTEXT_OFF: usize = 8;
const GRAPHICS_CONTEXT_SIZE: usize = 16;

thread_local! {
    static CURRENT_CONTEXT: Cell<u64> = const { Cell::new(0) };
}

fn alloc_graphics_context(cg_ctx: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, GRAPHICS_CONTEXT_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsgraphicscontext_class_addr();
        *((obj as usize + CG_CONTEXT_OFF) as *mut u64) = cg_ctx;
    }
    obj
}

// ── Public API for internal use ────────────────────────────────────

/// Push a CGContext handle as the current graphics context.
/// Returns the previous context address (to restore later).
pub fn push_current_context(cg_ctx: u64) -> u64 {
    let gc = alloc_graphics_context(cg_ctx);
    let prev = CURRENT_CONTEXT.get();
    CURRENT_CONTEXT.set(gc);
    prev
}

/// Restore the previous graphics context.
pub fn pop_current_context(prev: u64) {
    CURRENT_CONTEXT.set(prev);
}

/// Get the CGContextRef handle of the current context (for `UIGraphicsGetCurrentContext`).
pub fn current_cg_context_handle() -> u64 {
    let gc = CURRENT_CONTEXT.get();
    if gc == 0 {
        return 0;
    }
    unsafe { *((gc as usize + CG_CONTEXT_OFF) as *const u64) }
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSGraphicsContext currentContext]`
pub extern "C" fn shim_nsgraphicscontext_current_context(_cls: u64, _cmd: u64) -> u64 {
    CURRENT_CONTEXT.get()
}

/// `+[NSGraphicsContext graphicsContextWithCGContext:flipped:]`
pub extern "C" fn shim_nsgraphicscontext_with_cg_context(
    _cls: u64,
    _cmd: u64,
    cg_ctx: u64,
    _flipped: u8,
) -> u64 {
    alloc_graphics_context(cg_ctx)
}

/// `+[NSGraphicsContext saveGraphicsState]`
pub extern "C" fn shim_nsgraphicscontext_save_state(_cls: u64, _cmd: u64) {
    let cg_ctx = current_cg_context_handle();
    if cg_ctx != 0 {
        cg_shims::bridge::with_context(cg_ctx, |c| c.save_state());
    }
}

/// `+[NSGraphicsContext restoreGraphicsState]`
pub extern "C" fn shim_nsgraphicscontext_restore_state(_cls: u64, _cmd: u64) {
    let cg_ctx = current_cg_context_handle();
    if cg_ctx != 0 {
        cg_shims::bridge::with_context(cg_ctx, |c| c.restore_state());
    }
}

/// `+[NSGraphicsContext setCurrentContext:]`
pub extern "C" fn shim_nsgraphicscontext_set_current_context(_cls: u64, _cmd: u64, ctx: u64) {
    CURRENT_CONTEXT.set(ctx);
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSGraphicsContext CGContext]`
pub extern "C" fn shim_nsgraphicscontext_cg_context(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + CG_CONTEXT_OFF) as *const u64) }
}

/// `-[NSGraphicsContext graphicsPort]` — deprecated synonym for CGContext.
pub extern "C" fn shim_nsgraphicscontext_graphics_port(obj: u64, cmd: u64) -> u64 {
    shim_nsgraphicscontext_cg_context(obj, cmd)
}

/// `-[NSGraphicsContext isFlipped]`
pub extern "C" fn shim_nsgraphicscontext_is_flipped(_obj: u64, _cmd: u64) -> u8 {
    0
}

/// `-[NSGraphicsContext saveGraphicsState]` (instance)
pub extern "C" fn shim_nsgraphicscontext_instance_save(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let cg_ctx = unsafe { *((obj as usize + CG_CONTEXT_OFF) as *const u64) };
    if cg_ctx != 0 {
        cg_shims::bridge::with_context(cg_ctx, |c| c.save_state());
    }
}

/// `-[NSGraphicsContext restoreGraphicsState]` (instance)
pub extern "C" fn shim_nsgraphicscontext_instance_restore(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let cg_ctx = unsafe { *((obj as usize + CG_CONTEXT_OFF) as *const u64) };
    if cg_ctx != 0 {
        cg_shims::bridge::with_context(cg_ctx, |c| c.restore_state());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn current_context_default_nil() {
        // In a fresh thread, current context should be nil
        let handle = std::thread::spawn(|| shim_nsgraphicscontext_current_context(0, 0))
            .join()
            .unwrap();
        assert_eq!(handle, 0);
    }

    #[test]
    fn push_pop_context_roundtrip() {
        let prev = push_current_context(0x1234);
        assert_eq!(current_cg_context_handle(), 0x1234);
        pop_current_context(prev);
    }

    #[test]
    fn alloc_graphics_context_stores_cg_ref() {
        let gc = alloc_graphics_context(0xABCD);
        assert_ne!(gc, 0);
        let isa = read_field(gc, 0);
        assert_eq!(isa, crate::nsobj::nsgraphicscontext_class_addr());
        let cg = read_field(gc, CG_CONTEXT_OFF);
        assert_eq!(cg, 0xABCD);
        free_obj(gc);
    }

    #[test]
    fn cg_context_accessor() {
        let gc = alloc_graphics_context(0xBEEF);
        let cg = shim_nsgraphicscontext_cg_context(gc, 0);
        assert_eq!(cg, 0xBEEF);
        free_obj(gc);
    }

    #[test]
    fn cg_context_nil_safety() {
        assert_eq!(shim_nsgraphicscontext_cg_context(0, 0), 0);
    }

    #[test]
    fn with_cg_context_factory() {
        let gc = shim_nsgraphicscontext_with_cg_context(0, 0, 0x9999, 0);
        assert_ne!(gc, 0);
        assert_eq!(shim_nsgraphicscontext_cg_context(gc, 0), 0x9999);
        free_obj(gc);
    }

    #[test]
    fn set_current_context_class_method() {
        std::thread::spawn(|| {
            let gc = alloc_graphics_context(0x5555);
            shim_nsgraphicscontext_set_current_context(0, 0, gc);
            assert_eq!(shim_nsgraphicscontext_current_context(0, 0), gc);
            assert_eq!(current_cg_context_handle(), 0x5555);
            free_obj(gc);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn is_flipped_returns_false() {
        assert_eq!(shim_nsgraphicscontext_is_flipped(0, 0), 0);
    }

    #[test]
    fn graphics_port_same_as_cg_context() {
        let gc = alloc_graphics_context(0x7777);
        assert_eq!(
            shim_nsgraphicscontext_graphics_port(gc, 0),
            shim_nsgraphicscontext_cg_context(gc, 0),
        );
        free_obj(gc);
    }
}
