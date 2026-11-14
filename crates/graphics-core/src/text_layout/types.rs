//! Text layout engine — single-line and multi-line layout with word wrapping.
//!
//! Produces a `TextLayout` containing positioned `LayoutGlyph` entries that
//! can be rasterized by the `SoftwareRenderer`.
//!
//! Unicode support:
//! - NFC normalization (combining sequences → precomposed characters)
//! - Combining mark positioning (zero advance, overlay on base character)
//! - Zero-width character filtering (ZWJ, ZWNJ, BOM, etc.)
//! - Missing glyph detection for emoji placeholder rendering

use crate::color::Color;

/// A positioned glyph ready for rasterization.
#[derive(Debug, Clone)]
pub struct LayoutGlyph {
    /// The font-local glyph index.
    pub glyph_id: crate::GlyphId,
    /// X position (left edge of glyph bounding box) in pixels.
    pub x: f64,
    /// Y position (top edge of glyph bounding box) in pixels.
    pub y: f64,
    /// The font size this glyph was laid out at.
    pub size: f64,
    /// `true` if this glyph is not present in the font (`.notdef` / GlyphId 0).
    /// The renderer should draw a replacement placeholder (tofu box).
    pub missing: bool,
    /// Index of the font face used for this glyph (into the `fonts` list of
    /// the `AttributedLayout`). `0` for non-attributed layouts.
    pub font_index: usize,
    /// Per-glyph color override. `None` means use the default color passed
    /// to the draw call.
    pub color: Option<Color>,
}

/// Result of text layout: a list of positioned glyphs plus measurements.
pub struct TextLayout {
    /// Positioned glyphs in draw order.
    pub glyphs: Vec<LayoutGlyph>,
    /// Total width of the laid-out text.
    pub width: f64,
    /// Total height of the laid-out text.
    pub height: f64,
    /// Number of lines.
    pub line_count: usize,
}

/// How to handle text that exceeds the available width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    /// No wrapping — single line, may overflow.
    NoWrap,
    /// Wrap at word boundaries (spaces).
    Word,
    /// Wrap at character boundaries (last resort).
    Character,
}
