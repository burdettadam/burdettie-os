//! CoreGraphics geometry and CoreAnimation transform types.
//!
//! The geometry definitions live in `core-foundation-lite` and are re-exported
//! here. The CoreAnimation transform matches the 64-bit Darwin C layout.

pub use core_foundation_lite::*;

/// `CATransform3D` — the 16 `CGFloat` fields in Darwin declaration order.
///
/// This 128-byte aggregate is returned indirectly on ARM64 and is passed as a
/// single aggregate argument, rather than as 16 independent floating-point values.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CATransform3D {
    pub m11: CGFloat,
    pub m12: CGFloat,
    pub m13: CGFloat,
    pub m14: CGFloat,
    pub m21: CGFloat,
    pub m22: CGFloat,
    pub m23: CGFloat,
    pub m24: CGFloat,
    pub m31: CGFloat,
    pub m32: CGFloat,
    pub m33: CGFloat,
    pub m34: CGFloat,
    pub m41: CGFloat,
    pub m42: CGFloat,
    pub m43: CGFloat,
    pub m44: CGFloat,
}

impl From<[CGFloat; 16]> for CATransform3D {
    fn from(values: [CGFloat; 16]) -> Self {
        let [m11, m12, m13, m14, m21, m22, m23, m24, m31, m32, m33, m34, m41, m42, m43, m44] =
            values;
        Self {
            m11,
            m12,
            m13,
            m14,
            m21,
            m22,
            m23,
            m24,
            m31,
            m32,
            m33,
            m34,
            m41,
            m42,
            m43,
            m44,
        }
    }
}

impl From<CATransform3D> for [CGFloat; 16] {
    fn from(transform: CATransform3D) -> Self {
        [
            transform.m11,
            transform.m12,
            transform.m13,
            transform.m14,
            transform.m21,
            transform.m22,
            transform.m23,
            transform.m24,
            transform.m31,
            transform.m32,
            transform.m33,
            transform.m34,
            transform.m41,
            transform.m42,
            transform.m43,
            transform.m44,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, offset_of, size_of};

    #[test]
    fn darwin_aggregate_layouts() {
        assert_eq!(size_of::<CGSize>(), 16);
        assert_eq!(align_of::<CGSize>(), 8);
        assert_eq!(offset_of!(CGSize, width), 0);
        assert_eq!(offset_of!(CGSize, height), 8);

        assert_eq!(size_of::<CGRect>(), 32);
        assert_eq!(align_of::<CGRect>(), 8);
        assert_eq!(offset_of!(CGRect, origin), 0);
        assert_eq!(offset_of!(CGRect, size), 16);
        assert_eq!(offset_of!(CGPoint, x), 0);
        assert_eq!(offset_of!(CGPoint, y), 8);

        assert_eq!(size_of::<NSRange>(), 16);
        assert_eq!(align_of::<NSRange>(), 8);
        assert_eq!(offset_of!(NSRange, location), 0);
        assert_eq!(offset_of!(NSRange, length), 8);

        assert_eq!(size_of::<CATransform3D>(), 128);
        assert_eq!(align_of::<CATransform3D>(), 8);
        let offsets = [
            offset_of!(CATransform3D, m11),
            offset_of!(CATransform3D, m12),
            offset_of!(CATransform3D, m13),
            offset_of!(CATransform3D, m14),
            offset_of!(CATransform3D, m21),
            offset_of!(CATransform3D, m22),
            offset_of!(CATransform3D, m23),
            offset_of!(CATransform3D, m24),
            offset_of!(CATransform3D, m31),
            offset_of!(CATransform3D, m32),
            offset_of!(CATransform3D, m33),
            offset_of!(CATransform3D, m34),
            offset_of!(CATransform3D, m41),
            offset_of!(CATransform3D, m42),
            offset_of!(CATransform3D, m43),
            offset_of!(CATransform3D, m44),
        ];
        assert_eq!(
            offsets,
            [0, 8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120]
        );
    }
}
