// ── Color ──────────────────────────────────────────────────────────

/// An RGBA color with components in 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const GRAY: Color = Color {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    };
    pub const LIGHT_GRAY: Color = Color {
        r: 0.75,
        g: 0.75,
        b: 0.75,
        a: 1.0,
    };
    pub const DARK_GRAY: Color = Color {
        r: 0.25,
        g: 0.25,
        b: 0.25,
        a: 1.0,
    };

    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Convert to 32-bit ARGB pixel value.
    pub fn to_argb32(&self) -> u32 {
        let a = (self.a.clamp(0.0, 1.0) * 255.0) as u32;
        let r = (self.r.clamp(0.0, 1.0) * 255.0) as u32;
        let g = (self.g.clamp(0.0, 1.0) * 255.0) as u32;
        let b = (self.b.clamp(0.0, 1.0) * 255.0) as u32;
        (a << 24) | (r << 16) | (g << 8) | b
    }

    /// Create from a 32-bit ARGB pixel value.
    pub fn from_argb32(argb: u32) -> Self {
        let a = ((argb >> 24) & 0xFF) as f64 / 255.0;
        let r = ((argb >> 16) & 0xFF) as f64 / 255.0;
        let g = ((argb >> 8) & 0xFF) as f64 / 255.0;
        let b = (argb & 0xFF) as f64 / 255.0;
        Self { r, g, b, a }
    }

    /// Return a copy with the given alpha value.
    pub const fn with_alpha(self, a: f64) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
    }

    /// Linearly interpolate between two colors.
    pub fn lerp(self, other: Color, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Create from HSV (hue 0–360, saturation 0–1, value 0–1).
    pub fn from_hsv(h: f64, s: f64, v: f64) -> Self {
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);
        let h = ((h % 360.0) + 360.0) % 360.0;
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        let (r1, g1, b1) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        Self {
            r: r1 + m,
            g: g1 + m,
            b: b1 + m,
            a: 1.0,
        }
    }

    /// Convert to HSV (hue 0–360, saturation 0–1, value 0–1).
    pub fn to_hsv(&self) -> (f64, f64, f64) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let delta = max - min;
        let v = max;
        let s = if max == 0.0 { 0.0 } else { delta / max };
        let h = if delta == 0.0 {
            0.0
        } else if max == self.r {
            60.0 * (((self.g - self.b) / delta) % 6.0)
        } else if max == self.g {
            60.0 * (((self.b - self.r) / delta) + 2.0)
        } else {
            60.0 * (((self.r - self.g) / delta) + 4.0)
        };
        let h = ((h % 360.0) + 360.0) % 360.0;
        (h, s, v)
    }

    /// Return a lighter version (mix with white by factor 0–1).
    pub fn lighter(&self, factor: f64) -> Self {
        self.lerp(Color::WHITE, factor.clamp(0.0, 1.0))
    }

    /// Return a darker version (mix with black by factor 0–1).
    pub fn darker(&self, factor: f64) -> Self {
        self.lerp(Color::BLACK, factor.clamp(0.0, 1.0))
    }

    /// Format as hex string "#RRGGBB" or "#RRGGBBAA" if alpha < 1.0.
    pub fn to_hex(&self) -> String {
        let r = (self.r.clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (self.g.clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (self.b.clamp(0.0, 1.0) * 255.0).round() as u8;
        if self.a >= 1.0 {
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        } else {
            let a = (self.a.clamp(0.0, 1.0) * 255.0).round() as u8;
            format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
        }
    }

    /// Parse from hex string "#RGB", "#RRGGBB", or "#RRGGBBAA".
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#')?;
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                Some(Self {
                    r: (r * 17) as f64 / 255.0,
                    g: (g * 17) as f64 / 255.0,
                    b: (b * 17) as f64 / 255.0,
                    a: 1.0,
                })
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self {
                    r: r as f64 / 255.0,
                    g: g as f64 / 255.0,
                    b: b as f64 / 255.0,
                    a: 1.0,
                })
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self {
                    r: r as f64 / 255.0,
                    g: g as f64 / 255.0,
                    b: b as f64 / 255.0,
                    a: a as f64 / 255.0,
                })
            }
            _ => None,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01
    }

    fn color_approx_eq(a: &Color, b: &Color) -> bool {
        approx_eq(a.r, b.r) && approx_eq(a.g, b.g) && approx_eq(a.b, b.b) && approx_eq(a.a, b.a)
    }

    #[test]
    fn from_hsv_red() {
        let c = Color::from_hsv(0.0, 1.0, 1.0);
        assert!(
            color_approx_eq(&c, &Color::RED),
            "expected RED, got {:?}",
            c
        );
    }

    #[test]
    fn from_hsv_green() {
        let c = Color::from_hsv(120.0, 1.0, 1.0);
        assert!(
            color_approx_eq(&c, &Color::GREEN),
            "expected GREEN, got {:?}",
            c
        );
    }

    #[test]
    fn from_hsv_blue() {
        let c = Color::from_hsv(240.0, 1.0, 1.0);
        assert!(
            color_approx_eq(&c, &Color::BLUE),
            "expected BLUE, got {:?}",
            c
        );
    }

    #[test]
    fn from_hsv_white() {
        let c = Color::from_hsv(0.0, 0.0, 1.0);
        assert!(
            color_approx_eq(&c, &Color::WHITE),
            "expected WHITE, got {:?}",
            c
        );
    }

    #[test]
    fn from_hsv_black() {
        let c = Color::from_hsv(0.0, 0.0, 0.0);
        assert!(
            color_approx_eq(&c, &Color::BLACK),
            "expected BLACK, got {:?}",
            c
        );
    }

    #[test]
    fn to_hsv_round_trip() {
        let original = Color::rgb(0.8, 0.3, 0.6);
        let (h, s, v) = original.to_hsv();
        let reconstructed = Color::from_hsv(h, s, v);
        assert!(
            color_approx_eq(&original, &reconstructed),
            "round-trip failed: {:?} -> ({}, {}, {}) -> {:?}",
            original,
            h,
            s,
            v,
            reconstructed
        );
    }

    #[test]
    fn lighter_white() {
        let c = Color::WHITE.lighter(0.5);
        assert!(
            color_approx_eq(&c, &Color::WHITE),
            "WHITE.lighter should remain WHITE"
        );
    }

    #[test]
    fn darker_black() {
        let c = Color::BLACK.darker(0.5);
        assert!(
            color_approx_eq(&c, &Color::BLACK),
            "BLACK.darker should remain BLACK"
        );
    }

    #[test]
    fn lighter_makes_brighter() {
        let c = Color::DARK_GRAY.lighter(0.5);
        assert!(c.r > Color::DARK_GRAY.r, "lighter should increase r");
        assert!(c.g > Color::DARK_GRAY.g, "lighter should increase g");
        assert!(c.b > Color::DARK_GRAY.b, "lighter should increase b");
    }

    #[test]
    fn to_hex_red() {
        assert_eq!(Color::RED.to_hex(), "#FF0000");
    }

    #[test]
    fn to_hex_with_alpha() {
        let hex = Color::RED.with_alpha(0.5).to_hex();
        assert_eq!(hex.len(), 9, "should be #RRGGBBAA format");
        assert!(
            hex.starts_with("#FF0000"),
            "should start with red components"
        );
    }

    #[test]
    fn from_hex_valid() {
        let c = Color::from_hex("#FF0000").unwrap();
        assert!(
            color_approx_eq(&c, &Color::RED),
            "expected RED, got {:?}",
            c
        );
    }

    #[test]
    fn from_hex_short() {
        let c = Color::from_hex("#F00").unwrap();
        assert!(
            color_approx_eq(&c, &Color::RED),
            "expected RED, got {:?}",
            c
        );
    }

    #[test]
    fn from_hex_invalid() {
        assert!(Color::from_hex("garbage").is_none());
        assert!(Color::from_hex("#GG0000").is_none());
        assert!(Color::from_hex("").is_none());
    }

    #[test]
    fn gray_constants() {
        assert!(approx_eq(Color::GRAY.r, 0.5));
        assert!(approx_eq(Color::GRAY.g, 0.5));
        assert!(approx_eq(Color::GRAY.b, 0.5));
        assert!(approx_eq(Color::LIGHT_GRAY.r, 0.75));
        assert!(approx_eq(Color::DARK_GRAY.r, 0.25));
    }
}
