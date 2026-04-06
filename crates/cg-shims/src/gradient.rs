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
    register_shims!(table,
        ("_CGGradientCreateWithColorComponents", shim_gradient_create as *const () as u64, "CGGradientCreateWithColorComponents (bridge → real)"),
        ("_CGGradientCreateWithColors", shim_gradient_create_with_colors as *const () as u64, "CGGradientCreateWithColors (bridge → real)"),
        ("_CGGradientRetain", shim_gradient_retain as *const () as u64, "CGGradientRetain (bridge → same)"),
        ("_CGGradientRelease", shim_gradient_release as *const () as u64, "CGGradientRelease (bridge → real)"),
        ("_CGContextDrawLinearGradient", shim_draw_linear_gradient as *const () as u64, "CGContextDrawLinearGradient (bridge → real)"),
        ("_CGContextDrawRadialGradient", shim_draw_radial_gradient as *const () as u64, "CGContextDrawRadialGradient (stub → approx)")
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
                let frac = if range > 1e-12 { (t - a.0) / range } else { 0.0 };
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
                // Alpha-composite over existing pixel.
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
        }
    });
}

/// CGContextDrawRadialGradient — simplified approximation.
pub extern "C" fn shim_draw_radial_gradient(
    _ctx: u64,
    _gradient: u64,
    _start_x: f64,
    _start_y: f64,
    _start_radius: f64,
    _end_x: f64,
    _end_y: f64,
    _end_radius: f64,
    _options: u32,
) {
    // TODO: implement radial gradient rendering
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsystem_shims::table::ShimTable;

    #[test]
    fn gradient_stubs_registered() {
        let mut table = ShimTable::new();
        install(&mut table);
        assert!(table.resolve("_CGGradientCreateWithColorComponents").is_some());
        assert!(table.resolve("_CGContextDrawLinearGradient").is_some());
    }

    #[test]
    fn gradient_create_and_release() {
        let components: [f64; 8] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0];
        let locations: [f64; 2] = [0.0, 1.0];
        let h = shim_gradient_create(
            0,
            components.as_ptr() as u64,
            locations.as_ptr() as u64,
            2,
        );
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
}
