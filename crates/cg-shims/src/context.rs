//! CGContext stubs — the core drawing context API.
//!
//! `CGBitmapContextCreate` now allocates a real `Surface` via the bridge
//! module, and drawing calls (fill/stroke) operate on it via `SoftwareRenderer`.

use crate::bridge;
use libsystem_shims::table::ShimTable;

pub fn install(table: &mut ShimTable) {
    use libsystem_shims::register_shims;
    register_shims!(table,
        // ── Context creation ────────────────────────────────────
        ("_CGBitmapContextCreate", shim_cg_bitmap_context_create as u64, "CGBitmapContextCreate (bridge → Surface)"),
        ("_CGBitmapContextCreateImage", shim_cg_bitmap_create_image as u64, "CGBitmapContextCreateImage (bridge → handle)"),
        ("_CGBitmapContextGetData", shim_cg_bitmap_get_data as u64, "CGBitmapContextGetData (bridge → pixel ptr)"),
        ("_CGBitmapContextGetWidth", shim_cg_bitmap_get_width as u64, "CGBitmapContextGetWidth (bridge → real)"),
        ("_CGBitmapContextGetHeight", shim_cg_bitmap_get_height as u64, "CGBitmapContextGetHeight (bridge → real)"),

        // ── State ───────────────────────────────────────────────
        ("_CGContextSaveGState", shim_cg_save_gstate as u64, "CGContextSaveGState (bridge → real)"),
        ("_CGContextRestoreGState", shim_cg_restore_gstate as u64, "CGContextRestoreGState (bridge → real)"),

        // ── Transform ───────────────────────────────────────────
        ("_CGContextTranslateCTM", shim_cg_void3 as u64, "CGContextTranslateCTM (stub → void)"),
        ("_CGContextScaleCTM", shim_cg_void3 as u64, "CGContextScaleCTM (stub → void)"),
        ("_CGContextRotateCTM", shim_cg_void2 as u64, "CGContextRotateCTM (stub → void)"),
        ("_CGContextConcatCTM", shim_cg_void2 as u64, "CGContextConcatCTM (stub → void)"),

        // ── Color ───────────────────────────────────────────────
        ("_CGContextSetFillColorWithColor", shim_cg_set_fill_color as u64, "CGContextSetFillColorWithColor (bridge → real)"),
        ("_CGContextSetStrokeColorWithColor", shim_cg_set_stroke_color as u64, "CGContextSetStrokeColorWithColor (bridge → real)"),
        ("_CGContextSetRGBFillColor", shim_cg_set_rgb_fill as u64, "CGContextSetRGBFillColor (bridge → real)"),
        ("_CGContextSetRGBStrokeColor", shim_cg_set_rgb_stroke as u64, "CGContextSetRGBStrokeColor (bridge → real)"),
        ("_CGContextSetAlpha", shim_cg_set_alpha as u64, "CGContextSetAlpha (bridge → real)"),

        // ── Drawing ─────────────────────────────────────────────
        ("_CGContextFillRect", shim_cg_fill_rect as u64, "CGContextFillRect (bridge → real)"),
        ("_CGContextStrokeRect", shim_cg_stroke_rect as u64, "CGContextStrokeRect (bridge → real)"),
        ("_CGContextFillEllipseInRect", shim_cg_fill_ellipse as u64, "CGContextFillEllipseInRect (bridge → real)"),
        ("_CGContextStrokePath", shim_cg_stroke_path as u64, "CGContextStrokePath (bridge → real)"),
        ("_CGContextFillPath", shim_cg_fill_path as u64, "CGContextFillPath (bridge → real)"),
        ("_CGContextDrawImage", shim_cg_void6 as u64, "CGContextDrawImage (stub → void)"),
        ("_CGContextClipToRect", shim_cg_void5 as u64, "CGContextClipToRect (stub → void)"),
        ("_CGContextSetLineWidth", shim_cg_set_line_width as u64, "CGContextSetLineWidth (bridge → real)"),
        ("_CGContextSetLineCap", shim_cg_void2 as u64, "CGContextSetLineCap (stub → void)"),
        ("_CGContextSetLineJoin", shim_cg_void2 as u64, "CGContextSetLineJoin (stub → void)"),

        // ── Path in context ─────────────────────────────────────
        ("_CGContextBeginPath", shim_cg_begin_path as u64, "CGContextBeginPath (bridge → real)"),
        ("_CGContextMoveToPoint", shim_cg_move_to as u64, "CGContextMoveToPoint (bridge → real)"),
        ("_CGContextAddLineToPoint", shim_cg_line_to as u64, "CGContextAddLineToPoint (bridge → real)"),
        ("_CGContextAddCurveToPoint", shim_cg_curve_to as u64, "CGContextAddCurveToPoint (bridge → real)"),
        ("_CGContextAddRect", shim_cg_add_rect as u64, "CGContextAddRect (bridge → real)"),
        ("_CGContextClosePath", shim_cg_close_path as u64, "CGContextClosePath (bridge → real)"),

        // ── Text ────────────────────────────────────────────────
        ("_CGContextSetFont", shim_cg_void2 as u64, "CGContextSetFont (stub → void)"),
        ("_CGContextSetFontSize", shim_cg_void2 as u64, "CGContextSetFontSize (stub → void)"),
        ("_CGContextShowTextAtPoint", shim_cg_void5 as u64, "CGContextShowTextAtPoint (stub → void)"),

        // ── Lifecycle ───────────────────────────────────────────
        ("_CGContextRetain", shim_cg_retain as u64, "CGContextRetain (bridge → same ptr)"),
        ("_CGContextRelease", shim_cg_context_release as u64, "CGContextRelease (bridge → real)"),
        ("_CGContextFlush", shim_cg_void1 as u64, "CGContextFlush (stub → void)")
    );
}

/// CGBitmapContextCreate(data, w, h, bpc, bpr, cs, info) → CGContextRef
pub extern "C" fn shim_cg_bitmap_context_create(
    _data: u64, w: u64, h: u64, _bpc: u64, _bpr: u64, _cs: u64, _info: u32,
) -> u64 {
    let width = if w == 0 { 1 } else { w as u32 };
    let height = if h == 0 { 1 } else { h as u32 };
    bridge::context_create(width, height)
}

pub extern "C" fn shim_cg_retain(ctx: u64) -> u64 { ctx }
pub extern "C" fn shim_cg_return_null(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_cg_return_zero(_a: u64) -> u64 { 0 }

/// CGBitmapContextGetData → pointer to the underlying pixel buffer.
pub extern "C" fn shim_cg_bitmap_get_data(ctx: u64) -> u64 {
    bridge::context_get_data(ctx)
}

/// CGBitmapContextGetWidth → width of the context surface.
pub extern "C" fn shim_cg_bitmap_get_width(ctx: u64) -> u64 {
    bridge::context_get_width(ctx)
}

/// CGBitmapContextGetHeight → height of the context surface.
pub extern "C" fn shim_cg_bitmap_get_height(ctx: u64) -> u64 {
    bridge::context_get_height(ctx)
}

/// CGBitmapContextCreateImage → returns the context handle itself.
pub extern "C" fn shim_cg_bitmap_create_image(ctx: u64) -> u64 { ctx }

// ── State ───────────────────────────────────────────────────────

pub extern "C" fn shim_cg_save_gstate(ctx: u64) {
    bridge::with_context(ctx, |c| c.save_state());
}

pub extern "C" fn shim_cg_restore_gstate(ctx: u64) {
    bridge::with_context(ctx, |c| c.restore_state());
}

// ── Color ───────────────────────────────────────────────────────

pub extern "C" fn shim_cg_set_rgb_fill(ctx: u64, r: f64, g: f64, b: f64, a: f64) {
    bridge::with_context(ctx, |c| {
        c.fill_color = graphics_core::Color::new(r, g, b, a);
    });
}

pub extern "C" fn shim_cg_set_rgb_stroke(ctx: u64, r: f64, g: f64, b: f64, a: f64) {
    bridge::with_context(ctx, |c| {
        c.stroke_color = graphics_core::Color::new(r, g, b, a);
    });
}

pub extern "C" fn shim_cg_set_fill_color(ctx: u64, color_handle: u64) {
    if let Some(color) = bridge::color_get(color_handle) {
        bridge::with_context(ctx, |c| { c.fill_color = color; });
    }
}

pub extern "C" fn shim_cg_set_stroke_color(ctx: u64, color_handle: u64) {
    if let Some(color) = bridge::color_get(color_handle) {
        bridge::with_context(ctx, |c| { c.stroke_color = color; });
    }
}

pub extern "C" fn shim_cg_set_alpha(ctx: u64, alpha: f64) {
    bridge::with_context(ctx, |c| { c.alpha = alpha; });
}

// ── Drawing ─────────────────────────────────────────────────────

pub extern "C" fn shim_cg_fill_rect(ctx: u64, x: f64, y: f64, w: f64, h: f64) {
    bridge::with_context(ctx, |c| c.fill_rect(x, y, w, h));
}

pub extern "C" fn shim_cg_stroke_rect(ctx: u64, x: f64, y: f64, w: f64, h: f64) {
    bridge::with_context(ctx, |c| c.stroke_rect(x, y, w, h));
}

pub extern "C" fn shim_cg_fill_ellipse(ctx: u64, x: f64, y: f64, w: f64, h: f64) {
    bridge::with_context(ctx, |c| c.fill_ellipse(x, y, w, h));
}

pub extern "C" fn shim_cg_stroke_path(ctx: u64) {
    bridge::with_context(ctx, |c| c.stroke_path());
}

pub extern "C" fn shim_cg_fill_path(ctx: u64) {
    bridge::with_context(ctx, |c| c.fill_path());
}

pub extern "C" fn shim_cg_set_line_width(ctx: u64, width: f64) {
    bridge::with_context(ctx, |c| { c.line_width = width; });
}

// ── Path in context ─────────────────────────────────────────────

pub extern "C" fn shim_cg_begin_path(ctx: u64) {
    bridge::with_context(ctx, |c| { c.path = graphics_core::BezierPath::new(); });
}

pub extern "C" fn shim_cg_move_to(ctx: u64, x: f64, y: f64) {
    bridge::with_context(ctx, |c| { c.path.move_to(graphics_core::Point::new(x, y)); });
}

pub extern "C" fn shim_cg_line_to(ctx: u64, x: f64, y: f64) {
    bridge::with_context(ctx, |c| { c.path.line_to(graphics_core::Point::new(x, y)); });
}

pub extern "C" fn shim_cg_curve_to(ctx: u64, cp1x: f64, cp1y: f64, cp2x: f64, cp2y: f64, x: f64, y: f64) {
    bridge::with_context(ctx, |c| {
        c.path.cubic_to(
            graphics_core::Point::new(cp1x, cp1y),
            graphics_core::Point::new(cp2x, cp2y),
            graphics_core::Point::new(x, y),
        );
    });
}

pub extern "C" fn shim_cg_close_path(ctx: u64) {
    bridge::with_context(ctx, |c| { c.path.close(); });
}

pub extern "C" fn shim_cg_add_rect(ctx: u64, x: f64, y: f64, w: f64, h: f64) {
    bridge::with_context(ctx, |c| {
        c.path.move_to(graphics_core::Point::new(x, y));
        c.path.line_to(graphics_core::Point::new(x + w, y));
        c.path.line_to(graphics_core::Point::new(x + w, y + h));
        c.path.line_to(graphics_core::Point::new(x, y + h));
        c.path.close();
    });
}

// ── No-op stubs for operations not yet bridged ──────────────────

pub extern "C" fn shim_cg_void1(_a: u64) {}
pub extern "C" fn shim_cg_void2(_a: u64, _b: u64) {}
pub extern "C" fn shim_cg_void3(_a: u64, _b: u64, _c: u64) {}
pub extern "C" fn shim_cg_void5(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64) {}
pub extern "C" fn shim_cg_void6(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64) {}
pub extern "C" fn shim_cg_void7(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64, _g: u64) {}

pub extern "C" fn shim_cg_context_release(ctx: u64) {
    bridge::context_release(ctx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsystem_shims::table::ShimTable;

    #[test]
    fn context_stubs_registered() {
        let mut table = ShimTable::new();
        install(&mut table);
        assert!(table.resolve("_CGBitmapContextCreate").is_some());
        assert!(table.resolve("_CGContextFillRect").is_some());
        assert!(table.resolve("_CGContextRelease").is_some());
    }

    #[test]
    fn bitmap_context_is_non_null() {
        let ctx = shim_cg_bitmap_context_create(0, 100, 100, 8, 400, 0, 0);
        assert_ne!(ctx, 0);
    }
}
