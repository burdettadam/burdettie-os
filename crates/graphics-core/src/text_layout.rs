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

use ab_glyph::{Font as AbFont, ScaleFont};

use super::font_manager::FontFace;
use super::unicode::{is_combining_mark, is_zero_width, normalize_nfc};

/// A positioned glyph ready for rasterization.
#[derive(Debug, Clone)]
pub struct LayoutGlyph {
    /// The ab_glyph glyph ID.
    pub glyph_id: ab_glyph::GlyphId,
    /// X position (left edge of glyph bounding box) in pixels.
    pub x: f64,
    /// Y position (top edge of glyph bounding box) in pixels.
    pub y: f64,
    /// The font size this glyph was laid out at.
    pub size: f64,
    /// `true` if this glyph is not present in the font (`.notdef` / GlyphId 0).
    /// The renderer should draw a replacement placeholder (tofu box).
    pub missing: bool,
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

/// Layout a single line of text (no wrapping).
///
/// The input is NFC-normalized before layout. Combining marks are positioned
/// over the preceding base character. Zero-width formatting characters are skipped.
pub fn layout_single_line(text: &str, font: &FontFace, size: f64) -> TextLayout {
    let normalized = normalize_nfc(text);
    let ab_font = font.inner();
    let scaled = ab_font.as_scaled(size as f32);
    let ascent = scaled.ascent() as f64;

    let mut glyphs = Vec::new();
    let mut cursor_x = 0.0f64;
    let mut prev_glyph: Option<ab_glyph::GlyphId> = None;
    let mut last_base_x = 0.0f64;

    for ch in normalized.chars() {
        if is_zero_width(ch) {
            continue;
        }

        let glyph_id = ab_font.glyph_id(ch);
        let missing = glyph_id == ab_glyph::GlyphId(0);

        if is_combining_mark(ch) {
            // Position combining mark over the last base character (no advance)
            glyphs.push(LayoutGlyph {
                glyph_id,
                x: last_base_x,
                y: ascent,
                size,
                missing,
            });
            // Don't advance cursor or update prev_glyph for combining marks
        } else {
            // Apply kerning
            if let Some(prev) = prev_glyph {
                cursor_x += scaled.kern(prev, glyph_id) as f64;
            }

            last_base_x = cursor_x;
            glyphs.push(LayoutGlyph {
                glyph_id,
                x: cursor_x,
                y: ascent,
                size,
                missing,
            });

            cursor_x += scaled.h_advance(glyph_id) as f64;
            prev_glyph = Some(glyph_id);
        }
    }

    let metrics = font.metrics(size);
    TextLayout {
        glyphs,
        width: cursor_x,
        height: metrics.line_height(),
        line_count: 1,
    }
}

/// Layout multi-line text with word wrapping within a given width.
///
/// The input is NFC-normalized before layout.
pub fn layout_wrapped(
    text: &str,
    font: &FontFace,
    size: f64,
    max_width: f64,
    wrap: WrapMode,
) -> TextLayout {
    let normalized = normalize_nfc(text);
    let ab_font = font.inner();
    let scaled = ab_font.as_scaled(size as f32);
    let metrics = font.metrics(size);
    let line_height = metrics.line_height();
    let ascent = scaled.ascent() as f64;

    let mut glyphs = Vec::new();
    let mut line = 0usize;
    let mut max_line_width = 0.0f64;

    // Split into paragraphs (respect explicit newlines)
    for text_line in normalized.split('\n') {
        if wrap == WrapMode::NoWrap || max_width <= 0.0 {
            // No wrapping — lay out the entire line without breaking
            layout_line_no_wrap(
                text_line,
                ab_font,
                &scaled,
                size,
                ascent,
                line_height,
                &mut glyphs,
                &mut line,
                &mut max_line_width,
            );
        } else if wrap == WrapMode::Word {
            layout_line_word_wrap(
                text_line,
                ab_font,
                &scaled,
                size,
                ascent,
                line_height,
                max_width,
                &mut glyphs,
                &mut line,
                &mut max_line_width,
            );
        } else {
            layout_line_char_wrap(
                text_line,
                ab_font,
                &scaled,
                size,
                ascent,
                line_height,
                max_width,
                &mut glyphs,
                &mut line,
                &mut max_line_width,
            );
        }
    }

    TextLayout {
        glyphs,
        width: max_line_width,
        height: line as f64 * line_height,
        line_count: line,
    }
}

/// Word-wrap a single paragraph (no embedded newlines).
///
/// Handles combining marks (zero advance), zero-width chars (skipped),
/// and tracks missing glyphs.
#[allow(clippy::too_many_arguments)]
fn layout_line_word_wrap(
    text: &str,
    ab_font: &ab_glyph::FontVec,
    scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontVec>,
    size: f64,
    ascent: f64,
    line_height: f64,
    max_width: f64,
    glyphs: &mut Vec<LayoutGlyph>,
    line: &mut usize,
    max_line_width: &mut f64,
) {
    let words: Vec<&str> = text.split(' ').collect();
    let mut cursor_x = 0.0f64;
    let mut prev_glyph: Option<ab_glyph::GlyphId> = None;
    let mut last_base_x = 0.0f64;
    let mut first_word_on_line = true;

    for word in &words {
        // Measure the word (including preceding space if not first on line)
        let prefix = if first_word_on_line { "" } else { " " };
        let to_measure = format!("{prefix}{word}");
        let word_width = measure_str(&to_measure, ab_font, scaled);

        // Would this word overflow?
        if !first_word_on_line && cursor_x + word_width > max_width {
            if cursor_x > *max_line_width {
                *max_line_width = cursor_x;
            }
            *line += 1;
            cursor_x = 0.0;
            prev_glyph = None;
            first_word_on_line = true;
        }

        // Emit space before word (if not first on line)
        if !first_word_on_line {
            let space_id = ab_font.glyph_id(' ');
            if let Some(prev) = prev_glyph {
                cursor_x += scaled.kern(prev, space_id) as f64;
            }
            last_base_x = cursor_x;
            glyphs.push(LayoutGlyph {
                glyph_id: space_id,
                x: cursor_x,
                y: ascent + *line as f64 * line_height,
                size,
                missing: false,
            });
            cursor_x += scaled.h_advance(space_id) as f64;
            prev_glyph = Some(space_id);
        }

        // Emit each char of the word
        for ch in word.chars() {
            if is_zero_width(ch) {
                continue;
            }
            let glyph_id = ab_font.glyph_id(ch);
            let missing = glyph_id == ab_glyph::GlyphId(0);

            if is_combining_mark(ch) {
                glyphs.push(LayoutGlyph {
                    glyph_id,
                    x: last_base_x,
                    y: ascent + *line as f64 * line_height,
                    size,
                    missing,
                });
            } else {
                if let Some(prev) = prev_glyph {
                    cursor_x += scaled.kern(prev, glyph_id) as f64;
                }
                last_base_x = cursor_x;
                glyphs.push(LayoutGlyph {
                    glyph_id,
                    x: cursor_x,
                    y: ascent + *line as f64 * line_height,
                    size,
                    missing,
                });
                cursor_x += scaled.h_advance(glyph_id) as f64;
                prev_glyph = Some(glyph_id);
            }
        }

        first_word_on_line = false;
    }

    // Commit final line
    if cursor_x > *max_line_width {
        *max_line_width = cursor_x;
    }
    *line += 1;
}

/// Character-wrap a single paragraph (break anywhere).
///
/// Never breaks in the middle of a combining mark sequence.
#[allow(clippy::too_many_arguments)]
fn layout_line_char_wrap(
    text: &str,
    ab_font: &ab_glyph::FontVec,
    scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontVec>,
    size: f64,
    ascent: f64,
    line_height: f64,
    max_width: f64,
    glyphs: &mut Vec<LayoutGlyph>,
    line: &mut usize,
    max_line_width: &mut f64,
) {
    let mut cursor_x = 0.0f64;
    let mut prev_glyph: Option<ab_glyph::GlyphId> = None;
    let mut last_base_x = 0.0f64;

    for ch in text.chars() {
        if is_zero_width(ch) {
            continue;
        }

        let glyph_id = ab_font.glyph_id(ch);
        let missing = glyph_id == ab_glyph::GlyphId(0);

        if is_combining_mark(ch) {
            // Combining marks don't advance and can't trigger a line break
            glyphs.push(LayoutGlyph {
                glyph_id,
                x: last_base_x,
                y: ascent + *line as f64 * line_height,
                size,
                missing,
            });
            continue;
        }

        let advance = scaled.h_advance(glyph_id) as f64;
        let kern = prev_glyph
            .map(|prev| scaled.kern(prev, glyph_id) as f64)
            .unwrap_or(0.0);

        if cursor_x + kern + advance > max_width && cursor_x > 0.0 {
            if cursor_x > *max_line_width {
                *max_line_width = cursor_x;
            }
            *line += 1;
            cursor_x = 0.0;
            prev_glyph = None;
        }

        if let Some(prev) = prev_glyph {
            cursor_x += scaled.kern(prev, glyph_id) as f64;
        }

        last_base_x = cursor_x;
        glyphs.push(LayoutGlyph {
            glyph_id,
            x: cursor_x,
            y: ascent + *line as f64 * line_height,
            size,
            missing,
        });

        cursor_x += advance;
        prev_glyph = Some(glyph_id);
    }

    if cursor_x > *max_line_width {
        *max_line_width = cursor_x;
    }
    *line += 1;
}

/// Lay out a single line without any wrapping.
#[allow(clippy::too_many_arguments)]
fn layout_line_no_wrap(
    text: &str,
    ab_font: &ab_glyph::FontVec,
    scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontVec>,
    size: f64,
    ascent: f64,
    line_height: f64,
    glyphs: &mut Vec<LayoutGlyph>,
    line: &mut usize,
    max_line_width: &mut f64,
) {
    let mut cursor_x = 0.0f64;
    let mut prev_glyph: Option<ab_glyph::GlyphId> = None;
    let mut last_base_x = 0.0f64;

    for ch in text.chars() {
        if is_zero_width(ch) {
            continue;
        }

        let glyph_id = ab_font.glyph_id(ch);
        let missing = glyph_id == ab_glyph::GlyphId(0);

        if is_combining_mark(ch) {
            glyphs.push(LayoutGlyph {
                glyph_id,
                x: last_base_x,
                y: ascent + *line as f64 * line_height,
                size,
                missing,
            });
            continue;
        }

        if let Some(prev) = prev_glyph {
            cursor_x += scaled.kern(prev, glyph_id) as f64;
        }
        last_base_x = cursor_x;
        glyphs.push(LayoutGlyph {
            glyph_id,
            x: cursor_x,
            y: ascent + *line as f64 * line_height,
            size,
            missing,
        });
        cursor_x += scaled.h_advance(glyph_id) as f64;
        prev_glyph = Some(glyph_id);
    }

    if cursor_x > *max_line_width {
        *max_line_width = cursor_x;
    }
    *line += 1;
}

/// Measure a string's advance width, accounting for combining marks and
/// zero-width characters.
fn measure_str(
    text: &str,
    ab_font: &ab_glyph::FontVec,
    scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontVec>,
) -> f64 {
    let mut width = 0.0f32;
    let mut prev: Option<ab_glyph::GlyphId> = None;
    for ch in text.chars() {
        if is_zero_width(ch) || is_combining_mark(ch) {
            continue;
        }
        let gid = ab_font.glyph_id(ch);
        if let Some(p) = prev {
            width += scaled.kern(p, gid);
        }
        width += scaled.h_advance(gid);
        prev = Some(gid);
    }
    width as f64
}
