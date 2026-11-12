//! NSWindow shim: window stub for AppKit bootstrap.
//!
//! Object layout:
//!
//!   offset  0: isa              (u64)
//!   offset  8: content_view     (u64 — NSView*)
//!   offset 16: title_ptr        (u64 — NSString*)
//!   offset 24: style_mask       (u64)
//!   offset 32: frame_x          (f64)
//!   offset 40: frame_y          (f64)
//!   offset 48: frame_w          (f64)
//!   offset 56: frame_h          (f64)
//!   offset 64: is_visible       (u64 — 1 if shown, 0 otherwise)
//!   offset 72: first_responder  (u64 — current first responder, defaults to self)
//!   offset 80: delegate_ptr     (u64 — window delegate, or 0)
//!   offset 88: background_color (u64 — NSColor*, or 0)
//!   offset 96:  min_w           (f64 — minSize width, default 0)
//!   offset 104: min_h           (f64 — minSize height, default 0)
//!   offset 112: max_w           (f64 — maxSize width, default f64::MAX)
//!   offset 120: max_h           (f64 — maxSize height, default f64::MAX)
//!
//! Total: 128 bytes.

use crate::cg_types::{CGPoint, CGRect, CGSize};

const CONTENT_VIEW_OFF: usize = 8;
const TITLE_OFF: usize = 16;
const STYLE_MASK_OFF: usize = 24;
const FRAME_X_OFF: usize = 32;
const FRAME_W_OFF: usize = 48;
const FRAME_H_OFF: usize = 56;
const IS_VISIBLE_OFF: usize = 64;
const FIRST_RESPONDER_OFF: usize = 72;
const DELEGATE_OFF: usize = 80;
const BG_COLOR_OFF: usize = 88;
const MIN_W_OFF: usize = 96;
const MIN_H_OFF: usize = 104;
const MAX_W_OFF: usize = 112;
const MAX_H_OFF: usize = 120;
const WINDOW_SIZE: usize = 128;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn read_f64(obj: u64, offset: usize) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + offset) as *const f64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut u64) = value;
    }
}

fn effective_first_responder(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let responder = read_u64(obj, FIRST_RESPONDER_OFF);
    if responder != 0 {
        responder
    } else {
        obj
    }
}

fn object_is_kind_of(obj: u64, class_addr: u64) -> bool {
    if obj == 0 || class_addr == 0 {
        return false;
    }

    let isa = read_u64(obj, 0);
    if isa == class_addr {
        return true;
    }

    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return false;
    };

    let mut current = Some(isa);
    let mut depth = 0;
    while let Some(addr) = current {
        if depth > 64 {
            break;
        }
        if addr == class_addr {
            return true;
        }
        current = rt
            .get_class_by_addr(addr)
            .and_then(|class| class.superclass_addr);
        depth += 1;
    }
    false
}

fn normalize_first_responder(window: u64, responder: u64) -> u64 {
    if window == 0 {
        return 0;
    }
    if responder == 0 || responder == window {
        return window;
    }
    if object_is_kind_of(responder, crate::nsobj::nswindow_class_addr()) {
        return window;
    }
    if object_is_kind_of(responder, crate::nsobj::nsview_class_addr()) {
        if crate::nsview::shim_nsview_window(responder, 0) == window {
            return responder;
        }
        return window;
    }
    responder
}

pub(crate) fn is_window_visible(obj: u64) -> bool {
    read_u64(obj, IS_VISIBLE_OFF) != 0
}

/// Build a `WindowDesc` for the GUI bridge from a guest window.
pub(crate) fn window_desc(obj: u64) -> crate::gui_bridge::WindowDesc {
    let title = {
        let title_ptr = read_u64(obj, TITLE_OFF);
        if title_ptr != 0 {
            crate::cfstring::extract_cfstring_cstr(title_ptr)
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        }
    };
    let (x, y, w, h) = if obj != 0 {
        unsafe {
            (
                *((obj as usize + FRAME_X_OFF) as *const f64),
                *((obj as usize + FRAME_X_OFF + 8) as *const f64),
                *((obj as usize + FRAME_X_OFF + 16) as *const f64),
                *((obj as usize + FRAME_X_OFF + 24) as *const f64),
            )
        }
    } else {
        (0.0, 0.0, 800.0, 600.0)
    };
    crate::gui_bridge::WindowDesc {
        guest_window: obj,
        title,
        x,
        y,
        width: w,
        height: h,
    }
}

fn set_visible(obj: u64, is_visible: bool) {
    write_u64(obj, IS_VISIBLE_OFF, u64::from(is_visible));
}

fn alloc_window(rect: CGRect, style: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, WINDOW_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nswindow_class_addr();
        *((obj as usize + STYLE_MASK_OFF) as *mut u64) = style;
        *((obj as usize + FRAME_X_OFF) as *mut f64) = rect.origin.x;
        *((obj as usize + FRAME_X_OFF + 8) as *mut f64) = rect.origin.y;
        *((obj as usize + FRAME_X_OFF + 16) as *mut f64) = rect.size.width;
        *((obj as usize + FRAME_X_OFF + 24) as *mut f64) = rect.size.height;
        // Default min/max size
        *((obj as usize + MAX_W_OFF) as *mut f64) = f64::MAX;
        *((obj as usize + MAX_H_OFF) as *mut f64) = f64::MAX;
    }
    let content_view =
        crate::nsview::alloc_view_with_frame(0.0, 0.0, rect.size.width, rect.size.height);
    if content_view != 0 {
        crate::nsview::set_view_window(content_view, obj);
        write_u64(obj, CONTENT_VIEW_OFF, content_view);
    }
    write_u64(obj, FIRST_RESPONDER_OFF, obj);
    obj
}

/// `-[NSWindow initWithCoder:]`.
///
/// Real nib/storyboard windows are commonly keyed-archive objects.  Decode the
/// common AppKit keys and then reuse the normal content-view wiring path.
pub extern "C" fn shim_nswindow_init_with_coder(obj: u64, _cmd: u64, coder: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let (x, y, w, h) = crate::nskeyedarchiver::decode_rect_for_any_key(
        coder,
        &[
            "NSWindowFrame",
            "NSWindowFrameString",
            "NSWindowContentRect",
            "NSContentRect",
            "NSFrame",
            "frame",
            "_frame",
        ],
    )
    .unwrap_or((0.0, 0.0, 800.0, 600.0));
    let style = crate::nskeyedarchiver::decode_integer_for_any_key(
        coder,
        &[
            "NSWindowStyleMask",
            "NSStyleMask",
            "NSWindowStyle",
            "styleMask",
            "_styleMask",
        ],
    )
    .unwrap_or(0)
    .max(0) as u64;

    init_window_in_place(
        obj,
        CGRect {
            origin: CGPoint { x, y },
            size: CGSize {
                width: w,
                height: h,
            },
        },
        style,
    );

    let title = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &["NSTitle", "NSWindowTitle", "title", "_title"],
    );
    if title != 0 {
        shim_nswindow_set_title(obj, 0, title);
    }

    let content_view = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &[
            "NSContentView",
            "NS.contentView",
            "contentView",
            "_contentView",
        ],
    );
    if content_view != 0 {
        shim_nswindow_set_content_view(obj, 0, content_view);
    }

    let delegate = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &["NSDelegate", "delegate", "_delegate"],
    );
    if delegate != 0 {
        shim_nswindow_set_delegate(obj, 0, delegate);
    }

    let background = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &[
            "NSBackgroundColor",
            "NSWindowBackgroundColor",
            "backgroundColor",
            "_backgroundColor",
        ],
    );
    if background != 0 {
        shim_nswindow_set_background_color(obj, 0, background);
    }

    crate::nsapplication::register_window(obj);
    obj
}

/// `-[NSWindow initWithContentRect:styleMask:backing:defer:]`
pub extern "C" fn shim_nswindow_init_with_content_rect(
    obj: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64, // CGRect passed as 4 doubles on ARM64
    style_mask: u64,
    _backing: u64,
    _defer: u8,
) -> u64 {
    let rect = CGRect {
        origin: CGPoint { x, y },
        size: CGSize {
            width: w,
            height: h,
        },
    };
    log::debug!(
        "NSWindow initWithContentRect:{} styleMask:{}",
        rect,
        style_mask
    );

    // If the caller already allocated the object (e.g. via +alloc or
    // objc_allocateClassPair), initialise it in-place so that any dynamic
    // subclass ISA is preserved.
    let window = if obj != 0 {
        init_window_in_place(obj, rect, style_mask);
        obj
    } else {
        alloc_window(rect, style_mask)
    };
    crate::nsapplication::register_window(window);
    window
}

/// Initialise an already-allocated NSWindow object in-place (preserving its ISA).
fn init_window_in_place(obj: u64, rect: CGRect, style: u64) {
    unsafe {
        // ISA at offset 0 is already set from +alloc — do NOT overwrite.
        *((obj as usize + STYLE_MASK_OFF) as *mut u64) = style;
        *((obj as usize + FRAME_X_OFF) as *mut f64) = rect.origin.x;
        *((obj as usize + FRAME_X_OFF + 8) as *mut f64) = rect.origin.y;
        *((obj as usize + FRAME_X_OFF + 16) as *mut f64) = rect.size.width;
        *((obj as usize + FRAME_X_OFF + 24) as *mut f64) = rect.size.height;
        // Default min/max size
        *((obj as usize + MIN_W_OFF) as *mut f64) = 0.0;
        *((obj as usize + MIN_H_OFF) as *mut f64) = 0.0;
        *((obj as usize + MAX_W_OFF) as *mut f64) = f64::MAX;
        *((obj as usize + MAX_H_OFF) as *mut f64) = f64::MAX;
    }
    let content_view =
        crate::nsview::alloc_view_with_frame(0.0, 0.0, rect.size.width, rect.size.height);
    if content_view != 0 {
        crate::nsview::set_view_window(content_view, obj);
        write_u64(obj, CONTENT_VIEW_OFF, content_view);
    }
    write_u64(obj, FIRST_RESPONDER_OFF, obj);
}

/// `-[NSWindow contentView]`
pub extern "C" fn shim_nswindow_content_view(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, CONTENT_VIEW_OFF)
}

/// Internal accessor for other modules.
pub(crate) fn window_content_view(obj: u64) -> u64 {
    read_u64(obj, CONTENT_VIEW_OFF)
}

/// `-[NSWindow setContentView:]`
pub extern "C" fn shim_nswindow_set_content_view(obj: u64, _cmd: u64, view: u64) {
    if obj == 0 {
        return;
    }
    let previous = read_u64(obj, CONTENT_VIEW_OFF);
    if previous != 0 && previous != view {
        crate::nsview::set_view_superview(previous, 0);
        crate::nsview::set_view_window(previous, 0);
    }
    write_u64(obj, CONTENT_VIEW_OFF, view);
    if view != 0 {
        crate::nsview::remove_view_from_superview(view);
        crate::nsview::set_view_superview(view, 0);
        crate::nsview::set_view_window(view, obj);
    }

    let current = effective_first_responder(obj);
    if current == previous
        || (object_is_kind_of(current, crate::nsobj::nsview_class_addr())
            && crate::nsview::shim_nsview_window(current, 0) != obj)
    {
        write_u64(obj, FIRST_RESPONDER_OFF, obj);
    }
}

/// `-[NSWindow firstResponder]`
pub extern "C" fn shim_nswindow_first_responder(obj: u64, _cmd: u64) -> u64 {
    effective_first_responder(obj)
}

/// Dispatch a no-arg BOOL-returning selector through the ObjC runtime.
/// Returns `true` (YES) when the runtime or method is unavailable.
fn dispatch_bool_selector_default_yes(target: u64, selector: &str) -> bool {
    if target == 0 {
        return true;
    }

    let isa = unsafe { *(target as *const u64) };
    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return true;
    };
    let Some(imp) = rt.msg_lookup(isa, selector) else {
        return true;
    };
    let sel_ptr = rt.selector_ptr(selector).unwrap_or(0) as u64;
    drop(guard);

    let func: extern "C" fn(u64, u64) -> u8 = unsafe { std::mem::transmute(imp) };
    func(target, sel_ptr) != 0
}

/// `-[NSWindow makeFirstResponder:]`
pub extern "C" fn shim_nswindow_make_first_responder(obj: u64, _cmd: u64, responder: u64) -> u8 {
    if obj == 0 {
        return 0;
    }

    let new = normalize_first_responder(obj, responder);
    let old = effective_first_responder(obj);

    if new == old {
        return 1;
    }

    if old != 0 && old != obj && !dispatch_bool_selector_default_yes(old, "resignFirstResponder") {
        return 0;
    }

    if new != obj && !dispatch_bool_selector_default_yes(new, "becomeFirstResponder") {
        write_u64(obj, FIRST_RESPONDER_OFF, obj);
        return 0;
    }

    write_u64(obj, FIRST_RESPONDER_OFF, new);
    1
}

/// `-[NSWindow sendEvent:]`
pub extern "C" fn shim_nswindow_send_event(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 {
        return;
    }

    let previous_event = crate::nsapplication::push_current_event(event);
    if crate::nsevent::event_window(event) == 0 {
        crate::nsevent::set_event_window(event, obj);
    }

    let event_type = crate::nsevent::event_type_raw(event);

    let dispatch_target = if crate::nsevent::event_targets_first_responder(event) {
        // Key events → first responder
        let responder = shim_nswindow_first_responder(obj, 0);
        if responder != 0 {
            responder
        } else {
            obj
        }
    } else {
        // Mouse events → hitTest: to find actual target view
        let content_view = shim_nswindow_content_view(obj, 0);
        if content_view != 0 {
            let (px, py) = crate::nsevent::event_location_in_window(event);
            let hit = crate::nsview::shim_nsview_hit_test(content_view, 0, px, py);
            if hit != 0 {
                hit
            } else {
                content_view
            }
        } else {
            let responder = shim_nswindow_first_responder(obj, 0);
            if responder != 0 {
                responder
            } else {
                obj
            }
        }
    };

    // For mouse-moved events, process tracking areas (mouseEntered:/mouseExited:).
    if event_type == 5 {
        // NSEventTypeMouseMoved
        crate::nstrackingarea::process_mouse_tracking(obj, dispatch_target, event);
    }

    if let Some(selector) = crate::nsevent::event_dispatch_selector(event) {
        let _ = crate::nsapplication::dispatch_selector_named_to_target(
            dispatch_target,
            selector,
            event,
        );
    }

    crate::nsapplication::restore_previous_current_event(previous_event, event);
}

/// `-[NSWindow makeKeyAndOrderFront:]` — marks the window visible.
pub extern "C" fn shim_nswindow_make_key_and_order_front(obj: u64, _cmd: u64, _sender: u64) {
    set_visible(obj, true);
    crate::nsapplication::promote_window(obj);
    notify_key_window_change(obj);
    crate::nsapplication::set_main_window(obj);

    // Notify GUI bridge to create/show a real OS window.
    if let Some(bridge) = crate::gui_bridge::gui_bridge() {
        let desc = window_desc(obj);
        bridge.show_window(&desc);
    }
}

/// `-[NSWindow makeKeyWindow]` — makes this the application's key window.
pub extern "C" fn shim_nswindow_make_key_window(obj: u64, _cmd: u64) {
    notify_key_window_change(obj);
}

/// Notify old key window's delegate of `windowDidResignKey:` and new
/// key window's delegate of `windowDidBecomeKey:`.
fn notify_key_window_change(obj: u64) {
    if obj == 0 {
        return;
    }
    let old_key = crate::nsapplication::current_key_window();
    crate::nsapplication::set_key_window(obj);

    // Resign old key window.
    if old_key != 0 && old_key != obj {
        let delegate = read_u64(old_key, DELEGATE_OFF);
        if delegate != 0 {
            let name_cstr = std::ffi::CString::new("NSWindowDidResignKeyNotification").unwrap();
            let name_str = crate::cfstring::alloc_nsstring_from_cstr(name_cstr.as_ptr());
            let note = crate::nsnotificationcenter::make_nsnotification(name_str, old_key, 0);
            crate::nsapplication::dispatch_selector_named_to_target(
                delegate,
                "windowDidResignKey:",
                note,
            );
        }
    }

    // Become new key window.
    let delegate = read_u64(obj, DELEGATE_OFF);
    if delegate != 0 {
        let name_cstr = std::ffi::CString::new("NSWindowDidBecomeKeyNotification").unwrap();
        let name_str = crate::cfstring::alloc_nsstring_from_cstr(name_cstr.as_ptr());
        let note = crate::nsnotificationcenter::make_nsnotification(name_str, obj, 0);
        crate::nsapplication::dispatch_selector_named_to_target(
            delegate,
            "windowDidBecomeKey:",
            note,
        );
    }
}

/// `-[NSWindow makeMainWindow]` — makes this the application's main window.
pub extern "C" fn shim_nswindow_make_main_window(obj: u64, _cmd: u64) {
    crate::nsapplication::set_main_window(obj);
}

/// `-[NSWindow title]`
pub extern "C" fn shim_nswindow_title(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, TITLE_OFF)
}

/// `-[NSWindow setTitle:]`
pub extern "C" fn shim_nswindow_set_title(obj: u64, _cmd: u64, title: u64) {
    write_u64(obj, TITLE_OFF, title);

    if let Some(bridge) = crate::gui_bridge::gui_bridge() {
        let s = if title != 0 {
            crate::cfstring::extract_cfstring_cstr(title)
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };
        bridge.set_window_title(obj, &s);
    }
}

/// `-[NSWindow styleMask]`
pub extern "C" fn shim_nswindow_style_mask(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, STYLE_MASK_OFF)
}

/// `-[NSWindow frame]` — returns CGRect (32 bytes, x8 indirect return on ARM64).
pub extern "C" fn shim_nswindow_frame(obj: u64, _cmd: u64) -> CGRect {
    if obj == 0 {
        return CGRect::ZERO;
    }
    unsafe {
        CGRect::new(
            *((obj as usize + FRAME_X_OFF) as *const f64),
            *((obj as usize + FRAME_X_OFF + 8) as *const f64),
            *((obj as usize + FRAME_X_OFF + 16) as *const f64),
            *((obj as usize + FRAME_X_OFF + 24) as *const f64),
        )
    }
}

/// `-[NSWindow contentRectForFrameRect:]` — returns content rect (frame minus title bar).
pub extern "C" fn shim_nswindow_content_rect_for_frame_rect(
    _obj: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> CGRect {
    // Approximate: 22pt title bar
    CGRect::new(x, y, w, h - 22.0)
}

/// `-[NSWindow close]` — hides the window.
pub extern "C" fn shim_nswindow_close(obj: u64, _cmd: u64) {
    // Notify delegate of windowWillClose: before hiding.
    let delegate = read_u64(obj, DELEGATE_OFF);
    if delegate != 0 {
        let name_cstr = std::ffi::CString::new("NSWindowWillCloseNotification").unwrap();
        let name_str = crate::cfstring::alloc_nsstring_from_cstr(name_cstr.as_ptr());
        let note = crate::nsnotificationcenter::make_nsnotification(name_str, obj, 0);
        crate::nsapplication::dispatch_selector_named_to_target(delegate, "windowWillClose:", note);
    }

    set_visible(obj, false);
    crate::nsapplication::clear_window_roles(obj);
    release_window_cg_context(obj);

    if let Some(bridge) = crate::gui_bridge::gui_bridge() {
        bridge.hide_window(obj);
    }
}

/// `-[NSWindow orderOut:]` — hides the window without destroying it.
pub extern "C" fn shim_nswindow_order_out(obj: u64, _cmd: u64, _sender: u64) {
    set_visible(obj, false);
    crate::nsapplication::clear_window_roles(obj);

    if let Some(bridge) = crate::gui_bridge::gui_bridge() {
        bridge.hide_window(obj);
    }
}

/// `-[NSWindow orderFront:]` — marks the window visible.
pub extern "C" fn shim_nswindow_order_front(obj: u64, _cmd: u64, _sender: u64) {
    set_visible(obj, true);
    crate::nsapplication::promote_window(obj);

    if let Some(bridge) = crate::gui_bridge::gui_bridge() {
        let desc = window_desc(obj);
        bridge.show_window(&desc);
    }
}

/// `-[NSWindow isVisible]`
pub extern "C" fn shim_nswindow_is_visible(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(is_window_visible(obj))
}

/// `-[NSWindow isKeyWindow]`
pub extern "C" fn shim_nswindow_is_key_window(obj: u64, _cmd: u64) -> u8 {
    u8::from(obj != 0 && crate::nsapplication::current_key_window() == obj)
}

/// `-[NSWindow isMainWindow]`
pub extern "C" fn shim_nswindow_is_main_window(obj: u64, _cmd: u64) -> u8 {
    u8::from(obj != 0 && crate::nsapplication::current_main_window() == obj)
}

/// `-[NSWindow delegate]`
pub extern "C" fn shim_nswindow_delegate(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DELEGATE_OFF)
}

/// `-[NSWindow setDelegate:]`
pub extern "C" fn shim_nswindow_set_delegate(obj: u64, _cmd: u64, delegate: u64) {
    write_u64(obj, DELEGATE_OFF, delegate);
}

/// `-[NSWindow performKeyEquivalent:]`
pub extern "C" fn shim_nswindow_perform_key_equivalent(obj: u64, _cmd: u64, event: u64) -> u8 {
    if obj == 0 || event == 0 {
        return 0;
    }

    let content = shim_nswindow_content_view(obj, 0);
    if content != 0
        && crate::nsapplication::dispatch_bool_selector_named_to_target(
            content,
            "performKeyEquivalent:",
            event,
        )
    {
        return 1;
    }

    0
}

/// `-[NSWindow setFrame:display:]` — updates window frame geometry.
///
/// After updating the frame, resizes the content view and notifies the
/// delegate with `windowDidResize:` if the size actually changed.
pub extern "C" fn shim_nswindow_set_frame_display(
    obj: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    _display: u8,
) {
    if obj == 0 {
        return;
    }
    // Read old size for change detection.
    let old_w = unsafe { *((obj as usize + FRAME_W_OFF) as *const f64) };
    let old_h = unsafe { *((obj as usize + FRAME_H_OFF) as *const f64) };

    // Clamp to min/max size (max_w/max_h of 0 means unset → no upper limit)
    let min_w = unsafe { *((obj as usize + MIN_W_OFF) as *const f64) };
    let min_h = unsafe { *((obj as usize + MIN_H_OFF) as *const f64) };
    let max_w = unsafe { *((obj as usize + MAX_W_OFF) as *const f64) };
    let max_h = unsafe { *((obj as usize + MAX_H_OFF) as *const f64) };
    let w = if max_w > 0.0 {
        w.max(min_w).min(max_w)
    } else {
        w.max(min_w)
    };
    let h = if max_h > 0.0 {
        h.max(min_h).min(max_h)
    } else {
        h.max(min_h)
    };
    unsafe {
        *((obj as usize + FRAME_X_OFF) as *mut f64) = x;
        *((obj as usize + FRAME_X_OFF + 8) as *mut f64) = y;
        *((obj as usize + FRAME_X_OFF + 16) as *mut f64) = w;
        *((obj as usize + FRAME_X_OFF + 24) as *mut f64) = h;
    }

    // Resize the content view to match the new window size.
    let content_view = read_u64(obj, CONTENT_VIEW_OFF);
    if content_view != 0 && (old_w != w || old_h != h) {
        crate::nsview::shim_nsview_set_frame(content_view, 0, 0.0, 0.0, w, h);
    }

    if let Some(bridge) = crate::gui_bridge::gui_bridge() {
        bridge.set_window_frame(obj, x, y, w, h);
    }

    // Notify delegate of resize if size actually changed.
    if old_w != w || old_h != h {
        let delegate = read_u64(obj, DELEGATE_OFF);
        if delegate != 0 {
            // Create an NSNotification with the window as the object.
            let name_cstr = std::ffi::CString::new("NSWindowDidResizeNotification").unwrap();
            let name_str = crate::cfstring::alloc_nsstring_from_cstr(name_cstr.as_ptr());
            let note = crate::nsnotificationcenter::make_nsnotification(name_str, obj, 0);
            crate::nsapplication::dispatch_selector_named_to_target(
                delegate,
                "windowDidResize:",
                note,
            );
        }
    }
}

/// `-[NSWindow center]` — centres on the main screen (stub: 1920×1080).
pub extern "C" fn shim_nswindow_center(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let w: f64 = unsafe { *((obj as usize + FRAME_X_OFF + 16) as *const f64) };
    let h: f64 = unsafe { *((obj as usize + FRAME_X_OFF + 24) as *const f64) };
    let new_x = (1920.0 - w) / 2.0;
    let new_y = (1080.0 - h) / 2.0;
    unsafe {
        *((obj as usize + FRAME_X_OFF) as *mut f64) = new_x;
        *((obj as usize + FRAME_X_OFF + 8) as *mut f64) = new_y;
    }
}

/// `-[NSWindow screen]` — returns the main screen singleton.
pub extern "C" fn shim_nswindow_screen(_obj: u64, _cmd: u64) -> u64 {
    crate::nsscreen::shim_nsscreen_main_screen(0, 0)
}

/// `-[NSWindow backgroundColor]`
pub extern "C" fn shim_nswindow_background_color(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, BG_COLOR_OFF)
}

/// `-[NSWindow setBackgroundColor:]`
pub extern "C" fn shim_nswindow_set_background_color(obj: u64, _cmd: u64, color: u64) {
    write_u64(obj, BG_COLOR_OFF, color);
}

// ── Min / Max size ─────────────────────────────────────────────

/// `-[NSWindow minSize]`
pub extern "C" fn shim_nswindow_min_size(obj: u64, _cmd: u64) -> CGSize {
    if obj == 0 {
        return CGSize {
            width: 0.0,
            height: 0.0,
        };
    }
    CGSize {
        width: unsafe { *((obj as usize + MIN_W_OFF) as *const f64) },
        height: unsafe { *((obj as usize + MIN_H_OFF) as *const f64) },
    }
}

/// `-[NSWindow setMinSize:]`
pub extern "C" fn shim_nswindow_set_min_size(obj: u64, _cmd: u64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + MIN_W_OFF) as *mut f64) = w;
        *((obj as usize + MIN_H_OFF) as *mut f64) = h;
    }
}

/// `-[NSWindow maxSize]`
pub extern "C" fn shim_nswindow_max_size(obj: u64, _cmd: u64) -> CGSize {
    if obj == 0 {
        return CGSize {
            width: f64::MAX,
            height: f64::MAX,
        };
    }
    CGSize {
        width: unsafe { *((obj as usize + MAX_W_OFF) as *const f64) },
        height: unsafe { *((obj as usize + MAX_H_OFF) as *const f64) },
    }
}

/// `-[NSWindow setMaxSize:]`
pub extern "C" fn shim_nswindow_set_max_size(obj: u64, _cmd: u64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + MAX_W_OFF) as *mut f64) = w;
        *((obj as usize + MAX_H_OFF) as *mut f64) = h;
    }
}

/// `-[NSWindow performClose:]`
pub extern "C" fn shim_nswindow_perform_close(obj: u64, _cmd: u64, _sender: u64) {
    if obj == 0 {
        return;
    }

    let delegate = read_u64(obj, DELEGATE_OFF);
    if delegate != 0 {
        let isa = unsafe { *(delegate as *const u64) };
        let should_close = {
            let guard = match objc_runtime::OBJC_RUNTIME.read() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            guard.as_ref().and_then(|rt| {
                let imp = rt.msg_lookup(isa, "windowShouldClose:")?;
                let sel_ptr = rt.selector_ptr("windowShouldClose:").unwrap_or(0) as u64;
                Some((imp, sel_ptr))
            })
        };
        if let Some((imp, sel_ptr)) = should_close {
            let func: extern "C" fn(u64, u64, u64) -> u8 = unsafe { std::mem::transmute(imp) };
            if func(delegate, sel_ptr, obj) == 0 {
                return;
            }
        }
    }

    shim_nswindow_close(obj, 0);
}

/// `-[NSWindow contentLayoutGuide]` — returns an NSLayoutGuide whose anchors
/// match the window's content view bounds.
pub extern "C" fn shim_nswindow_content_layout_guide(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let content_view = read_u64(obj, CONTENT_VIEW_OFF);
    if content_view == 0 {
        return 0;
    }
    crate::nslayoutguide::make_layout_guide(content_view)
}

// ── Window drawing pipeline ────────────────────────────────────

use std::collections::HashMap as WindowMap;
use std::sync::Mutex;

/// Per-window CGContext handle, keyed by guest NSWindow address.
static WINDOW_CONTEXTS: std::sync::OnceLock<Mutex<WindowMap<u64, u64>>> =
    std::sync::OnceLock::new();

fn window_ctx_map() -> &'static Mutex<WindowMap<u64, u64>> {
    WINDOW_CONTEXTS.get_or_init(|| Mutex::new(WindowMap::new()))
}

/// Get or create a CGContext for a window. The context matches the window's frame size.
pub(crate) fn get_or_create_window_cg_context(window: u64) -> u64 {
    if window == 0 {
        return 0;
    }

    let w = read_f64(window, FRAME_W_OFF) as u32;
    let h = read_f64(window, FRAME_H_OFF) as u32;
    let w = if w == 0 { 800 } else { w };
    let h = if h == 0 { 600 } else { h };

    let mut map = window_ctx_map().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&ctx) = map.get(&window) {
        // Check if size matches; if not, recreate.
        let cur_w = cg_shims::bridge::context_get_width(ctx) as u32;
        let cur_h = cg_shims::bridge::context_get_height(ctx) as u32;
        if cur_w == w && cur_h == h {
            return ctx;
        }
        cg_shims::bridge::context_release(ctx);
    }

    let ctx = cg_shims::bridge::context_create(w, h);
    map.insert(window, ctx);
    ctx
}

/// Called by the GUI bridge on `RedrawRequested` to trigger the guest draw cycle.
///
/// Creates/updates the window's CGContext, runs the display cycle on the content view,
/// then copies the CGContext's pixels to the bridge's window surface.
pub(crate) fn display_window_content(window: u64) {
    if window == 0 {
        return;
    }

    let content_view = read_u64(window, CONTENT_VIEW_OFF);
    if content_view == 0 {
        return;
    }

    let cg_ctx = get_or_create_window_cg_context(window);
    if cg_ctx == 0 {
        return;
    }

    // Clear the surface before drawing.
    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.surface.clear(graphics_core::Color::WHITE);
    });

    // Run the display cycle: pushes NSGraphicsContext, calls drawRect: recursively.
    crate::nsview::display_view_recursive(content_view, cg_ctx);

    // Copy pixels from CGContext surface to the bridge's window surface.
    if let Some(bridge) = crate::gui_bridge::gui_bridge() {
        if let Some((dst_ptr, dst_w, dst_h, _stride)) = bridge.window_surface(window) {
            let src_pixels = cg_shims::bridge::with_context(cg_ctx, |c| {
                let pixels = c.surface.pixels();
                let src_w = c.surface.width();
                let src_h = c.surface.height();
                (pixels.as_ptr(), src_w, src_h)
            });
            if let Some((src_ptr, src_w, src_h)) = src_pixels {
                let copy_w = (src_w).min(dst_w) as usize;
                let copy_h = (src_h).min(dst_h) as usize;
                // Copy row by row (ARGB32 → u32 per pixel).
                unsafe {
                    let src = src_ptr as *const u32;
                    let dst = dst_ptr as *mut u32;
                    for row in 0..copy_h {
                        std::ptr::copy_nonoverlapping(
                            src.add(row * src_w as usize),
                            dst.add(row * dst_w as usize),
                            copy_w,
                        );
                    }
                }
            }
        }
    }
}

/// Release the CGContext for a window (called when window closes).
pub(crate) fn release_window_cg_context(window: u64) {
    let mut map = window_ctx_map().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ctx) = map.remove(&window) {
        cg_shims::bridge::context_release(ctx);
    }
}

// ── Full-screen toggle ────────────────────────────────────────────

use std::collections::HashMap as StdHashMap;

/// Saved pre-full-screen frames, keyed by window pointer.
static FULLSCREEN_SAVED_FRAMES: std::sync::Mutex<Option<StdHashMap<u64, (f64, f64, f64, f64)>>> =
    std::sync::Mutex::new(None);

fn fullscreen_saved_frame(obj: u64) -> Option<(f64, f64, f64, f64)> {
    FULLSCREEN_SAVED_FRAMES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|m| m.get(&obj).copied())
}

fn set_fullscreen_saved_frame(obj: u64, frame: Option<(f64, f64, f64, f64)>) {
    let mut guard = FULLSCREEN_SAVED_FRAMES
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(StdHashMap::new);
    match frame {
        Some(f) => {
            map.insert(obj, f);
        }
        None => {
            map.remove(&obj);
        }
    }
}

/// `-[NSWindow toggleFullScreen:]` — toggles between full-screen and windowed.
pub extern "C" fn shim_nswindow_toggle_full_screen(obj: u64, _cmd: u64, _sender: u64) {
    if obj == 0 {
        return;
    }

    if let Some((sx, sy, sw, sh)) = fullscreen_saved_frame(obj) {
        // Currently full-screen — restore saved frame.
        set_fullscreen_saved_frame(obj, None);
        shim_nswindow_set_frame_display(obj, 0, sx, sy, sw, sh, 1);

        // Notify delegate of windowDidExitFullScreen:.
        let delegate = read_u64(obj, DELEGATE_OFF);
        if delegate != 0 {
            let name_cstr =
                std::ffi::CString::new("NSWindowDidExitFullScreenNotification").unwrap();
            let name_str = crate::cfstring::alloc_nsstring_from_cstr(name_cstr.as_ptr());
            let note = crate::nsnotificationcenter::make_nsnotification(name_str, obj, 0);
            crate::nsapplication::dispatch_selector_named_to_target(
                delegate,
                "windowDidExitFullScreen:",
                note,
            );
        }
    } else {
        // Not full-screen — save current frame and go full-screen.
        let frame = shim_nswindow_frame(obj, 0);
        set_fullscreen_saved_frame(
            obj,
            Some((
                frame.origin.x,
                frame.origin.y,
                frame.size.width,
                frame.size.height,
            )),
        );

        // Get screen bounds (main screen).
        let screen_frame = crate::nsscreen::shim_nsscreen_frame(
            crate::nsscreen::shim_nsscreen_main_screen(0, 0),
            0,
        );
        shim_nswindow_set_frame_display(
            obj,
            0,
            screen_frame.origin.x,
            screen_frame.origin.y,
            screen_frame.size.width,
            screen_frame.size.height,
            1,
        );

        // Notify delegate of windowDidEnterFullScreen:.
        let delegate = read_u64(obj, DELEGATE_OFF);
        if delegate != 0 {
            let name_cstr =
                std::ffi::CString::new("NSWindowDidEnterFullScreenNotification").unwrap();
            let name_str = crate::cfstring::alloc_nsstring_from_cstr(name_cstr.as_ptr());
            let note = crate::nsnotificationcenter::make_nsnotification(name_str, obj, 0);
            crate::nsapplication::dispatch_selector_named_to_target(
                delegate,
                "windowDidEnterFullScreen:",
                note,
            );
        }
    }
}

/// `-[NSWindow beginSheet:completionHandler:]`
///
/// Presents a sheet window attached to this window.  For now, the sheet
/// is made visible immediately and the completion handler is invoked
/// synchronously with `NSModalResponseStop` (1001) when the sheet's
/// `orderOut:` or `endSheet:` API would normally dismiss it.
///
/// Since we lack a real modal-sheet event loop, we invoke the handler
/// right away so the calling code can proceed.
pub extern "C" fn shim_nswindow_begin_sheet(obj: u64, _cmd: u64, sheet_window: u64, handler: u64) {
    if obj == 0 {
        return;
    }

    // Make the sheet visible (attached to this window conceptually).
    if sheet_window != 0 {
        set_visible(sheet_window, true);
        crate::nsapplication::promote_window(sheet_window);
    }

    // Invoke the completion handler immediately with NSModalResponseStop.
    const NS_MODAL_RESPONSE_STOP: i64 = 1001;
    if handler != 0 {
        let invoke_ptr = unsafe { *((handler as usize + 16) as *const u64) };
        if invoke_ptr != 0 {
            let invoke: extern "C" fn(u64, i64) = unsafe { std::mem::transmute(invoke_ptr) };
            invoke(handler, NS_MODAL_RESPONSE_STOP);
        }
    }
}

/// `-[NSWindow endSheet:]` — dismisses a sheet previously presented via `beginSheet:`.
pub extern "C" fn shim_nswindow_end_sheet(obj: u64, _cmd: u64, sheet_window: u64) {
    if obj == 0 || sheet_window == 0 {
        return;
    }
    set_visible(sheet_window, false);
    if let Some(bridge) = crate::gui_bridge::gui_bridge() {
        bridge.hide_window(sheet_window);
    }
}

/// `-[NSWindow endSheet:returnCode:]` — dismisses a sheet with a return code.
pub extern "C" fn shim_nswindow_end_sheet_return_code(
    obj: u64,
    _cmd: u64,
    sheet_window: u64,
    _return_code: i64,
) {
    shim_nswindow_end_sheet(obj, 0, sheet_window);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    fn read_f64(obj: u64, offset: usize) -> f64 {
        unsafe { *((obj as usize + offset) as *const f64) }
    }

    const SIZE: usize = WINDOW_SIZE;

    static WINDOW_TEST_LOCK: Mutex<()> = Mutex::new(());
    static EVENT_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_EVENT_TARGET: AtomicU64 = AtomicU64::new(0);
    static LAST_EVENT_ARG: AtomicU64 = AtomicU64::new(0);
    static CURRENT_EVENT_DURING_CALL: AtomicU64 = AtomicU64::new(0);
    static EVENT_WINDOW_DURING_CALL: AtomicU64 = AtomicU64::new(0);
    static RESIGN_RETURN: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);
    static BECOME_RETURN: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);
    static KEY_EQUIV_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
    static KEY_EQUIV_RETURN: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
    static SHOULD_CLOSE_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
    static SHOULD_CLOSE_RETURN: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

    fn make_coder() -> u64 {
        crate::nskeyedarchiver::shim_nskeyedarchiver_alloc(0, 0)
    }

    fn encode_object(coder: u64, key: &str, value: u64) {
        crate::nskeyedarchiver::shim_nskeyedarchiver_encode_object_for_key(
            coder,
            0,
            value,
            make_string(key),
        );
    }

    fn encode_integer(coder: u64, key: &str, value: i64) {
        crate::nskeyedarchiver::shim_nskeyedarchiver_encode_integer_for_key(
            coder,
            0,
            value,
            make_string(key),
        );
    }

    const PROBE_VIEW_CLASS_ADDR: u64 = 0x0000_0001_0050_3000;
    const DELEGATE_PROBE_CLASS_ADDR: u64 = 0x0000_0001_0050_6000;

    extern "C" fn probe_resign_first_responder(_obj: u64, _cmd: u64) -> u8 {
        RESIGN_RETURN.load(Ordering::SeqCst)
    }

    extern "C" fn probe_become_first_responder(_obj: u64, _cmd: u64) -> u8 {
        BECOME_RETURN.load(Ordering::SeqCst)
    }

    extern "C" fn probe_perform_key_equivalent(_obj: u64, _cmd: u64, _event: u64) -> u8 {
        KEY_EQUIV_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        KEY_EQUIV_RETURN.load(Ordering::SeqCst)
    }

    extern "C" fn probe_window_should_close(_obj: u64, _cmd: u64, _window: u64) -> u8 {
        SHOULD_CLOSE_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        SHOULD_CLOSE_RETURN.load(Ordering::SeqCst)
    }

    extern "C" fn probe_key_down_imp(obj: u64, _cmd: u64, event: u64) {
        EVENT_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        LAST_EVENT_TARGET.store(obj, Ordering::SeqCst);
        LAST_EVENT_ARG.store(event, Ordering::SeqCst);
        let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
        CURRENT_EVENT_DURING_CALL.store(
            crate::nsapplication::shim_nsapplication_current_event(app, 0),
            Ordering::SeqCst,
        );
        EVENT_WINDOW_DURING_CALL.store(
            crate::nsevent::shim_nsevent_window(event, 0),
            Ordering::SeqCst,
        );
    }

    fn reset_event_probe() {
        EVENT_CALL_COUNT.store(0, Ordering::SeqCst);
        LAST_EVENT_TARGET.store(0, Ordering::SeqCst);
        LAST_EVENT_ARG.store(0, Ordering::SeqCst);
        CURRENT_EVENT_DURING_CALL.store(0, Ordering::SeqCst);
        EVENT_WINDOW_DURING_CALL.store(0, Ordering::SeqCst);
        RESIGN_RETURN.store(1, Ordering::SeqCst);
        BECOME_RETURN.store(1, Ordering::SeqCst);
        KEY_EQUIV_CALL_COUNT.store(0, Ordering::SeqCst);
        KEY_EQUIV_RETURN.store(0, Ordering::SeqCst);
        SHOULD_CLOSE_CALL_COUNT.store(0, Ordering::SeqCst);
        SHOULD_CLOSE_RETURN.store(1, Ordering::SeqCst);
    }

    fn install_probe_view_class(rt: &mut objc_runtime::ObjcRuntime) {
        let mut info = objc_runtime::ClassInfo {
            name: "WindowEventProbeView".to_string(),
            mapped_addr: PROBE_VIEW_CLASS_ADDR,
            methods: vec![
                objc_runtime::MethodEntry {
                    selector: "keyDown:".into(),
                    imp: probe_key_down_imp as *const () as u64,
                    type_encoding: "v@:@".into(),
                },
                objc_runtime::MethodEntry {
                    selector: "mouseDown:".into(),
                    imp: probe_key_down_imp as *const () as u64,
                    type_encoding: "v@:@".into(),
                },
                objc_runtime::MethodEntry {
                    selector: "resignFirstResponder".into(),
                    imp: probe_resign_first_responder as *const () as u64,
                    type_encoding: "B@:".into(),
                },
                objc_runtime::MethodEntry {
                    selector: "becomeFirstResponder".into(),
                    imp: probe_become_first_responder as *const () as u64,
                    type_encoding: "B@:".into(),
                },
                objc_runtime::MethodEntry {
                    selector: "performKeyEquivalent:".into(),
                    imp: probe_perform_key_equivalent as *const () as u64,
                    type_encoding: "B@:@".into(),
                },
            ],
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 72,
            is_root: false,
            is_meta: false,
            superclass_addr: Some(crate::nsobj::nsview_class_addr()),
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        };
        info.rebuild_indices();
        rt.register_synthetic_class(info);
    }

    fn install_delegate_probe_class(rt: &mut objc_runtime::ObjcRuntime) {
        let mut info = objc_runtime::ClassInfo {
            name: "WindowDelegateProbe".to_string(),
            mapped_addr: DELEGATE_PROBE_CLASS_ADDR,
            methods: vec![objc_runtime::MethodEntry {
                selector: "windowShouldClose:".into(),
                imp: probe_window_should_close as *const () as u64,
                type_encoding: "B@:@".into(),
            }],
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 8,
            is_root: false,
            is_meta: false,
            superclass_addr: Some(crate::nsobj::nsobject_class_addr()),
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        };
        info.rebuild_indices();
        rt.register_synthetic_class(info);
    }

    fn with_window_runtime<R>(f: impl FnOnce() -> R) -> R {
        let saved_runtime = {
            let mut guard = objc_runtime::OBJC_RUNTIME
                .write()
                .unwrap_or_else(|e| e.into_inner());
            let saved = std::mem::take(&mut *guard);

            let mut rt = objc_runtime::ObjcRuntime::new();
            crate::FoundationShims::register_nsobject_class(&mut rt);
            crate::FoundationShims::register_nsresponder_class(&mut rt);
            crate::FoundationShims::register_nsapplication_class(&mut rt);
            crate::FoundationShims::register_nswindow_class(&mut rt);
            crate::FoundationShims::register_nsview_class(&mut rt);
            crate::FoundationShims::register_nsevent_class(&mut rt);
            install_probe_view_class(&mut rt);
            install_delegate_probe_class(&mut rt);

            *guard = Some(rt);
            saved
        };

        let result = f();

        let mut guard = objc_runtime::OBJC_RUNTIME
            .write()
            .unwrap_or_else(|e| e.into_inner());
        *guard = saved_runtime;
        result
    }

    fn reset_app_state() {
        crate::nsapplication::reset_runtime_state_for_reuse();
    }

    fn isa() -> u64 {
        crate::nsobj::nswindow_class_addr()
    }

    // ---- alloc_window (internal helper) ----

    #[test]
    fn alloc_window_basic() {
        let rect = CGRect {
            origin: CGPoint { x: 10.0, y: 20.0 },
            size: CGSize {
                width: 800.0,
                height: 600.0,
            },
        };
        let obj = alloc_window(rect, 0x0F);
        assert_ne!(obj, 0);
        // ISA should be the real NSWindow class address
        assert_eq!(read_field(obj, 0), isa());
        // style_mask
        assert_eq!(read_field(obj, STYLE_MASK_OFF), 0x0F);
        // frame fields
        let fx = unsafe { *((obj as usize + FRAME_X_OFF) as *const f64) };
        let fy = unsafe { *((obj as usize + FRAME_X_OFF + 8) as *const f64) };
        let fw = unsafe { *((obj as usize + FRAME_X_OFF + 16) as *const f64) };
        let fh = unsafe { *((obj as usize + FRAME_X_OFF + 24) as *const f64) };
        assert_eq!(fx, 10.0);
        assert_eq!(fy, 20.0);
        assert_eq!(fw, 800.0);
        assert_eq!(fh, 600.0);
        let content_view = shim_nswindow_content_view(obj, 0);
        assert_ne!(content_view, 0);
        assert_eq!(crate::nsview::shim_nsview_window(content_view, 0), obj);
        assert_eq!(shim_nswindow_is_visible(obj, 0), 0);
        assert_eq!(shim_nswindow_first_responder(obj, 0), obj);
        free_obj(obj);
    }

    // ---- shim_nswindow_init_with_content_rect ----

    #[test]
    fn init_with_content_rect_basic() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let obj = shim_nswindow_init_with_content_rect(0, 0, 50.0, 100.0, 640.0, 480.0, 0x0F, 2, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), isa());
        assert_eq!(read_field(obj, STYLE_MASK_OFF), 0x0F);
        let fx = unsafe { *((obj as usize + FRAME_X_OFF) as *const f64) };
        assert_eq!(fx, 50.0);
        assert_ne!(shim_nswindow_content_view(obj, 0), 0);
        free_obj(obj);
        reset_app_state();
    }

    #[test]
    fn init_with_content_rect_zero_size() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let obj = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 0.0, 0.0, 0, 0, 0);
        assert_ne!(obj, 0);
        let fw = unsafe { *((obj as usize + FRAME_X_OFF + 16) as *const f64) };
        assert_eq!(fw, 0.0);
        free_obj(obj);
        reset_app_state();
    }

    #[test]
    fn init_with_coder_decodes_frame_title_style_and_content_view() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let win = alloc_obj(SIZE, crate::nsobj::nswindow_class_addr());
        let content = crate::nsview::alloc_view_with_frame(0.0, 0.0, 320.0, 200.0);
        let title = make_string("Archived Window");
        let coder = make_coder();

        encode_object(
            coder,
            "NSWindowFrame",
            make_string("{{44, 55}, {640, 480}}"),
        );
        encode_integer(coder, "NSWindowStyleMask", 15);
        encode_object(coder, "NSTitle", title);
        encode_object(coder, "NSContentView", content);

        assert_eq!(shim_nswindow_init_with_coder(win, 0, coder), win);
        assert_eq!(read_f64(win, FRAME_X_OFF), 44.0);
        assert_eq!(read_f64(win, FRAME_X_OFF + 8), 55.0);
        assert_eq!(read_f64(win, FRAME_X_OFF + 16), 640.0);
        assert_eq!(read_f64(win, FRAME_X_OFF + 24), 480.0);
        assert_eq!(shim_nswindow_style_mask(win, 0), 15);
        assert_eq!(shim_nswindow_title(win, 0), title);
        assert_eq!(shim_nswindow_content_view(win, 0), content);
        assert_eq!(crate::nsview::shim_nsview_window(content, 0), win);

        free_obj(content);
        free_obj(win);
        reset_app_state();
    }

    // ---- shim_nswindow_content_view / set_content_view ----

    #[test]
    fn content_view_nil() {
        assert_eq!(shim_nswindow_content_view(0, 0), 0);
    }

    #[test]
    fn set_content_view_nil() {
        // Must not crash
        shim_nswindow_set_content_view(0, 0, 0x1234);
    }

    #[test]
    fn content_view_roundtrip() {
        let obj = alloc_obj(SIZE, isa());
        assert_eq!(shim_nswindow_content_view(obj, 0), 0);
        let view = crate::nsview::shim_nsview_init_with_frame(0, 0, 0.0, 0.0, 10.0, 20.0);
        shim_nswindow_set_content_view(obj, 0, view);
        assert_eq!(shim_nswindow_content_view(obj, 0), view);
        free_obj(view);
        free_obj(obj);
    }

    #[test]
    fn set_content_view_updates_view_window() {
        let obj = alloc_obj(SIZE, isa());
        let view = crate::nsview::shim_nsview_init_with_frame(0, 0, 0.0, 0.0, 10.0, 20.0);
        shim_nswindow_set_content_view(obj, 0, view);
        assert_eq!(shim_nswindow_content_view(obj, 0), view);
        assert_eq!(crate::nsview::shim_nsview_window(view, 0), obj);
        assert_eq!(crate::nsview::shim_nsview_superview(view, 0), 0);
        free_obj(view);
        free_obj(obj);
    }

    #[test]
    fn set_content_view_detaches_view_from_previous_parent() {
        let obj = alloc_obj(SIZE, isa());
        let parent = crate::nsview::shim_nsview_init_with_frame(0, 0, 0.0, 0.0, 50.0, 50.0);
        let view = crate::nsview::shim_nsview_init_with_frame(0, 0, 0.0, 0.0, 10.0, 20.0);

        crate::nsview::shim_nsview_add_subview(parent, 0, view);
        shim_nswindow_set_content_view(obj, 0, view);

        let siblings = crate::nsview::shim_nsview_subviews(parent, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(siblings, 0), 0);
        assert_eq!(crate::nsview::shim_nsview_superview(view, 0), 0);
        assert_eq!(crate::nsview::shim_nsview_window(view, 0), obj);

        free_obj(view);
        free_obj(parent);
        free_obj(obj);
    }

    #[test]
    fn set_content_view_propagates_window_to_descendants() {
        let obj = alloc_obj(SIZE, isa());
        let root = crate::nsview::shim_nsview_init_with_frame(0, 0, 0.0, 0.0, 50.0, 50.0);
        let child = crate::nsview::shim_nsview_init_with_frame(0, 0, 1.0, 2.0, 3.0, 4.0);

        crate::nsview::shim_nsview_add_subview(root, 0, child);
        shim_nswindow_set_content_view(obj, 0, root);

        assert_eq!(crate::nsview::shim_nsview_window(root, 0), obj);
        assert_eq!(crate::nsview::shim_nsview_window(child, 0), obj);

        free_obj(child);
        free_obj(root);
        free_obj(obj);
    }

    #[test]
    fn set_content_view_rehomes_stale_view_first_responder_to_window() {
        let obj = alloc_obj(SIZE, isa());
        let old_root = crate::nsview::shim_nsview_init_with_frame(0, 0, 0.0, 0.0, 50.0, 50.0);
        let new_root = crate::nsview::shim_nsview_init_with_frame(0, 0, 5.0, 5.0, 40.0, 30.0);

        shim_nswindow_set_content_view(obj, 0, old_root);
        assert_eq!(shim_nswindow_make_first_responder(obj, 0, old_root), 1);
        assert_eq!(shim_nswindow_first_responder(obj, 0), old_root);

        shim_nswindow_set_content_view(obj, 0, new_root);
        assert_eq!(shim_nswindow_first_responder(obj, 0), obj);

        free_obj(new_root);
        free_obj(old_root);
        free_obj(obj);
    }

    // ---- shim_nswindow_first_responder / make_first_responder ----

    #[test]
    fn first_responder_nil_returns_zero() {
        assert_eq!(shim_nswindow_first_responder(0, 0), 0);
    }

    #[test]
    fn first_responder_defaults_to_window() {
        let obj = alloc_obj(SIZE, isa());
        assert_eq!(shim_nswindow_first_responder(obj, 0), obj);
        free_obj(obj);
    }

    #[test]
    fn make_first_responder_nil_window_returns_no() {
        assert_eq!(shim_nswindow_make_first_responder(0, 0, 0), 0);
    }

    #[test]
    fn make_first_responder_nil_restores_window_focus() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let obj = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
        let content = shim_nswindow_content_view(obj, 0);
        let subview = crate::nsview::shim_nsview_init_with_frame(0, 0, 1.0, 2.0, 3.0, 4.0);

        crate::nsview::shim_nsview_add_subview(content, 0, subview);
        assert_eq!(shim_nswindow_make_first_responder(obj, 0, subview), 1);
        assert_eq!(shim_nswindow_first_responder(obj, 0), subview);

        assert_eq!(shim_nswindow_make_first_responder(obj, 0, 0), 1);
        assert_eq!(shim_nswindow_first_responder(obj, 0), obj);

        free_obj(subview);
        free_obj(obj);
        reset_app_state();
    }

    #[test]
    fn make_first_responder_accepts_attached_subview() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let obj = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
        let content = shim_nswindow_content_view(obj, 0);
        let subview = crate::nsview::shim_nsview_init_with_frame(0, 0, 1.0, 2.0, 3.0, 4.0);

        crate::nsview::shim_nsview_add_subview(content, 0, subview);

        assert_eq!(crate::nsview::shim_nsview_window(subview, 0), obj);
        assert_eq!(shim_nswindow_make_first_responder(obj, 0, subview), 1);
        assert_eq!(shim_nswindow_first_responder(obj, 0), subview);

        free_obj(subview);
        free_obj(obj);
        reset_app_state();
    }

    #[test]
    fn make_first_responder_detached_view_falls_back_to_window() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let obj = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
        let detached = crate::nsview::shim_nsview_init_with_frame(0, 0, 1.0, 2.0, 3.0, 4.0);

        assert_eq!(crate::nsview::shim_nsview_window(detached, 0), 0);
        assert_eq!(shim_nswindow_make_first_responder(obj, 0, detached), 1);
        assert_eq!(shim_nswindow_first_responder(obj, 0), obj);

        free_obj(detached);
        free_obj(obj);
        reset_app_state();
    }

    #[test]
    fn make_first_responder_accepts_generic_responder() {
        let obj = alloc_obj(SIZE, isa());
        let responder = alloc_obj(16, crate::nsobj::nsresponder_class_addr());

        assert_eq!(shim_nswindow_make_first_responder(obj, 0, responder), 1);
        assert_eq!(shim_nswindow_first_responder(obj, 0), responder);

        free_obj(responder);
        free_obj(obj);
    }

    #[test]
    fn send_event_dispatches_key_event_to_first_responder() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        reset_event_probe();

        with_window_runtime(|| {
            let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
            let window = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
            let content = shim_nswindow_content_view(window, 0);
            let probe = alloc_obj(72, PROBE_VIEW_CLASS_ADDR);
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            crate::nsview::shim_nsview_add_subview(content, 0, probe);
            assert_eq!(shim_nswindow_make_first_responder(window, 0, probe), 1);
            write_field(event, 8, 10);

            shim_nswindow_send_event(window, 0, event);

            assert_eq!(EVENT_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_EVENT_TARGET.load(Ordering::SeqCst), probe);
            assert_eq!(LAST_EVENT_ARG.load(Ordering::SeqCst), event);
            assert_eq!(CURRENT_EVENT_DURING_CALL.load(Ordering::SeqCst), event);
            assert_eq!(EVENT_WINDOW_DURING_CALL.load(Ordering::SeqCst), window);
            assert_eq!(
                crate::nsapplication::shim_nsapplication_current_event(app, 0),
                event
            );

            free_obj(event);
            free_obj(probe);
            free_obj(window);
        });

        reset_app_state();
    }

    #[test]
    fn send_event_dispatches_mouse_event_to_content_view() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        reset_event_probe();

        with_window_runtime(|| {
            let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
            let window = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
            let probe = alloc_obj(72, PROBE_VIEW_CLASS_ADDR);
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            shim_nswindow_set_content_view(window, 0, probe);
            write_field(event, 8, 1);

            shim_nswindow_send_event(window, 0, event);

            assert_eq!(EVENT_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_EVENT_TARGET.load(Ordering::SeqCst), probe);
            assert_eq!(LAST_EVENT_ARG.load(Ordering::SeqCst), event);
            assert_eq!(CURRENT_EVENT_DURING_CALL.load(Ordering::SeqCst), event);
            assert_eq!(EVENT_WINDOW_DURING_CALL.load(Ordering::SeqCst), window);
            assert_eq!(
                crate::nsapplication::shim_nsapplication_current_event(app, 0),
                event
            );

            free_obj(event);
            free_obj(probe);
            free_obj(window);
        });

        reset_app_state();
    }

    // ---- delegate / setDelegate ----

    #[test]
    fn delegate_nil_returns_zero() {
        assert_eq!(shim_nswindow_delegate(0, 0), 0);
    }

    #[test]
    fn delegate_default_is_zero() {
        let obj = alloc_obj(SIZE, isa());
        assert_eq!(shim_nswindow_delegate(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn delegate_roundtrip() {
        let obj = alloc_obj(SIZE, isa());
        shim_nswindow_set_delegate(obj, 0, 0xCAFE);
        assert_eq!(shim_nswindow_delegate(obj, 0), 0xCAFE);
        free_obj(obj);
    }

    // ---- makeFirstResponder resign/become protocol ----

    #[test]
    fn make_first_responder_resign_rejected_keeps_old() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        reset_event_probe();

        with_window_runtime(|| {
            let window = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
            let content = shim_nswindow_content_view(window, 0);
            let old_target = alloc_obj(72, PROBE_VIEW_CLASS_ADDR);
            let new_target = alloc_obj(72, PROBE_VIEW_CLASS_ADDR);

            crate::nsview::shim_nsview_add_subview(content, 0, old_target);
            crate::nsview::shim_nsview_add_subview(content, 0, new_target);
            assert_eq!(shim_nswindow_make_first_responder(window, 0, old_target), 1);

            RESIGN_RETURN.store(0, Ordering::SeqCst);
            assert_eq!(shim_nswindow_make_first_responder(window, 0, new_target), 0);
            assert_eq!(shim_nswindow_first_responder(window, 0), old_target);

            free_obj(new_target);
            free_obj(old_target);
            free_obj(window);
        });

        reset_app_state();
    }

    #[test]
    fn make_first_responder_become_rejected_resets_to_window() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        reset_event_probe();

        with_window_runtime(|| {
            let window = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
            let content = shim_nswindow_content_view(window, 0);
            let target = alloc_obj(72, PROBE_VIEW_CLASS_ADDR);

            crate::nsview::shim_nsview_add_subview(content, 0, target);

            BECOME_RETURN.store(0, Ordering::SeqCst);
            assert_eq!(shim_nswindow_make_first_responder(window, 0, target), 0);
            assert_eq!(shim_nswindow_first_responder(window, 0), window);

            free_obj(target);
            free_obj(window);
        });

        reset_app_state();
    }

    // ---- performKeyEquivalent ----

    #[test]
    fn perform_key_equivalent_nil_returns_no() {
        assert_eq!(shim_nswindow_perform_key_equivalent(0, 0, 0), 0);
    }

    #[test]
    fn perform_key_equivalent_dispatches_to_content_view() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        reset_event_probe();

        with_window_runtime(|| {
            let window = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
            let probe = alloc_obj(72, PROBE_VIEW_CLASS_ADDR);
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            shim_nswindow_set_content_view(window, 0, probe);
            KEY_EQUIV_RETURN.store(1, Ordering::SeqCst);

            assert_eq!(shim_nswindow_perform_key_equivalent(window, 0, event), 1);
            assert_eq!(KEY_EQUIV_CALL_COUNT.load(Ordering::SeqCst), 1);

            free_obj(event);
            free_obj(probe);
            free_obj(window);
        });

        reset_app_state();
    }

    #[test]
    fn perform_key_equivalent_returns_no_when_unhandled() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        reset_event_probe();

        with_window_runtime(|| {
            let window = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            // Default content view is an NSView whose performKeyEquivalent: returns NO.
            assert_eq!(shim_nswindow_perform_key_equivalent(window, 0, event), 0);

            free_obj(event);
            free_obj(window);
        });

        reset_app_state();
    }

    // ---- performClose ----

    #[test]
    fn perform_close_nil_does_not_crash() {
        shim_nswindow_perform_close(0, 0, 0);
    }

    #[test]
    fn perform_close_closes_window() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();

        let window = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
        shim_nswindow_make_key_and_order_front(window, 0, 0);
        assert_eq!(shim_nswindow_is_visible(window, 0), 1);

        shim_nswindow_perform_close(window, 0, 0);
        assert_eq!(shim_nswindow_is_visible(window, 0), 0);

        free_obj(window);
        reset_app_state();
    }

    #[test]
    fn perform_close_respects_delegate_no() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        reset_event_probe();

        with_window_runtime(|| {
            let window = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
            let delegate = alloc_obj(8, DELEGATE_PROBE_CLASS_ADDR);

            shim_nswindow_make_key_and_order_front(window, 0, 0);
            shim_nswindow_set_delegate(window, 0, delegate);
            SHOULD_CLOSE_RETURN.store(0, Ordering::SeqCst);

            shim_nswindow_perform_close(window, 0, 0);
            assert_eq!(shim_nswindow_is_visible(window, 0), 1);
            assert_eq!(SHOULD_CLOSE_CALL_COUNT.load(Ordering::SeqCst), 1);

            free_obj(delegate);
            free_obj(window);
        });

        reset_app_state();
    }

    // ---- shim_nswindow_title / set_title ----

    #[test]
    fn title_nil() {
        assert_eq!(shim_nswindow_title(0, 0), 0);
    }

    #[test]
    fn set_title_nil() {
        shim_nswindow_set_title(0, 0, 0xABC);
    }

    #[test]
    fn title_roundtrip() {
        let obj = alloc_obj(SIZE, isa());
        assert_eq!(shim_nswindow_title(obj, 0), 0);
        let title = make_string("Hello Window");
        shim_nswindow_set_title(obj, 0, title);
        let got = shim_nswindow_title(obj, 0);
        assert_eq!(got, title);
        assert_string_eq(got, "Hello Window");
        free_obj(title);
        free_obj(obj);
    }

    // ---- shim_nswindow_style_mask ----

    #[test]
    fn style_mask_nil() {
        assert_eq!(shim_nswindow_style_mask(0, 0), 0);
    }

    #[test]
    fn style_mask_basic() {
        let obj = alloc_obj(SIZE, isa());
        write_field(obj, STYLE_MASK_OFF, 0x0F);
        assert_eq!(shim_nswindow_style_mask(obj, 0), 0x0F);
        free_obj(obj);
    }

    // ---- shim_nswindow_frame ----

    #[test]
    fn frame_nil() {
        assert_eq!(shim_nswindow_frame(0, 0), CGRect::ZERO);
    }

    #[test]
    fn frame_basic() {
        let obj = alloc_obj(SIZE, isa());
        unsafe {
            *((obj as usize + FRAME_X_OFF) as *mut f64) = 123.5;
        }
        assert_eq!(shim_nswindow_frame(obj, 0).origin.x, 123.5);
        free_obj(obj);
    }

    // ---- shim_nswindow_make_key_and_order_front ----

    #[test]
    fn make_key_and_order_front_does_not_crash() {
        shim_nswindow_make_key_and_order_front(0, 0, 0);
    }

    #[test]
    fn make_key_and_order_front_with_obj() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let obj = alloc_obj(SIZE, isa());
        shim_nswindow_make_key_and_order_front(obj, 0, 0);
        assert_eq!(shim_nswindow_is_visible(obj, 0), 1);
        assert_eq!(crate::nsapplication::current_key_window(), obj);
        assert_eq!(crate::nsapplication::current_main_window(), obj);
        reset_app_state();
        free_obj(obj);
    }

    #[test]
    fn make_key_and_order_front_promotes_window_to_front() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let first = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
        let second = shim_nswindow_init_with_content_rect(0, 0, 5.0, 5.0, 120.0, 90.0, 1, 2, 0);

        shim_nswindow_order_front(first, 0, 0);
        shim_nswindow_make_key_and_order_front(second, 0, 0);

        let ordered = crate::nsapplication::shim_nsapplication_ordered_windows(
            crate::nsapplication::shim_nsapplication_shared_application(0, 0),
            0,
        );
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(ordered, 0, 0),
            second
        );
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(ordered, 0, 1),
            first
        );

        free_obj(second);
        free_obj(first);
        reset_app_state();
    }

    #[test]
    fn make_key_window_sets_key_state() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let obj = alloc_obj(SIZE, isa());
        shim_nswindow_make_key_window(obj, 0);
        assert_eq!(shim_nswindow_is_key_window(obj, 0), 1);
        assert_eq!(shim_nswindow_is_main_window(obj, 0), 0);
        reset_app_state();
        free_obj(obj);
    }

    #[test]
    fn make_main_window_sets_main_state() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let obj = alloc_obj(SIZE, isa());
        shim_nswindow_make_main_window(obj, 0);
        assert_eq!(shim_nswindow_is_main_window(obj, 0), 1);
        assert_eq!(shim_nswindow_is_key_window(obj, 0), 0);
        reset_app_state();
        free_obj(obj);
    }

    // ---- shim_nswindow_close ----

    #[test]
    fn close_does_not_crash() {
        shim_nswindow_close(0, 0);
    }

    #[test]
    fn close_with_obj() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let obj = alloc_obj(SIZE, isa());
        write_field(obj, IS_VISIBLE_OFF, 1);
        crate::nsapplication::set_key_window(obj);
        crate::nsapplication::set_main_window(obj);
        shim_nswindow_close(obj, 0);
        assert_eq!(shim_nswindow_is_visible(obj, 0), 0);
        assert_eq!(shim_nswindow_is_key_window(obj, 0), 0);
        assert_eq!(shim_nswindow_is_main_window(obj, 0), 0);
        reset_app_state();
        free_obj(obj);
    }

    #[test]
    fn order_out_clears_visibility_and_focus() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let obj = alloc_obj(SIZE, isa());
        shim_nswindow_make_key_and_order_front(obj, 0, 0);
        shim_nswindow_order_out(obj, 0, 0);
        assert_eq!(shim_nswindow_is_visible(obj, 0), 0);
        assert_eq!(shim_nswindow_is_key_window(obj, 0), 0);
        assert_eq!(shim_nswindow_is_main_window(obj, 0), 0);
        reset_app_state();
        free_obj(obj);
    }

    // ---- shim_nswindow_order_front ----

    #[test]
    fn order_front_does_not_crash() {
        shim_nswindow_order_front(0, 0, 0);
    }

    #[test]
    fn order_front_with_obj() {
        let obj = alloc_obj(SIZE, isa());
        shim_nswindow_order_front(obj, 0, 0);
        assert_eq!(shim_nswindow_is_visible(obj, 0), 1);
        free_obj(obj);
    }

    #[test]
    fn order_front_promotes_existing_window() {
        let _guard = WINDOW_TEST_LOCK.lock().unwrap();
        reset_app_state();
        let first = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
        let second = shim_nswindow_init_with_content_rect(0, 0, 5.0, 5.0, 120.0, 90.0, 1, 2, 0);

        shim_nswindow_order_front(first, 0, 0);
        shim_nswindow_order_front(second, 0, 0);
        shim_nswindow_order_front(first, 0, 0);

        let ordered = crate::nsapplication::shim_nsapplication_ordered_windows(
            crate::nsapplication::shim_nsapplication_shared_application(0, 0),
            0,
        );
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(ordered, 0, 0),
            first
        );
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(ordered, 0, 1),
            second
        );

        free_obj(second);
        free_obj(first);
        reset_app_state();
    }

    // ---- shim_nswindow_is_visible ----

    #[test]
    fn is_visible_nil() {
        assert_eq!(shim_nswindow_is_visible(0, 0), 0);
    }

    #[test]
    fn is_visible_reflects_field() {
        let obj = alloc_obj(SIZE, isa());
        assert_eq!(shim_nswindow_is_visible(obj, 0), 0);
        write_field(obj, IS_VISIBLE_OFF, 1);
        assert_eq!(shim_nswindow_is_visible(obj, 0), 1);
        free_obj(obj);
    }

    #[test]
    fn is_key_and_main_window_nil_are_no() {
        assert_eq!(shim_nswindow_is_key_window(0, 0), 0);
        assert_eq!(shim_nswindow_is_main_window(0, 0), 0);
    }

    // ---- shim_nswindow_set_frame_display ----

    #[test]
    fn set_frame_display_nil_does_not_crash() {
        shim_nswindow_set_frame_display(0, 0, 10.0, 20.0, 300.0, 200.0, 1);
    }

    #[test]
    fn set_frame_display_updates_frame() {
        let obj = alloc_obj(SIZE, isa());
        shim_nswindow_set_frame_display(obj, 0, 50.0, 60.0, 400.0, 300.0, 1);
        let x = read_f64(obj, FRAME_X_OFF);
        let y = read_f64(obj, FRAME_X_OFF + 8);
        let w = read_f64(obj, FRAME_X_OFF + 16);
        let h = read_f64(obj, FRAME_X_OFF + 24);
        assert_eq!(x, 50.0);
        assert_eq!(y, 60.0);
        assert_eq!(w, 400.0);
        assert_eq!(h, 300.0);
        free_obj(obj);
    }

    // ---- shim_nswindow_center ----

    #[test]
    fn center_nil_does_not_crash() {
        shim_nswindow_center(0, 0);
    }

    #[test]
    fn center_positions_window_on_screen() {
        let win = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 200.0, 100.0, 1, 2, 0);
        shim_nswindow_center(win, 0);
        let x = read_f64(win, FRAME_X_OFF);
        let y = read_f64(win, FRAME_X_OFF + 8);
        assert_eq!(x, (1920.0 - 200.0) / 2.0);
        assert_eq!(y, (1080.0 - 100.0) / 2.0);
        free_obj(win);
    }

    // ---- shim_nswindow_screen ----

    #[test]
    fn screen_returns_main_screen() {
        let win = shim_nswindow_init_with_content_rect(0, 0, 0.0, 0.0, 100.0, 80.0, 1, 2, 0);
        let screen = shim_nswindow_screen(win, 0);
        assert_ne!(screen, 0);
        assert_eq!(screen, crate::nsscreen::shim_nsscreen_main_screen(0, 0));
        free_obj(win);
    }

    // ---- shim_nswindow_background_color / set_background_color ----

    #[test]
    fn background_color_nil_returns_zero() {
        assert_eq!(shim_nswindow_background_color(0, 0), 0);
    }

    #[test]
    fn background_color_default_is_zero() {
        let obj = alloc_obj(SIZE, isa());
        assert_eq!(shim_nswindow_background_color(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn set_background_color_roundtrip() {
        let obj = alloc_obj(SIZE, isa());
        let color: u64 = 0xCAFE;
        shim_nswindow_set_background_color(obj, 0, color);
        assert_eq!(shim_nswindow_background_color(obj, 0), color);
        free_obj(obj);
    }

    #[test]
    fn set_background_color_nil_does_not_crash() {
        shim_nswindow_set_background_color(0, 0, 0xBEEF);
    }

    // ── contentLayoutGuide ─────────────────────────────────────

    #[test]
    fn content_layout_guide_returns_guide_for_content_view() {
        let _lock = WINDOW_TEST_LOCK.lock();
        let rect = CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize {
                width: 400.0,
                height: 300.0,
            },
        };
        let window = alloc_window(rect, 0);
        let content_view = shim_nswindow_content_view(window, 0);
        assert_ne!(content_view, 0);

        let guide = shim_nswindow_content_layout_guide(window, 0);
        assert_ne!(guide, 0);

        // Guide's owning view should be the content view.
        let owning = crate::nslayoutguide::shim_nslayoutguide_owning_view(guide, 0);
        assert_eq!(owning, content_view);

        free_obj(guide);
    }

    #[test]
    fn content_layout_guide_nil_safety() {
        assert_eq!(shim_nswindow_content_layout_guide(0, 0), 0);
    }
}
