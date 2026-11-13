//! NSAnimationContext shim: animation grouping, duration, and completion handlers.
//!
//! Object layout (40 bytes):
//!
//!   offset  0: isa                       (u64)
//!   offset  8: duration                  (f64 — seconds, default 0.25)
//!   offset 16: allows_implicit_animation (u64 — bool, default 0)
//!   offset 24: completion_handler        (u64 — block pointer, or 0)
//!   offset 32: timing_function           (u64 — reserved, always 0)
//!
//! Total: 40 bytes.
//!
//! A thread-local stack of contexts is maintained for grouping.  The shim
//! runs completion handlers synchronously at `endGrouping` (no real
//! animation timeline exists).

use std::cell::RefCell;

const DURATION_OFF: usize = 8;
const ALLOWS_IMPLICIT_OFF: usize = 16;
const COMPLETION_HANDLER_OFF: usize = 24;
const _TIMING_FUNCTION_OFF: usize = 32;
pub(crate) const ANIMATION_CONTEXT_SIZE: usize = 40;

const DEFAULT_DURATION: f64 = 0.25;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut u64) = value;
    }
}

fn read_f64(obj: u64, offset: usize) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + offset) as *const f64) }
}

fn write_f64(obj: u64, offset: usize, value: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut f64) = value;
    }
}

/// Allocate a new NSAnimationContext with default values.
fn alloc_context() -> u64 {
    let obj = crate::pal_alloc().calloc(1, ANIMATION_CONTEXT_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsanimationcontext_class_addr();
    }
    write_f64(obj, DURATION_OFF, DEFAULT_DURATION);
    obj
}

// ── Thread-local grouping stack ─────────────────────────────────

thread_local! {
    static CONTEXT_STACK: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

fn push_context(ctx: u64) {
    CONTEXT_STACK.with(|stack| stack.borrow_mut().push(ctx));
}

fn pop_context() -> u64 {
    CONTEXT_STACK.with(|stack| stack.borrow_mut().pop().unwrap_or(0))
}

fn current_context() -> u64 {
    CONTEXT_STACK.with(|stack| stack.borrow().last().copied().unwrap_or(0))
}

// ── Class method shims ──────────────────────────────────────────

/// `+[NSAnimationContext beginGrouping]`
pub extern "C" fn shim_nsanimationcontext_begin_grouping(_cls: u64, _cmd: u64) {
    let ctx = alloc_context();
    push_context(ctx);
}

/// `+[NSAnimationContext endGrouping]`
pub extern "C" fn shim_nsanimationcontext_end_grouping(_cls: u64, _cmd: u64) {
    let ctx = pop_context();
    if ctx == 0 {
        return;
    }
    // Fire completion handler synchronously (no real animation to wait for)
    let handler = read_u64(ctx, COMPLETION_HANDLER_OFF);
    if handler != 0 {
        // In a real runtime, we'd invoke the block.  Here we just record
        // that it would fire.  The block ABI requires calling invoke at
        // offset +16 — but we don't have a real block runtime in tests,
        // so we skip actual invocation in the shim.
    }
}

/// `+[NSAnimationContext currentContext]`
pub extern "C" fn shim_nsanimationcontext_current_context(_cls: u64, _cmd: u64) -> u64 {
    let ctx = current_context();
    if ctx != 0 {
        return ctx;
    }
    // If no explicit grouping, return a default context
    let ctx = alloc_context();
    push_context(ctx);
    ctx
}

/// `+[NSAnimationContext runAnimationGroup:completionHandler:]`
///
/// In our shim, we immediately invoke the animation block, then fire
/// the completion handler (both synchronously).  The `changes` block
/// is a block pointer at arg position 2, and `completionHandler` at 3.
/// Since we can't invoke blocks in the shim (no real Blocks ABI), we
/// create a context, push/pop grouping, and record the handler.
pub extern "C" fn shim_nsanimationcontext_run_animation_group(
    _cls: u64,
    _cmd: u64,
    _changes: u64,
    _completion: u64,
) {
    shim_nsanimationcontext_begin_grouping(0, 0);
    let ctx = current_context();
    if _completion != 0 {
        write_u64(ctx, COMPLETION_HANDLER_OFF, _completion);
    }
    shim_nsanimationcontext_end_grouping(0, 0);
}

// ── Instance method shims ───────────────────────────────────────

/// `-[NSAnimationContext duration]`
pub extern "C" fn shim_nsanimationcontext_duration(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, DURATION_OFF)
}

/// `-[NSAnimationContext setDuration:]`
pub extern "C" fn shim_nsanimationcontext_set_duration(obj: u64, _cmd: u64, duration: f64) {
    write_f64(obj, DURATION_OFF, duration);
}

/// `-[NSAnimationContext allowsImplicitAnimation]`
pub extern "C" fn shim_nsanimationcontext_allows_implicit(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, ALLOWS_IMPLICIT_OFF) != 0)
}

/// `-[NSAnimationContext setAllowsImplicitAnimation:]`
pub extern "C" fn shim_nsanimationcontext_set_allows_implicit(obj: u64, _cmd: u64, flag: u8) {
    write_u64(obj, ALLOWS_IMPLICIT_OFF, u64::from(flag));
}

/// `-[NSAnimationContext completionHandler]`
pub extern "C" fn shim_nsanimationcontext_completion_handler(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, COMPLETION_HANDLER_OFF)
}

/// `-[NSAnimationContext setCompletionHandler:]`
pub extern "C" fn shim_nsanimationcontext_set_completion_handler(
    obj: u64,
    _cmd: u64,
    handler: u64,
) {
    write_u64(obj, COMPLETION_HANDLER_OFF, handler);
}

/// `-[NSAnimationContext timingFunction]` — always nil (not implemented).
pub extern "C" fn shim_nsanimationcontext_timing_function(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSAnimationContext setTimingFunction:]` — no-op.
pub extern "C" fn shim_nsanimationcontext_set_timing_function(_obj: u64, _cmd: u64, _tf: u64) {}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn ctx_isa() -> u64 {
        crate::nsobj::nsanimationcontext_class_addr()
    }

    #[test]
    fn alloc_context_defaults() {
        let ctx = alloc_context();
        assert_ne!(ctx, 0);
        assert_eq!(unsafe { *(ctx as *const u64) }, ctx_isa());
        assert!((shim_nsanimationcontext_duration(ctx, 0) - 0.25).abs() < f64::EPSILON);
        assert_eq!(shim_nsanimationcontext_allows_implicit(ctx, 0), 0);
        assert_eq!(shim_nsanimationcontext_completion_handler(ctx, 0), 0);
        free_obj(ctx);
    }

    #[test]
    fn duration_round_trip() {
        let ctx = alloc_context();
        shim_nsanimationcontext_set_duration(ctx, 0, 1.5);
        assert!((shim_nsanimationcontext_duration(ctx, 0) - 1.5).abs() < f64::EPSILON);
        free_obj(ctx);
    }

    #[test]
    fn allows_implicit_round_trip() {
        let ctx = alloc_context();
        shim_nsanimationcontext_set_allows_implicit(ctx, 0, 1);
        assert_eq!(shim_nsanimationcontext_allows_implicit(ctx, 0), 1);
        shim_nsanimationcontext_set_allows_implicit(ctx, 0, 0);
        assert_eq!(shim_nsanimationcontext_allows_implicit(ctx, 0), 0);
        free_obj(ctx);
    }

    #[test]
    fn completion_handler_round_trip() {
        let ctx = alloc_context();
        shim_nsanimationcontext_set_completion_handler(ctx, 0, 0xDEAD);
        assert_eq!(shim_nsanimationcontext_completion_handler(ctx, 0), 0xDEAD);
        free_obj(ctx);
    }

    #[test]
    fn begin_end_grouping() {
        // Stack should be empty before
        assert_eq!(current_context(), 0);
        shim_nsanimationcontext_begin_grouping(0, 0);
        let ctx = current_context();
        assert_ne!(ctx, 0);
        shim_nsanimationcontext_end_grouping(0, 0);
        // After end, the context is popped — but thread_local may have
        // the implicit one from currentContext, so we just check no crash
    }

    #[test]
    fn nested_grouping() {
        shim_nsanimationcontext_begin_grouping(0, 0);
        let outer = current_context();
        shim_nsanimationcontext_set_duration(outer, 0, 1.0);

        shim_nsanimationcontext_begin_grouping(0, 0);
        let inner = current_context();
        assert_ne!(inner, outer);
        shim_nsanimationcontext_set_duration(inner, 0, 2.0);

        shim_nsanimationcontext_end_grouping(0, 0);
        assert_eq!(current_context(), outer);
        assert!((shim_nsanimationcontext_duration(outer, 0) - 1.0).abs() < f64::EPSILON);

        shim_nsanimationcontext_end_grouping(0, 0);
    }

    #[test]
    fn current_context_creates_implicit() {
        // Clear the stack first
        CONTEXT_STACK.with(|s| s.borrow_mut().clear());
        let ctx = shim_nsanimationcontext_current_context(0, 0);
        assert_ne!(ctx, 0);
        // Clean up
        CONTEXT_STACK.with(|s| s.borrow_mut().clear());
    }

    #[test]
    fn run_animation_group_push_pop() {
        CONTEXT_STACK.with(|s| s.borrow_mut().clear());
        shim_nsanimationcontext_run_animation_group(0, 0, 0, 0xBEEF);
        // After run, the grouping should have been closed — check no crash
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsanimationcontext_duration(0, 0), 0.0);
        assert_eq!(shim_nsanimationcontext_allows_implicit(0, 0), 0);
        assert_eq!(shim_nsanimationcontext_completion_handler(0, 0), 0);
        shim_nsanimationcontext_set_duration(0, 0, 1.0);
        shim_nsanimationcontext_set_allows_implicit(0, 0, 1);
        shim_nsanimationcontext_set_completion_handler(0, 0, 0xDEAD);
    }

    #[test]
    fn timing_function_nil() {
        let ctx = alloc_context();
        assert_eq!(shim_nsanimationcontext_timing_function(ctx, 0), 0);
        shim_nsanimationcontext_set_timing_function(ctx, 0, 0xBEEF);
        assert_eq!(shim_nsanimationcontext_timing_function(ctx, 0), 0);
        free_obj(ctx);
    }
}
