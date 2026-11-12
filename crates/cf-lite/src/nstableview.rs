//! NSTableView shim: ObjC data-source-driven table view.
//!
//! Object layout (96 bytes):
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
//!   offset 72: selected_row     (i64 — -1 = no selection)
//!   offset 80: row_height       (f64 — default 22.0)
//!   offset 88: header_height    (f64 — default 17.0)
//!
//! Total: 96 bytes.
//!
//! Columns are tracked in a global HashMap keyed by the table view address.
//! `reloadData` queries the data source for row count.
//! `drawRect:` renders header, alternating rows, selection highlight, and cell text.

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
const SELECTED_ROW_OFF: usize = 72;
const ROW_HEIGHT_OFF: usize = 80;
const HEADER_HEIGHT_OFF: usize = 88;
pub(crate) const TABLE_VIEW_SIZE: usize = 96;

const DEFAULT_ROW_HEIGHT: f64 = 22.0;
const DEFAULT_HEADER_HEIGHT: f64 = 17.0;

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

fn read_i64(obj: u64, offset: usize) -> i64 {
    if obj == 0 {
        return -1;
    }
    unsafe { *((obj as usize + offset) as *const i64) }
}

fn write_i64(obj: u64, offset: usize, value: i64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut i64) = value;
    }
}

/// Allocate a new NSTableView.
pub(crate) fn alloc_table_view() -> u64 {
    let obj = crate::pal_alloc().calloc(1, TABLE_VIEW_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nstableview_class_addr();
        *((obj as usize + SELECTED_ROW_OFF) as *mut i64) = -1;
        *((obj as usize + ROW_HEIGHT_OFF) as *mut f64) = DEFAULT_ROW_HEIGHT;
        *((obj as usize + HEADER_HEIGHT_OFF) as *mut f64) = DEFAULT_HEADER_HEIGHT;
    }
    obj
}

// ── Instance method shims ───────────────────────────────────────

/// `-[NSTableView initWithFrame:]`
pub extern "C" fn shim_nstableview_init_with_frame(
    _obj: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, TABLE_VIEW_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nstableview_class_addr();
        *((obj as usize + _FRAME_X_OFF) as *mut f64) = x;
        *((obj as usize + _FRAME_Y_OFF) as *mut f64) = y;
        *((obj as usize + _FRAME_W_OFF) as *mut f64) = w;
        *((obj as usize + _FRAME_H_OFF) as *mut f64) = h;
        *((obj as usize + SELECTED_ROW_OFF) as *mut i64) = -1;
        *((obj as usize + ROW_HEIGHT_OFF) as *mut f64) = DEFAULT_ROW_HEIGHT;
        *((obj as usize + HEADER_HEIGHT_OFF) as *mut f64) = DEFAULT_HEADER_HEIGHT;
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
/// Queries the data source via `numberOfRowsInTableView:`.
/// Returns 0 when no data source is set or method is absent.
pub extern "C" fn shim_nstableview_number_of_rows(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    query_data_source_row_count(obj) as u64
}

/// Dispatch `numberOfRowsInTableView:` to the data source.
fn query_data_source_row_count(obj: u64) -> i64 {
    let ds = read_u64(obj, DATA_SOURCE_OFF);
    if ds == 0 {
        return 0;
    }
    let result = crate::nsapplication::dispatch_u64_selector_named_to_target(
        ds,
        "numberOfRowsInTableView:",
        obj,
    );
    if result == u64::MAX {
        0
    } else {
        result as i64
    }
}

/// Dispatch `tableView:objectValueForTableColumn:row:` to the data source.
/// Returns the NSString (or object) pointer, or 0.
fn query_data_source_object_value(obj: u64, column: u64, row: i64) -> u64 {
    let ds = read_u64(obj, DATA_SOURCE_OFF);
    if ds == 0 {
        return 0;
    }
    dispatch_u64_selector_3arg(
        ds,
        "tableView:objectValueForTableColumn:row:",
        obj,
        column,
        row as u64,
    )
}

/// Dispatch a u64-returning selector with 3 arguments through the ObjC runtime.
fn dispatch_u64_selector_3arg(target: u64, selector: &str, a1: u64, a2: u64, a3: u64) -> u64 {
    if target == 0 || selector.is_empty() {
        return 0;
    }
    let isa = unsafe { *(target as *const u64) };
    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return 0;
    };
    let imp = match rt.msg_lookup(isa, selector) {
        Some(p) => p,
        None => return 0,
    };
    let sel_ptr = rt.selector_ptr(selector).unwrap_or(0) as u64;
    drop(guard);
    let func: extern "C" fn(u64, u64, u64, u64, u64) -> u64 = unsafe { std::mem::transmute(imp) };
    func(target, sel_ptr, a1, a2, a3)
}

/// `-[NSTableView numberOfColumns]`
pub extern "C" fn shim_nstableview_number_of_columns(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    with_columns(obj, |cols| cols.len() as u64)
}

/// `-[NSTableView addTableColumn:]`
pub extern "C" fn shim_nstableview_add_table_column(obj: u64, _cmd: u64, column: u64) {
    if obj == 0 || column == 0 {
        return;
    }
    with_columns(obj, |cols| {
        if !cols.contains(&column) {
            cols.push(column);
        }
    });
}

/// `-[NSTableView removeTableColumn:]`
pub extern "C" fn shim_nstableview_remove_table_column(obj: u64, _cmd: u64, column: u64) {
    if obj == 0 || column == 0 {
        return;
    }
    with_columns(obj, |cols| cols.retain(|&c| c != column));
}

/// `-[NSTableView tableColumns]` — returns an NSArray of columns.
pub extern "C" fn shim_nstableview_table_columns(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    with_columns(obj, |cols| crate::nsarray::make_nsarray(cols))
}

/// Returns the first column address for a table/outline view, or 0.
pub(crate) fn first_column_addr(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    with_columns(obj, |cols| cols.first().copied().unwrap_or(0))
}

/// `-[NSTableView reloadData]` — queries data source, clamps selection, marks dirty.
pub extern "C" fn shim_nstableview_reload_data(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    // Clamp selection to new row count.
    let row_count = query_data_source_row_count(obj);
    let sel_row = read_i64(obj, SELECTED_ROW_OFF);
    if sel_row >= row_count {
        write_i64(obj, SELECTED_ROW_OFF, -1);
    }
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
    crate::layout_solver::set_needs_layout(obj, true);
}

/// `-[NSTableView superview]`
pub extern "C" fn shim_nstableview_superview(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, SUPERVIEW_OFF)
}

/// `-[NSTableView selectedRow]` — returns selected row index, or -1.
pub extern "C" fn shim_nstableview_selected_row(obj: u64, _cmd: u64) -> i64 {
    read_i64(obj, SELECTED_ROW_OFF)
}

/// `-[NSTableView selectRowIndexes:byExtendingSelection:]`
///
/// Simplified: treats the index set as a single row index.
pub extern "C" fn shim_nstableview_select_row_indexes(
    obj: u64,
    _cmd: u64,
    indexes: u64,
    _extend: u8,
) {
    if obj == 0 {
        return;
    }
    // Treat indexes as single row index (NSIndexSet first index).
    // If indexes is 0, deselect.
    if indexes == 0 {
        write_i64(obj, SELECTED_ROW_OFF, -1);
    } else {
        // Use firstIndex if it looks like an NSIndexSet, otherwise treat as raw row.
        let first =
            crate::nsapplication::dispatch_u64_selector_named_to_target(indexes, "firstIndex", 0);
        if first != u64::MAX {
            write_i64(obj, SELECTED_ROW_OFF, first as i64);
        }
    }
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSTableView deselectAll:]`
pub extern "C" fn shim_nstableview_deselect_all(obj: u64, _cmd: u64, _sender: u64) {
    if obj == 0 {
        return;
    }
    write_i64(obj, SELECTED_ROW_OFF, -1);
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSTableView rowHeight]`
pub extern "C" fn shim_nstableview_row_height(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, ROW_HEIGHT_OFF)
}

/// `-[NSTableView setRowHeight:]`
pub extern "C" fn shim_nstableview_set_row_height(obj: u64, _cmd: u64, h: f64) {
    write_f64(obj, ROW_HEIGHT_OFF, h);
}

/// `-[NSTableView headerView]` — returns 0 (no separate header object).
pub extern "C" fn shim_nstableview_header_view(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // We render headers inline in drawRect; no standalone NSTableHeaderView.
    0
}

// ── drawRect ────────────────────────────────────────────────────

/// `-[NSTableView drawRect:]` — renders header, alternating rows, selection, cell text.
pub extern "C" fn shim_nstableview_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 {
        return;
    }

    let row_height = read_f64(obj, ROW_HEIGHT_OFF);
    let header_h = read_f64(obj, HEADER_HEIGHT_OFF);
    let row_count = query_data_source_row_count(obj);
    let selected = read_i64(obj, SELECTED_ROW_OFF);

    // Collect column pointers.
    let columns: Vec<u64> = with_columns(obj, |cols| cols.clone());
    let num_cols = columns.len().max(1);
    let col_width = w / num_cols as f64;

    cg_shims::bridge::with_context(cg_ctx, |c| {
        // ── Background ──
        c.fill_color = graphics_core::Color::WHITE;
        c.fill_rect(x, y, w, h);

        // ── Column header ──
        c.fill_color = graphics_core::Color::new(0.93, 0.93, 0.93, 1.0);
        c.fill_rect(x, y, w, header_h);
        c.stroke_color = graphics_core::Color::new(0.78, 0.78, 0.78, 1.0);
        c.stroke_rect(x, y, w, header_h);

        // Header dividers and titles.
        for (ci, &col) in columns.iter().enumerate() {
            let cx = x + ci as f64 * col_width;
            if ci > 0 {
                c.stroke_color = graphics_core::Color::new(0.78, 0.78, 0.78, 1.0);
                c.fill_color = graphics_core::Color::new(0.78, 0.78, 0.78, 1.0);
                c.fill_rect(cx, y, 1.0, header_h);
            }
            // Draw column identifier as header text.
            if col != 0 {
                let id_ptr = crate::nsapplication::dispatch_u64_selector_named_to_target(
                    col,
                    "identifier",
                    0,
                );
                if id_ptr != 0 && id_ptr != u64::MAX {
                    let utf8 = crate::cfstring::shim_nsstring_utf8string(id_ptr, 0);
                    if !utf8.is_null() {
                        let text = unsafe { std::ffi::CStr::from_ptr(utf8) }
                            .to_str()
                            .unwrap_or("");
                        if !text.is_empty() {
                            c.fill_color = graphics_core::Color::new(0.2, 0.2, 0.2, 1.0);
                            c.draw_text(text, cx + 4.0, y + header_h * 0.75);
                        }
                    }
                }
            }
        }

        // ── Rows ──
        let body_y = y + header_h;
        let max_visible = ((h - header_h) / row_height).ceil() as i64;
        let draw_rows = row_count.min(max_visible);

        for row in 0..draw_rows {
            let ry = body_y + row as f64 * row_height;

            // Alternating background.
            if row == selected {
                c.fill_color = graphics_core::Color::new(0.22, 0.47, 0.96, 1.0);
                c.fill_rect(x, ry, w, row_height);
            } else if row % 2 == 1 {
                c.fill_color = graphics_core::Color::new(0.96, 0.96, 0.96, 1.0);
                c.fill_rect(x, ry, w, row_height);
            }

            // Cell text from data source.
            for (ci, &col) in columns.iter().enumerate() {
                let cx = x + ci as f64 * col_width;
                let val = query_data_source_object_value(obj, col, row);
                if val != 0 {
                    let utf8 = crate::cfstring::shim_nsstring_utf8string(val, 0);
                    if !utf8.is_null() {
                        let text = unsafe { std::ffi::CStr::from_ptr(utf8) }
                            .to_str()
                            .unwrap_or("");
                        if !text.is_empty() {
                            c.fill_color = if row == selected {
                                graphics_core::Color::WHITE
                            } else {
                                graphics_core::Color::BLACK
                            };
                            c.draw_text(text, cx + 4.0, ry + row_height * 0.72);
                        }
                    }
                }
            }

            // Row separator line.
            c.fill_color = graphics_core::Color::new(0.88, 0.88, 0.88, 1.0);
            c.fill_rect(x, ry + row_height - 1.0, w, 1.0);
        }
    });
}

// ── mouseDown: ──────────────────────────────────────────────────

/// `-[NSTableView mouseDown:]` — select row under click and notify delegate.
pub extern "C" fn shim_nstableview_mouse_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 {
        return;
    }

    let (loc_x, loc_y) = crate::nsevent::event_location_in_window(event);
    let frame = crate::nsview::view_frame(obj);
    let local_y = loc_y - frame.origin.y;

    let header_h = read_f64(obj, HEADER_HEIGHT_OFF);
    if local_y < header_h {
        return; // Click in header — ignore for now.
    }

    let row_height = read_f64(obj, ROW_HEIGHT_OFF);
    let clicked_row = ((local_y - header_h) / row_height) as i64;
    let row_count = query_data_source_row_count(obj);

    if clicked_row >= 0 && clicked_row < row_count {
        write_i64(obj, SELECTED_ROW_OFF, clicked_row);
    } else {
        write_i64(obj, SELECTED_ROW_OFF, -1);
    }

    // Notify delegate: tableViewSelectionDidChange:
    let delegate = read_u64(obj, DELEGATE_OFF);
    if delegate != 0 {
        // Build an NSNotification with name "NSTableViewSelectionDidChangeNotification".
        let _ = crate::nsapplication::dispatch_selector_named_to_target(
            delegate,
            "tableViewSelectionDidChange:",
            obj, // pass tableView as arg (simplified — real AppKit passes NSNotification)
        );
    }

    // Make window's first responder.
    let window = crate::nsview::shim_nsview_window(obj, 0);
    if window != 0 {
        crate::nswindow::shim_nswindow_make_first_responder(window, 0, obj);
    }

    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
    let _ = loc_x; // suppress unused warning
}

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
    fn alloc_defaults() {
        let tv = make_tv();
        assert_eq!(shim_nstableview_selected_row(tv, 0), -1);
        assert!((shim_nstableview_row_height(tv, 0) - 22.0).abs() < f64::EPSILON);
        free_obj(tv);
    }

    #[test]
    fn init_with_frame() {
        let tv = shim_nstableview_init_with_frame(0, 0, 10.0, 20.0, 300.0, 200.0);
        assert_ne!(tv, 0);
        assert_eq!(unsafe { *(tv as *const u64) }, tv_isa());
        assert_eq!(shim_nstableview_selected_row(tv, 0), -1);
        assert!((shim_nstableview_row_height(tv, 0) - 22.0).abs() < f64::EPSILON);
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
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0),
            0x100
        );
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 1),
            0x200
        );

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
    fn row_height_round_trip() {
        let tv = make_tv();
        shim_nstableview_set_row_height(tv, 0, 30.0);
        assert!((shim_nstableview_row_height(tv, 0) - 30.0).abs() < f64::EPSILON);
        free_obj(tv);
    }

    #[test]
    fn selected_row_direct_write() {
        let tv = make_tv();
        write_i64(tv, SELECTED_ROW_OFF, 5);
        assert_eq!(shim_nstableview_selected_row(tv, 0), 5);
        free_obj(tv);
    }

    #[test]
    fn deselect_all_clears_selection() {
        let tv = make_tv();
        write_i64(tv, SELECTED_ROW_OFF, 3);
        assert_eq!(shim_nstableview_selected_row(tv, 0), 3);
        shim_nstableview_deselect_all(tv, 0, 0);
        assert_eq!(shim_nstableview_selected_row(tv, 0), -1);
        free_obj(tv);
    }

    #[test]
    fn header_view_returns_zero() {
        let tv = make_tv();
        assert_eq!(shim_nstableview_header_view(tv, 0), 0);
        free_obj(tv);
    }

    #[test]
    fn draw_rect_nil_safe() {
        shim_nstableview_draw_rect(0, 0, 0.0, 0.0, 100.0, 100.0);
    }

    #[test]
    fn mouse_down_nil_safe() {
        shim_nstableview_mouse_down(0, 0, 0);
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
        assert!((shim_nstableview_row_height(0, 0)).abs() < f64::EPSILON);
    }

    #[test]
    fn select_deselect() {
        let tv = make_tv();
        // Direct selection via write.
        write_i64(tv, SELECTED_ROW_OFF, 2);
        assert_eq!(shim_nstableview_selected_row(tv, 0), 2);
        shim_nstableview_deselect_all(tv, 0, 0);
        assert_eq!(shim_nstableview_selected_row(tv, 0), -1);
        free_obj(tv);
    }
}
