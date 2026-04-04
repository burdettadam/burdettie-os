//! NSUndoManager shim: undo/redo stack with action recording.
//!
//! Object layout (56 bytes):
//!
//!   offset  0: isa              (u64)
//!   offset  8: undo_stack_ptr   (u64 — pointer to Vec<UndoAction> on heap)
//!   offset 16: redo_stack_ptr   (u64 — pointer to Vec<UndoAction> on heap)
//!   offset 24: group_depth      (u64 — nesting depth, 0 = no open group)
//!   offset 32: groups_by_event  (u64 — bool, default 1)
//!   offset 40: action_name      (u64 — NSString* for current undo action, or 0)
//!   offset 48: is_undoing       (u64 — bool, 1 while inside undo)
//!
//! Total: 56 bytes.
//!
//! The undo/redo stacks are stored as Rust Vec<UndoAction> behind raw
//! pointers.  Each action records (target, selector_name, object).

const UNDO_STACK_OFF: usize = 8;
const REDO_STACK_OFF: usize = 16;
const GROUP_DEPTH_OFF: usize = 24;
const GROUPS_BY_EVENT_OFF: usize = 32;
const ACTION_NAME_OFF: usize = 40;
const IS_UNDOING_OFF: usize = 48;
pub(crate) const UNDO_MANAGER_SIZE: usize = 56;

/// A recorded undo action.
struct UndoAction {
    target: u64,
    selector: u64,
    object: u64,
}

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + offset) as *mut u64) = value; }
}

fn undo_stack(obj: u64) -> &'static mut Vec<UndoAction> {
    let ptr = read_u64(obj, UNDO_STACK_OFF) as *mut Vec<UndoAction>;
    unsafe { &mut *ptr }
}

fn redo_stack(obj: u64) -> &'static mut Vec<UndoAction> {
    let ptr = read_u64(obj, REDO_STACK_OFF) as *mut Vec<UndoAction>;
    unsafe { &mut *ptr }
}

/// Allocate a new NSUndoManager.
pub(crate) fn alloc_undo_manager() -> u64 {
    let obj = crate::pal_alloc().calloc(1, UNDO_MANAGER_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = crate::nsobj::nsundomanager_class_addr(); }
    // Allocate undo/redo stacks on the heap
    let undo_vec = Box::into_raw(Box::new(Vec::<UndoAction>::new()));
    let redo_vec = Box::into_raw(Box::new(Vec::<UndoAction>::new()));
    write_u64(obj, UNDO_STACK_OFF, undo_vec as u64);
    write_u64(obj, REDO_STACK_OFF, redo_vec as u64);
    write_u64(obj, GROUPS_BY_EVENT_OFF, 1); // default: groups by event
    obj
}

// ── Instance method shims ───────────────────────────────────────

/// `-[NSUndoManager registerUndoWithTarget:selector:object:]`
pub extern "C" fn shim_nsundomanager_register_undo(
    obj: u64, _cmd: u64, target: u64, selector: u64, object: u64,
) {
    if obj == 0 { return; }
    let action = UndoAction { target, selector, object };
    let is_undoing = read_u64(obj, IS_UNDOING_OFF) != 0;
    if is_undoing {
        redo_stack(obj).push(action);
    } else {
        undo_stack(obj).push(action);
        // Clear redo stack when recording a new undo action (not during undo)
        redo_stack(obj).clear();
    }
}

/// `-[NSUndoManager undo]`
///
/// Pops the last undo action.  In our shim we don't actually invoke the
/// target/selector — we just move the action to the redo stack.
pub extern "C" fn shim_nsundomanager_undo(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let action = undo_stack(obj).pop();
    if let Some(a) = action {
        write_u64(obj, IS_UNDOING_OFF, 1);
        // In a real runtime, we'd `objc_msgSend(a.target, a.selector, a.object)`.
        // Move the action to redo stack instead.
        redo_stack(obj).push(a);
        write_u64(obj, IS_UNDOING_OFF, 0);
    }
}

/// `-[NSUndoManager redo]`
pub extern "C" fn shim_nsundomanager_redo(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let action = redo_stack(obj).pop();
    if let Some(a) = action {
        // Move back to undo stack
        undo_stack(obj).push(a);
    }
}

/// `-[NSUndoManager canUndo]`
pub extern "C" fn shim_nsundomanager_can_undo(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    u8::from(!undo_stack(obj).is_empty())
}

/// `-[NSUndoManager canRedo]`
pub extern "C" fn shim_nsundomanager_can_redo(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    u8::from(!redo_stack(obj).is_empty())
}

/// `-[NSUndoManager undoActionName]`
pub extern "C" fn shim_nsundomanager_undo_action_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let name = read_u64(obj, ACTION_NAME_OFF);
    if name != 0 { return name; }
    // Return empty NSString
    crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr())
}

/// `-[NSUndoManager redoActionName]`
pub extern "C" fn shim_nsundomanager_redo_action_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let name = read_u64(obj, ACTION_NAME_OFF);
    if name != 0 { return name; }
    crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr())
}

/// `-[NSUndoManager setActionName:]`
pub extern "C" fn shim_nsundomanager_set_action_name(obj: u64, _cmd: u64, name: u64) {
    write_u64(obj, ACTION_NAME_OFF, name);
}

/// `-[NSUndoManager removeAllActions]`
pub extern "C" fn shim_nsundomanager_remove_all_actions(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    undo_stack(obj).clear();
    redo_stack(obj).clear();
    write_u64(obj, ACTION_NAME_OFF, 0);
}

/// `-[NSUndoManager removeAllActionsWithTarget:]`
pub extern "C" fn shim_nsundomanager_remove_all_actions_with_target(
    obj: u64, _cmd: u64, target: u64,
) {
    if obj == 0 { return; }
    undo_stack(obj).retain(|a| a.target != target);
    redo_stack(obj).retain(|a| a.target != target);
}

/// `-[NSUndoManager groupsByEvent]`
pub extern "C" fn shim_nsundomanager_groups_by_event(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, GROUPS_BY_EVENT_OFF) != 0)
}

/// `-[NSUndoManager setGroupsByEvent:]`
pub extern "C" fn shim_nsundomanager_set_groups_by_event(obj: u64, _cmd: u64, flag: u8) {
    write_u64(obj, GROUPS_BY_EVENT_OFF, u64::from(flag));
}

/// `-[NSUndoManager beginUndoGrouping]`
pub extern "C" fn shim_nsundomanager_begin_undo_grouping(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let depth = read_u64(obj, GROUP_DEPTH_OFF);
    write_u64(obj, GROUP_DEPTH_OFF, depth + 1);
}

/// `-[NSUndoManager endUndoGrouping]`
pub extern "C" fn shim_nsundomanager_end_undo_grouping(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let depth = read_u64(obj, GROUP_DEPTH_OFF);
    if depth > 0 {
        write_u64(obj, GROUP_DEPTH_OFF, depth - 1);
    }
}

/// `-[NSUndoManager groupingLevel]`
pub extern "C" fn shim_nsundomanager_grouping_level(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, GROUP_DEPTH_OFF)
}

/// `-[NSUndoManager isUndoing]`
pub extern "C" fn shim_nsundomanager_is_undoing(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, IS_UNDOING_OFF) != 0)
}

/// `-[NSUndoManager isRedoing]` — always false in this shim.
pub extern "C" fn shim_nsundomanager_is_redoing(_obj: u64, _cmd: u64) -> u8 { 0 }

/// `-[NSUndoManager levelsOfUndo]` — returns 0 (unlimited).
pub extern "C" fn shim_nsundomanager_levels_of_undo(_obj: u64, _cmd: u64) -> u64 { 0 }

/// `-[NSUndoManager setLevelsOfUndo:]` — no-op.
pub extern "C" fn shim_nsundomanager_set_levels_of_undo(_obj: u64, _cmd: u64, _levels: u64) {}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn um_isa() -> u64 {
        crate::nsobj::nsundomanager_class_addr()
    }

    fn make_um() -> u64 {
        alloc_undo_manager()
    }

    fn drop_um(obj: u64) {
        // Must free the heap-allocated Vecs first
        if obj == 0 { return; }
        let undo_ptr = read_u64(obj, UNDO_STACK_OFF) as *mut Vec<UndoAction>;
        let redo_ptr = read_u64(obj, REDO_STACK_OFF) as *mut Vec<UndoAction>;
        if !undo_ptr.is_null() { unsafe { drop(Box::from_raw(undo_ptr)); } }
        if !redo_ptr.is_null() { unsafe { drop(Box::from_raw(redo_ptr)); } }
        free_obj(obj);
    }

    #[test]
    fn alloc_undo_manager_basic() {
        let um = make_um();
        assert_ne!(um, 0);
        assert_eq!(unsafe { *(um as *const u64) }, um_isa());
        assert_eq!(shim_nsundomanager_can_undo(um, 0), 0);
        assert_eq!(shim_nsundomanager_can_redo(um, 0), 0);
        assert_eq!(shim_nsundomanager_groups_by_event(um, 0), 1);
        drop_um(um);
    }

    #[test]
    fn register_undo_then_can_undo() {
        let um = make_um();
        assert_eq!(shim_nsundomanager_can_undo(um, 0), 0);
        shim_nsundomanager_register_undo(um, 0, 0xAA, 0, 0xBB);
        assert_eq!(shim_nsundomanager_can_undo(um, 0), 1);
        assert_eq!(shim_nsundomanager_can_redo(um, 0), 0);
        drop_um(um);
    }

    #[test]
    fn undo_moves_to_redo() {
        let um = make_um();
        shim_nsundomanager_register_undo(um, 0, 0xAA, 0, 0xBB);
        shim_nsundomanager_undo(um, 0);
        assert_eq!(shim_nsundomanager_can_undo(um, 0), 0);
        assert_eq!(shim_nsundomanager_can_redo(um, 0), 1);
        drop_um(um);
    }

    #[test]
    fn redo_moves_back_to_undo() {
        let um = make_um();
        shim_nsundomanager_register_undo(um, 0, 0xAA, 0, 0xBB);
        shim_nsundomanager_undo(um, 0);
        shim_nsundomanager_redo(um, 0);
        assert_eq!(shim_nsundomanager_can_undo(um, 0), 1);
        assert_eq!(shim_nsundomanager_can_redo(um, 0), 0);
        drop_um(um);
    }

    #[test]
    fn new_action_clears_redo() {
        let um = make_um();
        shim_nsundomanager_register_undo(um, 0, 0xAA, 0, 0xBB);
        shim_nsundomanager_undo(um, 0);
        assert_eq!(shim_nsundomanager_can_redo(um, 0), 1);
        // Register new action — should clear redo
        shim_nsundomanager_register_undo(um, 0, 0xCC, 0, 0xDD);
        assert_eq!(shim_nsundomanager_can_redo(um, 0), 0);
        drop_um(um);
    }

    #[test]
    fn remove_all_actions() {
        let um = make_um();
        shim_nsundomanager_register_undo(um, 0, 0xAA, 0, 0);
        shim_nsundomanager_register_undo(um, 0, 0xBB, 0, 0);
        shim_nsundomanager_remove_all_actions(um, 0);
        assert_eq!(shim_nsundomanager_can_undo(um, 0), 0);
        assert_eq!(shim_nsundomanager_can_redo(um, 0), 0);
        drop_um(um);
    }

    #[test]
    fn remove_all_actions_with_target() {
        let um = make_um();
        shim_nsundomanager_register_undo(um, 0, 0xAA, 0, 0);
        shim_nsundomanager_register_undo(um, 0, 0xBB, 0, 0);
        shim_nsundomanager_remove_all_actions_with_target(um, 0, 0xAA);
        assert_eq!(shim_nsundomanager_can_undo(um, 0), 1);
        // Only 0xBB should remain
        shim_nsundomanager_undo(um, 0);
        assert_eq!(shim_nsundomanager_can_undo(um, 0), 0);
        drop_um(um);
    }

    #[test]
    fn groups_by_event_round_trip() {
        let um = make_um();
        shim_nsundomanager_set_groups_by_event(um, 0, 0);
        assert_eq!(shim_nsundomanager_groups_by_event(um, 0), 0);
        shim_nsundomanager_set_groups_by_event(um, 0, 1);
        assert_eq!(shim_nsundomanager_groups_by_event(um, 0), 1);
        drop_um(um);
    }

    #[test]
    fn grouping_level() {
        let um = make_um();
        assert_eq!(shim_nsundomanager_grouping_level(um, 0), 0);
        shim_nsundomanager_begin_undo_grouping(um, 0);
        assert_eq!(shim_nsundomanager_grouping_level(um, 0), 1);
        shim_nsundomanager_begin_undo_grouping(um, 0);
        assert_eq!(shim_nsundomanager_grouping_level(um, 0), 2);
        shim_nsundomanager_end_undo_grouping(um, 0);
        assert_eq!(shim_nsundomanager_grouping_level(um, 0), 1);
        shim_nsundomanager_end_undo_grouping(um, 0);
        assert_eq!(shim_nsundomanager_grouping_level(um, 0), 0);
        drop_um(um);
    }

    #[test]
    fn end_grouping_at_zero_no_underflow() {
        let um = make_um();
        shim_nsundomanager_end_undo_grouping(um, 0);
        assert_eq!(shim_nsundomanager_grouping_level(um, 0), 0);
        drop_um(um);
    }

    #[test]
    fn levels_of_undo_default_unlimited() {
        let um = make_um();
        assert_eq!(shim_nsundomanager_levels_of_undo(um, 0), 0);
        shim_nsundomanager_set_levels_of_undo(um, 0, 10);
        // Still returns 0 (no-op setter)
        assert_eq!(shim_nsundomanager_levels_of_undo(um, 0), 0);
        drop_um(um);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsundomanager_can_undo(0, 0), 0);
        assert_eq!(shim_nsundomanager_can_redo(0, 0), 0);
        shim_nsundomanager_register_undo(0, 0, 0, 0, 0);
        shim_nsundomanager_undo(0, 0);
        shim_nsundomanager_redo(0, 0);
        shim_nsundomanager_remove_all_actions(0, 0);
        shim_nsundomanager_begin_undo_grouping(0, 0);
        shim_nsundomanager_end_undo_grouping(0, 0);
    }

    #[test]
    fn set_action_name() {
        let um = make_um();
        shim_nsundomanager_set_action_name(um, 0, 0xDEAD);
        assert_eq!(read_u64(um, ACTION_NAME_OFF), 0xDEAD);
        drop_um(um);
    }
}
