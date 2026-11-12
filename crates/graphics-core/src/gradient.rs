//! Gradient fills: linear and radial.

use crate::color::Color;
use crate::geometry::Point;

/// A color stop in a gradient.
#[derive(Debug, Clone, Copy)]
pub struct GradientStop {
    /// Position along the gradient axis (0.0 = start, 1.0 = end).
    pub offset: f64,
    /// Color at this stop.
    pub color: Color,
}

impl GradientStop {
    pub const fn new(offset: f64, color: Color) -> Self {
        Self { offset, color }
    }
}

/// A gradient definition.
#[derive(Debug, Clone)]
pub enum Gradient {
    /// Linear gradient between two points.
    Linear {
        start: Point,
        end: Point,
        stops: Vec<GradientStop>,
    },
    /// Radial gradient from a center point outward.
    Radial {
        center: Point,
        radius: f64,
        stops: Vec<GradientStop>,
    },
}

impl Gradient {
    /// Create a linear gradient.
    pub fn linear(start: Point, end: Point, stops: Vec<GradientStop>) -> Self {
        Self::Linear { start, end, stops }
    }

    /// Create a radial gradient.
    pub fn radial(center: Point, radius: f64, stops: Vec<GradientStop>) -> Self {
        Self::Radial {
            center,
            radius,
            stops,
        }
    }

    /// Create a simple two-color linear gradient.
    pub fn linear_two(start: Point, end: Point, from: Color, to: Color) -> Self {
        Self::Linear {
            start,
            end,
            stops: vec![GradientStop::new(0.0, from), GradientStop::new(1.0, to)],
        }
    }

    /// Create a simple two-color radial gradient.
    pub fn radial_two(center: Point, radius: f64, inner: Color, outer: Color) -> Self {
        Self::Radial {
            center,
            radius,
            stops: vec![GradientStop::new(0.0, inner), GradientStop::new(1.0, outer)],
        }
    }

    /// Sample the gradient color at a given parameter `t` (clamped to 0..1).
    pub fn sample(&self, t: f64) -> Color {
        let stops = match self {
            Gradient::Linear { stops, .. } => stops,
            Gradient::Radial { stops, .. } => stops,
        };
        if stops.is_empty() {
            return Color::TRANSPARENT;
        }
        if stops.len() == 1 {
            return stops[0].color;
        }
        let t = t.clamp(0.0, 1.0);
        // Find the two surrounding stops.
        if t <= stops[0].offset {
            return stops[0].color;
        }
        if t >= stops[stops.len() - 1].offset {
            return stops[stops.len() - 1].color;
        }
        for i in 0..stops.len() - 1 {
            let s0 = &stops[i];
            let s1 = &stops[i + 1];
            if t >= s0.offset && t <= s1.offset {
                let range = s1.offset - s0.offset;
                if range <= 0.0 {
                    return s0.color;
                }
                let local_t = (t - s0.offset) / range;
                return s0.color.lerp(s1.color, local_t);
            }
        }
        stops[stops.len() - 1].color
    }

    /// Compute the gradient parameter `t` for a pixel at position `(px, py)`.
    pub fn parameter_at(&self, px: f64, py: f64) -> f64 {
        match self {
            Gradient::Linear { start, end, .. } => {
                let dx = end.x - start.x;
                let dy = end.y - start.y;
                let len_sq = dx * dx + dy * dy;
                if len_sq < 1e-12 {
                    return 0.0;
                }
                let px_off = px - start.x;
                let py_off = py - start.y;
                ((px_off * dx + py_off * dy) / len_sq).clamp(0.0, 1.0)
            }
            Gradient::Radial { center, radius, .. } => {
                if *radius <= 0.0 {
                    return 0.0;
                }
                let dx = px - center.x;
                let dy = py - center.y;
                let dist = (dx * dx + dy * dy).sqrt();
                (dist / radius).clamp(0.0, 1.0)
            }
        }
    }

    /// Sample the color at a specific pixel position.
    pub fn color_at(&self, px: f64, py: f64) -> Color {
        self.sample(self.parameter_at(px, py))
    }
}
