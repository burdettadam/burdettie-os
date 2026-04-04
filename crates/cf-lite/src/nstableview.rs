//! NSTableView shim: ObjC data-source-driven table view.
//!
//! Object layout (72 bytes):
//!
//!   offset  0: isa              (u64)
//!   offset  8: superview        (u64 — parent NSView*)
//!   offset 16: window           (u64 — owning NSWindow*)
//!   offset 24: frame_x          (f64)
//!   offset 32: frame_y          (f64)
//!   offset 40: frame_w          (f64)
//!   offset 48: frame_h          (f64)
//!   offset 56: data_source      (u64 — id<NSTableViewDataSource>, or 0)
//!   offset 64: delegate         (u64 — id<NSTableViewDelegate>, or 0)
//!
//! Total: 72 bytes.
//!
//! Columns are tracked in a global HashMap keyed by the table view address.
//! `reloadData` currently marks the view as needing display.

use std::collections::HashMap;
use std::sync::Mutex;

const SUPERVIEW_OFF: usize = 8;
const _WINDOW_OFF: usize = 16;
const _FRAME_X_OFF: usize = 24;
const _FRAME_Y_OFF: usize = 32;
const _FRAME_W_OFF: usize = 40;
const _FRAME_H_OFF: usize = 48;
const DATA_SOURCE_OFF: usize = 56;
const DELEGATE_OFF: usize = 64;
pub(crate) const TABLE_VIEW_SIZE: usize = 72;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + offset) as *mut u64) = value; }
}

/// Per-table-view column list.
static TABLE_COLUMNS: Mutex<Option<HashMap<u64, Vec<u64>>>> = Mutex::new(None);

fn with_columns<F, R>(table: u64, f: F) -> R
where
    F: FnOnce(&mut Vec<u64>) -> R,
{
    let mut map = TABLE_COLUMNS.lock().unwrap();
    let map = map.get_or_insert_with(HashMap::new);
    let cols = map.entry(table).or_insert_with(Vec::new);
    f(cols)
}

/// Allocate a new NSTableView.
pub(crate) fn alloc_table_view() -> u64 {
    let obj = crate::pal_alloc().calloc(1, TABLE_VIEW_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = crate::nsobj::nstableview_class_addr(); }
    obj
}

// ── Instance method shims ───────────────────────────────────────

/// `-[NSTableView initWithFrame:]`
pub extern "C" fn shim_nstableview_init_with_frame(
    _obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, TABLE_VIEW_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nstableview_class_addr();
        *((obj as usize + _FRAME_X_OFF) as *mut f64) = x;
        *((obj as usize + _FRAME_Y_OFF) as *mut f64) = y;
        *((obj as usize + _FRAME_W_OFF) as *mut f64) = w;
        *((obj as usize + _FRAME_H_OFF) as *mut f64) = h;
    }
    obj
}

/// `-[NSTableView dataSource]`
pub extern "C" fn shim_nstableview_data_source(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DATA_SOURCE_OFF)
}

/// `-[NSTableView setDataSource:]`
pub extern "C" fn shim_nstableview_set_data_source(obj: u64, _cmd: u64, ds: u64) {
    write_u64(obj, DATA_SOURCE_OFF, ds);
}

/// `-[NSTableView delegate]`
pub extern "C" fn shim_nstableview_delegate(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DELEGATE_OFF)
}

/// `-[NSTableView setDelegate:]`
pub extern "C" fn shim_nstableview_set_delegate(obj: u64, _cmd: u64, del: u64) {
    write_u64(obj, DELEGATE_OFF, del);
}

/// `-[NSTableView numberOfRows]`
///
/// In a real implementation this queries the data source.  Our shim
/// returns 0 (no data source invocation — just structural plumbing).
pub extern "C" fn shim_nstableview_number_of_rows(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    0
}

/// `-[NSTableView numberOfColumns]`
pub extern "C" fn shim_nstableview_number_of_columns(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    with_columns(obj, |cols| cols.len() as u64)
}

/// `-[NSTableView addTableColumn:]`
pub extern "C" fn shim_nstableview_add_table_column(obj: u64, _cmd: u64, column: u64) {
    if obj == 0 || column == 0 { return; }
    with_columns(obj, |cols| {
        if !cols.contains(&column) {
            cols.push(column);
        }
    });
}

/// `-[NSTableView removeTableColumn:]`
pub extern "C" fn shim_nstableview_remove_table_column(obj: u64, _cmd: u64, column: u64) {
    if obj == 0 || column == 0 { return; }
    with_columns(obj, |cols| cols.retain(|&c| c != column));
}

/// `-[NSTableView tableColumns]` — returns an NSArray of columns.
pub extern "C" fn shim_nstableview_table_columns(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    with_columns(obj, |cols| crate::nsarray::make_nsarray(cols))
}

/// `-[NSTableView reloadData]` — marks view as needing display.
pub extern "C" fn shim_nstableview_reload_data(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    // In a real AppKit, reloadData would query the data source and rebuild
    // the table.  For our shim, just mark the view dirty.
    crate::layout_solver::set_needs_layout(obj, true);
}

/// `-[NSTableView superview]`
pub extern "C" fn shim_nstableview_superview(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, SUPERVIEW_OFF)
}

/// `-[NSTableView selectedRow]` — returns -1 (no selection).
pub extern "C" fn shim_nstableview_selected_row(_obj: u64, _cmd: u64) -> i64 {
    -1
}

/// `-[NSTableView selectRowIndexes:byExtendingSelection:]` — no-op.
pub extern "C" fn shim_nstableview_select_row_indexes(_obj: u64, _cmd: u64, _indexes: u64, _extend: u8) {}

/// `-[NSTableView deselectAll:]` — no-op.
pub extern "C" fn shim_nstableview_deselect_all(_obj: u64, _cmd: u64, _sender: u64) {}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn tv_isa() -> u64 {
        crate::nsobj::nstableview_class_addr()
    }

    fn make_tv() -> u64 {
        alloc_table_view()
    }

    #[test]
    fn alloc_table_view_basic() {
        let tv = make_tv();
        assert_ne!(tv, 0);
        assert_eq!(unsafe { *(tv as *const u64) }, tv_isa());
        assert_eq!(shim_nstableview_data_source(tv, 0), 0);
        assert_eq!(shim_nstableview_delegate(tv, 0), 0);
        free_obj(tv);
    }

    #[test]
    fn init_with_frame() {
        let tv = shim_nstableview_init_with_frame(0, 0, 10.0, 20.0, 300.0, 200.0);
        assert_ne!(tv, 0);
        assert_eq!(unsafe { *(tv as *const u64) }, tv_isa());
        free_obj(tv);
    }

    #[test]
    fn data_source_round_trip() {
        let tv = make_tv();
        shim_nstableview_set_data_source(tv, 0, 0xAAAA);
        assert_eq!(shim_nstableview_data_source(tv, 0), 0xAAAA);
        free_obj(tv);
    }

    #[test]
    fn delegate_round_trip() {
        let tv = make_tv();
        shim_nstableview_set_delegate(tv, 0, 0xBBBB);
        assert_eq!(shim_nstableview_delegate(tv, 0), 0xBBBB);
        free_obj(tv);
    }

    #[test]
    fn add_remove_table_column() {
        let tv = make_tv();
        assert_eq!(shim_nstableview_number_of_columns(tv, 0), 0);

        shim_nstableview_add_table_column(tv, 0, 0x100);
        assert_eq!(shim_nstableview_number_of_columns(tv, 0), 1);

        shim_nstableview_add_table_column(tv, 0, 0x200);
        assert_eq!(shim_nstableview_number_of_columns(tv, 0), 2);

        // Duplicate add is idempotent
        shim_nstableview_add_table_column(tv, 0, 0x100);
        assert_eq!(shim_nstableview_number_of_columns(tv, 0), 2);

        shim_nstableview_remove_table_column(tv, 0, 0x100);
        assert_eq!(shim_nstableview_number_of_columns(tv, 0), 1);

        // Clean up global state
        TABLE_COLUMNS.lock().unwrap().as_mut().unwrap().remove(&tv);
        free_obj(tv);
    }

    #[test]
    fn table_columns_returns_nsarray() {
        let tv = make_tv();
        shim_nstableview_add_table_column(tv, 0, 0x100);
        shim_nstableview_add_table_column(tv, 0, 0x200);

        let arr = shim_nstableview_table_columns(tv, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 2);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0x100);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0x200);

        TABLE_COLUMNS.lock().unwrap().as_mut().unwrap().remove(&tv);
        free_obj(tv);
    }

    #[test]
    fn number_of_rows_default_zero() {
        let tv = make_tv();
        assert_eq!(shim_nstableview_number_of_rows(tv, 0), 0);
        free_obj(tv);
    }

    #[test]
    fn reload_data_does_not_crash() {
        let tv = make_tv();
        shim_nstableview_reload_data(tv, 0);
        free_obj(tv);
    }

    #[test]
    fn selected_row_minus_one() {
        let tv = make_tv();
        assert_eq!(shim_nstableview_selected_row(tv, 0), -1);
        free_obj(tv);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstableview_data_source(0, 0), 0);
        assert_eq!(shim_nstableview_delegate(0, 0), 0);
        assert_eq!(shim_nstableview_number_of_rows(0, 0), 0);
        assert_eq!(shim_nstableview_number_of_columns(0, 0), 0);
        shim_nstableview_set_data_source(0, 0, 1);
        shim_nstableview_set_delegate(0, 0, 1);
        shim_nstableview_add_table_column(0, 0, 1);
        shim_nstableview_remove_table_column(0, 0, 1);
        shim_nstableview_reload_data(0, 0);
        shim_nstableview_select_row_indexes(0, 0, 0, 0);
        shim_nstableview_deselect_all(0, 0, 0);
    }

    #[test]
    fn select_deselect_no_op() {
        let tv = make_tv();
        shim_nstableview_select_row_indexes(tv, 0, 0xAA, 1);
        shim_nstableview_deselect_all(tv, 0, 0);
        assert_eq!(shim_nstableview_selected_row(tv, 0), -1);
        free_obj(tv);
    }
}
