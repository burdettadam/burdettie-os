//! NSComboBox shim: searchable drop-down control built on top of NSTextField.
//!
//! Object layout (80 bytes, inherits NSTextField at 48):
//!
//!   offset  0: isa                  (u64)
//!   offset  8: string_value_ptr     (u64 — inherited NSTextField storage)
//!   offset 16: target_ptr           (u64 — inherited NSTextField storage)
//!   offset 24: action_sel           (u64 — inherited NSTextField storage)
//!   offset 32: placeholder_ptr      (u64 — inherited NSTextField storage)
//!   offset 40: textfield_flags      (u64 — inherited NSTextField storage)
//!   offset 48: items_ptr            (u64 — pointer to heap-allocated Vec<u64> of NSString items)
//!   offset 56: selected_index       (i64 — -1 if none)
//!   offset 64: flags                (u64 — bit 0: autocompletes, bit 1: is_usable_when_modding)
//!
//! Total: 80 bytes.
//!
//! Note: the request listed custom fields at offsets 8/16/24 while also stating the
//! object inherits 48 bytes of NSTextField storage. To preserve the superclass layout
//! and the requested total size, this shim appends its custom fields at 48/56/64.

const ITEMS_PTR_OFF: usize = 48;
const SELECTED_INDEX_OFF: usize = 56;
const FLAGS_OFF: usize = 64;
pub(crate) const COMBO_BOX_SIZE: usize = 80;

const FLAG_AUTOCOMPLETES: u64 = 1 << 0;
const FLAG_IS_USABLE_WHEN_MODDING: u64 = 1 << 1;

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

fn read_flags(obj: u64) -> u64 {
    read_u64(obj, FLAGS_OFF)
}

fn set_flag(obj: u64, flag: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_flags(obj);
    if val != 0 {
        flags |= flag;
    } else {
        flags &= !flag;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

fn get_flag(obj: u64, flag: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(read_flags(obj) & flag != 0)
}

/// Get or lazily create the item storage Vec.
fn ensure_items(obj: u64) -> *mut Vec<u64> {
    let ptr = read_u64(obj, ITEMS_PTR_OFF);
    if ptr != 0 {
        return ptr as *mut Vec<u64>;
    }
    let items = Box::new(Vec::<u64>::new());
    let raw = Box::into_raw(items);
    write_u64(obj, ITEMS_PTR_OFF, raw as u64);
    if read_i64(obj, SELECTED_INDEX_OFF) == 0 {
        write_i64(obj, SELECTED_INDEX_OFF, -1);
    }
    raw
}

fn get_items(obj: u64) -> Option<&'static Vec<u64>> {
    let ptr = read_u64(obj, ITEMS_PTR_OFF);
    if ptr == 0 {
        return None;
    }
    Some(unsafe { &*(ptr as *const Vec<u64>) })
}

fn selected_index(obj: u64) -> i64 {
    if obj == 0 {
        return -1;
    }
    let idx = read_i64(obj, SELECTED_INDEX_OFF);
    if idx == 0 && read_u64(obj, ITEMS_PTR_OFF) == 0 {
        -1
    } else {
        idx
    }
}

/// `-[NSComboBox numberOfItems]`
pub extern "C" fn shim_nscombobox_number_of_items(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    match get_items(obj) {
        Some(items) => items.len() as i64,
        None => 0,
    }
}

/// `-[NSComboBox addItemWithObjectValue:]`
pub extern "C" fn shim_nscombobox_add_item_with_object_value(obj: u64, _cmd: u64, value: u64) {
    if obj == 0 {
        return;
    }
    let items = ensure_items(obj);
    unsafe {
        (*items).push(value);
    }
}

/// `-[NSComboBox removeAllItems]`
pub extern "C" fn shim_nscombobox_remove_all_items(obj: u64, _cmd: u64) {
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

/// `-[NSComboBox selectItemAtIndex:]`
pub extern "C" fn shim_nscombobox_select_item_at_index(obj: u64, _cmd: u64, idx: i64) {
    write_i64(obj, SELECTED_INDEX_OFF, idx);
}

/// `-[NSComboBox indexOfSelectedItem]`
pub extern "C" fn shim_nscombobox_index_of_selected_item(obj: u64, _cmd: u64) -> i64 {
    selected_index(obj)
}

/// `-[NSComboBox objectValueOfSelectedItem]`
pub extern "C" fn shim_nscombobox_object_value_of_selected_item(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let idx = selected_index(obj);
    if idx < 0 {
        return 0;
    }
    match get_items(obj) {
        Some(items) if (idx as usize) < items.len() => items[idx as usize],
        _ => 0,
    }
}

/// `-[NSComboBox itemObjectValueAtIndex:]`
pub extern "C" fn shim_nscombobox_item_object_value_at_index(obj: u64, _cmd: u64, idx: i64) -> u64 {
    if obj == 0 || idx < 0 {
        return 0;
    }
    match get_items(obj) {
        Some(items) if (idx as usize) < items.len() => items[idx as usize],
        _ => 0,
    }
}

/// `-[NSComboBox autocompletes]`
pub extern "C" fn shim_nscombobox_autocompletes(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_AUTOCOMPLETES)
}

/// `-[NSComboBox setAutocompletes:]`
pub extern "C" fn shim_nscombobox_set_autocompletes(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_AUTOCOMPLETES, val);
}

/// `-[NSComboBox usesDataSource]` — stubbed off for now.
pub extern "C" fn shim_nscombobox_uses_data_source(_obj: u64, _cmd: u64) -> u8 {
    0
}

/// `-[NSComboBox setUsesDataSource:]` — no-op stub.
pub extern "C" fn shim_nscombobox_set_uses_data_source(_obj: u64, _cmd: u64, _val: u8) {}

// ── Drawing ────────────────────────────────────────────────────

/// `-[NSComboBox drawRect:]` — renders the combo box: text field + dropdown arrow.
pub extern "C" fn shim_nscombobox_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 { return; }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 { return; }

    let arrow_w = 20.0;

    cg_shims::bridge::with_context(cg_ctx, |c| {
        // White background
        c.fill_color = graphics_core::Color::WHITE;
        c.fill_rect(x, y, w, h);
        // Border
        c.stroke_color = graphics_core::Color::new(0.7, 0.7, 0.7, 1.0);
        c.stroke_rect(x, y, w, h);
        // Arrow region separator + indicator
        c.stroke_rect(x + w - arrow_w, y, arrow_w, h);

        let font_size = c.font_size.max(12.0);
        let sw = c.surface.width();
        let sh = c.surface.height();
        // Draw "▾" in arrow region
        let _ = graphics_core::text_system::draw_shaped_text(
            c.surface.pixels_mut(), sw, sh,
            "▾", x + w - arrow_w + 4.0, y + (h - font_size * 1.2) / 2.0,
            font_size, "System", graphics_core::Color::BLACK, None,
        );

        // Draw selected item text (or placeholder)
        let idx = selected_index(obj);
        let text_ptr = if idx >= 0 {
            get_items(obj).and_then(|items| items.get(idx as usize).copied()).unwrap_or(0)
        } else {
            0
        };
        if text_ptr != 0 {
            let utf8 = crate::cfstring::shim_nsstring_utf8string(text_ptr, 0);
            if !utf8.is_null() {
                let text = unsafe { std::ffi::CStr::from_ptr(utf8) }
                    .to_str().unwrap_or("");
                if !text.is_empty() {
                    let _ = graphics_core::text_system::draw_shaped_text(
                        c.surface.pixels_mut(), sw, sh,
                        text, x + 4.0, y + (h - font_size * 1.2) / 2.0,
                        font_size, "System", graphics_core::Color::BLACK, None,
                    );
                }
            }
        }
    });
}

/// `-[NSComboBox mouseDown:]` — cycle through items on arrow click.
pub extern "C" fn shim_nscombobox_mouse_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 { return; }

    let (loc_x, _loc_y) = crate::nsevent::event_location_in_window(event);
    let frame = crate::nsview::view_frame(obj);
    let local_x = loc_x - frame.origin.x;
    let arrow_w = 20.0;

    if local_x >= frame.size.width - arrow_w {
        // Click in arrow region — cycle to next item
        let count = shim_nscombobox_number_of_items(obj, 0);
        if count > 0 {
            let cur = selected_index(obj);
            let next = if cur < 0 || cur >= count - 1 { 0 } else { cur + 1 };
            shim_nscombobox_select_item_at_index(obj, 0, next);
        }
        crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
    } else {
        // Click in text region — focus the control
        let window = unsafe { *((obj as usize + 16) as *const u64) }; // WINDOW_OFF
        if window != 0 {
            crate::nswindow::shim_nswindow_make_first_responder(window, 0, obj);
        }
        crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn free_items_vec(combo: u64) {
        let ptr = read_u64(combo, ITEMS_PTR_OFF);
        if ptr != 0 {
            unsafe {
                drop(Box::from_raw(ptr as *mut Vec<u64>));
            }
            write_u64(combo, ITEMS_PTR_OFF, 0);
        }
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nscombobox_number_of_items(0, 0), 0);
        shim_nscombobox_add_item_with_object_value(0, 0, 0x1234);
        shim_nscombobox_remove_all_items(0, 0);
        shim_nscombobox_select_item_at_index(0, 0, 1);
        assert_eq!(shim_nscombobox_index_of_selected_item(0, 0), -1);
        assert_eq!(shim_nscombobox_object_value_of_selected_item(0, 0), 0);
        assert_eq!(shim_nscombobox_item_object_value_at_index(0, 0, 0), 0);
        assert_eq!(shim_nscombobox_autocompletes(0, 0), 0);
        shim_nscombobox_set_autocompletes(0, 0, 1);
        assert_eq!(shim_nscombobox_uses_data_source(0, 0), 0);
        shim_nscombobox_set_uses_data_source(0, 0, 1);
        shim_nscombobox_draw_rect(0, 0, 0.0, 0.0, 100.0, 30.0);
        shim_nscombobox_mouse_down(0, 0, 0);
    }

    #[test]
    fn add_items_and_count() {
        let combo = alloc_obj(COMBO_BOX_SIZE, 0xDEAD);
        let one = make_string("One");
        let two = make_string("Two");

        assert_eq!(shim_nscombobox_number_of_items(combo, 0), 0);
        shim_nscombobox_add_item_with_object_value(combo, 0, one);
        shim_nscombobox_add_item_with_object_value(combo, 0, two);

        assert_eq!(shim_nscombobox_number_of_items(combo, 0), 2);
        assert_eq!(shim_nscombobox_item_object_value_at_index(combo, 0, 0), one);
        assert_eq!(shim_nscombobox_item_object_value_at_index(combo, 0, 1), two);

        free_items_vec(combo);
        free_obj(combo);
    }

    #[test]
    fn selection_round_trip() {
        let combo = alloc_obj(COMBO_BOX_SIZE, 0xDEAD);
        let alpha = make_string("Alpha");
        let beta = make_string("Beta");

        shim_nscombobox_add_item_with_object_value(combo, 0, alpha);
        shim_nscombobox_add_item_with_object_value(combo, 0, beta);
        assert_eq!(shim_nscombobox_index_of_selected_item(combo, 0), -1);

        shim_nscombobox_select_item_at_index(combo, 0, 1);
        assert_eq!(shim_nscombobox_index_of_selected_item(combo, 0), 1);
        assert_eq!(shim_nscombobox_object_value_of_selected_item(combo, 0), beta);
        assert_eq!(read_string(beta), "Beta");

        free_items_vec(combo);
        free_obj(combo);
    }

    #[test]
    fn out_of_bounds_selection_returns_nil_object() {
        let combo = alloc_obj(COMBO_BOX_SIZE, 0xDEAD);
        let only = make_string("Only");

        shim_nscombobox_add_item_with_object_value(combo, 0, only);
        shim_nscombobox_select_item_at_index(combo, 0, 9);

        assert_eq!(shim_nscombobox_index_of_selected_item(combo, 0), 9);
        assert_eq!(shim_nscombobox_object_value_of_selected_item(combo, 0), 0);
        assert_eq!(shim_nscombobox_item_object_value_at_index(combo, 0, -1), 0);
        assert_eq!(shim_nscombobox_item_object_value_at_index(combo, 0, 9), 0);

        free_items_vec(combo);
        free_obj(combo);
    }

    #[test]
    fn remove_all_items_clears_selection() {
        let combo = alloc_obj(COMBO_BOX_SIZE, 0xDEAD);
        let red = make_string("Red");

        shim_nscombobox_add_item_with_object_value(combo, 0, red);
        shim_nscombobox_select_item_at_index(combo, 0, 0);
        assert_eq!(shim_nscombobox_object_value_of_selected_item(combo, 0), red);

        shim_nscombobox_remove_all_items(combo, 0);
        assert_eq!(shim_nscombobox_number_of_items(combo, 0), 0);
        assert_eq!(shim_nscombobox_index_of_selected_item(combo, 0), -1);
        assert_eq!(shim_nscombobox_object_value_of_selected_item(combo, 0), 0);

        free_items_vec(combo);
        free_obj(combo);
    }

    #[test]
    fn autocompletes_flag_toggles_without_touching_other_bits() {
        let combo = alloc_obj(COMBO_BOX_SIZE, 0xDEAD);

        write_u64(combo, FLAGS_OFF, FLAG_IS_USABLE_WHEN_MODDING);
        assert_eq!(shim_nscombobox_autocompletes(combo, 0), 0);

        shim_nscombobox_set_autocompletes(combo, 0, 1);
        assert_eq!(shim_nscombobox_autocompletes(combo, 0), 1);
        assert_eq!(read_u64(combo, FLAGS_OFF) & FLAG_IS_USABLE_WHEN_MODDING, FLAG_IS_USABLE_WHEN_MODDING);

        shim_nscombobox_set_autocompletes(combo, 0, 0);
        assert_eq!(shim_nscombobox_autocompletes(combo, 0), 0);
        assert_eq!(read_u64(combo, FLAGS_OFF), FLAG_IS_USABLE_WHEN_MODDING);

        free_obj(combo);
    }

    #[test]
    fn uses_data_source_is_stubbed() {
        let combo = alloc_obj(COMBO_BOX_SIZE, 0xDEAD);

        assert_eq!(shim_nscombobox_uses_data_source(combo, 0), 0);
        shim_nscombobox_set_uses_data_source(combo, 0, 1);
        assert_eq!(shim_nscombobox_uses_data_source(combo, 0), 0);

        free_obj(combo);
    }
}
