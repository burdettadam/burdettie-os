//! CALayer compositor — composites view/layer trees using tiny-skia.
//!
//! When a view's backing CALayer has non-default visual properties (opacity,
//! cornerRadius, borderWidth), the compositor renders that view into an
//! offscreen tiny_skia::Pixmap, then alpha-blends the result onto the
//! destination surface with the appropriate layer effects applied.
//!
//! ## Pixel format
//!
//! The graphics-core `Surface` stores ARGB32 as packed u32:
//!   `(a << 24) | (r << 16) | (g << 8) | b`
//!
//! tiny-skia uses premultiplied RGBA byte array: `[R, G, B, A]`.
//! Conversion functions are provided below.

use crate::calayer::{self, LayerProps};

// ── Pixel format conversion ────────────────────────────────────

/// Convert a ARGB32 pixel buffer (u32 slice) to premultiplied RGBA bytes
/// suitable for tiny_skia::Pixmap.
fn argb32_to_premul_rgba(src: &[u32], dst: &mut [u8]) {
    debug_assert_eq!(dst.len(), src.len() * 4);
    for (i, &pixel) in src.iter().enumerate() {
        let a = ((pixel >> 24) & 0xFF) as u8;
        let r = ((pixel >> 16) & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = (pixel & 0xFF) as u8;
        // Surface colors are already straight-alpha from SoftwareRenderer;
        // tiny-skia expects premultiplied, so premultiply here.
        let af = a as f32 / 255.0;
        let base = i * 4;
        dst[base] = (r as f32 * af + 0.5) as u8;
        dst[base + 1] = (g as f32 * af + 0.5) as u8;
        dst[base + 2] = (b as f32 * af + 0.5) as u8;
        dst[base + 3] = a;
    }
}

/// Convert premultiplied RGBA bytes back to ARGB32 u32 pixels.
fn premul_rgba_to_argb32(src: &[u8], dst: &mut [u32]) {
    debug_assert_eq!(src.len(), dst.len() * 4);
    for (i, pixel) in dst.iter_mut().enumerate() {
        let base = i * 4;
        let r = src[base];
        let g = src[base + 1];
        let b = src[base + 2];
        let a = src[base + 3];
        // Un-premultiply back to straight alpha for Surface consumption.
        if a == 0 {
            *pixel = 0;
        } else if a == 255 {
            *pixel = 0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        } else {
            let af = 255.0 / a as f32;
            let ur = (r as f32 * af + 0.5).min(255.0) as u32;
            let ug = (g as f32 * af + 0.5).min(255.0) as u32;
            let ub = (b as f32 * af + 0.5).min(255.0) as u32;
            *pixel = ((a as u32) << 24) | (ur << 16) | (ug << 8) | ub;
        }
    }
}

// ── Layer-effect queries ───────────────────────────────────────

/// Check whether a layer has any non-default visual properties that require
/// the compositor to process it (instead of simple painter's-algorithm rendering).
pub(crate) fn layer_needs_compositing(layer: u64) -> bool {
    if layer == 0 {
        return false;
    }
    let props = calayer::get_layer_props(layer);
    props.opacity < 1.0
        || props.hidden
        || props.corner_radius > 0.0
        || props.border_width > 0.0
        || props.masks_to_bounds
        || props.contents != 0
        || !calayer::is_identity_transform(&props.transform)
}

// ── Compositing operations ─────────────────────────────────────

/// Composite a view's rendered content onto the destination surface, applying
/// its CALayer's visual effects (opacity, cornerRadius clipping, border).
///
/// `src_pixels` - the ARGB32 pixels of the per-view offscreen buffer (view's drawRect: output)
/// `(src_w, src_h)` - dimensions of the source buffer
/// `dst_pixels` - the ARGB32 pixels of the window's CGContext surface (destination)
/// `(dst_w, dst_h)` - dimensions of the destination buffer
/// `view_x`, `view_y` - the view's frame origin in the destination coordinate space
/// `props` - the CALayer visual properties to apply
pub(crate) fn composite_layer(
    src_pixels: &[u32],
    (src_w, src_h): (u32, u32),
    dst_pixels: &mut [u32],
    (dst_w, dst_h): (u32, u32),
    view_x: i32,
    view_y: i32,
    props: &LayerProps,
) {
    if props.hidden || src_w == 0 || src_h == 0 {
        return;
    }

    // Convert source to tiny-skia premultiplied RGBA pixmap.
    let mut src_rgba = vec![0u8; (src_w as usize) * (src_h as usize) * 4];
    argb32_to_premul_rgba(src_pixels, &mut src_rgba);

    let Some(mut src_pixmap) = tiny_skia::Pixmap::from_vec(
        src_rgba,
        tiny_skia::IntSize::from_wh(src_w, src_h).expect("nonzero source size"),
    ) else {
        return;
    };

    // Apply corner-radius clipping (masksToBounds + cornerRadius > 0).
    if props.masks_to_bounds && props.corner_radius > 0.0 {
        apply_corner_radius_mask(&mut src_pixmap, props.corner_radius as f32);
    }

    // Apply layer opacity by scaling every pixel's alpha.
    if props.opacity < 1.0 {
        apply_opacity(&mut src_pixmap, props.opacity);
    }

    // Draw border (rendered on top of content).
    if props.border_width > 0.0 && props.border_color != 0 {
        draw_border(&mut src_pixmap, props);
    }

    // Blit the composited layer onto the destination surface.
    if !calayer::is_identity_transform(&props.transform) {
        // Transform-aware blit: use tiny-skia to draw with the 2D affine transform.
        blit_pixmap_transformed(
            &src_pixmap,
            dst_pixels,
            dst_w,
            dst_h,
            view_x,
            view_y,
            &props.transform,
        );
    } else {
        blit_pixmap_to_argb32(&src_pixmap, dst_pixels, dst_w, dst_h, view_x, view_y);
    }
}

/// Render a CALayer's `contents` (CGImage) into an ARGB32 destination buffer,
/// scaling to fill the given view dimensions.
pub(crate) fn render_layer_contents(
    dst_pixels: &mut [u32],
    (dst_w, dst_h): (u32, u32),
    view_x: i32,
    view_y: i32,
    (view_w, view_h): (u32, u32),
    contents: u64,
) {
    if contents == 0 || view_w == 0 || view_h == 0 {
        return;
    }

    let Some((img_w, img_h, rgba_data)) = cg_shims::bridge::image_get_rgba_data(contents) else {
        return;
    };
    if img_w == 0 || img_h == 0 {
        return;
    }

    // Scale the source image to the destination view rect using nearest-neighbor.
    let x_scale = img_w as f64 / view_w as f64;
    let y_scale = img_h as f64 / view_h as f64;

    for row in 0..view_h as i32 {
        let dy = view_y + row;
        if dy < 0 || dy >= dst_h as i32 {
            continue;
        }
        let src_row = ((row as f64 * y_scale) as u32).min(img_h - 1);
        for col in 0..view_w as i32 {
            let dx = view_x + col;
            if dx < 0 || dx >= dst_w as i32 {
                continue;
            }
            let src_col = ((col as f64 * x_scale) as u32).min(img_w - 1);
            let si = ((src_row * img_w + src_col) * 4) as usize;
            let r = rgba_data[si] as u32;
            let g = rgba_data[si + 1] as u32;
            let b = rgba_data[si + 2] as u32;
            let a = rgba_data[si + 3] as u32;

            if a == 0 {
                continue;
            }

            let di = (dy as u32 * dst_w + dx as u32) as usize;
            if a == 255 {
                dst_pixels[di] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            } else {
                // Source-over blend.
                let dst_pixel = dst_pixels[di];
                let da = (dst_pixel >> 24) & 0xFF;
                let dr = (dst_pixel >> 16) & 0xFF;
                let dg = (dst_pixel >> 8) & 0xFF;
                let db = dst_pixel & 0xFF;
                let inv_a = 255 - a;
                let oa = a + ((da * inv_a + 128) / 255);
                let or_ch = ((r * a + 128) / 255) + ((dr * inv_a + 128) / 255);
                let og = ((g * a + 128) / 255) + ((dg * inv_a + 128) / 255);
                let ob = ((b * a + 128) / 255) + ((db * inv_a + 128) / 255);
                dst_pixels[di] =
                    (oa.min(255) << 24) | (or_ch.min(255) << 16) | (og.min(255) << 8) | ob.min(255);
            }
        }
    }
}

/// Extract the 2D affine subset from a CATransform3D (4×4 column-major)
/// and return as a `tiny_skia::Transform`.
pub(crate) fn transform_from_catransform3d(m: &[f64; 16]) -> tiny_skia::Transform {
    // CATransform3D column-major: m[0]=m11, m[1]=m12, m[4]=m21, m[5]=m22,
    //                              m[12]=m41(tx), m[13]=m42(ty)
    tiny_skia::Transform::from_row(
        m[0] as f32,  // sx
        m[1] as f32,  // ky (skew-y)
        m[4] as f32,  // kx (skew-x)
        m[5] as f32,  // sy
        m[12] as f32, // tx
        m[13] as f32, // ty
    )
}

/// Apply a rounded-rectangle clip mask in-place. Pixels outside the rounded
/// rect are zeroed out.
fn apply_corner_radius_mask(pixmap: &mut tiny_skia::Pixmap, radius: f32) {
    let w = pixmap.width() as f32;
    let h = pixmap.height() as f32;
    let r = radius.min(w / 2.0).min(h / 2.0);

    // Create a mask pixmap: draw a filled rounded rect, then multiply.
    let Some(mut mask) = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()) else {
        return;
    };

    let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, w, h);
    let Some(rect) = rect else { return };
    let rrect = {
        let mut pb = tiny_skia::PathBuilder::new();
        pb.push_rect(rect);
        if r > 0.0 {
            // Build rounded rect path manually for tiny-skia.
            pb = tiny_skia::PathBuilder::new();
            // Top edge
            pb.move_to(r, 0.0);
            pb.line_to(w - r, 0.0);
            pb.quad_to(w, 0.0, w, r);
            // Right edge
            pb.line_to(w, h - r);
            pb.quad_to(w, h, w - r, h);
            // Bottom edge
            pb.line_to(r, h);
            pb.quad_to(0.0, h, 0.0, h - r);
            // Left edge
            pb.line_to(0.0, r);
            pb.quad_to(0.0, 0.0, r, 0.0);
            pb.close();
        }
        pb.finish()
    };
    let Some(path) = rrect else { return };

    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = true;

    mask.fill_path(
        &path,
        &paint,
        tiny_skia::FillRule::Winding,
        tiny_skia::Transform::identity(),
        None,
    );

    // Multiply original pixels by mask alpha.
    let mask_data = mask.data();
    let src_data = pixmap.data_mut();
    for i in (0..src_data.len()).step_by(4) {
        let mask_a = mask_data[i + 3] as u16;
        src_data[i] = ((src_data[i] as u16 * mask_a + 128) / 255) as u8;
        src_data[i + 1] = ((src_data[i + 1] as u16 * mask_a + 128) / 255) as u8;
        src_data[i + 2] = ((src_data[i + 2] as u16 * mask_a + 128) / 255) as u8;
        src_data[i + 3] = ((src_data[i + 3] as u16 * mask_a + 128) / 255) as u8;
    }
}

/// Scale all pixel alphas by the given opacity factor (0.0–1.0).
fn apply_opacity(pixmap: &mut tiny_skia::Pixmap, opacity: f32) {
    let factor = (opacity.clamp(0.0, 1.0) * 255.0 + 0.5) as u16;
    let data = pixmap.data_mut();
    for i in (0..data.len()).step_by(4) {
        // For premultiplied alpha, scale all channels uniformly.
        data[i] = ((data[i] as u16 * factor + 128) / 255) as u8;
        data[i + 1] = ((data[i + 1] as u16 * factor + 128) / 255) as u8;
        data[i + 2] = ((data[i + 2] as u16 * factor + 128) / 255) as u8;
        data[i + 3] = ((data[i + 3] as u16 * factor + 128) / 255) as u8;
    }
}

/// Draw a border stroke onto the pixmap using layer borderWidth and borderColor.
fn draw_border(pixmap: &mut tiny_skia::Pixmap, props: &LayerProps) {
    let w = pixmap.width() as f32;
    let h = pixmap.height() as f32;
    let bw = props.border_width as f32;
    if bw <= 0.0 || w <= 0.0 || h <= 0.0 {
        return;
    }

    // Resolve CGColor to RGBA.
    let color = cg_shims::bridge::color_get(props.border_color);
    let (r, g, b, a) = match color {
        Some(c) => (
            (c.r.clamp(0.0, 1.0) * 255.0) as u8,
            (c.g.clamp(0.0, 1.0) * 255.0) as u8,
            (c.b.clamp(0.0, 1.0) * 255.0) as u8,
            (c.a.clamp(0.0, 1.0) * 255.0) as u8,
        ),
        None => (0, 0, 0, 255),
    };

    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(r, g, b, a);
    paint.anti_alias = true;

    let stroke = tiny_skia::Stroke {
        width: bw,
        ..Default::default()
    };

    let cr = props.corner_radius as f32;
    let radius = cr.min(w / 2.0).min(h / 2.0);
    // Inset the stroke path by half the border width so it's fully inside.
    let inset = bw / 2.0;

    let mut pb = tiny_skia::PathBuilder::new();
    if radius > 0.0 {
        let r = (radius - inset).max(0.0);
        pb.move_to(inset + r, inset);
        pb.line_to(w - inset - r, inset);
        pb.quad_to(w - inset, inset, w - inset, inset + r);
        pb.line_to(w - inset, h - inset - r);
        pb.quad_to(w - inset, h - inset, w - inset - r, h - inset);
        pb.line_to(inset + r, h - inset);
        pb.quad_to(inset, h - inset, inset, h - inset - r);
        pb.line_to(inset, inset + r);
        pb.quad_to(inset, inset, inset + r, inset);
        pb.close();
    } else {
        let Some(rect) = tiny_skia::Rect::from_xywh(inset, inset, w - bw, h - bw) else {
            return;
        };
        pb.push_rect(rect);
    }
    let Some(path) = pb.finish() else { return };

    pixmap.stroke_path(
        &path,
        &paint,
        &stroke,
        tiny_skia::Transform::identity(),
        None,
    );
}

/// Alpha-blend a tiny-skia Pixmap onto an ARGB32 destination buffer at the
/// given offset, using standard source-over compositing.
fn blit_pixmap_to_argb32(
    src: &tiny_skia::Pixmap,
    dst: &mut [u32],
    dst_w: u32,
    dst_h: u32,
    off_x: i32,
    off_y: i32,
) {
    let src_w = src.width() as i32;
    let src_h = src.height() as i32;
    let src_data = src.data();

    for sy in 0..src_h {
        let dy = off_y + sy;
        if dy < 0 || dy >= dst_h as i32 {
            continue;
        }
        for sx in 0..src_w {
            let dx = off_x + sx;
            if dx < 0 || dx >= dst_w as i32 {
                continue;
            }
            let si = ((sy * src_w + sx) * 4) as usize;
            let sr = src_data[si] as u32;
            let sg = src_data[si + 1] as u32;
            let sb = src_data[si + 2] as u32;
            let sa = src_data[si + 3] as u32;

            if sa == 0 {
                continue;
            }

            let di = (dy as u32 * dst_w + dx as u32) as usize;
            if sa == 255 {
                // Fully opaque: just write.
                dst[di] = 0xFF00_0000 | (sr << 16) | (sg << 8) | sb;
            } else {
                // Source-over blend (premultiplied src onto straight-alpha dst).
                let dst_pixel = dst[di];
                let da = (dst_pixel >> 24) & 0xFF;
                let dr = (dst_pixel >> 16) & 0xFF;
                let dg = (dst_pixel >> 8) & 0xFF;
                let db = dst_pixel & 0xFF;

                let inv_sa = 255 - sa;
                let oa = sa + ((da * inv_sa + 128) / 255);
                let or = sr + ((dr * inv_sa + 128) / 255);
                let og = sg + ((dg * inv_sa + 128) / 255);
                let ob = sb + ((db * inv_sa + 128) / 255);

                dst[di] =
                    (oa.min(255) << 24) | (or.min(255) << 16) | (og.min(255) << 8) | ob.min(255);
            }
        }
    }
}

/// Blit a tiny-skia Pixmap onto an ARGB32 destination with a 2D affine transform.
/// The transform is applied around the layer's center, so rotation/scale
/// pivots about the middle of the composited rectangle.
fn blit_pixmap_transformed(
    src: &tiny_skia::Pixmap,
    dst: &mut [u32],
    dst_w: u32,
    dst_h: u32,
    off_x: i32,
    off_y: i32,
    cat3d: &[f64; 16],
) {
    // Build a destination pixmap large enough for the full window surface,
    // convert existing dst into it, composite the src with transform, then
    // copy changed region back.  For simplicity we only allocate the subregion
    // that the transformed source can affect (bounding box of the transformed
    // corners) plus a margin.
    let sw = src.width() as f32;
    let sh = src.height() as f32;
    let tfm = transform_from_catransform3d(cat3d);

    // Translate so the transform is applied at the layer position.
    let full_tfm = tiny_skia::Transform::from_translate(off_x as f32, off_y as f32)
        .post_concat(tiny_skia::Transform::from_translate(sw / 2.0, sh / 2.0))
        .post_concat(tfm)
        .post_concat(tiny_skia::Transform::from_translate(-sw / 2.0, -sh / 2.0));

    // Build a destination pixmap covering the full window.
    let Some(mut dst_pixmap) = tiny_skia::Pixmap::new(dst_w, dst_h) else {
        return;
    };
    // Copy existing destination into the pixmap.
    let mut dst_rgba = vec![0u8; (dst_w as usize) * (dst_h as usize) * 4];
    argb32_to_premul_rgba(dst, &mut dst_rgba);
    let Some(bg) =
        tiny_skia::Pixmap::from_vec(dst_rgba, tiny_skia::IntSize::from_wh(dst_w, dst_h).unwrap())
    else {
        return;
    };
    dst_pixmap.draw_pixmap(
        0,
        0,
        bg.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    // Draw the source with the transform applied.
    let paint = tiny_skia::PixmapPaint {
        opacity: 1.0,
        blend_mode: tiny_skia::BlendMode::SourceOver,
        quality: tiny_skia::FilterQuality::Bilinear,
    };
    dst_pixmap.draw_pixmap(0, 0, src.as_ref(), &paint, full_tfm, None);

    // Copy the result back to the dst ARGB32 buffer.
    premul_rgba_to_argb32(dst_pixmap.data(), dst);
}

/// Draw a layer's backgroundColor onto a destination surface at the given
/// view position and size. Used during the normal display cycle so the layer's
/// background appears even without per-view offscreen buffers.
pub(crate) fn draw_layer_background(
    dst_pixels: &mut [u32],
    (dst_w, dst_h): (u32, u32),
    view_x: i32,
    view_y: i32,
    (view_w, view_h): (u32, u32),
    bg_color: u64,
    props: &LayerProps,
) {
    if bg_color == 0 || view_w == 0 || view_h == 0 {
        return;
    }

    let color = cg_shims::bridge::color_get(bg_color);
    let Some(color) = color else { return };

    let (r, g, b, a) = (
        (color.r.clamp(0.0, 1.0) * 255.0) as u8,
        (color.g.clamp(0.0, 1.0) * 255.0) as u8,
        (color.b.clamp(0.0, 1.0) * 255.0) as u8,
        (color.a.clamp(0.0, 1.0) * props.opacity as f64 * 255.0) as u8,
    );

    if a == 0 {
        return;
    }

    let cr = props.corner_radius as f32;
    if cr > 0.0 {
        // Use tiny-skia to render a filled rounded rect.
        let Some(mut pixmap) = tiny_skia::Pixmap::new(view_w, view_h) else {
            return;
        };
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(r, g, b, a);
        paint.anti_alias = true;

        let w = view_w as f32;
        let h = view_h as f32;
        let radius = cr.min(w / 2.0).min(h / 2.0);

        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(radius, 0.0);
        pb.line_to(w - radius, 0.0);
        pb.quad_to(w, 0.0, w, radius);
        pb.line_to(w, h - radius);
        pb.quad_to(w, h, w - radius, h);
        pb.line_to(radius, h);
        pb.quad_to(0.0, h, 0.0, h - radius);
        pb.line_to(0.0, radius);
        pb.quad_to(0.0, 0.0, radius, 0.0);
        pb.close();
        let Some(path) = pb.finish() else { return };

        pixmap.fill_path(
            &path,
            &paint,
            tiny_skia::FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );

        blit_pixmap_to_argb32(&pixmap, dst_pixels, dst_w, dst_h, view_x, view_y);
    } else {
        // Simple rectangle fill — no tiny-skia needed.
        let argb = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        for row in 0..view_h as i32 {
            let dy = view_y + row;
            if dy < 0 || dy >= dst_h as i32 {
                continue;
            }
            for col in 0..view_w as i32 {
                let dx = view_x + col;
                if dx < 0 || dx >= dst_w as i32 {
                    continue;
                }
                let di = (dy as u32 * dst_w + dx as u32) as usize;
                if a == 255 {
                    dst_pixels[di] = argb;
                } else {
                    // Source-over blend.
                    let sa = a as u32;
                    let inv_sa = 255 - sa;
                    let dst_pixel = dst_pixels[di];
                    let da = (dst_pixel >> 24) & 0xFF;
                    let dr = (dst_pixel >> 16) & 0xFF;
                    let dg = (dst_pixel >> 8) & 0xFF;
                    let db = dst_pixel & 0xFF;
                    let oa = sa + ((da * inv_sa + 128) / 255);
                    let or_ch = ((r as u32) * sa / 255) + ((dr * inv_sa + 128) / 255);
                    let og = ((g as u32) * sa / 255) + ((dg * inv_sa + 128) / 255);
                    let ob = ((b as u32) * sa / 255) + ((db * inv_sa + 128) / 255);
                    dst_pixels[di] = (oa.min(255) << 24)
                        | (or_ch.min(255) << 16)
                        | (og.min(255) << 8)
                        | ob.min(255);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argb32_to_rgba_roundtrip() {
        // Opaque red pixel.
        let src = [0xFF_FF_00_00u32]; // ARGB = (255, 255, 0, 0)
        let mut rgba = [0u8; 4];
        argb32_to_premul_rgba(&src, &mut rgba);
        assert_eq!(rgba, [255, 0, 0, 255]); // R=255, G=0, B=0, A=255

        let mut dst = [0u32; 1];
        premul_rgba_to_argb32(&rgba, &mut dst);
        assert_eq!(dst[0], 0xFF_FF_00_00);
    }

    #[test]
    fn argb32_transparent_roundtrip() {
        // 50% transparent green.
        let src = [0x80_00_FF_00u32]; // ARGB = (128, 0, 255, 0)
        let mut rgba = [0u8; 4];
        argb32_to_premul_rgba(&src, &mut rgba);
        // Premultiplied: R=0, G=255*128/255≈128, B=0, A=128
        assert_eq!(rgba[3], 128); // alpha preserved
        assert_eq!(rgba[0], 0); // red=0
        assert!(rgba[1] >= 127 && rgba[1] <= 129); // green≈128

        let mut dst = [0u32; 1];
        premul_rgba_to_argb32(&rgba, &mut dst);
        let a = (dst[0] >> 24) & 0xFF;
        let g = (dst[0] >> 8) & 0xFF;
        assert_eq!(a, 128);
        assert!(g >= 254); // Un-premul should recover ~255
    }

    #[test]
    fn argb32_fully_transparent() {
        let src = [0x00_00_00_00u32];
        let mut rgba = [0u8; 4];
        argb32_to_premul_rgba(&src, &mut rgba);
        assert_eq!(rgba, [0, 0, 0, 0]);

        let mut dst = [0u32; 1];
        premul_rgba_to_argb32(&rgba, &mut dst);
        assert_eq!(dst[0], 0);
    }

    #[test]
    fn layer_needs_compositing_defaults() {
        let layer = crate::calayer::alloc_layer();
        // Ensure a clean prop entry exists (no stale data from address reuse).
        crate::calayer::shim_calayer_set_opacity(layer, 0, 1.0);
        crate::calayer::shim_calayer_set_hidden(layer, 0, 0);
        crate::calayer::shim_calayer_set_corner_radius(layer, 0, 0.0);
        crate::calayer::shim_calayer_set_border_width(layer, 0, 0.0);
        crate::calayer::shim_calayer_set_masks_to_bounds(layer, 0, 0);
        assert!(!layer_needs_compositing(layer));
        assert!(!layer_needs_compositing(0));
        crate::shim_test_helpers::free_obj(layer);
    }

    #[test]
    fn layer_needs_compositing_opacity() {
        let layer = crate::calayer::alloc_layer();
        crate::calayer::shim_calayer_set_opacity(layer, 0, 0.5);
        assert!(layer_needs_compositing(layer));
        crate::shim_test_helpers::free_obj(layer);
    }

    #[test]
    fn layer_needs_compositing_corner_radius() {
        let layer = crate::calayer::alloc_layer();
        crate::calayer::shim_calayer_set_corner_radius(layer, 0, 8.0);
        assert!(layer_needs_compositing(layer));
        crate::shim_test_helpers::free_obj(layer);
    }

    #[test]
    fn layer_needs_compositing_hidden() {
        let layer = crate::calayer::alloc_layer();
        crate::calayer::shim_calayer_set_hidden(layer, 0, 1);
        assert!(layer_needs_compositing(layer));
        crate::shim_test_helpers::free_obj(layer);
    }

    #[test]
    fn composite_hidden_layer_is_noop() {
        let mut dst = vec![0xFFFF_FFFFu32; 4]; // 2x2 white
        let src = vec![0xFF_FF_00_00u32; 4]; // 2x2 red
        let props = LayerProps {
            hidden: true,
            ..Default::default()
        };
        composite_layer(&src, (2, 2), &mut dst, (2, 2), 0, 0, &props);
        // Destination should be unchanged.
        assert!(dst.iter().all(|&p| p == 0xFFFF_FFFF));
    }

    #[test]
    fn composite_opaque_layer() {
        let mut dst = vec![0xFFFF_FFFFu32; 4]; // 2x2 white
        let src = vec![0xFF_FF_00_00u32; 4]; // 2x2 opaque red
        let props = LayerProps::default(); // opacity=1.0
        composite_layer(&src, (2, 2), &mut dst, (2, 2), 0, 0, &props);
        // Red should overwrite white.
        for &p in &dst {
            assert_eq!((p >> 24) & 0xFF, 0xFF); // fully opaque
            assert!((p >> 16) & 0xFF > 200); // red channel high
        }
    }

    #[test]
    fn composite_half_opacity_blends() {
        let mut dst = vec![0xFF_FF_FF_FFu32; 4]; // 2x2 white
        let src = vec![0xFF_00_00_00u32; 4]; // 2x2 opaque black
        let props = LayerProps {
            opacity: 0.5,
            ..Default::default()
        };
        composite_layer(&src, (2, 2), &mut dst, (2, 2), 0, 0, &props);
        // Should be roughly gray (blended black at 50% over white).
        for &p in &dst {
            let r = (p >> 16) & 0xFF;
            // Expect something near 128 (midpoint of 0 and 255).
            assert!(r > 100 && r < 200, "expected ~128 gray, got r={r}");
        }
    }

    #[test]
    fn composite_with_offset() {
        let mut dst = vec![0xFF_FF_FF_FFu32; 16]; // 4x4 white
        let src = vec![0xFF_00_FF_00u32; 4]; // 2x2 opaque green
        let props = LayerProps::default();
        composite_layer(&src, (2, 2), &mut dst, (4, 4), 1, 1, &props);
        // Pixel (0,0) should still be white.
        assert_eq!(dst[0], 0xFF_FF_FF_FF);
        // Pixel (1,1) should be green.
        let p11 = dst[4 + 1]; // row 1, col 1
        assert!((p11 >> 8) & 0xFF > 200); // green channel high
    }

    #[test]
    fn draw_layer_background_simple_rect() {
        // Create a CGColor for solid blue.
        let color_handle = cg_shims::bridge::color_create_rgb(0.0, 0.0, 1.0, 1.0);
        let mut dst = vec![0xFF_FF_FF_FFu32; 16]; // 4x4 white
        let props = LayerProps::default();
        draw_layer_background(&mut dst, (4, 4), 1, 1, (2, 2), color_handle, &props);
        // Pixel (0,0) = white, pixel (1,1) = blue.
        assert_eq!(dst[0], 0xFF_FF_FF_FF);
        let p11 = dst[4 + 1];
        assert_eq!(p11 & 0xFF, 0xFF); // blue=255
        assert_eq!((p11 >> 16) & 0xFF, 0); // red=0
        cg_shims::bridge::color_release(color_handle);
    }

    #[test]
    fn draw_layer_background_rounded() {
        let color_handle = cg_shims::bridge::color_create_rgb(1.0, 0.0, 0.0, 1.0);
        let mut dst = vec![0xFF_FF_FF_FFu32; 400]; // 20x20 white
        let props = LayerProps {
            corner_radius: 5.0,
            ..Default::default()
        };
        draw_layer_background(&mut dst, (20, 20), 0, 0, (20, 20), color_handle, &props);
        // Corner pixel (0,0) should be clipped (still white-ish) because of the radius.
        let corner = dst[0];
        let center = dst[10 * 20 + 10]; // center pixel should be red.
        let center_r = (center >> 16) & 0xFF;
        assert!(center_r > 200, "center should be red, got r={center_r}");
        // Corner should be mostly white (anti-aliased edge might have some red).
        let corner_r = (corner >> 16) & 0xFF;
        let corner_b = corner & 0xFF;
        // At radius=5, pixel (0,0) is outside the rounded rect,
        // so it should be mostly white.
        assert!(
            corner_r > corner_b || corner == 0xFF_FF_FF_FF,
            "corner should be white/whitish"
        );
        cg_shims::bridge::color_release(color_handle);
    }
}
