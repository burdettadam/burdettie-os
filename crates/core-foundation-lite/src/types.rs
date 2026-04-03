//! CoreGraphics geometry types with C ABI compatibility.
//!
//! These are `#[repr(C)]` structs matching Apple's CoreGraphics layout:
//! - `CGFloat` = `f64` on ARM64 (64-bit)
//! - `CGPoint` = `{ x: CGFloat, y: CGFloat }` — 16 bytes
//! - `CGSize` = `{ width: CGFloat, height: CGFloat }` — 16 bytes
//! - `CGRect` = `{ origin: CGPoint, size: CGSize }` — 32 bytes
//! - `NSRange` = `{ location: u64, length: u64 }` — 16 bytes
//! - `NSEdgeInsets` = `{ top, left, bottom, right: CGFloat }` — 32 bytes
//!
//! On ARM64, structs ≤16 bytes are returned in x0/x1. Structs >16 bytes
//! (CGRect, NSEdgeInsets) use x8 as the indirect result pointer, which
//! is handled by the objc_msgSend trampoline.

/// `CGFloat` — `f64` on 64-bit platforms.
pub type CGFloat = f64;

/// `CGPoint` — 16 bytes, returned in x0/x1 on ARM64.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}

impl CGPoint {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
}

/// `CGSize` — 16 bytes, returned in x0/x1 on ARM64.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CGSize {
    pub width: CGFloat,
    pub height: CGFloat,
}

impl CGSize {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };
}

/// `CGRect` — 32 bytes, uses x8 indirect return on ARM64.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

impl CGRect {
    pub const ZERO: Self = Self {
        origin: CGPoint::ZERO,
        size: CGSize::ZERO,
    };

    pub const fn new(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat) -> Self {
        Self {
            origin: CGPoint { x, y },
            size: CGSize { width, height },
        }
    }
}

/// `NSRange` — 16 bytes, returned in x0/x1 on ARM64.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NSRange {
    pub location: u64,
    pub length: u64,
}

impl NSRange {
    pub const NOT_FOUND: u64 = u64::MAX; // NSNotFound
}

/// `NSEdgeInsets` — 32 bytes, uses x8 indirect return on ARM64.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NSEdgeInsets {
    pub top: CGFloat,
    pub left: CGFloat,
    pub bottom: CGFloat,
    pub right: CGFloat,
}

impl std::fmt::Display for CGPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}, {}}}", self.x, self.y)
    }
}

impl std::fmt::Display for CGSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}, {}}}", self.width, self.height)
    }
}

impl std::fmt::Display for CGRect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{{{{}, {}}}, {{{}, {}}}}}",
            self.origin.x, self.origin.y, self.size.width, self.size.height
        )
    }
}

impl std::fmt::Display for NSRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}, {}}}", self.location, self.length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cg_rect_layout() {
        assert_eq!(std::mem::size_of::<CGPoint>(), 16);
        assert_eq!(std::mem::size_of::<CGSize>(), 16);
        assert_eq!(std::mem::size_of::<CGRect>(), 32);
        assert_eq!(std::mem::size_of::<NSRange>(), 16);
        assert_eq!(std::mem::size_of::<NSEdgeInsets>(), 32);
    }

    #[test]
    fn cg_rect_display() {
        let r = CGRect::new(10.0, 20.0, 100.0, 200.0);
        assert_eq!(format!("{r}"), "{{10, 20}, {100, 200}}");
    }

    #[test]
    fn ns_range_display() {
        let r = NSRange {
            location: 5,
            length: 10,
        };
        assert_eq!(format!("{r}"), "{5, 10}");
    }
}
