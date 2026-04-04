use ab_glyph::{Font as AbFont, ScaleFont};

use crate::color::Color;
use crate::font::{TextRun};
use crate::font_manager::{FontFace, FontManager};
use crate::unicode::{is_combining_mark, is_zero_width, normalize_nfc};

use super::types::LayoutGlyph;

// ── Attributed text layout ─────────────────────────────────────────

/// Result of attributed text layout: positioned glyphs with per-glyph
/// font and color information, plus the font faces used (indexed by
/// `LayoutGlyph::font_index`).
pub struct AttributedLayout<'a> {
    /// Positioned glyphs with font_index and optional per-glyph color.
    pub glyphs: Vec<LayoutGlyph>,
    /// Font faces referenced by `font_index`.
    pub fonts: Vec<&'a FontFace>,
    /// Total layout width.
    pub width: f64,
    /// Total layout height.
    pub height: f64,
}

/// Layout a single line of attributed text.
///
/// Each `TextRun` specifies a byte range in `text` and optional overrides
/// for font, size, color, bold, and italic. The default font face and size
/// are used where a run doesn't override them.
///
/// Returns an `AttributedLayout` whose glyphs carry per-glyph font and
/// color info so the renderer can switch styles mid-line.
pub fn layout_attributed<'a>(
    text: &str,
    runs: &[TextRun],
    fm: &'a FontManager,
    default_font: &'a FontFace,
    default_size: f64,
) -> AttributedLayout<'a> {
    let normalized = normalize_nfc(text);

    // Build a deduplicated list of font faces used and map run → font_index.
    let mut font_list: Vec<&FontFace> = vec![default_font];
    let mut run_font_indices: Vec<usize> = Vec::with_capacity(runs.len());
    let mut run_sizes: Vec<f64> = Vec::with_capacity(runs.len());

    for run in runs {
        let face = resolve_run_font(run, fm, default_font);
        let idx = font_list.iter().position(|f| std::ptr::eq(*f, face))
            .unwrap_or_else(|| {
                font_list.push(face);
                font_list.len() - 1
            });
        run_font_indices.push(idx);
        run_sizes.push(run.size.unwrap_or(default_size));
    }

    // Layout: iterate through chars, find the active run for each byte offset.
    let mut glyphs = Vec::new();
    let mut cursor_x = 0.0f64;
    let mut prev_glyph: Option<(ab_glyph::GlyphId, usize)> = None; // (glyph_id, font_index)
    let mut last_base_x = 0.0f64;
    let mut max_ascent = 0.0f64;
    let mut max_line_height = 0.0f64;

    // Precompute ascent/line_height for each run.
    for (i, _run) in runs.iter().enumerate() {
        let face = font_list[run_font_indices[i]];
        let sz = run_sizes[i];
        let m = face.metrics(sz);
        if m.ascent > max_ascent {
            max_ascent = m.ascent;
        }
        let lh = m.line_height();
        if lh > max_line_height {
            max_line_height = lh;
        }
    }

    // If no runs, fall back to default metrics.
    if runs.is_empty() {
        let m = default_font.metrics(default_size);
        max_ascent = m.ascent;
        max_line_height = m.line_height();
    }

    let mut byte_offset = 0usize;
    for ch in normalized.chars() {
        let ch_len = ch.len_utf8();

        if is_zero_width(ch) {
            byte_offset += ch_len;
            continue;
        }

        // Find active run for this byte offset.
        let (font_index, size, color) = find_run_for_offset(
            byte_offset, runs, &run_font_indices, &run_sizes, default_size,
        );

        let face = font_list[font_index];
        let ab_font = face.inner();
        let scaled = ab_font.as_scaled(size as f32);
        let glyph_id = ab_font.glyph_id(ch);
        let missing = glyph_id == ab_glyph::GlyphId(0);

        if is_combining_mark(ch) {
            glyphs.push(LayoutGlyph {
                glyph_id,
                x: last_base_x,
                y: max_ascent,
                size,
                missing,
                font_index,
                color,
            });
        } else {
            // Kerning only within same font
            if let Some((prev_id, prev_fi)) = prev_glyph {
                if prev_fi == font_index {
                    cursor_x += scaled.kern(prev_id, glyph_id) as f64;
                }
            }
            last_base_x = cursor_x;
            glyphs.push(LayoutGlyph {
                glyph_id,
                x: cursor_x,
                y: max_ascent,
                size,
                missing,
                font_index,
                color,
            });
            cursor_x += scaled.h_advance(glyph_id) as f64;
            prev_glyph = Some((glyph_id, font_index));
        }

        byte_offset += ch_len;
    }

    AttributedLayout {
        glyphs,
        fonts: font_list,
        width: cursor_x,
        height: max_line_height,
    }
}

/// Resolve the font face for a text run based on its bold/italic flags.
fn resolve_run_font<'a>(
    run: &TextRun,
    fm: &'a FontManager,
    default: &'a FontFace,
) -> &'a FontFace {
    if run.bold && run.italic {
        fm.get("System-BoldItalic")
    } else if run.bold {
        fm.get("System-Bold")
    } else if run.italic {
        fm.get("System-Italic")
    } else if let Some(ref name) = run.font_name {
        fm.get(name)
    } else {
        default
    }
}

/// Find the active run for a given byte offset, returning (font_index, size, color).
fn find_run_for_offset(
    byte_offset: usize,
    runs: &[TextRun],
    run_font_indices: &[usize],
    run_sizes: &[f64],
    default_size: f64,
) -> (usize, f64, Option<Color>) {
    for (i, run) in runs.iter().enumerate() {
        if byte_offset >= run.range.start && byte_offset < run.range.end {
            return (run_font_indices[i], run_sizes[i], run.color);
        }
    }
    (0, default_size, None)
}
