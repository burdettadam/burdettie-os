//! NSPopUpButton shim: drop-down selection control for AppKit.
//!
//! Object layout (72 bytes, inherits NSButton at 56):
//!
//!   offset  0: isa                  (u64)
//!   offset  8: title_ptr            (u64 — NSString*)
//!   offset 16: alternate_title_ptr  (u64 — NSString*)
//!   offset 24: image_ptr            (u64 — NSImage*)
//!   offset 32: target_ptr           (u64)
//!   offset 40: action_sel           (u64 — SEL)
//!   offset 48: flags                (u64 — inherited from NSButton)
//!   offset 56: items_ptr            (u64 — pointer to heap-allocated Vec<u64>)
//!   offset 64: selected_index       (i64 — -1 = none)
//!
//! Total: 72 bytes.

const ITEMS_PTR_OFF: usize = 56;
const SELECTED_INDEX_OFF: usize = 64;
pub(crate) const POPUP_BUTTON_SIZE: usize = 72;

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

// ── Items list helpers ─────────────────────────────────────────

/// Get or lazily create the items Vec for the popup button.
fn ensure_items(obj: u64) -> *mut Vec<u64> {
    let ptr = read_u64(obj, ITEMS_PTR_OFF);
    if ptr != 0 {
        return ptr as *mut Vec<u64>;
    }
    let items = Box::new(Vec::<u64>::new());
    let raw = Box::into_raw(items);
    write_u64(obj, ITEMS_PTR_OFF, raw as u64);
    raw
}

/// Get the items Vec, returning None if not yet created.
fn get_items(obj: u64) -> Option<&'static Vec<u64>> {
    let ptr = read_u64(obj, ITEMS_PTR_OFF);
    if ptr == 0 {
        return None;
    }
    Some(unsafe { &*(ptr as *const Vec<u64>) })
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSPopUpButton numberOfItems]`
pub extern "C" fn shim_nspopupbutton_number_of_items(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    match get_items(obj) {
        Some(items) => items.len() as i64,
        None => 0,
    }
}

/// `-[NSPopUpButton addItemWithTitle:]`
pub extern "C" fn shim_nspopupbutton_add_item_with_title(obj: u64, _cmd: u64, title: u64) {
    if obj == 0 {
        return;
    }
    let items = ensure_items(obj);
    unsafe {
        (*items).push(title);
    }
}

/// `-[NSPopUpButton removeAllItems]`
pub extern "C" fn shim_nspopupbutton_remove_all_items(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let ptr = read_u64(obj, ITEMS_PTR_OFF);
    if ptr != 0 {
        unsafe {
            (*(ptr as *mut Vec<u64>)).clear();
        }
    }
    write_i64(obj, SELECTED_INDEX_OFF, -1);
}

/// `-[NSPopUpButton indexOfSelectedItem]`
pub extern "C" fn shim_nspopupbutton_index_of_selected_item(obj: u64, _cmd: u64) -> i64 {
    read_i64(obj, SELECTED_INDEX_OFF)
}

/// `-[NSPopUpButton selectItemAtIndex:]`
pub extern "C" fn shim_nspopupbutton_select_item_at_index(obj: u64, _cmd: u64, idx: i64) {
    write_i64(obj, SELECTED_INDEX_OFF, idx);
}

/// `-[NSPopUpButton titleOfSelectedItem]`
pub extern "C" fn shim_nspopupbutton_title_of_selected_item(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let idx = read_i64(obj, SELECTED_INDEX_OFF);
    if idx < 0 {
        return 0;
    }
    match get_items(obj) {
        Some(items) if (idx as usize) < items.len() => items[idx as usize],
        _ => 0,
    }
}

/// `-[NSPopUpButton itemTitleAtIndex:]`
pub extern "C" fn shim_nspopupbutton_item_title_at_index(obj: u64, _cmd: u64, idx: i64) -> u64 {
    if obj == 0 || idx < 0 {
        return 0;
    }
    match get_items(obj) {
        Some(items) if (idx as usize) < items.len() => items[idx as usize],
        _ => 0,
    }
}

/// `-[NSPopUpButton drawRect:]` — renders selected title + dropdown indicator.
pub extern "C" fn shim_nspopupbutton_draw_rect(
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

    cg_shims::bridge::with_context(cg_ctx, |c| {
        // Button background.
        c.fill_color = graphics_core::Color::new(0.95, 0.95, 0.95, 1.0);
        c.fill_rect(x, y, w, h);
        c.stroke_color = graphics_core::Color::new(0.7, 0.7, 0.7, 1.0);
        c.stroke_rect(x, y, w, h);
    });

    // Draw selected item title.
    let title = shim_nspopupbutton_title_of_selected_item(obj, 0);
    if title != 0 {
        let utf8 = crate::cfstring::shim_nsstring_utf8string(title, 0);
        if !utf8.is_null() {
            let text = unsafe { std::ffi::CStr::from_ptr(utf8) }
                .to_str()
                .unwrap_or("");
            if !text.is_empty() {
                cg_shims::bridge::with_context(cg_ctx, |c| {
                    c.fill_color = graphics_core::Color::BLACK;
                    c.draw_text(text, x + 6.0, y + h * 0.65);
                });
            }
        }
    }

    // Dropdown indicator "▾" on the right side.
    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.fill_color = graphics_core::Color::new(0.3, 0.3, 0.3, 1.0);
        c.draw_text("\u{25BE}", x + w - 16.0, y + h * 0.65);
    });
}

/// `-[NSPopUpButton mouseDown:]` — cycle to next item, fire action.
pub extern "C" fn shim_nspopupbutton_mouse_down(obj: u64, _cmd: u64, _event: u64) {
    if obj == 0 {
        return;
    }

    let count = shim_nspopupbutton_number_of_items(obj, 0);
    if count <= 0 {
        return;
    }

    let current = read_i64(obj, SELECTED_INDEX_OFF);
    let next = if current < 0 || current >= count - 1 {
        0
    } else {
        current + 1
    };
    write_i64(obj, SELECTED_INDEX_OFF, next);

    // Fire target/action (inherited NSButton offsets 32/40).
    const TARGET_OFF: usize = 32;
    const ACTION_OFF: usize = 40;
    let action = read_u64(obj, ACTION_OFF);
    let target = read_u64(obj, TARGET_OFF);
    if action != 0 {
        let nsapp = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
        crate::nsapplication::shim_nsapplication_send_action_to_from(nsapp, 0, action, target, obj);
    }

    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nspopupbutton_number_of_items(0, 0), 0);
        shim_nspopupbutton_add_item_with_title(0, 0, 0x1234);
        shim_nspopupbutton_remove_all_items(0, 0);
        assert_eq!(shim_nspopupbutton_index_of_selected_item(0, 0), -1);
        shim_nspopupbutton_select_item_at_index(0, 0, 0);
        assert_eq!(shim_nspopupbutton_title_of_selected_item(0, 0), 0);
        assert_eq!(shim_nspopupbutton_item_title_at_index(0, 0, 0), 0);
        shim_nspopupbutton_draw_rect(0, 0, 0.0, 0.0, 100.0, 22.0);
        shim_nspopupbutton_mouse_down(0, 0, 0);
    }

    #[test]
    fn add_items_and_count() {
        let btn = alloc_obj(POPUP_BUTTON_SIZE, 0xDEAD);
        // Initially zero items, selected = -1 (calloc zeroed → reinterpret as 0, correct via default)
        assert_eq!(shim_nspopupbutton_number_of_items(btn, 0), 0);

        let t1 = make_string("Apple");
        let t2 = make_string("Banana");
        let t3 = make_string("Cherry");
        shim_nspopupbutton_add_item_with_title(btn, 0, t1);
        shim_nspopupbutton_add_item_with_title(btn, 0, t2);
        shim_nspopupbutton_add_item_with_title(btn, 0, t3);
        assert_eq!(shim_nspopupbutton_number_of_items(btn, 0), 3);

        // Clean up the items Vec
        let ptr = read_u64(btn, ITEMS_PTR_OFF);
        if ptr != 0 {
            unsafe {
                drop(Box::from_raw(ptr as *mut Vec<u64>));
            }
        }
        free_obj(btn);
    }

    #[test]
    fn select_and_retrieve_title() {
        let btn = alloc_obj(POPUP_BUTTON_SIZE, 0xDEAD);
        let t1 = make_string("Red");
        let t2 = make_string("Green");
        shim_nspopupbutton_add_item_with_title(btn, 0, t1);
        shim_nspopupbutton_add_item_with_title(btn, 0, t2);

        shim_nspopupbutton_select_item_at_index(btn, 0, 1);
        assert_eq!(shim_nspopupbutton_index_of_selected_item(btn, 0), 1);
        let selected = shim_nspopupbutton_title_of_selected_item(btn, 0);
        assert_eq!(selected, t2);
        assert_eq!(read_string(selected), "Green");

        // item_title_at_index
        assert_eq!(shim_nspopupbutton_item_title_at_index(btn, 0, 0), t1);
        assert_eq!(shim_nspopupbutton_item_title_at_index(btn, 0, 1), t2);
        assert_eq!(shim_nspopupbutton_item_title_at_index(btn, 0, 5), 0);

        let ptr = read_u64(btn, ITEMS_PTR_OFF);
        if ptr != 0 {
            unsafe {
                drop(Box::from_raw(ptr as *mut Vec<u64>));
            }
        }
        free_obj(btn);
    }

    #[test]
    fn remove_all_items() {
        let btn = alloc_obj(POPUP_BUTTON_SIZE, 0xDEAD);
        let t1 = make_string("X");
        shim_nspopupbutton_add_item_with_title(btn, 0, t1);
        shim_nspopupbutton_select_item_at_index(btn, 0, 0);
        assert_eq!(shim_nspopupbutton_number_of_items(btn, 0), 1);

        shim_nspopupbutton_remove_all_items(btn, 0);
        assert_eq!(shim_nspopupbutton_number_of_items(btn, 0), 0);
        assert_eq!(shim_nspopupbutton_index_of_selected_item(btn, 0), -1);
        assert_eq!(shim_nspopupbutton_title_of_selected_item(btn, 0), 0);

        let ptr = read_u64(btn, ITEMS_PTR_OFF);
        if ptr != 0 {
            unsafe {
                drop(Box::from_raw(ptr as *mut Vec<u64>));
            }
        }
        free_obj(btn);
    }

    #[test]
    fn no_selection_returns_nil() {
        let btn = alloc_obj(POPUP_BUTTON_SIZE, 0xDEAD);
        let t1 = make_string("Only");
        shim_nspopupbutton_add_item_with_title(btn, 0, t1);
        // selected_index was zeroed by calloc → 0, which is a valid index
        // Explicitly set to -1 to test no-selection
        shim_nspopupbutton_select_item_at_index(btn, 0, -1);
        assert_eq!(shim_nspopupbutton_title_of_selected_item(btn, 0), 0);

        let ptr = read_u64(btn, ITEMS_PTR_OFF);
        if ptr != 0 {
            unsafe {
                drop(Box::from_raw(ptr as *mut Vec<u64>));
            }
        }
        free_obj(btn);
    }
}
