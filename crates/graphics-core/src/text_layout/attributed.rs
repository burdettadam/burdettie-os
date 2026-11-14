use crate::font::TextRun;
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
/// NFC normalization is applied within each style span, after resolving
/// ranges against the original text, so composition cannot shift styles or
/// merge a combining mark into a base character with a different style.
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
    // Build a deduplicated list of font faces used and map run → font_index.
    let mut font_list: Vec<&FontFace> = vec![default_font];
    let mut run_font_indices: Vec<usize> = Vec::with_capacity(runs.len());
    let mut run_sizes: Vec<f64> = Vec::with_capacity(runs.len());

    for run in runs {
        let face = resolve_run_font(run, fm, default_font);
        let idx = font_list
            .iter()
            .position(|f| std::ptr::eq(*f, face))
            .unwrap_or_else(|| {
                font_list.push(face);
                font_list.len() - 1
            });
        run_font_indices.push(idx);
        run_sizes.push(run.size.unwrap_or(default_size));
    }

    // Resolve styles before normalization changes the original byte offsets.
    let spans = styled_spans(text, runs);

    // Layout the normalized characters of each style span.
    let mut glyphs = Vec::new();
    let mut cursor_x = 0.0f64;
    let mut prev_glyph: Option<(crate::GlyphId, usize, f64)> = None;
    let mut last_base_x = 0.0f64;
    let mut max_ascent = 0.0f64;
    let mut max_line_height = 0.0f64;

    // Include default-styled spans; a small explicit run must not clip the
    // larger unstyled text that shares its baseline.
    for (_, run) in &spans {
        let (font_index, sz) = run
            .map(|i| (run_font_indices[i], run_sizes[i]))
            .unwrap_or((0, default_size));
        let face = font_list[font_index];
        let m = face.metrics(sz);
        if m.ascent > max_ascent {
            max_ascent = m.ascent;
        }
        let lh = m.line_height();
        if lh > max_line_height {
            max_line_height = lh;
        }
    }

    if spans.is_empty() {
        let m = default_font.metrics(default_size);
        max_ascent = m.ascent;
        max_line_height = m.line_height();
    }

    for (text_span, run) in spans {
        let (font_index, size, color) = run
            .map(|i| (run_font_indices[i], run_sizes[i], runs[i].color))
            .unwrap_or((0, default_size, None));

        let face = font_list[font_index];
        let font_data = face.inner();
        let scaled = font_data.as_scaled(size as f32);
        for ch in normalize_nfc(text_span).chars() {
            if is_zero_width(ch) {
                continue;
            }
            let glyph_id = font_data.glyph_id(ch);
            let missing = glyph_id == crate::GlyphId(0);

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
                // Kerning only within the same font and scale.
                if let Some((prev_id, prev_fi, prev_size)) = prev_glyph {
                    if prev_fi == font_index && prev_size == size {
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
                prev_glyph = Some((glyph_id, font_index, size));
            }
        }
    }

    AttributedLayout {
        glyphs,
        fonts: font_list,
        width: cursor_x,
        height: max_line_height,
    }
}

/// Resolve the font face for a text run based on its bold/italic flags.
fn resolve_run_font<'a>(run: &TextRun, fm: &'a FontManager, default: &'a FontFace) -> &'a FontFace {
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

/// Partition at character boundaries while preserving first-run precedence.
fn styled_spans<'a>(text: &'a str, runs: &[TextRun]) -> Vec<(&'a str, Option<usize>)> {
    let mut spans = Vec::new();
    let mut start = 0;
    let mut active_run = None;
    for (offset, _) in text.char_indices() {
        let run = runs.iter().position(|run| run.range.contains(&offset));
        if run != active_run {
            if start < offset {
                spans.push((&text[start..offset], active_run));
            }
            start = offset;
            active_run = run;
        }
    }
    if start < text.len() {
        spans.push((&text[start..], active_run));
    }
    spans
}
