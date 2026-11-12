//! NSOutlineView shim: tree-view subclass of NSTableView.
//!
//! Inherits NSTableView object layout (96 bytes) — no additional instance fields.
//! Tree expansion state is tracked externally via a global HashMap keyed by object address.
//!
//! Data source protocol (dispatched to the table view's dataSource):
//!   - `outlineView:numberOfChildrenOfItem:`   → i64
//!   - `outlineView:child:ofItem:`             → id
//!   - `outlineView:isItemExpandable:`         → BOOL
//!   - `outlineView:objectValueForTableColumn:byItem:` → id

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

pub(crate) const OUTLINE_VIEW_SIZE: usize = crate::nstableview::TABLE_VIEW_SIZE;

/// Per-outline-view expansion state.
struct OutlineState {
    expanded: HashSet<u64>,
    /// Flattened visible row list: each entry is an (item, depth) pair.
    rows: Vec<(u64, usize)>,
}

static OUTLINE_STATES: Mutex<Option<HashMap<u64, OutlineState>>> = Mutex::new(None);

fn with_state<F, R>(obj: u64, f: F) -> R
where
    F: FnOnce(&mut OutlineState) -> R,
{
    let mut guard = OUTLINE_STATES.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let state = map.entry(obj).or_insert_with(|| OutlineState {
        expanded: HashSet::new(),
        rows: Vec::new(),
    });
    f(state)
}

// ── Data source dispatch helpers ───────────────────────────────

fn data_source(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // DATA_SOURCE_OFF = 56 in nstableview
    unsafe { *((obj as usize + 56) as *const u64) }
}

fn ds_number_of_children(obj: u64, item: u64) -> i64 {
    let ds = data_source(obj);
    if ds == 0 {
        return 0;
    }
    dispatch_3arg(ds, "outlineView:numberOfChildrenOfItem:", obj, item, 0) as i64
}

fn ds_child(obj: u64, index: i64, item: u64) -> u64 {
    let ds = data_source(obj);
    if ds == 0 {
        return 0;
    }
    dispatch_3arg(ds, "outlineView:child:ofItem:", obj, index as u64, item)
}

fn ds_is_expandable(obj: u64, item: u64) -> bool {
    let ds = data_source(obj);
    if ds == 0 {
        return false;
    }
    dispatch_3arg(ds, "outlineView:isItemExpandable:", obj, item, 0) != 0
}

fn dispatch_3arg(target: u64, selector: &str, a1: u64, a2: u64, a3: u64) -> u64 {
    if target == 0 {
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

// ── Row flattening ─────────────────────────────────────────────

/// Rebuild the flattened row list by walking the data source tree.
fn rebuild_rows(obj: u64) {
    with_state(obj, |state| {
        state.rows.clear();
        flatten_children(obj, 0, 0, &state.expanded, &mut state.rows);
    });
}

fn flatten_children(
    obj: u64,
    parent_item: u64,
    depth: usize,
    expanded: &HashSet<u64>,
    out: &mut Vec<(u64, usize)>,
) {
    let count = ds_number_of_children(obj, parent_item);
    for i in 0..count {
        let child = ds_child(obj, i, parent_item);
        if child == 0 {
            continue;
        }
        out.push((child, depth));
        if expanded.contains(&child) {
            flatten_children(obj, child, depth + 1, expanded, out);
        }
    }
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSOutlineView reloadData]` — rebuild the flattened row model.
pub extern "C" fn shim_nsoutlineview_reload_data(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    rebuild_rows(obj);
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSOutlineView isItemExpanded:]`
pub extern "C" fn shim_nsoutlineview_is_item_expanded(obj: u64, _cmd: u64, item: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    with_state(obj, |s| u8::from(s.expanded.contains(&item)))
}

/// `-[NSOutlineView expandItem:]`
pub extern "C" fn shim_nsoutlineview_expand_item(obj: u64, _cmd: u64, item: u64) {
    if obj == 0 || item == 0 {
        return;
    }
    with_state(obj, |s| {
        s.expanded.insert(item);
    });
    rebuild_rows(obj);
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSOutlineView collapseItem:]`
pub extern "C" fn shim_nsoutlineview_collapse_item(obj: u64, _cmd: u64, item: u64) {
    if obj == 0 || item == 0 {
        return;
    }
    with_state(obj, |s| {
        s.expanded.remove(&item);
    });
    rebuild_rows(obj);
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSOutlineView itemAtRow:]`
pub extern "C" fn shim_nsoutlineview_item_at_row(obj: u64, _cmd: u64, row: i64) -> u64 {
    if obj == 0 || row < 0 {
        return 0;
    }
    with_state(obj, |s| {
        s.rows.get(row as usize).map(|&(item, _)| item).unwrap_or(0)
    })
}

/// `-[NSOutlineView rowForItem:]`
pub extern "C" fn shim_nsoutlineview_row_for_item(obj: u64, _cmd: u64, item: u64) -> i64 {
    if obj == 0 || item == 0 {
        return -1;
    }
    with_state(obj, |s| {
        s.rows
            .iter()
            .position(|&(i, _)| i == item)
            .map(|p| p as i64)
            .unwrap_or(-1)
    })
}

/// `-[NSOutlineView numberOfRows]` — returns the flattened visible row count.
pub extern "C" fn shim_nsoutlineview_number_of_rows(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    with_state(obj, |s| s.rows.len() as u64)
}

/// `-[NSOutlineView levelForItem:]`
pub extern "C" fn shim_nsoutlineview_level_for_item(obj: u64, _cmd: u64, item: u64) -> i64 {
    if obj == 0 || item == 0 {
        return -1;
    }
    with_state(obj, |s| {
        s.rows
            .iter()
            .find(|&&(i, _)| i == item)
            .map(|&(_, depth)| depth as i64)
            .unwrap_or(-1)
    })
}

/// `-[NSOutlineView levelForRow:]`
pub extern "C" fn shim_nsoutlineview_level_for_row(obj: u64, _cmd: u64, row: i64) -> i64 {
    if obj == 0 || row < 0 {
        return -1;
    }
    with_state(obj, |s| {
        s.rows
            .get(row as usize)
            .map(|&(_, d)| d as i64)
            .unwrap_or(-1)
    })
}

/// `-[NSOutlineView indentationPerLevel]`
pub extern "C" fn shim_nsoutlineview_indentation_per_level(_obj: u64, _cmd: u64) -> f64 {
    16.0
}

/// `-[NSOutlineView drawRect:]` — renders disclosure triangles + row text.
pub extern "C" fn shim_nsoutlineview_draw_rect(
    obj: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    if obj == 0 {
        return;
    }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 {
        return;
    }

    let row_height: f64 = unsafe { *((obj as usize + 80) as *const f64) }; // ROW_HEIGHT_OFF
    let rh = if row_height > 0.0 { row_height } else { 22.0 };
    let selected_row: i64 = unsafe { *((obj as usize + 72) as *const i64) }; // SELECTED_ROW_OFF
    let indent = 16.0_f64;

    let rows: Vec<(u64, usize)> = with_state(obj, |s| s.rows.clone());
    let expanded: HashSet<u64> = with_state(obj, |s| s.expanded.clone());

    cg_shims::bridge::with_context(cg_ctx, |c| {
        let font_size = c.font_size.max(12.0);
        let sw = c.surface.width();
        let sh = c.surface.height();

        for (i, &(item, depth)) in rows.iter().enumerate() {
            let row_y = y + i as f64 * rh;
            if row_y > y + h {
                break;
            }

            // Background: selection highlight or alternating.
            if i as i64 == selected_row {
                c.fill_color = graphics_core::Color::new(0.0, 0.48, 1.0, 1.0);
            } else if i % 2 == 0 {
                c.fill_color = graphics_core::Color::WHITE;
            } else {
                c.fill_color = graphics_core::Color::new(0.95, 0.95, 0.95, 1.0);
            }
            c.fill_rect(x, row_y, w, rh);

            let text_x = x + 4.0 + depth as f64 * indent;
            let text_y = row_y + (rh - font_size * 1.2) / 2.0;

            // Disclosure triangle for expandable items.
            let expandable = ds_is_expandable(obj, item);
            if expandable {
                let glyph = if expanded.contains(&item) {
                    "▾"
                } else {
                    "▸"
                };
                c.fill_color = graphics_core::Color::new(0.3, 0.3, 0.3, 1.0);
                let _ = graphics_core::text_system::draw_shaped_text(
                    c.surface.pixels_mut(),
                    sw,
                    sh,
                    glyph,
                    text_x - 14.0,
                    text_y,
                    font_size,
                    "System",
                    graphics_core::Color::new(0.3, 0.3, 0.3, 1.0),
                    None,
                );
            }

            // Item text — try to get the string representation via objectValue.
            let text_color = if i as i64 == selected_row {
                graphics_core::Color::WHITE
            } else {
                graphics_core::Color::BLACK
            };
            // Use first column if available.
            let col: u64 = crate::nstableview::first_column_addr(obj);
            let text_ptr = if col != 0 {
                dispatch_3arg(
                    data_source(obj),
                    "outlineView:objectValueForTableColumn:byItem:",
                    obj,
                    col,
                    item,
                )
            } else {
                0
            };
            if text_ptr != 0 {
                let utf8 = crate::cfstring::shim_nsstring_utf8string(text_ptr, 0);
                if !utf8.is_null() {
                    let text = unsafe { std::ffi::CStr::from_ptr(utf8) }
                        .to_str()
                        .unwrap_or("");
                    let _ = graphics_core::text_system::draw_shaped_text(
                        c.surface.pixels_mut(),
                        sw,
                        sh,
                        text,
                        text_x,
                        text_y,
                        font_size,
                        "System",
                        text_color,
                        None,
                    );
                }
            }
        }
    });
}

/// `-[NSOutlineView mouseDown:]` — handle row selection + triangle toggle.
pub extern "C" fn shim_nsoutlineview_mouse_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 {
        return;
    }

    let (loc_x, loc_y) = crate::nsevent::event_location_in_window(event);
    let frame = crate::nsview::view_frame(obj);
    let row_height: f64 = unsafe { *((obj as usize + 80) as *const f64) };
    let rh = if row_height > 0.0 { row_height } else { 22.0 };
    let local_y = loc_y - frame.origin.y;
    let local_x = loc_x - frame.origin.x;
    let clicked_row = (local_y / rh) as i64;

    let (item, depth) = with_state(obj, |s| {
        s.rows.get(clicked_row as usize).copied().unwrap_or((0, 0))
    });
    if item == 0 {
        return;
    }

    let indent = 16.0_f64;
    let triangle_x = 4.0 + depth as f64 * indent - 14.0;

    // Did user click on the disclosure triangle?
    if local_x >= triangle_x && local_x <= triangle_x + 14.0 && ds_is_expandable(obj, item) {
        let is_expanded = with_state(obj, |s| s.expanded.contains(&item));
        if is_expanded {
            shim_nsoutlineview_collapse_item(obj, 0, item);
        } else {
            shim_nsoutlineview_expand_item(obj, 0, item);
        }
        return;
    }

    // Otherwise: select the row.
    unsafe {
        *((obj as usize + 72) as *mut i64) = clicked_row;
    } // SELECTED_ROW_OFF

    // Notify delegate: outlineViewSelectionDidChange:
    let delegate = unsafe { *((obj as usize + 64) as *const u64) }; // DELEGATE_OFF
    if delegate != 0 {
        let _ = crate::nsapplication::dispatch_selector_named_to_target(
            delegate,
            "outlineViewSelectionDidChange:",
            obj,
        );
    }

    // Make first responder.
    let window = unsafe { *((obj as usize + 16) as *const u64) }; // WINDOW_OFF
    if window != 0 {
        crate::nswindow::shim_nswindow_make_first_responder(window, 0, obj);
    }
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        shim_nsoutlineview_reload_data(0, 0);
        assert_eq!(shim_nsoutlineview_is_item_expanded(0, 0, 0x1234), 0);
        shim_nsoutlineview_expand_item(0, 0, 0);
        shim_nsoutlineview_collapse_item(0, 0, 0);
        assert_eq!(shim_nsoutlineview_item_at_row(0, 0, 0), 0);
        assert_eq!(shim_nsoutlineview_row_for_item(0, 0, 0), -1);
        assert_eq!(shim_nsoutlineview_number_of_rows(0, 0), 0);
        assert_eq!(shim_nsoutlineview_level_for_item(0, 0, 0), -1);
        assert_eq!(shim_nsoutlineview_level_for_row(0, 0, 0), -1);
        assert_eq!(shim_nsoutlineview_indentation_per_level(0, 0), 16.0);
        shim_nsoutlineview_draw_rect(0, 0, 0.0, 0.0, 100.0, 100.0);
        shim_nsoutlineview_mouse_down(0, 0, 0);
    }

    #[test]
    fn expand_collapse_state() {
        let ov = alloc_obj(OUTLINE_VIEW_SIZE, 0xDEAD);
        let item = 0xCAFE_u64;

        assert_eq!(shim_nsoutlineview_is_item_expanded(ov, 0, item), 0);

        // Manually insert expansion (no data source, so rebuild_rows won't add rows).
        with_state(ov, |s| {
            s.expanded.insert(item);
        });
        assert_eq!(shim_nsoutlineview_is_item_expanded(ov, 0, item), 1);

        shim_nsoutlineview_collapse_item(ov, 0, item);
        assert_eq!(shim_nsoutlineview_is_item_expanded(ov, 0, item), 0);

        // Clean up global state.
        {
            let mut guard = OUTLINE_STATES.lock().unwrap();
            if let Some(map) = guard.as_mut() {
                map.remove(&ov);
            }
        }
        free_obj(ov);
    }

    #[test]
    fn item_at_row_and_row_for_item() {
        let ov = alloc_obj(OUTLINE_VIEW_SIZE, 0xDEAD);
        // Manually populate rows (no data source needed).
        with_state(ov, |s| {
            s.rows.push((0xA, 0));
            s.rows.push((0xB, 1));
            s.rows.push((0xC, 0));
        });

        assert_eq!(shim_nsoutlineview_item_at_row(ov, 0, 0), 0xA);
        assert_eq!(shim_nsoutlineview_item_at_row(ov, 0, 1), 0xB);
        assert_eq!(shim_nsoutlineview_item_at_row(ov, 0, 2), 0xC);
        assert_eq!(shim_nsoutlineview_item_at_row(ov, 0, 99), 0);

        assert_eq!(shim_nsoutlineview_row_for_item(ov, 0, 0xA), 0);
        assert_eq!(shim_nsoutlineview_row_for_item(ov, 0, 0xB), 1);
        assert_eq!(shim_nsoutlineview_row_for_item(ov, 0, 0xC), 2);
        assert_eq!(shim_nsoutlineview_row_for_item(ov, 0, 0xDEAD), -1);

        assert_eq!(shim_nsoutlineview_number_of_rows(ov, 0), 3);
        assert_eq!(shim_nsoutlineview_level_for_item(ov, 0, 0xB), 1);
        assert_eq!(shim_nsoutlineview_level_for_row(ov, 0, 1), 1);

        // Clean up.
        {
            let mut guard = OUTLINE_STATES.lock().unwrap();
            if let Some(map) = guard.as_mut() {
                map.remove(&ov);
            }
        }
        free_obj(ov);
    }
}
