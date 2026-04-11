//! Cosmic-text based text system for shaped text layout and rendering.
//!
//! Provides a global `FontSystem` + `SwashCache` pair that uses the same
//! embedded DejaVu fonts as the ab_glyph `FontManager`. Consumers call
//! `draw_shaped_text` to render text onto a `Surface` pixel buffer, or
//! `measure_shaped_text` / `layout_shaped_text` for measurement.

use std::sync::Mutex;

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, SwashCache,
};

pub use cosmic_text::{Action, Cursor, Edit, Editor, Motion, Selection};

use crate::color::Color;
use crate::font_manager::{
    DEJAVU_SANS, DEJAVU_SANS_BOLD, DEJAVU_SANS_BOLD_OBLIQUE, DEJAVU_SANS_MONO,
    DEJAVU_SANS_OBLIQUE,
};

// ── Global singletons ──────────────────────────────────────────

static FONT_SYSTEM: Mutex<Option<FontSystem>> = Mutex::new(None);
static SWASH_CACHE: Mutex<Option<SwashCache>> = Mutex::new(None);

/// Ensure the global `FontSystem` is initialised with our embedded fonts
/// plus any system fonts found on the host.
fn ensure_init() {
    let mut fs_guard = FONT_SYSTEM.lock().unwrap_or_else(|e| e.into_inner());
    if fs_guard.is_some() {
        return;
    }

    // Build a fontdb database with our embedded fonts + system fonts.
    let mut db = cosmic_text::fontdb::Database::new();
    db.load_font_data(DEJAVU_SANS.to_vec());
    db.load_font_data(DEJAVU_SANS_BOLD.to_vec());
    db.load_font_data(DEJAVU_SANS_MONO.to_vec());
    db.load_font_data(DEJAVU_SANS_OBLIQUE.to_vec());
    db.load_font_data(DEJAVU_SANS_BOLD_OBLIQUE.to_vec());
    // Scan system font directories (e.g. /System/Library/Fonts, /usr/share/fonts).
    db.load_system_fonts();

    let font_system = FontSystem::new_with_locale_and_db("en-US".into(), db);
    *fs_guard = Some(font_system);

    let mut sc_guard = SWASH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    *sc_guard = Some(SwashCache::new());
}

/// Access the global font system + swash cache under their respective locks.
/// The callback receives `(&mut FontSystem, &mut SwashCache)`.
fn with_text_system<R>(f: impl FnOnce(&mut FontSystem, &mut SwashCache) -> R) -> R {
    ensure_init();
    let mut fs_guard = FONT_SYSTEM.lock().unwrap_or_else(|e| e.into_inner());
    let mut sc_guard = SWASH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let fs = fs_guard.as_mut().expect("font system initialised");
    let sc = sc_guard.as_mut().expect("swash cache initialised");
    f(fs, sc)
}

// ── Font name mapping ──────────────────────────────────────────

/// Map the font names used by the rest of the codebase to cosmic-text `Attrs`.
///
/// Known system-font aliases map to generic families; arbitrary names use
/// `Family::Name` so cosmic-text can resolve them against loaded system fonts.
fn attrs_for_font<'a>(font_name: &'a str) -> Attrs<'a> {
    match font_name {
        "Monospace" | "DejaVuSansMono" | "Courier" | "Courier New" | "Monaco" => {
            Attrs::new().family(Family::Monospace)
        }
        "System-Bold" | "DejaVuSans-Bold" | ".AppleSystemUIFontBold" => Attrs::new()
            .family(Family::SansSerif)
            .weight(cosmic_text::Weight::BOLD),
        "System-Italic" | "DejaVuSans-Oblique" => Attrs::new()
            .family(Family::SansSerif)
            .style(cosmic_text::Style::Italic),
        "System-BoldItalic" | "DejaVuSans-BoldOblique" => Attrs::new()
            .family(Family::SansSerif)
            .weight(cosmic_text::Weight::BOLD)
            .style(cosmic_text::Style::Italic),
        // System font synonyms fall back to generic SansSerif.
        "System" | ".AppleSystemUIFont" | ".SFUI-Regular"
        | ".AppleSystemUIFontMonospacedDigit" => Attrs::new().family(Family::SansSerif),
        // Arbitrary font names: pass through to cosmic-text for resolution
        // against system fonts loaded in fontdb.
        _ => Attrs::new().family(Family::Name(font_name)),
    }
}

// ── Font enumeration ───────────────────────────────────────────

/// Return a sorted, deduplicated list of all available font family names.
///
/// Includes both our embedded fonts and any system fonts found on the host.
pub fn available_font_families() -> Vec<String> {
    ensure_init();
    let fs_guard = FONT_SYSTEM.lock().unwrap_or_else(|e| e.into_inner());
    let fs = fs_guard.as_ref().expect("font system initialised");
    let mut families: Vec<String> = fs
        .db()
        .faces()
        .flat_map(|face| face.families.iter().map(|(name, _lang)| name.clone()))
        .collect();
    families.sort();
    families.dedup();
    families
}

// ── Public API ─────────────────────────────────────────────────

/// Result of text measurement.
#[derive(Debug, Clone, Copy)]
pub struct ShapedTextMetrics {
    /// Total advance width of text (single-line) or widest line (multi-line).
    pub width: f64,
    /// Total height of all lines.
    pub height: f64,
    /// Number of lines after layout.
    pub line_count: usize,
}

/// Measure text without rendering. With `max_width = None`, assumes infinite
/// width (single line). With `Some(w)`, enables word wrapping.
pub fn measure_shaped_text(
    text: &str,
    font_size: f64,
    font_name: &str,
    max_width: Option<f64>,
) -> ShapedTextMetrics {
    let line_height = (font_size * 1.2).ceil();
    let metrics = Metrics::new(font_size as f32, line_height as f32);
    let attrs = attrs_for_font(font_name);
    let buf_width = max_width.map(|w| w as f32);

    with_text_system(|fs, _sc| {
        let mut buffer = Buffer::new(fs, metrics);
        buffer.set_size(fs, buf_width, None);
        buffer.set_text(fs, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(fs, false);

        let mut max_w: f32 = 0.0;
        let mut line_count = 0usize;
        for run in buffer.layout_runs() {
            if run.line_w > max_w {
                max_w = run.line_w;
            }
            line_count += 1;
        }
        // Ensure at least 1 line for empty text.
        if line_count == 0 {
            line_count = 1;
        }

        ShapedTextMetrics {
            width: max_w as f64,
            height: line_height * line_count as f64,
            line_count,
        }
    })
}

/// Draw shaped text onto a raw ARGB32 pixel buffer.
///
/// `origin_x` / `origin_y` is the top-left corner of the text box.
/// If `max_width` is `Some(w)`, text is word-wrapped to that width.
///
/// Returns the advance width of the last/only line (useful for cursor
/// positioning).
pub fn draw_shaped_text(
    pixels: &mut [u32],
    surface_width: u32,
    surface_height: u32,
    text: &str,
    origin_x: f64,
    origin_y: f64,
    font_size: f64,
    font_name: &str,
    color: Color,
    max_width: Option<f64>,
) -> f64 {
    let line_height = (font_size * 1.2).ceil();
    let metrics = Metrics::new(font_size as f32, line_height as f32);
    let attrs = attrs_for_font(font_name);
    let buf_width = max_width.map(|w| w as f32);

    let cr = (color.r.clamp(0.0, 1.0) * 255.0) as u8;
    let cg = (color.g.clamp(0.0, 1.0) * 255.0) as u8;
    let cb = (color.b.clamp(0.0, 1.0) * 255.0) as u8;
    let ca = (color.a.clamp(0.0, 1.0) * 255.0) as u8;
    let text_color = CosmicColor::rgba(cr, cg, cb, ca);

    with_text_system(|fs, sc| {
        let mut buffer = Buffer::new(fs, metrics);
        buffer.set_size(fs, buf_width, None);
        buffer.set_text(fs, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(fs, false);

        let sw = surface_width as i32;
        let sh = surface_height as i32;
        let ox = origin_x as i32;
        let oy = origin_y as i32;

        let mut last_line_w: f32 = 0.0;

        buffer.draw(fs, sc, text_color, |px, py, _pw, _ph, glyph_color| {
            let x = ox + px;
            let y = oy + py;
            if x < 0 || y < 0 || x >= sw || y >= sh {
                return;
            }

            let alpha = glyph_color.a() as u32;
            if alpha == 0 {
                return;
            }

            let sr = glyph_color.r() as u32;
            let sg = glyph_color.g() as u32;
            let sb = glyph_color.b() as u32;

            let idx = (y * sw + x) as usize;
            let dst = pixels[idx];

            if alpha >= 255 {
                pixels[idx] = (255 << 24) | (sr << 16) | (sg << 8) | sb;
            } else {
                let inv = 255 - alpha;
                let dr = (dst >> 16) & 0xFF;
                let dg_c = (dst >> 8) & 0xFF;
                let db = dst & 0xFF;
                let da = (dst >> 24) & 0xFF;
                let out_r = (sr * alpha + dr * inv) / 255;
                let out_g = (sg * alpha + dg_c * inv) / 255;
                let out_b = (sb * alpha + db * inv) / 255;
                let out_a = alpha + (da * inv) / 255;
                pixels[idx] = (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b;
            }
        });

        // Compute last-line width for cursor positioning.
        for run in buffer.layout_runs() {
            last_line_w = run.line_w;
        }
        last_line_w as f64
    })
}

// ── Editor-backed text editing ─────────────────────────────────
//
// Per-view `Editor` instances stored in a global map, keyed by guest
// NSTextView pointer. The Editor owns its Buffer and handles cursor,
// selection, and editing actions via cosmic-text.

use std::collections::HashMap;

static EDITORS: Mutex<Option<HashMap<u64, EditorState>>> = Mutex::new(None);

struct EditorState {
    editor: Editor<'static>,
    font_size: f32,
    line_height: f32,
}

fn editors_map() -> std::sync::MutexGuard<'static, Option<HashMap<u64, EditorState>>> {
    EDITORS.lock().unwrap_or_else(|e| e.into_inner())
}

fn ensure_editors() {
    let mut guard = editors_map();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
}

/// Create an editor for a text view. If one already exists it is replaced.
pub fn editor_create(view_id: u64, text: &str, font_size: f64, max_width: f64) {
    let fs_val = font_size as f32;
    let lh = (font_size * 1.2).ceil() as f32;
    let metrics = Metrics::new(fs_val, lh);
    let attrs = attrs_for_font("System");

    with_text_system(|fs, _sc| {
        let mut editor = Editor::new(Buffer::new(fs, metrics));
        editor.with_buffer_mut(|buf| {
            buf.set_size(fs, Some(max_width as f32), None);
            buf.set_text(fs, text, &attrs, Shaping::Advanced, None);
            buf.shape_until_scroll(fs, false);
        });
        let state = EditorState {
            editor,
            font_size: fs_val,
            line_height: lh,
        };
        ensure_editors();
        let mut guard = editors_map();
        guard.as_mut().unwrap().insert(view_id, state);
    });
}

/// Remove an editor when its view is deallocated.
pub fn editor_remove(view_id: u64) {
    let mut guard = editors_map();
    if let Some(map) = guard.as_mut() {
        map.remove(&view_id);
    }
}

/// Replace the editor text (e.g. from `setString:`).
pub fn editor_set_text(view_id: u64, text: &str) {
    let attrs = attrs_for_font("System");
    with_text_system(|fs, _sc| {
        let mut guard = editors_map();
        if let Some(state) = guard.as_mut().and_then(|m| m.get_mut(&view_id)) {
            state.editor.with_buffer_mut(|buf| {
                buf.set_text(fs, text, &attrs, Shaping::Advanced, None);
                buf.shape_until_scroll(fs, false);
            });
        }
    });
}

/// Get the current text from the editor's buffer.
pub fn editor_get_text(view_id: u64) -> String {
    let guard = editors_map();
    if let Some(state) = guard.as_ref().and_then(|m| m.get(&view_id)) {
        let mut result = String::new();
        state.editor.with_buffer(|buf| {
            for (i, line) in buf.lines.iter().enumerate() {
                if i > 0 {
                    result.push('\n');
                }
                result.push_str(line.text());
            }
        });
        result
    } else {
        String::new()
    }
}

/// Perform an editing action. Returns true if the editor needs redraw.
pub fn editor_action(view_id: u64, action: Action) -> bool {
    with_text_system(|fs, _sc| {
        let mut guard = editors_map();
        if let Some(state) = guard.as_mut().and_then(|m| m.get_mut(&view_id)) {
            state.editor.action(fs, action);
            state.editor.with_buffer_mut(|buf| buf.shape_until_scroll(fs, false));
            true
        } else {
            false
        }
    })
}

/// Update the editor's wrapping width (e.g. on view resize).
pub fn editor_set_width(view_id: u64, max_width: f64) {
    with_text_system(|fs, _sc| {
        let mut guard = editors_map();
        if let Some(state) = guard.as_mut().and_then(|m| m.get_mut(&view_id)) {
            state.editor.with_buffer_mut(|buf| {
                buf.set_size(fs, Some(max_width as f32), None);
                buf.shape_until_scroll(fs, false);
            });
        }
    });
}

/// Get cursor pixel position (relative to buffer origin).
pub fn editor_cursor_position(view_id: u64) -> Option<(i32, i32)> {
    let guard = editors_map();
    guard
        .as_ref()
        .and_then(|m| m.get(&view_id))
        .and_then(|state| state.editor.cursor_position())
}

/// Get the line height for a view's editor.
pub fn editor_line_height(view_id: u64) -> f32 {
    let guard = editors_map();
    guard
        .as_ref()
        .and_then(|m| m.get(&view_id))
        .map(|s| s.line_height)
        .unwrap_or(17.0)
}

/// Copy the current selection text, if any.
pub fn editor_copy_selection(view_id: u64) -> Option<String> {
    let guard = editors_map();
    guard
        .as_ref()
        .and_then(|m| m.get(&view_id))
        .and_then(|state| state.editor.copy_selection())
}

/// Draw the editor contents with selection highlights and cursor bar.
///
/// `show_cursor` should be true only when the text view is first responder.
pub fn editor_draw(
    view_id: u64,
    pixels: &mut [u32],
    surface_width: u32,
    surface_height: u32,
    origin_x: f64,
    origin_y: f64,
    text_color: Color,
    selection_bg: Color,
    show_cursor: bool,
) {
    let cr = (text_color.r.clamp(0.0, 1.0) * 255.0) as u8;
    let cg = (text_color.g.clamp(0.0, 1.0) * 255.0) as u8;
    let cb = (text_color.b.clamp(0.0, 1.0) * 255.0) as u8;
    let ca = (text_color.a.clamp(0.0, 1.0) * 255.0) as u8;
    let cosmic_color = CosmicColor::rgba(cr, cg, cb, ca);

    with_text_system(|fs, sc| {
        let mut guard = editors_map();
        let state = match guard.as_mut().and_then(|m| m.get_mut(&view_id)) {
            Some(s) => s,
            None => return,
        };
        let line_height = state.line_height;
        let cursor_pos = state.editor.cursor_position();

        // Collect selection bounds for highlighting.
        let sel_bounds = state.editor.selection_bounds();

        let sw = surface_width as i32;
        let sh = surface_height as i32;
        let ox = origin_x as i32;
        let oy = origin_y as i32;

        // Draw selection highlight rectangles.
        if let Some((start, end)) = sel_bounds {
            let sel_argb = color_to_argb32(selection_bg);
            // Walk layout runs to find selected glyph ranges.
            state.editor.with_buffer(|buf| {
                for run in buf.layout_runs() {
                    let run_line = run.line_i;
                    let run_y = oy + run.line_top as i32;
                    let run_h = line_height as i32;

                    // Determine if this run is within the selection.
                    let line_in_sel = (run_line > start.line && run_line < end.line)
                        || (run_line == start.line && run_line == end.line)
                        || run_line == start.line
                        || run_line == end.line;
                    if !line_in_sel {
                        continue;
                    }

                    // Full-line selection for middle lines.
                    if run_line > start.line && run_line < end.line {
                        fill_rect_alpha(pixels, sw, sh, ox, run_y, run.line_w as i32, run_h, sel_argb);
                        continue;
                    }

                    // Partial line: walk glyphs to find pixel bounds.
                    let mut sel_x_min = i32::MAX;
                    let mut sel_x_max = i32::MIN;
                    for glyph in run.glyphs.iter() {
                        let glyph_start = glyph.start;
                        let glyph_end = glyph.end;
                        let in_sel = if start.line == end.line {
                            glyph_start >= start.index && glyph_end <= end.index
                        } else if run_line == start.line {
                            glyph_start >= start.index
                        } else {
                            glyph_end <= end.index
                        };
                        if in_sel {
                            let gx = ox + glyph.x as i32;
                            let gw = glyph.w as i32;
                            sel_x_min = sel_x_min.min(gx);
                            sel_x_max = sel_x_max.max(gx + gw);
                        }
                    }
                    if sel_x_min < sel_x_max {
                        fill_rect_alpha(pixels, sw, sh, sel_x_min, run_y, sel_x_max - sel_x_min, run_h, sel_argb);
                    }
                }
            });
        }

        // Draw text glyphs.
        state.editor.with_buffer_mut(|buf| {
            buf.draw(fs, sc, cosmic_color, |px, py, _pw, _ph, glyph_color| {
                let x = ox + px;
                let y = oy + py;
                if x < 0 || y < 0 || x >= sw || y >= sh {
                    return;
                }
                let alpha = glyph_color.a() as u32;
                if alpha == 0 {
                    return;
                }
                let sr = glyph_color.r() as u32;
                let sg = glyph_color.g() as u32;
                let sb = glyph_color.b() as u32;
                let idx = (y * sw + x) as usize;
                let dst = pixels[idx];
                if alpha >= 255 {
                    pixels[idx] = (255 << 24) | (sr << 16) | (sg << 8) | sb;
                } else {
                    let inv = 255 - alpha;
                    let dr = (dst >> 16) & 0xFF;
                    let dg_c = (dst >> 8) & 0xFF;
                    let db = dst & 0xFF;
                    let da = (dst >> 24) & 0xFF;
                    let out_r = (sr * alpha + dr * inv) / 255;
                    let out_g = (sg * alpha + dg_c * inv) / 255;
                    let out_b = (sb * alpha + db * inv) / 255;
                    let out_a = alpha + (da * inv) / 255;
                    pixels[idx] = (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b;
                }
            });
        });

        // Draw cursor bar.
        if show_cursor {
            if let Some((cx, cy)) = cursor_pos {
                let cursor_x = ox + cx;
                let cursor_y = oy + cy;
                let cursor_h = line_height as i32;
                // 2px wide cursor bar, black.
                fill_rect_alpha(pixels, sw, sh, cursor_x, cursor_y, 2, cursor_h, 0xFF000000);
            }
        }
    });
}

/// Fill a rectangle with alpha blending onto ARGB32 pixels.
fn fill_rect_alpha(pixels: &mut [u32], sw: i32, sh: i32, x: i32, y: i32, w: i32, h: i32, argb: u32) {
    let alpha = (argb >> 24) & 0xFF;
    if alpha == 0 {
        return;
    }
    let sr = (argb >> 16) & 0xFF;
    let sg = (argb >> 8) & 0xFF;
    let sb = argb & 0xFF;

    for py in y.max(0)..((y + h).min(sh)) {
        for px in x.max(0)..((x + w).min(sw)) {
            let idx = (py * sw + px) as usize;
            if alpha >= 255 {
                pixels[idx] = argb;
            } else {
                let inv = 255 - alpha;
                let dst = pixels[idx];
                let dr = (dst >> 16) & 0xFF;
                let dg = (dst >> 8) & 0xFF;
                let db = dst & 0xFF;
                let out_r = (sr * alpha + dr * inv) / 255;
                let out_g = (sg * alpha + dg * inv) / 255;
                let out_b = (sb * alpha + db * inv) / 255;
                pixels[idx] = (255 << 24) | (out_r << 16) | (out_g << 8) | out_b;
            }
        }
    }
}

fn color_to_argb32(c: Color) -> u32 {
    let a = (c.a.clamp(0.0, 1.0) * 255.0) as u32;
    let r = (c.r.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (c.g.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (c.b.clamp(0.0, 1.0) * 255.0) as u32;
    (a << 24) | (r << 16) | (g << 8) | b
}

/// Reset all editors (for runtime reuse).
pub fn reset_editors() {
    let mut guard = editors_map();
    if let Some(map) = guard.as_mut() {
        map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_empty_text() {
        let m = measure_shaped_text("", 14.0, "System", None);
        assert_eq!(m.line_count, 1);
        assert!(m.width < 1.0);
    }

    #[test]
    fn measure_single_line() {
        let m = measure_shaped_text("Hello", 14.0, "System", None);
        assert!(m.width > 10.0, "expected non-trivial width, got {}", m.width);
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn measure_word_wrap() {
        // With a tiny max_width, "Hello World" should wrap to 2+ lines.
        let m = measure_shaped_text("Hello World", 14.0, "System", Some(40.0));
        assert!(m.line_count >= 2, "expected wrapping, got {} lines", m.line_count);
    }

    #[test]
    fn draw_does_not_crash() {
        let mut pixels = vec![0u32; 100 * 50];
        let advance = draw_shaped_text(
            &mut pixels,
            100,
            50,
            "Hi",
            0.0,
            0.0,
            14.0,
            "System",
            Color::BLACK,
            None,
        );
        assert!(advance > 0.0);
        // Some pixels should be non-zero (text was drawn).
        assert!(pixels.iter().any(|&p| p != 0));
    }

    #[test]
    fn draw_monospace() {
        let mut pixels = vec![0u32; 200 * 50];
        let advance = draw_shaped_text(
            &mut pixels,
            200,
            50,
            "Test",
            0.0,
            0.0,
            14.0,
            "Monospace",
            Color::BLACK,
            None,
        );
        assert!(advance > 0.0);
    }

    #[test]
    fn draw_bold() {
        let mut pixels = vec![0u32; 200 * 50];
        let advance = draw_shaped_text(
            &mut pixels,
            200,
            50,
            "Bold",
            0.0,
            0.0,
            14.0,
            "System-Bold",
            Color::BLACK,
            None,
        );
        assert!(advance > 0.0);
    }

    // ── measure_shaped_text ─────────────────────────────────────────

    #[test]
    fn measure_height_single_line() {
        let m = measure_shaped_text("Hello", 14.0, "System", None);
        let expected_line_height = (14.0_f64 * 1.2).ceil();
        assert!(
            (m.height - expected_line_height).abs() < 1.0,
            "height {} should be ~{expected_line_height}",
            m.height
        );
    }

    #[test]
    fn measure_height_scales_with_lines() {
        let single = measure_shaped_text("Hello", 14.0, "System", None);
        let wrapped = measure_shaped_text("Hello World Foo Bar", 14.0, "System", Some(40.0));
        assert!(
            wrapped.height > single.height,
            "wrapped height {} should exceed single-line height {}",
            wrapped.height,
            single.height
        );
    }

    #[test]
    fn measure_wider_text_has_wider_width() {
        let short = measure_shaped_text("Hi", 14.0, "System", None);
        let long = measure_shaped_text("Hello, World!", 14.0, "System", None);
        assert!(
            long.width > short.width,
            "longer text width {} should exceed shorter {}",
            long.width,
            short.width
        );
    }

    #[test]
    fn measure_larger_font_wider() {
        let small = measure_shaped_text("Test", 10.0, "System", None);
        let big = measure_shaped_text("Test", 24.0, "System", None);
        assert!(
            big.width > small.width,
            "bigger font width {} should exceed smaller {}",
            big.width,
            small.width
        );
    }

    #[test]
    fn measure_larger_font_taller() {
        let small = measure_shaped_text("Test", 10.0, "System", None);
        let big = measure_shaped_text("Test", 24.0, "System", None);
        assert!(
            big.height > small.height,
            "bigger font height {} should exceed smaller {}",
            big.height,
            small.height
        );
    }

    #[test]
    fn measure_monospace_font() {
        let m = measure_shaped_text("Hello", 14.0, "Monospace", None);
        assert!(m.width > 0.0);
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn measure_bold_font() {
        let m = measure_shaped_text("Hello", 14.0, "System-Bold", None);
        assert!(m.width > 0.0);
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn measure_italic_font() {
        let m = measure_shaped_text("Hello", 14.0, "System-Italic", None);
        assert!(m.width > 0.0);
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn measure_bold_italic_font() {
        let m = measure_shaped_text("Hello", 14.0, "System-BoldItalic", None);
        assert!(m.width > 0.0);
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn measure_multiline_newlines() {
        let m = measure_shaped_text("Line1\nLine2\nLine3", 14.0, "System", None);
        assert_eq!(m.line_count, 3, "expected 3 lines, got {}", m.line_count);
    }

    #[test]
    fn measure_single_char() {
        let m = measure_shaped_text("X", 14.0, "System", None);
        assert!(m.width > 0.0);
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn measure_spaces_only() {
        let m = measure_shaped_text("     ", 14.0, "System", None);
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn measure_unicode_text() {
        let m = measure_shaped_text("日本語テスト", 14.0, "System", None);
        assert!(m.width > 0.0);
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn measure_emoji_text() {
        let m = measure_shaped_text("Hello 🌍🎉", 14.0, "System", None);
        assert!(m.width > 0.0);
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn measure_wrap_wide_limit_no_wrap() {
        // Very wide limit should not break a short string
        let m = measure_shaped_text("Hello", 14.0, "System", Some(1000.0));
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn measure_wrap_exact_word_boundary() {
        // Measure the width of "Hello" first, then set max_width just below it
        let single = measure_shaped_text("Hello World", 14.0, "System", None);
        let m = measure_shaped_text("Hello World", 14.0, "System", Some(single.width * 0.6));
        assert!(
            m.line_count >= 2,
            "should wrap when max_width is ~60% of full width, got {} lines",
            m.line_count
        );
    }

    #[test]
    fn measure_empty_text_has_zero_width() {
        let m = measure_shaped_text("", 14.0, "System", None);
        assert!(m.width < 0.01, "empty text width should be ~0, got {}", m.width);
    }

    // ── Font name aliases ───────────────────────────────────────────

    #[test]
    fn measure_dejavu_sans_mono_alias() {
        let m = measure_shaped_text("Test", 14.0, "DejaVuSansMono", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_courier_alias() {
        let m = measure_shaped_text("Test", 14.0, "Courier", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_courier_new_alias() {
        let m = measure_shaped_text("Test", 14.0, "Courier New", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_monaco_alias() {
        let m = measure_shaped_text("Test", 14.0, "Monaco", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_dejavu_bold_alias() {
        let m = measure_shaped_text("Test", 14.0, "DejaVuSans-Bold", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_apple_system_bold_alias() {
        let m = measure_shaped_text("Test", 14.0, ".AppleSystemUIFontBold", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_dejavu_oblique_alias() {
        let m = measure_shaped_text("Test", 14.0, "DejaVuSans-Oblique", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_dejavu_bold_oblique_alias() {
        let m = measure_shaped_text("Test", 14.0, "DejaVuSans-BoldOblique", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_apple_system_font_alias() {
        let m = measure_shaped_text("Test", 14.0, ".AppleSystemUIFont", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_sfui_regular_alias() {
        let m = measure_shaped_text("Test", 14.0, ".SFUI-Regular", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_apple_monospaced_digit_alias() {
        let m = measure_shaped_text("Test", 14.0, ".AppleSystemUIFontMonospacedDigit", None);
        assert!(m.width > 0.0);
    }

    #[test]
    fn measure_arbitrary_font_falls_back() {
        // Unknown font name should still produce a measurement (fallback)
        let m = measure_shaped_text("Test", 14.0, "SomeUnknownFont", None);
        assert!(m.width > 0.0);
        assert_eq!(m.line_count, 1);
    }

    // ── draw_shaped_text ────────────────────────────────────────────

    #[test]
    fn draw_italic() {
        let mut pixels = vec![0u32; 200 * 50];
        let advance = draw_shaped_text(
            &mut pixels, 200, 50, "Italic", 0.0, 0.0, 14.0,
            "System-Italic", Color::BLACK, None,
        );
        assert!(advance > 0.0);
    }

    #[test]
    fn draw_bold_italic() {
        let mut pixels = vec![0u32; 200 * 50];
        let advance = draw_shaped_text(
            &mut pixels, 200, 50, "BoldItalic", 0.0, 0.0, 14.0,
            "System-BoldItalic", Color::BLACK, None,
        );
        assert!(advance > 0.0);
    }

    #[test]
    fn draw_empty_text() {
        let mut pixels = vec![0u32; 100 * 50];
        let advance = draw_shaped_text(
            &mut pixels, 100, 50, "", 0.0, 0.0, 14.0,
            "System", Color::BLACK, None,
        );
        // Empty text → zero or near-zero advance
        assert!(advance < 1.0);
        // No pixels should have been drawn
        assert!(pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn draw_with_offset() {
        let mut pixels = vec![0u32; 200 * 100];
        let advance = draw_shaped_text(
            &mut pixels, 200, 100, "Hi", 50.0, 40.0, 14.0,
            "System", Color::BLACK, None,
        );
        assert!(advance > 0.0);
        // Top-left corner should still be clear (text drawn at offset)
        assert_eq!(pixels[0], 0, "pixel at (0,0) should be untouched");
    }

    #[test]
    fn draw_with_color() {
        let mut pixels = vec![0u32; 200 * 50];
        let red = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        let advance = draw_shaped_text(
            &mut pixels, 200, 50, "Red", 0.0, 0.0, 14.0,
            "System", red, None,
        );
        assert!(advance > 0.0);
        // At least one pixel should have a non-zero red channel
        assert!(pixels.iter().any(|&p| (p >> 16) & 0xFF > 0));
    }

    #[test]
    fn draw_white_on_black() {
        let mut pixels = vec![0xFF000000u32; 200 * 50]; // black background
        let advance = draw_shaped_text(
            &mut pixels, 200, 50, "White", 0.0, 0.0, 14.0,
            "System", Color::WHITE, None,
        );
        assert!(advance > 0.0);
        // Some pixels should have changed from pure black
        assert!(pixels.iter().any(|&p| p != 0xFF000000));
    }

    #[test]
    fn draw_with_word_wrap() {
        let mut pixels = vec![0u32; 100 * 100];
        let advance = draw_shaped_text(
            &mut pixels, 100, 100, "Hello World Foo Bar",
            0.0, 0.0, 14.0, "System", Color::BLACK, Some(50.0),
        );
        // Should have drawn something
        assert!(pixels.iter().any(|&p| p != 0));
        // advance is the last line's width
        assert!(advance >= 0.0);
    }

    #[test]
    fn draw_clipped_negative_origin() {
        // Drawing with negative origin — glyphs partially off-screen should not panic
        let mut pixels = vec![0u32; 100 * 50];
        let advance = draw_shaped_text(
            &mut pixels, 100, 50, "Hello", -20.0, -10.0, 14.0,
            "System", Color::BLACK, None,
        );
        assert!(advance > 0.0);
    }

    #[test]
    fn draw_clipped_beyond_surface() {
        // Drawing past the surface edge — should not panic or corrupt memory
        let mut pixels = vec![0u32; 100 * 50];
        let advance = draw_shaped_text(
            &mut pixels, 100, 50, "Hello", 90.0, 45.0, 14.0,
            "System", Color::BLACK, None,
        );
        assert!(advance > 0.0);
    }

    #[test]
    fn draw_zero_alpha_color_does_not_crash() {
        // Fully transparent text — cosmic-text still rasterises glyphs
        // with coverage alpha, but the function must not panic.
        let transparent = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
        let mut pixels = vec![0u32; 200 * 50];
        draw_shaped_text(
            &mut pixels, 200, 50, "Ghost", 0.0, 0.0, 14.0,
            "System", transparent, None,
        );
    }

    #[test]
    fn draw_large_font() {
        let mut pixels = vec![0u32; 400 * 200];
        let advance = draw_shaped_text(
            &mut pixels, 400, 200, "Big", 0.0, 0.0, 48.0,
            "System", Color::BLACK, None,
        );
        assert!(advance > 0.0);
        assert!(pixels.iter().any(|&p| p != 0));
    }

    #[test]
    fn draw_small_font() {
        let mut pixels = vec![0u32; 200 * 50];
        let advance = draw_shaped_text(
            &mut pixels, 200, 50, "tiny", 0.0, 0.0, 6.0,
            "System", Color::BLACK, None,
        );
        assert!(advance > 0.0);
    }

    #[test]
    fn draw_multiline() {
        let mut pixels = vec![0u32; 200 * 100];
        let advance = draw_shaped_text(
            &mut pixels, 200, 100, "Line1\nLine2", 0.0, 0.0, 14.0,
            "System", Color::BLACK, None,
        );
        // advance is the last line's width
        assert!(advance > 0.0);
        assert!(pixels.iter().any(|&p| p != 0));
    }

    #[test]
    fn draw_unicode() {
        let mut pixels = vec![0u32; 300 * 50];
        let advance = draw_shaped_text(
            &mut pixels, 300, 50, "café ñ", 0.0, 0.0, 14.0,
            "System", Color::BLACK, None,
        );
        assert!(advance > 0.0);
    }

    #[test]
    fn draw_advance_increases_with_more_text() {
        let mut pixels1 = vec![0u32; 400 * 50];
        let a1 = draw_shaped_text(
            &mut pixels1, 400, 50, "Hi", 0.0, 0.0, 14.0,
            "System", Color::BLACK, None,
        );
        let mut pixels2 = vec![0u32; 400 * 50];
        let a2 = draw_shaped_text(
            &mut pixels2, 400, 50, "Hello, World!", 0.0, 0.0, 14.0,
            "System", Color::BLACK, None,
        );
        assert!(
            a2 > a1,
            "longer text advance {a2} should exceed shorter {a1}"
        );
    }

    #[test]
    fn draw_semi_transparent() {
        let semi = Color { r: 0.0, g: 1.0, b: 0.0, a: 0.5 };
        let mut pixels = vec![0xFFFFFFFFu32; 200 * 50]; // white background
        draw_shaped_text(
            &mut pixels, 200, 50, "Semi", 0.0, 0.0, 14.0,
            "System", semi, None,
        );
        // Some pixels should have been blended (not all pure white)
        assert!(pixels.iter().any(|&p| p != 0xFFFFFFFF));
    }

    // ── available_font_families ─────────────────────────────────────

    #[test]
    fn available_families_includes_dejavu() {
        let families = available_font_families();
        assert!(
            families.iter().any(|f| f.contains("DejaVu")),
            "should include DejaVu fonts, got {:?}",
            &families[..families.len().min(10)]
        );
    }

    #[test]
    fn available_families_sorted_and_deduped() {
        let families = available_font_families();
        for w in families.windows(2) {
            assert!(
                w[0] <= w[1],
                "families should be sorted, but found {:?} before {:?}",
                w[0],
                w[1]
            );
            assert_ne!(w[0], w[1], "families should be deduped, found duplicate {:?}", w[0]);
        }
    }

    #[test]
    fn available_families_non_empty() {
        let families = available_font_families();
        assert!(!families.is_empty(), "should have at least embedded fonts");
    }

    // ── attrs_for_font mapping ──────────────────────────────────────

    #[test]
    fn attrs_monospace_variants() {
        for name in &["Monospace", "DejaVuSansMono", "Courier", "Courier New", "Monaco"] {
            let attrs = attrs_for_font(name);
            assert_eq!(attrs.family, Family::Monospace, "font {name} should map to Monospace");
        }
    }

    #[test]
    fn attrs_system_sans_serif() {
        for name in &["System", ".AppleSystemUIFont", ".SFUI-Regular", ".AppleSystemUIFontMonospacedDigit"] {
            let attrs = attrs_for_font(name);
            assert_eq!(attrs.family, Family::SansSerif, "font {name} should map to SansSerif");
        }
    }

    #[test]
    fn attrs_bold_has_bold_weight() {
        for name in &["System-Bold", "DejaVuSans-Bold", ".AppleSystemUIFontBold"] {
            let attrs = attrs_for_font(name);
            assert_eq!(attrs.weight, cosmic_text::Weight::BOLD, "font {name} should be bold");
        }
    }

    #[test]
    fn attrs_italic_has_italic_style() {
        for name in &["System-Italic", "DejaVuSans-Oblique"] {
            let attrs = attrs_for_font(name);
            assert_eq!(attrs.style, cosmic_text::Style::Italic, "font {name} should be italic");
        }
    }

    #[test]
    fn attrs_bold_italic_has_both() {
        for name in &["System-BoldItalic", "DejaVuSans-BoldOblique"] {
            let attrs = attrs_for_font(name);
            assert_eq!(attrs.weight, cosmic_text::Weight::BOLD, "font {name} should be bold");
            assert_eq!(attrs.style, cosmic_text::Style::Italic, "font {name} should be italic");
        }
    }

    #[test]
    fn attrs_unknown_font_uses_family_name() {
        let attrs = attrs_for_font("CustomFont");
        assert_eq!(attrs.family, Family::Name("CustomFont"));
    }

    // ── ShapedTextMetrics struct ────────────────────────────────────

    #[test]
    fn shaped_text_metrics_clone() {
        let m = measure_shaped_text("Test", 14.0, "System", None);
        let m2 = m;
        assert_eq!(m.width, m2.width);
        assert_eq!(m.height, m2.height);
        assert_eq!(m.line_count, m2.line_count);
    }

    #[test]
    fn shaped_text_metrics_debug() {
        let m = measure_shaped_text("Test", 14.0, "System", None);
        let debug = format!("{m:?}");
        assert!(debug.contains("ShapedTextMetrics"));
        assert!(debug.contains("width"));
    }

    // ── Consistency checks ──────────────────────────────────────────

    #[test]
    fn measure_and_draw_consistent_advance() {
        let m = measure_shaped_text("Hello", 14.0, "System", None);
        let mut pixels = vec![0u32; 400 * 50];
        let advance = draw_shaped_text(
            &mut pixels, 400, 50, "Hello", 0.0, 0.0, 14.0,
            "System", Color::BLACK, None,
        );
        // The draw advance and measure width should be reasonably close
        assert!(
            (advance - m.width).abs() < 2.0,
            "draw advance {advance} should be close to measured width {}",
            m.width
        );
    }

    #[test]
    fn measure_monospace_equal_width_chars() {
        // In a monospace font, "III" and "WWW" should have equal width
        let m_i = measure_shaped_text("III", 14.0, "Monospace", None);
        let m_w = measure_shaped_text("WWW", 14.0, "Monospace", None);
        assert!(
            (m_i.width - m_w.width).abs() < 2.0,
            "monospace III width {} should be ~= WWW width {}",
            m_i.width,
            m_w.width
        );
    }
}
