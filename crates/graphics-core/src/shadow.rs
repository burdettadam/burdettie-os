//! Drop-shadow specification.

use crate::color::Color;

/// A drop shadow applied to drawing operations.
#[derive(Debug, Clone, Copy)]
pub struct Shadow {
    /// Horizontal offset (positive = right).
    pub offset_x: f64,
    /// Vertical offset (positive = down).
    pub offset_y: f64,
    /// Gaussian blur radius.
    pub blur_radius: f64,
    /// Shadow color (typically semi-transparent black).
    pub color: Color,
}

impl Shadow {
    pub const fn new(offset_x: f64, offset_y: f64, blur_radius: f64, color: Color) -> Self {
        Self {
            offset_x,
            offset_y,
            blur_radius,
            color,
        }
    }

    /// A standard macOS-style drop shadow.
    pub fn standard() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 3.0,
            blur_radius: 6.0,
            color: Color::new(0.0, 0.0, 0.0, 0.3),
        }
    }
}
