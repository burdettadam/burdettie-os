use ab_glyph::{Font as AbFont, ScaleFont};

use crate::font_manager::FontFace;
use crate::unicode::{is_combining_mark, is_zero_width, normalize_nfc};

use super::helpers::{layout_line_char_wrap, layout_line_no_wrap, layout_line_word_wrap};
use super::types::{LayoutGlyph, TextLayout, WrapMode};

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
                font_index: 0,
                color: None,
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
                font_index: 0,
                color: None,
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
