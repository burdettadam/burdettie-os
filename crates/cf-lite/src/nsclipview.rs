//! NSClipView — content clipping view used by NSScrollView.
//!
//! Object layout: 104 bytes
//!   [0..80):   NSView fields
//!   offset 80: document_view     (u64 — the clipped child view)
//!   offset 88: bounds_origin_x   (f64 — scroll x offset)
//!   offset 96: bounds_origin_y   (f64 — scroll y offset)

use crate::nsobj;

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

pub(crate) const CLIP_VIEW_SIZE: usize = 104;
const DOC_VIEW_OFF: usize = 80;
const BOUNDS_X_OFF: usize = 88;
const BOUNDS_Y_OFF: usize = 96;

/// `+[NSClipView alloc]` + `-[NSClipView init]`
pub extern "C" fn shim_nsclipview_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        let ptr = crate::pal_alloc().calloc(1, CLIP_VIEW_SIZE) as u64;
        if ptr == 0 {
            return 0;
        }
        write_u64(ptr, 0, nsobj::nsclipview_class_addr());
        return ptr;
    }
    obj
}

/// `-[NSClipView documentView]`
pub extern "C" fn shim_nsclipview_document_view(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DOC_VIEW_OFF)
}

/// `-[NSClipView setDocumentView:]`
pub extern "C" fn shim_nsclipview_set_document_view(obj: u64, _cmd: u64, view: u64) {
    if obj == 0 {
        return;
    }
    write_u64(obj, DOC_VIEW_OFF, view);
    // Also add as subview for display pipeline.
    if view != 0 {
        crate::nsview::shim_nsview_add_subview(obj, 0, view);
    }
}

/// `-[NSClipView boundsOrigin]` — returns (x, y) as CGPoint via x0/x1.
pub extern "C" fn shim_nsclipview_bounds_origin_x(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, BOUNDS_X_OFF)
}

pub extern "C" fn shim_nsclipview_bounds_origin_y(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, BOUNDS_Y_OFF)
}

/// `-[NSClipView setBoundsOrigin:]` — scrolls the clip view.
pub extern "C" fn shim_nsclipview_set_bounds_origin(obj: u64, _cmd: u64, x: f64, y: f64) {
    if obj == 0 {
        return;
    }
    write_f64(obj, BOUNDS_X_OFF, x);
    write_f64(obj, BOUNDS_Y_OFF, y);
    // Offset the document view's draw origin.
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSClipView scrollToPoint:]`
pub extern "C" fn shim_nsclipview_scroll_to_point(obj: u64, _cmd: u64, x: f64, y: f64) {
    shim_nsclipview_set_bounds_origin(obj, _cmd, x, y);
}

/// `-[NSClipView constrainBoundsRect:]` — returns proposed rect (no clipping).
pub extern "C" fn shim_nsclipview_constrain_bounds_rect(
    _obj: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> (f64, f64, f64, f64) {
    (x, y, w, h)
}

/// `-[NSClipView drawRect:]` — applies the bounds offset as a CTM translate,
/// then draws the document view.
pub extern "C" fn shim_nsclipview_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 {
        return;
    }

    let off_x = read_f64(obj, BOUNDS_X_OFF);
    let off_y = read_f64(obj, BOUNDS_Y_OFF);

    // Clip to the clip view's bounds and translate.
    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.save_state();
        c.clip_to_rect(x, y, w, h);
        c.translate_ctm(-off_x, -off_y);
    });

    // Draw the document view at its natural frame.
    let doc_view = read_u64(obj, DOC_VIEW_OFF);
    if doc_view != 0 {
        let cg_ctx_inner = crate::nsgraphicscontext::current_cg_context_handle();
        crate::nsview::display_view_recursive(doc_view, cg_ctx_inner);
    }

    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.restore_state();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn init_sets_isa() {
        let cv = shim_nsclipview_init(0, 0);
        assert_ne!(cv, 0);
        assert_eq!(read_u64(cv, 0), nsobj::nsclipview_class_addr());
        free_obj(cv);
    }

    #[test]
    fn document_view_round_trip() {
        let cv = shim_nsclipview_init(0, 0);
        assert_eq!(shim_nsclipview_document_view(cv, 0), 0);
        let fake_view = 0x1234_u64;
        write_u64(cv, DOC_VIEW_OFF, fake_view);
        assert_eq!(shim_nsclipview_document_view(cv, 0), fake_view);
        free_obj(cv);
    }

    #[test]
    fn scroll_to_point() {
        let cv = shim_nsclipview_init(0, 0);
        shim_nsclipview_scroll_to_point(cv, 0, 100.0, 200.0);
        assert_eq!(read_f64(cv, BOUNDS_X_OFF), 100.0);
        assert_eq!(read_f64(cv, BOUNDS_Y_OFF), 200.0);
        free_obj(cv);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsclipview_document_view(0, 0), 0);
        shim_nsclipview_set_document_view(0, 0, 0);
        shim_nsclipview_scroll_to_point(0, 0, 0.0, 0.0);
    }
}
