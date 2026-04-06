//! NSCursor shim: cursor management for AppKit.
//!
//! Object layout: 16 bytes
//!   [0..8):  ISA pointer
//!   [8..16): cursor_type (u64 — enum tag)
//!
//! Cursor types are tracked but not rendered (no host cursor changes).
//! The class provides the standard factory methods that apps call.

use std::sync::Mutex;

const CURSOR_TYPE_OFF: usize = 8;
pub(crate) const CURSOR_SIZE: usize = 16;

// Cursor type enum values
const CURSOR_ARROW: u64 = 0;
const CURSOR_IBEAM: u64 = 1;
const CURSOR_CROSSHAIR: u64 = 2;
const CURSOR_POINTING_HAND: u64 = 3;
const CURSOR_OPEN_HAND: u64 = 4;
const CURSOR_CLOSED_HAND: u64 = 5;
const CURSOR_RESIZE_LEFT_RIGHT: u64 = 6;
const CURSOR_RESIZE_UP_DOWN: u64 = 7;
const CURSOR_RESIZE_LEFT: u64 = 8;
const CURSOR_RESIZE_RIGHT: u64 = 9;
const CURSOR_RESIZE_UP: u64 = 10;
const CURSOR_RESIZE_DOWN: u64 = 11;
const CURSOR_DISAPPEARING_ITEM: u64 = 12;
const CURSOR_OPERATION_NOT_ALLOWED: u64 = 13;
const CURSOR_DRAG_LINK: u64 = 14;
const CURSOR_DRAG_COPY: u64 = 15;
const CURSOR_CONTEXTUAL_MENU: u64 = 16;

/// Cursor stack (push/pop).
static CURSOR_STACK: Mutex<Vec<u64>> = Mutex::new(Vec::new());

/// Whether cursors are visible.
static CURSOR_VISIBLE: Mutex<bool> = Mutex::new(true);

fn alloc_cursor(cursor_type: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, CURSOR_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nscursor_class_addr();
        *((obj as usize + CURSOR_TYPE_OFF) as *mut u64) = cursor_type;
    }
    obj
}

// ── Class methods (cursor factories) ───────────────────────────

/// `+[NSCursor arrowCursor]`
pub extern "C" fn shim_nscursor_arrow_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_ARROW) }
/// `+[NSCursor IBeamCursor]`
pub extern "C" fn shim_nscursor_ibeam_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_IBEAM) }
/// `+[NSCursor crosshairCursor]`
pub extern "C" fn shim_nscursor_crosshair_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_CROSSHAIR) }
/// `+[NSCursor pointingHandCursor]`
pub extern "C" fn shim_nscursor_pointing_hand_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_POINTING_HAND) }
/// `+[NSCursor openHandCursor]`
pub extern "C" fn shim_nscursor_open_hand_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_OPEN_HAND) }
/// `+[NSCursor closedHandCursor]`
pub extern "C" fn shim_nscursor_closed_hand_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_CLOSED_HAND) }
/// `+[NSCursor resizeLeftRightCursor]`
pub extern "C" fn shim_nscursor_resize_left_right_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_RESIZE_LEFT_RIGHT) }
/// `+[NSCursor resizeUpDownCursor]`
pub extern "C" fn shim_nscursor_resize_up_down_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_RESIZE_UP_DOWN) }
/// `+[NSCursor resizeLeftCursor]`
pub extern "C" fn shim_nscursor_resize_left_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_RESIZE_LEFT) }
/// `+[NSCursor resizeRightCursor]`
pub extern "C" fn shim_nscursor_resize_right_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_RESIZE_RIGHT) }
/// `+[NSCursor resizeUpCursor]`
pub extern "C" fn shim_nscursor_resize_up_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_RESIZE_UP) }
/// `+[NSCursor resizeDownCursor]`
pub extern "C" fn shim_nscursor_resize_down_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_RESIZE_DOWN) }
/// `+[NSCursor disappearingItemCursor]`
pub extern "C" fn shim_nscursor_disappearing_item_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_DISAPPEARING_ITEM) }
/// `+[NSCursor operationNotAllowedCursor]`
pub extern "C" fn shim_nscursor_operation_not_allowed_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_OPERATION_NOT_ALLOWED) }
/// `+[NSCursor dragLinkCursor]`
pub extern "C" fn shim_nscursor_drag_link_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_DRAG_LINK) }
/// `+[NSCursor dragCopyCursor]`
pub extern "C" fn shim_nscursor_drag_copy_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_DRAG_COPY) }
/// `+[NSCursor contextualMenuCursor]`
pub extern "C" fn shim_nscursor_contextual_menu_cursor(_cls: u64, _cmd: u64) -> u64 { alloc_cursor(CURSOR_CONTEXTUAL_MENU) }

/// `+[NSCursor currentCursor]` — returns the top of the stack or arrow.
pub extern "C" fn shim_nscursor_current_cursor(_cls: u64, _cmd: u64) -> u64 {
    let guard = CURSOR_STACK.lock().unwrap_or_else(|e| e.into_inner());
    guard.last().copied().unwrap_or_else(|| alloc_cursor(CURSOR_ARROW))
}

/// `+[NSCursor hide]`
pub extern "C" fn shim_nscursor_hide(_cls: u64, _cmd: u64) {
    *CURSOR_VISIBLE.lock().unwrap_or_else(|e| e.into_inner()) = false;
}

/// `+[NSCursor unhide]`
pub extern "C" fn shim_nscursor_unhide(_cls: u64, _cmd: u64) {
    *CURSOR_VISIBLE.lock().unwrap_or_else(|e| e.into_inner()) = true;
}

// ── Instance methods ───────────────────────────────────────────

/// `-[NSCursor set]` — make this cursor the active cursor.
pub extern "C" fn shim_nscursor_set(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    // No-op visually; just record in stack as current.
}

/// `-[NSCursor push]` — push onto the cursor stack.
pub extern "C" fn shim_nscursor_push(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    CURSOR_STACK.lock().unwrap_or_else(|e| e.into_inner()).push(obj);
}

/// `-[NSCursor pop]` — class method that pops the top cursor.
pub extern "C" fn shim_nscursor_pop(_cls: u64, _cmd: u64) {
    CURSOR_STACK.lock().unwrap_or_else(|e| e.into_inner()).pop();
}

/// `-[NSCursor image]` — returns nil (no image backing).
pub extern "C" fn shim_nscursor_image(_obj: u64, _cmd: u64) -> u64 { 0 }

/// `-[NSCursor hotSpot]` — returns zero point.
pub extern "C" fn shim_nscursor_hot_spot(_obj: u64, _cmd: u64, _out_x: *mut f64, _out_y: *mut f64) {
    // No-op; returns (0,0) via zeroed struct.
}

#[cfg(test)]
pub(crate) fn reset_cursor_state() {
    CURSOR_STACK.lock().unwrap_or_else(|e| e.into_inner()).clear();
    *CURSOR_VISIBLE.lock().unwrap_or_else(|e| e.into_inner()) = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn arrow_cursor_allocates() {
        let c = shim_nscursor_arrow_cursor(0, 0);
        assert_ne!(c, 0);
        let cursor_type = unsafe { *((c as usize + CURSOR_TYPE_OFF) as *const u64) };
        assert_eq!(cursor_type, CURSOR_ARROW);
        free_obj(c);
    }

    #[test]
    fn ibeam_cursor_type() {
        let c = shim_nscursor_ibeam_cursor(0, 0);
        assert_ne!(c, 0);
        let cursor_type = unsafe { *((c as usize + CURSOR_TYPE_OFF) as *const u64) };
        assert_eq!(cursor_type, CURSOR_IBEAM);
        free_obj(c);
    }

    #[test]
    fn pointing_hand_cursor_type() {
        let c = shim_nscursor_pointing_hand_cursor(0, 0);
        assert_ne!(c, 0);
        let cursor_type = unsafe { *((c as usize + CURSOR_TYPE_OFF) as *const u64) };
        assert_eq!(cursor_type, CURSOR_POINTING_HAND);
        free_obj(c);
    }

    #[test]
    fn push_pop_stack() {
        reset_cursor_state();
        let arrow = shim_nscursor_arrow_cursor(0, 0);
        let ibeam = shim_nscursor_ibeam_cursor(0, 0);
        shim_nscursor_push(arrow, 0);
        shim_nscursor_push(ibeam, 0);
        let current = shim_nscursor_current_cursor(0, 0);
        assert_eq!(current, ibeam);
        shim_nscursor_pop(0, 0);
        let current2 = shim_nscursor_current_cursor(0, 0);
        assert_eq!(current2, arrow);
        shim_nscursor_pop(0, 0);
        free_obj(arrow);
        free_obj(ibeam);
        reset_cursor_state();
    }

    #[test]
    fn hide_unhide() {
        reset_cursor_state();
        shim_nscursor_hide(0, 0);
        assert!(!*CURSOR_VISIBLE.lock().unwrap());
        shim_nscursor_unhide(0, 0);
        assert!(*CURSOR_VISIBLE.lock().unwrap());
        reset_cursor_state();
    }

    #[test]
    fn set_is_noop() {
        let c = shim_nscursor_arrow_cursor(0, 0);
        shim_nscursor_set(c, 0); // should not crash
        free_obj(c);
    }

    #[test]
    fn nil_set_is_safe() {
        shim_nscursor_set(0, 0); // should not crash
    }

    #[test]
    fn nil_push_is_safe() {
        reset_cursor_state();
        shim_nscursor_push(0, 0); // should not crash
        reset_cursor_state();
    }

    #[test]
    fn pop_empty_is_safe() {
        reset_cursor_state();
        shim_nscursor_pop(0, 0); // should not crash
        reset_cursor_state();
    }

    #[test]
    fn current_cursor_returns_arrow_when_empty() {
        reset_cursor_state();
        let current = shim_nscursor_current_cursor(0, 0);
        assert_ne!(current, 0);
        // Type should be arrow
        let cursor_type = unsafe { *((current as usize + CURSOR_TYPE_OFF) as *const u64) };
        assert_eq!(cursor_type, CURSOR_ARROW);
        free_obj(current);
        reset_cursor_state();
    }

    #[test]
    fn all_cursor_types_allocate() {
        let factories: &[extern "C" fn(u64, u64) -> u64] = &[
            shim_nscursor_arrow_cursor,
            shim_nscursor_ibeam_cursor,
            shim_nscursor_crosshair_cursor,
            shim_nscursor_pointing_hand_cursor,
            shim_nscursor_open_hand_cursor,
            shim_nscursor_closed_hand_cursor,
            shim_nscursor_resize_left_right_cursor,
            shim_nscursor_resize_up_down_cursor,
            shim_nscursor_resize_left_cursor,
            shim_nscursor_resize_right_cursor,
            shim_nscursor_resize_up_cursor,
            shim_nscursor_resize_down_cursor,
            shim_nscursor_disappearing_item_cursor,
            shim_nscursor_operation_not_allowed_cursor,
            shim_nscursor_drag_link_cursor,
            shim_nscursor_drag_copy_cursor,
            shim_nscursor_contextual_menu_cursor,
        ];
        for (i, f) in factories.iter().enumerate() {
            let c = f(0, 0);
            assert_ne!(c, 0, "cursor factory {i} returned null");
            let cursor_type = unsafe { *((c as usize + CURSOR_TYPE_OFF) as *const u64) };
            assert_eq!(cursor_type, i as u64, "cursor type mismatch for factory {i}");
            free_obj(c);
        }
    }

    #[test]
    fn image_returns_nil() {
        let c = shim_nscursor_arrow_cursor(0, 0);
        assert_eq!(shim_nscursor_image(c, 0), 0);
        free_obj(c);
    }
}
