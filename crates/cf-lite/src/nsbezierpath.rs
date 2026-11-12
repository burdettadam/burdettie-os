//! NSBezierPath shim: Cocoa vector path for custom drawing.
//!
//! Object layout:
//!
//!   offset  0: isa            (u64)
//!   offset  8: path_handle    (u64 — cg-shims bridge path handle, or 0)
//!   offset 16: line_width     (f64 — default 1.0)
//!
//! Total: 24 bytes.
//!
//! Internally delegates to the `cg_shims::bridge` path API so drawing
//! operations produce real pixels via `graphics-core`.

const PATH_HANDLE_OFF: usize = 8;
const LINE_WIDTH_OFF: usize = 16;
const BEZIER_PATH_SIZE: usize = 24;

fn alloc_bezier_path() -> u64 {
    let handle = cg_shims::bridge::path_create();
    let obj = crate::pal_alloc().calloc(1, BEZIER_PATH_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsbezierpath_class_addr();
        *((obj as usize + PATH_HANDLE_OFF) as *mut u64) = handle;
        *((obj as usize + LINE_WIDTH_OFF) as *mut f64) = 1.0;
    }
    obj
}

fn path_handle(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + PATH_HANDLE_OFF) as *const u64) }
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSBezierPath bezierPath]`
pub extern "C" fn shim_nsbezierpath_bezier_path(_cls: u64, _cmd: u64) -> u64 {
    alloc_bezier_path()
}

/// `+[NSBezierPath bezierPathWithRect:]`
pub extern "C" fn shim_nsbezierpath_with_rect(
    _cls: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> u64 {
    let obj = alloc_bezier_path();
    if obj == 0 {
        return 0;
    }
    let h_path = path_handle(obj);
    cg_shims::bridge::with_path(h_path, |p| {
        p.move_to(graphics_core::Point::new(x, y));
        p.line_to(graphics_core::Point::new(x + w, y));
        p.line_to(graphics_core::Point::new(x + w, y + h));
        p.line_to(graphics_core::Point::new(x, y + h));
        p.close();
    });
    obj
}

/// `+[NSBezierPath bezierPathWithOvalInRect:]`
pub extern "C" fn shim_nsbezierpath_with_oval(
    _cls: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> u64 {
    // Approximate oval with 4 cubic Bézier curves (standard technique).
    let obj = alloc_bezier_path();
    if obj == 0 {
        return 0;
    }
    let h_path = path_handle(obj);
    let kappa = 0.552_284_75; // 4*(sqrt(2)-1)/3
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let rx = w / 2.0;
    let ry = h / 2.0;
    let ox = rx * kappa;
    let oy = ry * kappa;
    cg_shims::bridge::with_path(h_path, |p| {
        p.move_to(graphics_core::Point::new(cx + rx, cy));
        p.cubic_to(
            graphics_core::Point::new(cx + rx, cy + oy),
            graphics_core::Point::new(cx + ox, cy + ry),
            graphics_core::Point::new(cx, cy + ry),
        );
        p.cubic_to(
            graphics_core::Point::new(cx - ox, cy + ry),
            graphics_core::Point::new(cx - rx, cy + oy),
            graphics_core::Point::new(cx - rx, cy),
        );
        p.cubic_to(
            graphics_core::Point::new(cx - rx, cy - oy),
            graphics_core::Point::new(cx - ox, cy - ry),
            graphics_core::Point::new(cx, cy - ry),
        );
        p.cubic_to(
            graphics_core::Point::new(cx + ox, cy - ry),
            graphics_core::Point::new(cx + rx, cy - oy),
            graphics_core::Point::new(cx + rx, cy),
        );
        p.close();
    });
    obj
}

/// `+[NSBezierPath bezierPathWithRoundedRect:xRadius:yRadius:]`
pub extern "C" fn shim_nsbezierpath_with_rounded_rect(
    _cls: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    xr: f64,
    yr: f64,
) -> u64 {
    let obj = alloc_bezier_path();
    if obj == 0 {
        return 0;
    }
    let h_path = path_handle(obj);
    let kappa = 0.552_284_75_f64;
    let ox = xr * kappa;
    let oy = yr * kappa;
    cg_shims::bridge::with_path(h_path, |p| {
        // Start at top-left corner, just past the arc
        p.move_to(graphics_core::Point::new(x + xr, y));
        // Top edge → top-right corner
        p.line_to(graphics_core::Point::new(x + w - xr, y));
        p.cubic_to(
            graphics_core::Point::new(x + w - xr + ox, y),
            graphics_core::Point::new(x + w, y + yr - oy),
            graphics_core::Point::new(x + w, y + yr),
        );
        // Right edge → bottom-right corner
        p.line_to(graphics_core::Point::new(x + w, y + h - yr));
        p.cubic_to(
            graphics_core::Point::new(x + w, y + h - yr + oy),
            graphics_core::Point::new(x + w - xr + ox, y + h),
            graphics_core::Point::new(x + w - xr, y + h),
        );
        // Bottom edge → bottom-left corner
        p.line_to(graphics_core::Point::new(x + xr, y + h));
        p.cubic_to(
            graphics_core::Point::new(x + xr - ox, y + h),
            graphics_core::Point::new(x, y + h - yr + oy),
            graphics_core::Point::new(x, y + h - yr),
        );
        // Left edge → top-left corner
        p.line_to(graphics_core::Point::new(x, y + yr));
        p.cubic_to(
            graphics_core::Point::new(x, y + yr - oy),
            graphics_core::Point::new(x + xr - ox, y),
            graphics_core::Point::new(x + xr, y),
        );
        p.close();
    });
    obj
}

/// `+[NSBezierPath fillRect:]`
pub extern "C" fn shim_nsbezierpath_fill_rect(
    _cls: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx != 0 {
        cg_shims::bridge::with_context(cg_ctx, |c| c.fill_rect(x, y, w, h));
    }
}

/// `+[NSBezierPath strokeRect:]`
pub extern "C" fn shim_nsbezierpath_stroke_rect(
    _cls: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx != 0 {
        cg_shims::bridge::with_context(cg_ctx, |c| c.stroke_rect(x, y, w, h));
    }
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSBezierPath moveToPoint:]`
pub extern "C" fn shim_nsbezierpath_move_to(obj: u64, _cmd: u64, x: f64, y: f64) {
    let h = path_handle(obj);
    if h != 0 {
        cg_shims::bridge::with_path(h, |p| {
            p.move_to(graphics_core::Point::new(x, y));
        });
    }
}

/// `-[NSBezierPath lineToPoint:]`
pub extern "C" fn shim_nsbezierpath_line_to(obj: u64, _cmd: u64, x: f64, y: f64) {
    let h = path_handle(obj);
    if h != 0 {
        cg_shims::bridge::with_path(h, |p| {
            p.line_to(graphics_core::Point::new(x, y));
        });
    }
}

/// `-[NSBezierPath curveToPoint:controlPoint1:controlPoint2:]`
pub extern "C" fn shim_nsbezierpath_curve_to(
    obj: u64,
    _cmd: u64,
    x: f64,
    y: f64, // endpoint
    cp1x: f64,
    cp1y: f64, // control point 1
    cp2x: f64,
    cp2y: f64, // control point 2
) {
    let h = path_handle(obj);
    if h != 0 {
        cg_shims::bridge::with_path(h, |p| {
            p.cubic_to(
                graphics_core::Point::new(cp1x, cp1y),
                graphics_core::Point::new(cp2x, cp2y),
                graphics_core::Point::new(x, y),
            );
        });
    }
}

/// `-[NSBezierPath closePath]`
pub extern "C" fn shim_nsbezierpath_close_path(obj: u64, _cmd: u64) {
    let h = path_handle(obj);
    if h != 0 {
        cg_shims::bridge::with_path(h, |p| {
            p.close();
        });
    }
}

/// `-[NSBezierPath fill]` — fill the path using the current context's fill color.
pub extern "C" fn shim_nsbezierpath_fill(obj: u64, _cmd: u64) {
    let h = path_handle(obj);
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if h == 0 || cg_ctx == 0 {
        return;
    }

    // Copy the path elements into the context's path, then fill.
    cg_shims::bridge::copy_path_to_context(h, cg_ctx);
    cg_shims::bridge::with_context(cg_ctx, |c| c.fill_path());
}

/// `-[NSBezierPath stroke]` — stroke the path using the current context's stroke color.
pub extern "C" fn shim_nsbezierpath_stroke(obj: u64, _cmd: u64) {
    let h = path_handle(obj);
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if h == 0 || cg_ctx == 0 {
        return;
    }

    cg_shims::bridge::copy_path_to_context(h, cg_ctx);
    // Use NSBezierPath's lineWidth
    let lw = if obj != 0 {
        unsafe { *((obj as usize + LINE_WIDTH_OFF) as *const f64) }
    } else {
        1.0
    };
    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.line_width = lw;
        c.stroke_path();
    });
}

/// `-[NSBezierPath lineWidth]`
pub extern "C" fn shim_nsbezierpath_line_width(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 1.0;
    }
    unsafe { *((obj as usize + LINE_WIDTH_OFF) as *const f64) }
}

/// `-[NSBezierPath setLineWidth:]`
pub extern "C" fn shim_nsbezierpath_set_line_width(obj: u64, _cmd: u64, width: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + LINE_WIDTH_OFF) as *mut f64) = width;
    }
}

/// `-[NSBezierPath addClip]` — stub (no-op for now).
pub extern "C" fn shim_nsbezierpath_add_clip(_obj: u64, _cmd: u64) {
    // Clipping requires CG context clip support — deferred.
}

/// `-[NSBezierPath appendBezierPath:]` — appends elements from another path.
pub extern "C" fn shim_nsbezierpath_append_bezier_path(obj: u64, _cmd: u64, other: u64) {
    if obj == 0 || other == 0 {
        return;
    }
    let dst = path_handle(obj);
    let src = path_handle(other);
    if dst == 0 || src == 0 {
        return;
    }
    cg_shims::bridge::path_append_path(dst, src);
}

/// `-[NSBezierPath appendBezierPathWithRect:]` — appends a rectangle.
pub extern "C" fn shim_nsbezierpath_append_rect(
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
    let h_path = path_handle(obj);
    if h_path == 0 {
        return;
    }
    cg_shims::bridge::with_path(h_path, |p| {
        p.move_to(graphics_core::Point::new(x, y));
        p.line_to(graphics_core::Point::new(x + w, y));
        p.line_to(graphics_core::Point::new(x + w, y + h));
        p.line_to(graphics_core::Point::new(x, y + h));
        p.close();
    });
}

/// `-[NSBezierPath appendBezierPathWithOvalInRect:]` — appends an oval.
pub extern "C" fn shim_nsbezierpath_append_oval(
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
    let h_path = path_handle(obj);
    if h_path == 0 {
        return;
    }
    cg_shims::bridge::with_path(h_path, |p| {
        p.add_ellipse(graphics_core::Rect::new(x, y, w, h));
    });
}

/// `-[NSBezierPath isEmpty]`
pub extern "C" fn shim_nsbezierpath_is_empty(obj: u64, _cmd: u64) -> u8 {
    let h = path_handle(obj);
    if h == 0 {
        return 1;
    }
    let empty = cg_shims::bridge::path_is_empty(h);
    u8::from(empty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn bezier_path_factory() {
        let bp = shim_nsbezierpath_bezier_path(0, 0);
        assert_ne!(bp, 0);
        let isa = read_field(bp, 0);
        assert_eq!(isa, crate::nsobj::nsbezierpath_class_addr());
        free_obj(bp);
    }

    #[test]
    fn bezier_path_default_line_width() {
        let bp = alloc_bezier_path();
        assert_eq!(shim_nsbezierpath_line_width(bp, 0), 1.0);
        free_obj(bp);
    }

    #[test]
    fn set_line_width() {
        let bp = alloc_bezier_path();
        shim_nsbezierpath_set_line_width(bp, 0, 3.5);
        assert_eq!(shim_nsbezierpath_line_width(bp, 0), 3.5);
        free_obj(bp);
    }

    #[test]
    fn line_width_nil_safety() {
        assert_eq!(shim_nsbezierpath_line_width(0, 0), 1.0);
    }

    #[test]
    fn with_rect_factory() {
        let bp = shim_nsbezierpath_with_rect(0, 0, 10.0, 20.0, 100.0, 50.0);
        assert_ne!(bp, 0);
        assert_eq!(shim_nsbezierpath_is_empty(bp, 0), 0);
        free_obj(bp);
    }

    #[test]
    fn with_oval_factory() {
        let bp = shim_nsbezierpath_with_oval(0, 0, 0.0, 0.0, 100.0, 100.0);
        assert_ne!(bp, 0);
        assert_eq!(shim_nsbezierpath_is_empty(bp, 0), 0);
        free_obj(bp);
    }

    #[test]
    fn with_rounded_rect_factory() {
        let bp = shim_nsbezierpath_with_rounded_rect(0, 0, 0.0, 0.0, 100.0, 50.0, 5.0, 5.0);
        assert_ne!(bp, 0);
        assert_eq!(shim_nsbezierpath_is_empty(bp, 0), 0);
        free_obj(bp);
    }

    #[test]
    fn move_and_line_to() {
        let bp = alloc_bezier_path();
        assert_eq!(shim_nsbezierpath_is_empty(bp, 0), 1);
        shim_nsbezierpath_move_to(bp, 0, 0.0, 0.0);
        shim_nsbezierpath_line_to(bp, 0, 100.0, 100.0);
        assert_eq!(shim_nsbezierpath_is_empty(bp, 0), 0);
        free_obj(bp);
    }

    #[test]
    fn is_empty_nil_safety() {
        assert_eq!(shim_nsbezierpath_is_empty(0, 0), 1);
    }

    #[test]
    fn fill_rect_no_context_no_crash() {
        // Should not panic even without a current context
        shim_nsbezierpath_fill_rect(0, 0, 0.0, 0.0, 10.0, 10.0);
    }

    #[test]
    fn stroke_rect_no_context_no_crash() {
        shim_nsbezierpath_stroke_rect(0, 0, 0.0, 0.0, 10.0, 10.0);
    }
}
