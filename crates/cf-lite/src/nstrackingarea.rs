//! NSTrackingArea — mouse tracking for views.
//!
//! Object layout: 48 bytes
//!   offset 0:  isa          (u64)
//!   offset 8:  rect_x       (f64)
//!   offset 16: rect_y       (f64)
//!   offset 24: rect_w       (f64)
//!   offset 32: rect_h       (f64)
//!   offset 40: options      (u64 — tracking options flags)
//!
//! Owner is stored in a global map (not in the object).

use crate::nsobj;
use std::collections::HashMap;
use std::sync::Mutex;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u8).add(offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, val: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as *mut u8).add(offset) as *mut u64) = val;
    }
}

fn read_f64(obj: u64, offset: usize) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as *const u8).add(offset) as *const f64) }
}

fn write_f64(obj: u64, offset: usize, val: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as *mut u8).add(offset) as *mut f64) = val;
    }
}

pub(crate) const TRACKING_AREA_SIZE: usize = 48;
const RECT_X_OFF: usize = 8;
const RECT_Y_OFF: usize = 16;
const RECT_W_OFF: usize = 24;
const RECT_H_OFF: usize = 32;
const OPTIONS_OFF: usize = 40;

// NSTrackingAreaOptions
pub const MOUSE_ENTERED_AND_EXITED: u64 = 0x01;
pub const MOUSE_MOVED: u64 = 0x02;
pub const CURSOR_UPDATE: u64 = 0x04;
pub const ACTIVE_WHEN_FIRST_RESPONDER: u64 = 0x10;
pub const ACTIVE_IN_KEY_WINDOW: u64 = 0x20;
pub const ACTIVE_IN_ACTIVE_APP: u64 = 0x40;
pub const ACTIVE_ALWAYS: u64 = 0x80;
pub const ASSUME_INSIDE: u64 = 0x100;
pub const IN_VISIBLE_RECT: u64 = 0x200;
pub const ENABLED_DURING_MOUSE_DRAG: u64 = 0x400;

/// Owner and userInfo storage (keyed by tracking area pointer).
static TRACKING_OWNERS: Mutex<Option<HashMap<u64, u64>>> = Mutex::new(None);

fn set_owner(obj: u64, owner: u64) {
    let mut guard = TRACKING_OWNERS.lock().unwrap();
    guard.get_or_insert_with(HashMap::new).insert(obj, owner);
}

fn get_owner(obj: u64) -> u64 {
    let guard = TRACKING_OWNERS.lock().unwrap();
    guard
        .as_ref()
        .and_then(|m| m.get(&obj))
        .copied()
        .unwrap_or(0)
}

/// `-[NSTrackingArea initWithRect:options:owner:userInfo:]`
pub extern "C" fn shim_nstrackingarea_init(
    obj: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    options: u64,
    owner: u64,
    _user_info: u64,
) -> u64 {
    let ptr = if obj == 0 {
        crate::pal_alloc().calloc(1, TRACKING_AREA_SIZE) as u64
    } else {
        obj
    };
    if ptr == 0 {
        return 0;
    }

    write_u64(ptr, 0, nsobj::nstrackingarea_class_addr());
    write_f64(ptr, RECT_X_OFF, x);
    write_f64(ptr, RECT_Y_OFF, y);
    write_f64(ptr, RECT_W_OFF, w);
    write_f64(ptr, RECT_H_OFF, h);
    write_u64(ptr, OPTIONS_OFF, options);
    if owner != 0 {
        set_owner(ptr, owner);
    }

    ptr
}

/// `-[NSTrackingArea rect]` — returns CGRect via x8 (sret).
pub extern "C" fn shim_nstrackingarea_rect_x(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, RECT_X_OFF)
}

pub extern "C" fn shim_nstrackingarea_rect_y(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, RECT_Y_OFF)
}

pub extern "C" fn shim_nstrackingarea_rect_w(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, RECT_W_OFF)
}

pub extern "C" fn shim_nstrackingarea_rect_h(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, RECT_H_OFF)
}

/// `-[NSTrackingArea options]`
pub extern "C" fn shim_nstrackingarea_options(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, OPTIONS_OFF)
}

/// `-[NSTrackingArea owner]`
pub extern "C" fn shim_nstrackingarea_owner(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    get_owner(obj)
}

/// `-[NSTrackingArea userInfo]` — stub returns nil.
pub extern "C" fn shim_nstrackingarea_user_info(_obj: u64, _cmd: u64) -> u64 {
    0
}

// ── NSView tracking area support ────────────────────────────────

/// Per-view tracking area storage.
static VIEW_TRACKING_AREAS: Mutex<Option<HashMap<u64, Vec<u64>>>> = Mutex::new(None);

/// `-[NSView addTrackingArea:]`
pub extern "C" fn shim_nsview_add_tracking_area(obj: u64, _cmd: u64, area: u64) {
    if obj == 0 || area == 0 {
        return;
    }
    let mut guard = VIEW_TRACKING_AREAS.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.entry(obj).or_default().push(area);
}

/// `-[NSView removeTrackingArea:]`
pub extern "C" fn shim_nsview_remove_tracking_area(obj: u64, _cmd: u64, area: u64) {
    if obj == 0 || area == 0 {
        return;
    }
    let mut guard = VIEW_TRACKING_AREAS.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        if let Some(areas) = map.get_mut(&obj) {
            areas.retain(|&a| a != area);
        }
    }
}

/// `-[NSView trackingAreas]`
pub extern "C" fn shim_nsview_tracking_areas(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let guard = VIEW_TRACKING_AREAS.lock().unwrap();
    let areas = guard
        .as_ref()
        .and_then(|m| m.get(&obj))
        .cloned()
        .unwrap_or_default();
    crate::nsarray::make_nsarray(&areas)
}

/// `-[NSView updateTrackingAreas]` — no-op stub (views can override).
pub extern "C" fn shim_nsview_update_tracking_areas(_obj: u64, _cmd: u64) {}

/// Check tracking areas for a view at a given point (in the view's coordinate
/// system).  Returns a list of `(tracking_area, owner)` pairs whose rect
/// contains the point and whose options include `MouseEnteredAndExited`.
pub(crate) fn tracking_areas_containing_point(view: u64, px: f64, py: f64) -> Vec<(u64, u64)> {
    if view == 0 {
        return Vec::new();
    }
    let guard = VIEW_TRACKING_AREAS.lock().unwrap();
    let areas = match guard.as_ref().and_then(|m| m.get(&view)) {
        Some(a) => a,
        None => return Vec::new(),
    };
    let mut result = Vec::new();
    for &area in areas {
        let opts = read_u64(area, OPTIONS_OFF);
        if opts & MOUSE_ENTERED_AND_EXITED == 0 {
            continue;
        }
        let rx = read_f64(area, RECT_X_OFF);
        let ry = read_f64(area, RECT_Y_OFF);
        let rw = read_f64(area, RECT_W_OFF);
        let rh = read_f64(area, RECT_H_OFF);
        if px >= rx && px <= rx + rw && py >= ry && py <= ry + rh {
            result.push((area, get_owner(area)));
        }
    }
    result
}

// ── Per-window mouse-hover tracking ─────────────────────────────

/// Tracks the last view the mouse was hovering over, per-window.
static LAST_HOVER_VIEW: Mutex<Option<HashMap<u64, u64>>> = Mutex::new(None);

/// Process mouse tracking for a mouse-moved event.
///
/// Compares the target view at the mouse position against the last known
/// hover view and dispatches `mouseExited:` / `mouseEntered:` as needed.
pub(crate) fn process_mouse_tracking(window: u64, current_view: u64, event: u64) {
    let previous_view = {
        let guard = LAST_HOVER_VIEW.lock().unwrap();
        guard
            .as_ref()
            .and_then(|m| m.get(&window))
            .copied()
            .unwrap_or(0)
    };

    if previous_view == current_view {
        return;
    }

    // mouseExited: on the previous view's tracking area owner.
    if previous_view != 0 {
        // We don't know the exact point in the old view, but the event itself
        // carries the window-level coordinates.
        let owners_exited = tracking_areas_containing_point(previous_view, f64::MAX, f64::MAX);
        // Since the mouse *left* the view entirely, fire exit on all tracking areas.
        for &(_area, owner) in &owners_exited {
            if owner != 0 {
                crate::nsapplication::dispatch_selector_named_to_target(
                    owner,
                    "mouseExited:",
                    event,
                );
            }
        }
        if owners_exited.is_empty() {
            // Fall back to the view itself.
            crate::nsapplication::dispatch_selector_named_to_target(
                previous_view,
                "mouseExited:",
                event,
            );
        }
    }

    // mouseEntered: on the new view's tracking area owners.
    if current_view != 0 {
        let (px, py) = crate::nsevent::event_location_in_window(event);
        let local = crate::nsview::shim_nsview_convert_point_from_view(current_view, 0, px, py, 0);
        let owners_entered = tracking_areas_containing_point(current_view, local.x, local.y);
        for &(_area, owner) in &owners_entered {
            if owner != 0 {
                crate::nsapplication::dispatch_selector_named_to_target(
                    owner,
                    "mouseEntered:",
                    event,
                );
            }
        }
        if owners_entered.is_empty() {
            // Fall back to the view itself.
            crate::nsapplication::dispatch_selector_named_to_target(
                current_view,
                "mouseEntered:",
                event,
            );
        }
    }

    // Update tracking state.
    let mut guard = LAST_HOVER_VIEW.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    if current_view != 0 {
        map.insert(window, current_view);
    } else {
        map.remove(&window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn init_and_accessors() {
        let ta = shim_nstrackingarea_init(
            0,
            0,
            10.0,
            20.0,
            100.0,
            50.0,
            MOUSE_ENTERED_AND_EXITED | ACTIVE_ALWAYS,
            0xBEEF,
            0,
        );
        assert_ne!(ta, 0);
        assert_eq!(read_u64(ta, 0), nsobj::nstrackingarea_class_addr());
        assert_eq!(shim_nstrackingarea_rect_x(ta, 0), 10.0);
        assert_eq!(shim_nstrackingarea_rect_y(ta, 0), 20.0);
        assert_eq!(shim_nstrackingarea_rect_w(ta, 0), 100.0);
        assert_eq!(shim_nstrackingarea_rect_h(ta, 0), 50.0);
        assert_eq!(
            shim_nstrackingarea_options(ta, 0),
            MOUSE_ENTERED_AND_EXITED | ACTIVE_ALWAYS
        );
        assert_eq!(shim_nstrackingarea_owner(ta, 0), 0xBEEF);
        free_obj(ta);
    }

    #[test]
    fn view_tracking_areas() {
        let view = alloc_obj(80, nsobj::nsview_class_addr());
        let ta1 = shim_nstrackingarea_init(0, 0, 0.0, 0.0, 50.0, 50.0, MOUSE_MOVED, 0, 0);
        let ta2 = shim_nstrackingarea_init(0, 0, 50.0, 0.0, 50.0, 50.0, CURSOR_UPDATE, 0, 0);

        shim_nsview_add_tracking_area(view, 0, ta1);
        shim_nsview_add_tracking_area(view, 0, ta2);

        let arr = shim_nsview_tracking_areas(view, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 2);

        shim_nsview_remove_tracking_area(view, 0, ta1);
        let arr2 = shim_nsview_tracking_areas(view, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr2, 0), 1);

        free_obj(ta1);
        free_obj(ta2);
        free_obj(view);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstrackingarea_options(0, 0), 0);
        assert_eq!(shim_nstrackingarea_owner(0, 0), 0);
        shim_nsview_add_tracking_area(0, 0, 0);
        shim_nsview_remove_tracking_area(0, 0, 0);
    }
}
