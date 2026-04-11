//! NSTabView shim: tabbed container view for Cocoa GUI apps.
//!
//! Object layout (inherits NSView, 72 bytes):
//!
//!   offset  0: isa                  (u64) — from NSView
//!   offset  8..72: inherited NSView fields
//!   offset 72: delegate_ptr         (u64 — id, or 0)
//!   offset 80: tab_view_data        (u64 — selected_index as i64)
//!
//! Total: 88 bytes.

const DELEGATE_OFF: usize = 72;
const TAB_VIEW_DATA_OFF: usize = 80;
pub(crate) const TAB_VIEW_SIZE: usize = 88;

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

// ── Instance method shims ──────────────────────────────────────

/// `-[NSTabView numberOfTabViewItems]` — returns actual count of items.
pub extern "C" fn shim_nstabview_number_of_tab_view_items(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 { return 0; }
    let guard = TAB_ITEMS.lock().unwrap();
    guard.as_ref().and_then(|m| m.get(&obj)).map(|v| v.len() as i64).unwrap_or(0)
}

/// `-[NSTabView delegate]`
pub extern "C" fn shim_nstabview_delegate(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DELEGATE_OFF)
}

/// `-[NSTabView setDelegate:]`
pub extern "C" fn shim_nstabview_set_delegate(obj: u64, _cmd: u64, del: u64) {
    write_u64(obj, DELEGATE_OFF, del);
}

/// `-[NSTabView selectTabViewItemAtIndex:]`
pub extern "C" fn shim_nstabview_select_tab_view_item_at_index(obj: u64, _cmd: u64, idx: i64) {
    if obj == 0 {
        return;
    }
    let old_idx = read_u64(obj, TAB_VIEW_DATA_OFF) as i64;
    if old_idx == idx { return; }

    let delegate = read_u64(obj, DELEGATE_OFF);

    // Notify delegate: tabView:willSelectTabViewItem:
    let new_item = tab_item_at(obj, idx);
    if delegate != 0 && new_item != 0 {
        crate::nsapplication::dispatch_selector_named_2arg(
            delegate, "tabView:willSelectTabViewItem:", obj, new_item,
        );
    }

    write_u64(obj, TAB_VIEW_DATA_OFF, idx as u64);

    // Notify delegate: tabView:didSelectTabViewItem:
    if delegate != 0 && new_item != 0 {
        crate::nsapplication::dispatch_selector_named_2arg(
            delegate, "tabView:didSelectTabViewItem:", obj, new_item,
        );
    }
}

/// `-[NSTabView selectedTabViewItem]` — returns the currently selected tab view item.
pub extern "C" fn shim_nstabview_selected_tab_view_item(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let idx = read_u64(obj, TAB_VIEW_DATA_OFF) as i64;
    tab_item_at(obj, idx)
}

/// `-[NSTabView tabViewItemAtIndex:]`
pub extern "C" fn shim_nstabview_tab_view_item_at_index(obj: u64, _cmd: u64, idx: i64) -> u64 {
    tab_item_at(obj, idx)
}

/// `-[NSTabView indexOfTabViewItem:]`
pub extern "C" fn shim_nstabview_index_of_tab_view_item(obj: u64, _cmd: u64, item: u64) -> i64 {
    if obj == 0 || item == 0 { return -1; }
    let guard = TAB_ITEMS.lock().unwrap();
    if let Some(items) = guard.as_ref().and_then(|m| m.get(&obj)) {
        for (i, &stored) in items.iter().enumerate() {
            if stored == item { return i as i64; }
        }
    }
    -1  // NSNotFound
}

fn tab_item_at(obj: u64, idx: i64) -> u64 {
    if obj == 0 || idx < 0 { return 0; }
    let guard = TAB_ITEMS.lock().unwrap();
    guard.as_ref()
        .and_then(|m| m.get(&obj))
        .and_then(|v| v.get(idx as usize).copied())
        .unwrap_or(0)
}

/// `-[NSTabView addTabViewItem:]` — stores the item.
pub extern "C" fn shim_nstabview_add_tab_view_item(obj: u64, _cmd: u64, item: u64) {
    if obj == 0 || item == 0 { return; }
    let mut guard = TAB_ITEMS.lock().unwrap();
    let map = guard.get_or_insert_with(std::collections::HashMap::new);
    map.entry(obj).or_default().push(item);
}

/// `-[NSTabView removeTabViewItem:]` — removes the item.
pub extern "C" fn shim_nstabview_remove_tab_view_item(obj: u64, _cmd: u64, item: u64) {
    if obj == 0 || item == 0 { return; }
    let mut guard = TAB_ITEMS.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        if let Some(items) = map.get_mut(&obj) {
            items.retain(|&i| i != item);
        }
    }
}

/// `-[NSTabView drawRect:]` — renders tab bar and selected content.
pub extern "C" fn shim_nstabview_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 { return; }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 { return; }

    let selected = read_u64(obj, TAB_VIEW_DATA_OFF) as i64;
    let items = {
        let guard = TAB_ITEMS.lock().unwrap();
        guard.as_ref().and_then(|m| m.get(&obj)).cloned().unwrap_or_default()
    };
    let count = items.len();

    // Draw tab bar at top.
    let tab_height = 28.0;
    if count > 0 {
        let tab_w = w / count as f64;
        for (i, &item) in items.iter().enumerate() {
            let tab_x = x + i as f64 * tab_w;
            let is_selected = i as i64 == selected;

            cg_shims::bridge::with_context(cg_ctx, |c| {
                c.fill_color = if is_selected {
                    graphics_core::Color::WHITE
                } else {
                    graphics_core::Color::new(0.88, 0.88, 0.88, 1.0)
                };
                c.fill_rect(tab_x, y, tab_w, tab_height);
                c.stroke_color = graphics_core::Color::new(0.7, 0.7, 0.7, 1.0);
                c.stroke_rect(tab_x, y, tab_w, tab_height);
            });

            // Draw tab label.
            let label = crate::nstabviewitem::shim_nstabviewitem_label(item, 0);
            if label != 0 {
                let utf8 = crate::cfstring::shim_nsstring_utf8string(label, 0);
                if !utf8.is_null() {
                    let text = unsafe { std::ffi::CStr::from_ptr(utf8) }.to_str().unwrap_or("");
                    if !text.is_empty() {
                        cg_shims::bridge::with_context(cg_ctx, |c| {
                            c.fill_color = graphics_core::Color::BLACK;
                            c.draw_text(text, tab_x + 6.0, y + tab_height * 0.7);
                        });
                    }
                }
            }
        }
    }

    // Content area background.
    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.fill_color = graphics_core::Color::WHITE;
        c.fill_rect(x, y + tab_height, w, h - tab_height);
        c.stroke_color = graphics_core::Color::new(0.7, 0.7, 0.7, 1.0);
        c.stroke_rect(x, y + tab_height, w, h - tab_height);
    });

    // Draw the selected tab's view.
    if selected >= 0 && (selected as usize) < count {
        let item = items[selected as usize];
        let view = crate::nstabviewitem::shim_nstabviewitem_view(item, 0);
        if view != 0 {
            let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
            crate::nsview::display_view_recursive(view, cg_ctx);
        }
    }
}

/// `-[NSTabView mouseDown:]` — select tab under click.
pub extern "C" fn shim_nstabview_mouse_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 { return; }

    let (loc_x, loc_y) = crate::nsevent::event_location_in_window(event);
    let frame = crate::nsview::view_frame(obj);
    let local_y = loc_y - frame.origin.y;

    // Tab bar is 28px tall at the top.
    let tab_height = 28.0;
    if local_y >= tab_height { return; } // Click in content area, not tab bar.

    let items = {
        let guard = TAB_ITEMS.lock().unwrap();
        guard.as_ref().and_then(|m| m.get(&obj)).cloned().unwrap_or_default()
    };
    let count = items.len();
    if count == 0 { return; }

    let local_x = loc_x - frame.origin.x;
    let tab_w = frame.size.width / count as f64;
    let clicked_tab = (local_x / tab_w) as i64;

    if clicked_tab >= 0 && (clicked_tab as usize) < count {
        shim_nstabview_select_tab_view_item_at_index(obj, 0, clicked_tab);
    }

    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

use std::sync::Mutex;
static TAB_ITEMS: Mutex<Option<std::collections::HashMap<u64, Vec<u64>>>> = Mutex::new(None);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstabview_number_of_tab_view_items(0, 0), 0);
        assert_eq!(shim_nstabview_delegate(0, 0), 0);
        shim_nstabview_set_delegate(0, 0, 0x1234);
        shim_nstabview_select_tab_view_item_at_index(0, 0, 5);
        assert_eq!(shim_nstabview_selected_tab_view_item(0, 0), 0);
        shim_nstabview_add_tab_view_item(0, 0, 0x1234);
        shim_nstabview_remove_tab_view_item(0, 0, 0x1234);
        shim_nstabview_mouse_down(0, 0, 0);
    }

    #[test]
    fn delegate_round_trip() {
        let tv = alloc_obj(TAB_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nstabview_delegate(tv, 0), 0);
        let fake_del = 0xBEEF_u64;
        shim_nstabview_set_delegate(tv, 0, fake_del);
        assert_eq!(shim_nstabview_delegate(tv, 0), fake_del);
        free_obj(tv);
    }

    #[test]
    fn select_tab_index() {
        let tv = alloc_obj(TAB_VIEW_SIZE, 0xDEAD);
        shim_nstabview_select_tab_view_item_at_index(tv, 0, 3);
        let stored = read_field(tv, TAB_VIEW_DATA_OFF);
        assert_eq!(stored, 3);
        shim_nstabview_select_tab_view_item_at_index(tv, 0, 0);
        let stored2 = read_field(tv, TAB_VIEW_DATA_OFF);
        assert_eq!(stored2, 0);
        free_obj(tv);
    }

    #[test]
    fn stubs_return_defaults() {
        let tv = alloc_obj(TAB_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nstabview_number_of_tab_view_items(tv, 0), 0);
        assert_eq!(shim_nstabview_selected_tab_view_item(tv, 0), 0);
        free_obj(tv);
    }
}
