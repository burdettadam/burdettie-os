use ab_glyph::{Font as AbFont, ScaleFont};

use crate::unicode::{is_combining_mark, is_zero_width};

use super::types::LayoutGlyph;

/// Word-wrap a single paragraph (no embedded newlines).
///
/// Handles combining marks (zero advance), zero-width chars (skipped),
/// and tracks missing glyphs.
#[allow(clippy::too_many_arguments)]
pub(super) fn layout_line_word_wrap(
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
                font_index: 0,
                color: None,
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
                    font_index: 0,
                    color: None,
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
                    font_index: 0,
                    color: None,
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
pub(super) fn layout_line_char_wrap(
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
                font_index: 0,
                color: None,
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
            font_index: 0,
            color: None,
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
pub(super) fn layout_line_no_wrap(
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
                font_index: 0,
                color: None,
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
            font_index: 0,
            color: None,
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
pub(super) fn measure_str(
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
