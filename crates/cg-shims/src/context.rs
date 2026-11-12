//! CGContext stubs — the core drawing context API.
//!
//! `CGBitmapContextCreate` now allocates a real `Surface` via the bridge
//! module, and drawing calls (fill/stroke) operate on it via `SoftwareRenderer`.

use crate::bridge;
use libsystem_shims::table::ShimTable;

pub fn install(table: &mut ShimTable) {
    use libsystem_shims::register_shims;
    register_shims!(
        table,
        // ── Context creation ────────────────────────────────────
        (
            "_CGBitmapContextCreate",
            shim_cg_bitmap_context_create as *const () as u64,
            "CGBitmapContextCreate (bridge → Surface)"
        ),
        (
            "_CGBitmapContextCreateImage",
            shim_cg_bitmap_create_image as *const () as u64,
            "CGBitmapContextCreateImage (bridge → handle)"
        ),
        (
            "_CGBitmapContextGetData",
            shim_cg_bitmap_get_data as *const () as u64,
            "CGBitmapContextGetData (bridge → pixel ptr)"
        ),
        (
            "_CGBitmapContextGetWidth",
            shim_cg_bitmap_get_width as *const () as u64,
            "CGBitmapContextGetWidth (bridge → real)"
        ),
        (
            "_CGBitmapContextGetHeight",
            shim_cg_bitmap_get_height as *const () as u64,
            "CGBitmapContextGetHeight (bridge → real)"
        ),
        // ── State ───────────────────────────────────────────────
        (
            "_CGContextSaveGState",
            shim_cg_save_gstate as *const () as u64,
            "CGContextSaveGState (bridge → real)"
        ),
        (
            "_CGContextRestoreGState",
            shim_cg_restore_gstate as *const () as u64,
            "CGContextRestoreGState (bridge → real)"
        ),
        // ── Transform ───────────────────────────────────────────
        (
            "_CGContextTranslateCTM",
            shim_cg_translate_ctm as *const () as u64,
            "CGContextTranslateCTM (bridge → real)"
        ),
        (
            "_CGContextScaleCTM",
            shim_cg_scale_ctm as *const () as u64,
            "CGContextScaleCTM (bridge → real)"
        ),
        (
            "_CGContextRotateCTM",
            shim_cg_rotate_ctm as *const () as u64,
            "CGContextRotateCTM (bridge → real)"
        ),
        (
            "_CGContextConcatCTM",
            shim_cg_concat_ctm as *const () as u64,
            "CGContextConcatCTM (bridge → real)"
        ),
        (
            "_CGContextGetCTM",
            shim_cg_get_ctm as *const () as u64,
            "CGContextGetCTM (bridge → real)"
        ),
        // ── Color ───────────────────────────────────────────────
        (
            "_CGContextSetFillColorWithColor",
            shim_cg_set_fill_color as *const () as u64,
            "CGContextSetFillColorWithColor (bridge → real)"
        ),
        (
            "_CGContextSetStrokeColorWithColor",
            shim_cg_set_stroke_color as *const () as u64,
            "CGContextSetStrokeColorWithColor (bridge → real)"
        ),
        (
            "_CGContextSetRGBFillColor",
            shim_cg_set_rgb_fill as *const () as u64,
            "CGContextSetRGBFillColor (bridge → real)"
        ),
        (
            "_CGContextSetRGBStrokeColor",
            shim_cg_set_rgb_stroke as *const () as u64,
            "CGContextSetRGBStrokeColor (bridge → real)"
        ),
        (
            "_CGContextSetAlpha",
            shim_cg_set_alpha as *const () as u64,
            "CGContextSetAlpha (bridge → real)"
        ),
        // ── Drawing ─────────────────────────────────────────────
        (
            "_CGContextFillRect",
            shim_cg_fill_rect as *const () as u64,
            "CGContextFillRect (bridge → real)"
        ),
        (
            "_CGContextStrokeRect",
            shim_cg_stroke_rect as *const () as u64,
            "CGContextStrokeRect (bridge → real)"
        ),
        (
            "_CGContextFillEllipseInRect",
            shim_cg_fill_ellipse as *const () as u64,
            "CGContextFillEllipseInRect (bridge → real)"
        ),
        (
            "_CGContextStrokePath",
            shim_cg_stroke_path as *const () as u64,
            "CGContextStrokePath (bridge → real)"
        ),
        (
            "_CGContextFillPath",
            shim_cg_fill_path as *const () as u64,
            "CGContextFillPath (bridge → real)"
        ),
        (
            "_CGContextDrawImage",
            shim_cg_draw_image as *const () as u64,
            "CGContextDrawImage (bridge → real)"
        ),
        (
            "_CGContextClipToRect",
            shim_cg_clip_to_rect as *const () as u64,
            "CGContextClipToRect (bridge → real)"
        ),
        (
            "_CGContextClip",
            shim_cg_void1 as *const () as u64,
            "CGContextClip (stub → void)"
        ),
        (
            "_CGContextEOClip",
            shim_cg_void1 as *const () as u64,
            "CGContextEOClip (stub → void)"
        ),
        (
            "_CGContextSetLineWidth",
            shim_cg_set_line_width as *const () as u64,
            "CGContextSetLineWidth (bridge → real)"
        ),
        (
            "_CGContextSetLineCap",
            shim_cg_void2 as *const () as u64,
            "CGContextSetLineCap (stub → void)"
        ),
        (
            "_CGContextSetLineJoin",
            shim_cg_void2 as *const () as u64,
            "CGContextSetLineJoin (stub → void)"
        ),
        // ── Path in context ─────────────────────────────────────
        (
            "_CGContextBeginPath",
            shim_cg_begin_path as *const () as u64,
            "CGContextBeginPath (bridge → real)"
        ),
        (
            "_CGContextMoveToPoint",
            shim_cg_move_to as *const () as u64,
            "CGContextMoveToPoint (bridge → real)"
        ),
        (
            "_CGContextAddLineToPoint",
            shim_cg_line_to as *const () as u64,
            "CGContextAddLineToPoint (bridge → real)"
        ),
        (
            "_CGContextAddCurveToPoint",
            shim_cg_curve_to as *const () as u64,
            "CGContextAddCurveToPoint (bridge → real)"
        ),
        (
            "_CGContextAddRect",
            shim_cg_add_rect as *const () as u64,
            "CGContextAddRect (bridge → real)"
        ),
        (
            "_CGContextClosePath",
            shim_cg_close_path as *const () as u64,
            "CGContextClosePath (bridge → real)"
        ),
        (
            "_CGContextAddQuadCurveToPoint",
            shim_cg_quad_curve_to as *const () as u64,
            "CGContextAddQuadCurveToPoint (bridge → real)"
        ),
        (
            "_CGContextAddArc",
            shim_cg_add_arc as *const () as u64,
            "CGContextAddArc (bridge → real)"
        ),
        (
            "_CGContextSetLineDash",
            shim_cg_set_line_dash as *const () as u64,
            "CGContextSetLineDash (bridge → real)"
        ),
        // ── Text ────────────────────────────────────────────────
        (
            "_CGContextSetFont",
            shim_cg_void2 as *const () as u64,
            "CGContextSetFont (stub → void, font name ignored)"
        ),
        (
            "_CGContextSetFontSize",
            shim_cg_set_font_size as *const () as u64,
            "CGContextSetFontSize (bridge → real)"
        ),
        (
            "_CGContextShowTextAtPoint",
            shim_cg_show_text_at_point as *const () as u64,
            "CGContextShowTextAtPoint (bridge → real)"
        ),
        (
            "_CGContextSelectFont",
            shim_cg_select_font as *const () as u64,
            "CGContextSelectFont (bridge → font size only)"
        ),
        (
            "_CGContextSetTextDrawingMode",
            shim_cg_void2 as *const () as u64,
            "CGContextSetTextDrawingMode (stub → void)"
        ),
        (
            "_CGContextSetTextPosition",
            shim_cg_set_text_position as *const () as u64,
            "CGContextSetTextPosition (bridge → real)"
        ),
        (
            "_CGContextGetTextPosition",
            shim_cg_get_text_position as *const () as u64,
            "CGContextGetTextPosition (bridge → real)"
        ),
        (
            "_CGContextSetCharacterSpacing",
            shim_cg_void2 as *const () as u64,
            "CGContextSetCharacterSpacing (stub → void)"
        ),
        (
            "_CGContextShowText",
            shim_cg_show_text as *const () as u64,
            "CGContextShowText (bridge → real)"
        ),
        // ── Lifecycle ───────────────────────────────────────────
        (
            "_CGContextRetain",
            shim_cg_retain as *const () as u64,
            "CGContextRetain (bridge → same ptr)"
        ),
        (
            "_CGContextRelease",
            shim_cg_context_release as *const () as u64,
            "CGContextRelease (bridge → real)"
        ),
        (
            "_CGContextFlush",
            shim_cg_void1 as *const () as u64,
            "CGContextFlush (stub → void)"
        )
    );
}

/// CGBitmapContextCreate(data, w, h, bpc, bpr, cs, info) → CGContextRef
pub extern "C" fn shim_cg_bitmap_context_create(
    _data: u64,
    w: u64,
    h: u64,
    _bpc: u64,
    _bpr: u64,
    _cs: u64,
    _info: u32,
) -> u64 {
    let width = if w == 0 { 1 } else { w as u32 };
    let height = if h == 0 { 1 } else { h as u32 };
    bridge::context_create(width, height)
}

pub extern "C" fn shim_cg_retain(ctx: u64) -> u64 {
    ctx
}
pub extern "C" fn shim_cg_return_null(_a: u64) -> u64 {
    0
}
pub extern "C" fn shim_cg_return_zero(_a: u64) -> u64 {
    0
}

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

/// CGBitmapContextCreateImage → copies context pixels into a CGImage handle.
pub extern "C" fn shim_cg_bitmap_create_image(ctx: u64) -> u64 {
    bridge::image_create_from_context(ctx)
}

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
        bridge::with_context(ctx, |c| {
            c.fill_color = color;
        });
    }
}

pub extern "C" fn shim_cg_set_stroke_color(ctx: u64, color_handle: u64) {
    if let Some(color) = bridge::color_get(color_handle) {
        bridge::with_context(ctx, |c| {
            c.stroke_color = color;
        });
    }
}

pub extern "C" fn shim_cg_set_alpha(ctx: u64, alpha: f64) {
    bridge::with_context(ctx, |c| {
        c.alpha = alpha;
    });
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
    bridge::with_context(ctx, |c| {
        c.line_width = width;
    });
}

// ── Path in context ─────────────────────────────────────────────

pub extern "C" fn shim_cg_begin_path(ctx: u64) {
    bridge::with_context(ctx, |c| {
        c.path = graphics_core::BezierPath::new();
    });
}

pub extern "C" fn shim_cg_move_to(ctx: u64, x: f64, y: f64) {
    bridge::with_context(ctx, |c| {
        c.path.move_to(graphics_core::Point::new(x, y));
    });
}

pub extern "C" fn shim_cg_line_to(ctx: u64, x: f64, y: f64) {
    bridge::with_context(ctx, |c| {
        c.path.line_to(graphics_core::Point::new(x, y));
    });
}

pub extern "C" fn shim_cg_curve_to(
    ctx: u64,
    cp1x: f64,
    cp1y: f64,
    cp2x: f64,
    cp2y: f64,
    x: f64,
    y: f64,
) {
    bridge::with_context(ctx, |c| {
        c.path.cubic_to(
            graphics_core::Point::new(cp1x, cp1y),
            graphics_core::Point::new(cp2x, cp2y),
            graphics_core::Point::new(x, y),
        );
    });
}

pub extern "C" fn shim_cg_close_path(ctx: u64) {
    bridge::with_context(ctx, |c| {
        c.path.close();
    });
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

/// CGContextAddQuadCurveToPoint(ctx, cpx, cpy, x, y)
pub extern "C" fn shim_cg_quad_curve_to(ctx: u64, cpx: f64, cpy: f64, x: f64, y: f64) {
    bridge::with_context(ctx, |c| {
        c.path.quad_to(
            graphics_core::Point::new(cpx, cpy),
            graphics_core::Point::new(x, y),
        );
    });
}

/// CGContextAddArc(ctx, x, y, radius, startAngle, endAngle, clockwise)
///
/// Approximates the arc with cubic Bézier segments (≤π/2 each).
/// `clockwise` follows CG convention: 1 = clockwise in default (Y-up) coords.
pub extern "C" fn shim_cg_add_arc(
    ctx: u64,
    cx: f64,
    cy: f64,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    clockwise: i32,
) {
    bridge::with_context(ctx, |c| {
        add_arc_to_path(
            &mut c.path,
            cx,
            cy,
            radius,
            start_angle,
            end_angle,
            clockwise != 0,
        );
    });
}

/// Generate cubic Bézier segments approximating a circular arc.
pub fn add_arc_to_path(
    path: &mut graphics_core::BezierPath,
    cx: f64,
    cy: f64,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    clockwise: bool,
) {
    use std::f64::consts::PI;

    // Normalize the sweep direction
    let mut sweep = end_angle - start_angle;
    if clockwise {
        // CG clockwise = negative sweep in math coords
        while sweep > 0.0 {
            sweep -= 2.0 * PI;
        }
    } else {
        while sweep < 0.0 {
            sweep += 2.0 * PI;
        }
    }

    if sweep.abs() < 1e-10 {
        // Degenerate — just line to start point
        let sx = cx + radius * start_angle.cos();
        let sy = cy + radius * start_angle.sin();
        path.line_to(graphics_core::Point::new(sx, sy));
        return;
    }

    // Split into segments of at most π/2
    let n = (sweep.abs() / (PI / 2.0)).ceil() as usize;
    let n = n.max(1);
    let seg_angle = sweep / n as f64;

    let mut angle = start_angle;
    // Move/line to the start of the arc
    let sx = cx + radius * angle.cos();
    let sy = cy + radius * angle.sin();

    // If path is empty, move to start; otherwise line to start
    if path.elements().is_empty() {
        path.move_to(graphics_core::Point::new(sx, sy));
    } else {
        path.line_to(graphics_core::Point::new(sx, sy));
    }

    for _ in 0..n {
        let next_angle = angle + seg_angle;
        // Magic constant for cubic Bézier approximation of a circular arc
        let alpha = (seg_angle / 2.0).tan() * (4.0 / 3.0);

        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let cos_b = next_angle.cos();
        let sin_b = next_angle.sin();

        let cp1x = cx + radius * (cos_a - alpha * sin_a);
        let cp1y = cy + radius * (sin_a + alpha * cos_a);
        let cp2x = cx + radius * (cos_b + alpha * sin_b);
        let cp2y = cy + radius * (sin_b - alpha * cos_b);
        let ex = cx + radius * cos_b;
        let ey = cy + radius * sin_b;

        path.cubic_to(
            graphics_core::Point::new(cp1x, cp1y),
            graphics_core::Point::new(cp2x, cp2y),
            graphics_core::Point::new(ex, ey),
        );
        angle = next_angle;
    }
}

/// CGContextSetLineDash(ctx, phase, lengths_ptr, count)
pub extern "C" fn shim_cg_set_line_dash(ctx: u64, phase: f64, lengths_ptr: u64, count: u64) {
    bridge::with_context(ctx, |c| {
        if count == 0 || lengths_ptr == 0 {
            c.line_dash_lengths.clear();
            c.line_dash_phase = 0.0;
        } else {
            let count = count as usize;
            let lengths: Vec<f64> = (0..count)
                .map(|i| unsafe { *((lengths_ptr as *const f64).add(i)) })
                .collect();
            c.line_dash_lengths = lengths;
            c.line_dash_phase = phase;
        }
    });
}

// ── No-op stubs for operations not yet bridged ──────────────────

pub extern "C" fn shim_cg_void1(_a: u64) {}
pub extern "C" fn shim_cg_void2(_a: u64, _b: u64) {}
pub extern "C" fn shim_cg_void3(_a: u64, _b: u64, _c: u64) {}
pub extern "C" fn shim_cg_void5(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64) {}
pub extern "C" fn shim_cg_void6(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64) {}
pub extern "C" fn shim_cg_void7(_a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64, _g: u64) {}

// ── CTM transforms ──────────────────────────────────────────────

/// CGContextTranslateCTM(ctx, tx, ty) — tx/ty arrive in d0/d1.
pub extern "C" fn shim_cg_translate_ctm(ctx: u64, tx: f64, ty: f64) {
    bridge::with_context(ctx, |c| c.translate_ctm(tx, ty));
}

/// CGContextScaleCTM(ctx, sx, sy) — sx/sy arrive in d0/d1.
pub extern "C" fn shim_cg_scale_ctm(ctx: u64, sx: f64, sy: f64) {
    bridge::with_context(ctx, |c| c.scale_ctm(sx, sy));
}

/// CGContextRotateCTM(ctx, angle) — angle arrives in d0.
pub extern "C" fn shim_cg_rotate_ctm(ctx: u64, angle: f64) {
    bridge::with_context(ctx, |c| c.rotate_ctm(angle));
}

/// CGContextConcatCTM(ctx, transform_ptr) — transform is >16 bytes, passed by
/// invisible reference on ARM64 (pointer in x1).
pub extern "C" fn shim_cg_concat_ctm(ctx: u64, transform_ptr: u64) {
    if transform_ptr == 0 {
        return;
    }
    let m = unsafe {
        let p = transform_ptr as *const f64;
        [*p, *p.add(1), *p.add(2), *p.add(3), *p.add(4), *p.add(5)]
    };
    bridge::with_context(ctx, |c| c.concat_ctm(m));
}

/// CGContextGetCTM(ctx) → CGAffineTransform (48 bytes via x8 sret).
pub extern "C" fn shim_cg_get_ctm(sret: u64, ctx: u64) {
    let ctm = bridge::with_context(ctx, |c| c.ctm).unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    if sret != 0 {
        unsafe {
            let p = sret as *mut f64;
            for (i, v) in ctm.iter().enumerate() {
                *p.add(i) = *v;
            }
        }
    }
}

/// CGContextClipToRect(ctx, x, y, w, h)
pub extern "C" fn shim_cg_clip_to_rect(ctx: u64, x: f64, y: f64, w: f64, h: f64) {
    bridge::with_context(ctx, |c| c.clip_to_rect(x, y, w, h));
}

/// CGContextDrawImage(ctx, rect.x, rect.y, rect.w, rect.h, image_handle)
///
/// On ARM64: ctx in x0, CGRect in d0-d3, image in x1.
/// The shim table signature delivers: ctx, x, y, w, h, image.
pub extern "C" fn shim_cg_draw_image(ctx: u64, x: f64, y: f64, w: f64, h: f64, image: u64) {
    bridge::draw_image_in_context(ctx, image, x, y, w, h);
}

// ── Text ────────────────────────────────────────────────────────

/// CGContextSetFontSize(ctx, size)
pub extern "C" fn shim_cg_set_font_size(ctx: u64, size: f64) {
    bridge::with_context(ctx, |c| {
        c.font_size = size;
    });
}

/// CGContextSelectFont(ctx, name, size, encoding) — we only use `size`.
pub extern "C" fn shim_cg_select_font(ctx: u64, _name: u64, size: f64, _encoding: u32) {
    bridge::with_context(ctx, |c| {
        c.font_size = size;
    });
}

/// CGContextSetTextPosition(ctx, x, y)
pub extern "C" fn shim_cg_set_text_position(ctx: u64, x: f64, y: f64) {
    bridge::with_context(ctx, |c| {
        c.text_x = x;
        c.text_y = y;
    });
}

/// CGContextGetTextPosition(ctx) → CGPoint (returned as two f64 via x8 indirect).
pub extern "C" fn shim_cg_get_text_position(result: u64, ctx: u64) {
    let (x, y) = bridge::with_context(ctx, |c| (c.text_x, c.text_y)).unwrap_or((0.0, 0.0));
    if result != 0 {
        unsafe {
            *(result as *mut f64) = x;
            *((result as usize + 8) as *mut f64) = y;
        }
    }
}

/// CGContextShowTextAtPoint(ctx, x, y, chars, length)
///
/// Draws `length` bytes of C string at (x, y) using the current fill color
/// and font size. The text position is advanced by the rendered width.
pub extern "C" fn shim_cg_show_text_at_point(ctx: u64, x: f64, y: f64, chars: u64, length: u64) {
    if chars == 0 || length == 0 {
        return;
    }
    let text = unsafe {
        let slice = std::slice::from_raw_parts(chars as *const u8, length as usize);
        String::from_utf8_lossy(slice).into_owned()
    };
    bridge::with_context(ctx, |c| {
        c.draw_text(&text, x, y);
    });
}

/// CGContextShowText(ctx, chars, length)
///
/// Draws text at the current text position.
pub extern "C" fn shim_cg_show_text(ctx: u64, chars: u64, length: u64) {
    if chars == 0 || length == 0 {
        return;
    }
    let text = unsafe {
        let slice = std::slice::from_raw_parts(chars as *const u8, length as usize);
        String::from_utf8_lossy(slice).into_owned()
    };
    bridge::with_context(ctx, |c| {
        let x = c.text_x;
        let y = c.text_y;
        c.draw_text(&text, x, y);
    });
}

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
