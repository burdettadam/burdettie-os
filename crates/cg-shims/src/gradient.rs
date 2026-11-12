//! CGGradient shims — linear and radial gradient drawing.
//!
//! Stores gradient color stops in the bridge handle table and renders
//! them using `SoftwareRenderer` when `CGContextDrawLinearGradient` or
//! `CGContextDrawRadialGradient` is called.

use crate::bridge;
use libsystem_shims::table::ShimTable;
use std::collections::HashMap;
use std::sync::Mutex;

pub fn install(table: &mut ShimTable) {
    use libsystem_shims::register_shims;
    register_shims!(
        table,
        (
            "_CGGradientCreateWithColorComponents",
            shim_gradient_create as *const () as u64,
            "CGGradientCreateWithColorComponents (bridge → real)"
        ),
        (
            "_CGGradientCreateWithColors",
            shim_gradient_create_with_colors as *const () as u64,
            "CGGradientCreateWithColors (bridge → real)"
        ),
        (
            "_CGGradientRetain",
            shim_gradient_retain as *const () as u64,
            "CGGradientRetain (bridge → same)"
        ),
        (
            "_CGGradientRelease",
            shim_gradient_release as *const () as u64,
            "CGGradientRelease (bridge → real)"
        ),
        (
            "_CGContextDrawLinearGradient",
            shim_draw_linear_gradient as *const () as u64,
            "CGContextDrawLinearGradient (bridge → real)"
        ),
        (
            "_CGContextDrawRadialGradient",
            shim_draw_radial_gradient as *const () as u64,
            "CGContextDrawRadialGradient (bridge → approx)"
        )
    );
}

/// A gradient with color stops.
struct Gradient {
    /// Each stop is (location 0..1, r, g, b, a).
    stops: Vec<(f64, f64, f64, f64, f64)>,
}

impl Gradient {
    /// Interpolate color at parameter `t` (0..1) along the gradient.
    fn color_at(&self, t: f64) -> (f64, f64, f64, f64) {
        if self.stops.is_empty() {
            return (0.0, 0.0, 0.0, 1.0);
        }
        if self.stops.len() == 1 || t <= self.stops[0].0 {
            let s = &self.stops[0];
            return (s.1, s.2, s.3, s.4);
        }
        for i in 1..self.stops.len() {
            if t <= self.stops[i].0 {
                let a = &self.stops[i - 1];
                let b = &self.stops[i];
                let range = b.0 - a.0;
                let frac = if range > 1e-12 {
                    (t - a.0) / range
                } else {
                    0.0
                };
                return (
                    a.1 + (b.1 - a.1) * frac,
                    a.2 + (b.2 - a.2) * frac,
                    a.3 + (b.3 - a.3) * frac,
                    a.4 + (b.4 - a.4) * frac,
                );
            }
        }
        let s = self.stops.last().unwrap();
        (s.1, s.2, s.3, s.4)
    }
}

fn composite_pixel(pixel: &mut u32, r: f64, g: f64, b: f64, a: f64) {
    if a > 0.999 {
        *pixel = (255 << 24)
            | ((r * 255.0) as u32) << 16
            | ((g * 255.0) as u32) << 8
            | (b * 255.0) as u32;
    } else if a > 0.001 {
        let dst_a = ((*pixel >> 24) & 0xFF) as f64 / 255.0;
        let dst_r = ((*pixel >> 16) & 0xFF) as f64 / 255.0;
        let dst_g = ((*pixel >> 8) & 0xFF) as f64 / 255.0;
        let dst_b = (*pixel & 0xFF) as f64 / 255.0;
        let out_a = a + dst_a * (1.0 - a);
        let out_r = (r * a + dst_r * dst_a * (1.0 - a)) / out_a;
        let out_g = (g * a + dst_g * dst_a * (1.0 - a)) / out_a;
        let out_b = (b * a + dst_b * dst_a * (1.0 - a)) / out_a;
        *pixel = ((out_a * 255.0) as u32) << 24
            | ((out_r * 255.0) as u32) << 16
            | ((out_g * 255.0) as u32) << 8
            | (out_b * 255.0) as u32;
    }
}

fn radial_gradient_t(
    (px, py): (f64, f64),
    (start_x, start_y): (f64, f64),
    start_radius: f64,
    (end_x, end_y): (f64, f64),
    end_radius: f64,
) -> f64 {
    let mx = px - start_x;
    let my = py - start_y;
    let dx = end_x - start_x;
    let dy = end_y - start_y;
    let dr = end_radius - start_radius;

    // Solve |(P - S) - t(E - S)| = r0 + t(r1 - r0) for t.
    let a = dx * dx + dy * dy - dr * dr;
    let b = -2.0 * (mx * dx + my * dy + start_radius * dr);
    let c = mx * mx + my * my - start_radius * start_radius;

    if a.abs() < 1e-12 {
        if b.abs() < 1e-12 {
            return if c <= 0.0 { 0.0 } else { 1.0 };
        }
        return (-c / b).clamp(0.0, 1.0);
    }

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        if dr.abs() < 1e-12 {
            return if c <= 0.0 { 0.0 } else { 1.0 };
        }
        let dist = (mx * mx + my * my).sqrt();
        return ((dist - start_radius) / dr).clamp(0.0, 1.0);
    }

    let sqrt_disc = discriminant.sqrt();
    let t0 = (-b - sqrt_disc) / (2.0 * a);
    let t1 = (-b + sqrt_disc) / (2.0 * a);
    let mut valid = [t0, t1]
        .into_iter()
        .filter(|t| t.is_finite() && (0.0..=1.0).contains(t));
    if let Some(first) = valid.next() {
        return valid.fold(first, |chosen, candidate| {
            if dr >= 0.0 {
                chosen.max(candidate)
            } else {
                chosen.min(candidate)
            }
        });
    }

    let start_dist = (mx * mx + my * my).sqrt() - start_radius;
    let end_dx = px - end_x;
    let end_dy = py - end_y;
    let end_dist = (end_dx * end_dx + end_dy * end_dy).sqrt() - end_radius;
    if end_dist.abs() < start_dist.abs() {
        1.0
    } else {
        0.0
    }
}

static GRADIENTS: Mutex<Option<HashMap<u64, Box<Gradient>>>> = Mutex::new(None);

static NEXT_GRAD_HANDLE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0x4347_4752_1000);

fn next_handle() -> u64 {
    NEXT_GRAD_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// CGGradientCreateWithColorComponents(colorspace, components, locations, count)
///
/// `components` is a flat array of RGBA values: [r0, g0, b0, a0, r1, g1, b1, a1, ...]
/// `locations` is an array of `count` f64 values (0..1). If NULL, evenly spaced.
pub extern "C" fn shim_gradient_create(
    _colorspace: u64,
    components: u64,
    locations: u64,
    count: u64,
) -> u64 {
    let count = count as usize;
    if components == 0 || count == 0 {
        return 0;
    }
    let mut stops = Vec::with_capacity(count);
    for i in 0..count {
        let (r, g, b, a) = unsafe {
            let p = (components as *const f64).add(i * 4);
            (*p, *p.add(1), *p.add(2), *p.add(3))
        };
        let loc = if locations != 0 {
            unsafe { *(locations as *const f64).add(i) }
        } else if count > 1 {
            i as f64 / (count - 1) as f64
        } else {
            0.0
        };
        stops.push((loc, r, g, b, a));
    }
    stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let handle = next_handle();
    let mut guard = GRADIENTS.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_or_insert_with(HashMap::new)
        .insert(handle, Box::new(Gradient { stops }));
    handle
}

/// CGGradientCreateWithColors(colorspace, colors_cfarray, locations) — simplified.
/// Since we don't have full CFArray support, this returns a dummy gradient.
pub extern "C" fn shim_gradient_create_with_colors(
    _colorspace: u64,
    _colors: u64,
    _locations: u64,
) -> u64 {
    // Create a simple black-to-white fallback gradient.
    let stops = vec![(0.0, 0.0, 0.0, 0.0, 1.0), (1.0, 1.0, 1.0, 1.0, 1.0)];
    let handle = next_handle();
    let mut guard = GRADIENTS.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_or_insert_with(HashMap::new)
        .insert(handle, Box::new(Gradient { stops }));
    handle
}

pub extern "C" fn shim_gradient_retain(g: u64) -> u64 {
    g
}

pub extern "C" fn shim_gradient_release(g: u64) {
    let mut guard = GRADIENTS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(m) = guard.as_mut() {
        m.remove(&g);
    }
}

/// CGContextDrawLinearGradient(ctx, gradient, startPoint, endPoint, options)
///
/// On ARM64: ctx in x0, gradient in x1, startPoint.x in d0, startPoint.y in d1,
/// endPoint.x in d2, endPoint.y in d3, options in x2.
pub extern "C" fn shim_draw_linear_gradient(
    ctx: u64,
    gradient: u64,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    _options: u32,
) {
    // Read gradient stops under the gradient lock.
    let stops: Vec<(f64, f64, f64, f64, f64)> = {
        let guard = GRADIENTS.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .and_then(|m| m.get(&gradient))
            .map(|g| g.stops.clone())
            .unwrap_or_default()
    };
    if stops.is_empty() {
        return;
    }
    let grad = Gradient { stops };

    bridge::with_context(ctx, |c| {
        // Direction vector of the gradient line.
        let dx = end_x - start_x;
        let dy = end_y - start_y;
        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-12 {
            return;
        }
        // For each pixel, compute projection onto the gradient line.
        let width = c.surface.width() as usize;
        let height = c.surface.height() as usize;
        let pixels = c.surface.pixels_mut();
        for py in 0..height {
            for px in 0..width {
                let fx = px as f64 + 0.5;
                let fy = py as f64 + 0.5;
                // Project pixel onto gradient line.
                let t = ((fx - start_x) * dx + (fy - start_y) * dy) / len_sq;
                let t_clamped = t.clamp(0.0, 1.0);
                let (r, g, b, a) = grad.color_at(t_clamped);
                let pixel = &mut pixels[py * width + px];
                composite_pixel(pixel, r, g, b, a);
            }
        }
    });
}

/// CGContextDrawRadialGradient — simplified approximation.
pub extern "C" fn shim_draw_radial_gradient(
    ctx: u64,
    gradient: u64,
    start_x: f64,
    start_y: f64,
    start_radius: f64,
    end_x: f64,
    end_y: f64,
    end_radius: f64,
    _options: u32,
) {
    let stops: Vec<(f64, f64, f64, f64, f64)> = {
        let guard = GRADIENTS.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .and_then(|m| m.get(&gradient))
            .map(|g| g.stops.clone())
            .unwrap_or_default()
    };
    if stops.is_empty() {
        return;
    }
    let grad = Gradient { stops };

    bridge::with_context(ctx, |c| {
        let width = c.surface.width() as usize;
        let height = c.surface.height() as usize;
        let pixels = c.surface.pixels_mut();
        for py in 0..height {
            for px in 0..width {
                let fx = px as f64 + 0.5;
                let fy = py as f64 + 0.5;
                let t = radial_gradient_t(
                    (fx, fy),
                    (start_x, start_y),
                    start_radius,
                    (end_x, end_y),
                    end_radius,
                );
                let (r, g, b, a) = grad.color_at(t.clamp(0.0, 1.0));
                composite_pixel(&mut pixels[py * width + px], r, g, b, a);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsystem_shims::table::ShimTable;

    fn pixel_rgba(pixel: u32) -> (u8, u8, u8, u8) {
        (
            ((pixel >> 16) & 0xFF) as u8,
            ((pixel >> 8) & 0xFF) as u8,
            (pixel & 0xFF) as u8,
            ((pixel >> 24) & 0xFF) as u8,
        )
    }

    #[test]
    fn gradient_stubs_registered() {
        let mut table = ShimTable::new();
        install(&mut table);
        assert!(table
            .resolve("_CGGradientCreateWithColorComponents")
            .is_some());
        assert!(table.resolve("_CGContextDrawLinearGradient").is_some());
        assert!(table.resolve("_CGContextDrawRadialGradient").is_some());
    }

    #[test]
    fn gradient_create_and_release() {
        let components: [f64; 8] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0];
        let locations: [f64; 2] = [0.0, 1.0];
        let h = shim_gradient_create(0, components.as_ptr() as u64, locations.as_ptr() as u64, 2);
        assert_ne!(h, 0);
        shim_gradient_release(h);
    }

    #[test]
    fn gradient_color_interpolation() {
        let stops = vec![
            (0.0, 1.0, 0.0, 0.0, 1.0), // red at 0
            (1.0, 0.0, 0.0, 1.0, 1.0), // blue at 1
        ];
        let g = Gradient { stops };
        let (r, _, b, _) = g.color_at(0.5);
        assert!((r - 0.5).abs() < 1e-6);
        assert!((b - 0.5).abs() < 1e-6);
    }

    #[test]
    fn radial_gradient_renders_center_and_edge_colors() {
        let components: [f64; 8] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0];
        let locations: [f64; 2] = [0.0, 1.0];
        let gradient =
            shim_gradient_create(0, components.as_ptr() as u64, locations.as_ptr() as u64, 2);
        let ctx = bridge::context_create(9, 9);

        shim_draw_radial_gradient(ctx, gradient, 4.5, 4.5, 0.0, 4.5, 4.5, 4.0, 0);

        let (center, edge) = bridge::with_context(ctx, |c| {
            let pixels = c.surface.pixels();
            (pixels[4 * 9 + 4], pixels[0])
        })
        .expect("context should exist");
        let (center_r, _, center_b, center_a) = pixel_rgba(center);
        let (edge_r, _, edge_b, edge_a) = pixel_rgba(edge);

        assert!(center_r > 200, "center should stay near start color red");
        assert!(
            center_b < 80,
            "center should not drift strongly toward end blue"
        );
        assert!(edge_b > 200, "edge should reach end color blue");
        assert!(edge_r < 80, "edge should no longer be strongly red");
        assert_eq!(center_a, 255);
        assert_eq!(edge_a, 255);

        bridge::context_release(ctx);
        shim_gradient_release(gradient);
    }
}
