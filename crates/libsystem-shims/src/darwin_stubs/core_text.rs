//! CoreText framework stubs.
//!
//! macOS text rendering API. On Linux there is no CoreText, so we provide
//! handle-based stubs that store lightweight `CtFont` records in a global
//! registry.  Font metrics are derived from the requested size so callers
//! that probe metrics get plausible proportional values.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::table::ShimTable;

// ---------------------------------------------------------------------------
// Handle allocator (shared across CTFont / CTFontDescriptor / CTParagraphStyle / CTLine)
// ---------------------------------------------------------------------------

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn new_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// CtFont registry
// ---------------------------------------------------------------------------

struct CtFont {
    #[allow(dead_code)]
    name: String,
    size: f64,
    ascent: f64,
    descent: f64,
    leading: f64,
    x_height: f64,
    cap_height: f64,
    units_per_em: u32,
}

impl CtFont {
    fn new(name: String, size: f64) -> Self {
        Self {
            name,
            size,
            ascent: size * 0.8,
            descent: size * 0.2,
            leading: size * 0.1,
            x_height: size * 0.5,
            cap_height: size * 0.7,
            units_per_em: 2048,
        }
    }
}

static FONT_REGISTRY: Mutex<Option<HashMap<u64, CtFont>>> = Mutex::new(None);

fn with_registry<R>(f: impl FnOnce(&mut HashMap<u64, CtFont>) -> R) -> R {
    let mut guard = FONT_REGISTRY.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

fn lookup_font<R>(handle: u64, f: impl FnOnce(&CtFont) -> R, default: R) -> R {
    with_registry(|map| map.get(&handle).map_or(default, f))
}

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

pub struct CoreTextStubs;

impl CoreTextStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            // -- CTFontManager (existing, still NULL stubs) --
            ("_CTFontManagerCopyAvailableFontFamilyNames", shim_ct_null_stub as *const () as u64, "CTFontManagerCopyAvailableFontFamilyNames (stub → NULL)"),
            ("_CTFontManagerCopyAvailablePostScriptNames", shim_ct_null_stub as *const () as u64, "CTFontManagerCopyAvailablePostScriptNames (stub → NULL)"),

            // -- CTFont creation --
            ("_CTFontCreateWithName", shim_ct_font_create_with_name as *const () as u64, "CTFontCreateWithName (handle-based)"),
            ("_CTFontCopyFullName", shim_ct_font_copy_full_name as *const () as u64, "CTFontCopyFullName (stub → NULL)"),
            ("_CTFontGetGlyphCount", shim_ct_zero_stub as *const () as u64, "CTFontGetGlyphCount (stub → 0)"),

            // -- CTFont metrics --
            ("_CTFontGetAscent", shim_ct_font_get_ascent as *const () as u64, "CTFontGetAscent"),
            ("_CTFontGetDescent", shim_ct_font_get_descent as *const () as u64, "CTFontGetDescent"),
            ("_CTFontGetLeading", shim_ct_font_get_leading as *const () as u64, "CTFontGetLeading"),
            ("_CTFontGetXHeight", shim_ct_font_get_x_height as *const () as u64, "CTFontGetXHeight"),
            ("_CTFontGetCapHeight", shim_ct_font_get_cap_height as *const () as u64, "CTFontGetCapHeight"),
            ("_CTFontGetUnitsPerEm", shim_ct_font_get_units_per_em as *const () as u64, "CTFontGetUnitsPerEm"),
            ("_CTFontGetSize", shim_ct_font_get_size as *const () as u64, "CTFontGetSize"),
            ("_CTFontGetBoundingBox", shim_ct_font_get_bounding_box as *const () as u64, "CTFontGetBoundingBox"),

            // -- CTFontDescriptor --
            ("_CTFontDescriptorCreateWithAttributes", shim_ct_descriptor_create as *const () as u64, "CTFontDescriptorCreateWithAttributes"),
            ("_CTFontDescriptorCreateMatchingFontDescriptor", shim_ct_descriptor_match as *const () as u64, "CTFontDescriptorCreateMatchingFontDescriptor"),
            ("_CTFontDescriptorCopyAttribute", shim_ct_descriptor_copy_attr as *const () as u64, "CTFontDescriptorCopyAttribute"),

            // -- CTParagraphStyle --
            ("_CTParagraphStyleCreate", shim_ct_paragraph_style_create as *const () as u64, "CTParagraphStyleCreate"),
            ("_CTParagraphStyleGetValueForSpecifier", shim_ct_paragraph_style_get_value as *const () as u64, "CTParagraphStyleGetValueForSpecifier"),

            // -- CTLine --
            ("_CTLineCreateWithAttributedString", shim_ct_line_create as *const () as u64, "CTLineCreateWithAttributedString (handle-based)"),

            // -- CTRun --
            ("_CTRunGetGlyphCount", shim_ct_run_get_glyph_count as *const () as u64, "CTRunGetGlyphCount (stub → 0)"),
            ("_CTRunGetPositions", shim_ct_run_get_positions as *const () as u64, "CTRunGetPositions (no-op)"),
            ("_CTRunGetGlyphs", shim_ct_run_get_glyphs as *const () as u64, "CTRunGetGlyphs (no-op)"),
            ("_CTRunGetStringRange", shim_ct_run_get_string_range as *const () as u64, "CTRunGetStringRange (sret → {0,0})"),
            ("_CTRunGetAdvances", shim_ct_run_get_advances as *const () as u64, "CTRunGetAdvances (no-op)"),
            ("_CTRunGetAttributes", shim_ct_null_stub as *const () as u64, "CTRunGetAttributes (stub → NULL)"),
            ("_CTRunGetStringIndices", shim_ct_run_get_glyphs as *const () as u64, "CTRunGetStringIndices (no-op)"),
            ("_CTRunGetStatus", shim_ct_zero_stub as *const () as u64, "CTRunGetStatus (stub → 0)"),
            ("_CTRunGetTypographicBounds", shim_ct_run_get_typo_bounds as *const () as u64, "CTRunGetTypographicBounds (stub)"),

            // -- CTLine extended --
            ("_CTLineDraw", shim_ct_line_draw as *const () as u64, "CTLineDraw (no-op)"),
            ("_CTLineGetGlyphRuns", shim_ct_null_stub as *const () as u64, "CTLineGetGlyphRuns (stub → NULL/empty)"),
            ("_CTLineGetGlyphCount", shim_ct_zero_stub as *const () as u64, "CTLineGetGlyphCount (stub → 0)"),
            ("_CTLineGetStringRange", shim_ct_line_get_string_range as *const () as u64, "CTLineGetStringRange (sret → {0,0})"),
            ("_CTLineGetTypographicBounds", shim_ct_line_get_typo_bounds as *const () as u64, "CTLineGetTypographicBounds (stub)"),
            ("_CTLineGetBoundsWithOptions", shim_ct_line_get_bounds as *const () as u64, "CTLineGetBoundsWithOptions (sret → zero)"),
            ("_CTLineGetOffsetForStringIndex", shim_ct_line_get_offset as *const () as u64, "CTLineGetOffsetForStringIndex (stub → 0.0)"),
            ("_CTLineGetStringIndexForPosition", shim_ct_zero_stub as *const () as u64, "CTLineGetStringIndexForPosition (stub → 0)"),
            ("_CTLineCreateTruncatedLine", shim_ct_line_create_truncated as *const () as u64, "CTLineCreateTruncatedLine (stub → same)"),
            ("_CTLineCreateJustifiedLine", shim_ct_line_create_truncated as *const () as u64, "CTLineCreateJustifiedLine (stub → same)"),

            // -- CTFramesetter --
            ("_CTFramesetterCreateWithAttributedString", shim_ct_framesetter_create as *const () as u64, "CTFramesetterCreateWithAttributedString (handle)"),
            ("_CTFramesetterCreateFrame", shim_ct_framesetter_create_frame as *const () as u64, "CTFramesetterCreateFrame (handle)"),
            ("_CTFramesetterSuggestFrameSizeWithConstraints", shim_ct_framesetter_suggest_size as *const () as u64, "CTFramesetterSuggestFrameSizeWithConstraints (sret)"),

            // -- CTFrame --
            ("_CTFrameDraw", shim_ct_frame_draw as *const () as u64, "CTFrameDraw (no-op)"),
            ("_CTFrameGetLines", shim_ct_null_stub as *const () as u64, "CTFrameGetLines (stub → NULL)"),
            ("_CTFrameGetLineOrigins", shim_ct_frame_get_line_origins as *const () as u64, "CTFrameGetLineOrigins (no-op)"),
            ("_CTFrameGetPath", shim_ct_null_stub as *const () as u64, "CTFrameGetPath (stub → NULL)"),
            ("_CTFrameGetVisibleStringRange", shim_ct_frame_get_visible_range as *const () as u64, "CTFrameGetVisibleStringRange (sret → {0,0})"),

            // -- CTFont extended --
            ("_CTFontCreateWithFontDescriptor", shim_ct_font_create_with_descriptor as *const () as u64, "CTFontCreateWithFontDescriptor (handle)"),
            ("_CTFontCreateCopyWithAttributes", shim_ct_font_create_copy as *const () as u64, "CTFontCreateCopyWithAttributes (handle)"),
            ("_CTFontCreateCopyWithSymbolicTraits", shim_ct_font_create_copy_traits as *const () as u64, "CTFontCreateCopyWithSymbolicTraits (handle)"),
            ("_CTFontCopyFamilyName", shim_ct_font_copy_full_name as *const () as u64, "CTFontCopyFamilyName (stub → NULL)"),
            ("_CTFontCopyPostScriptName", shim_ct_font_copy_full_name as *const () as u64, "CTFontCopyPostScriptName (stub → NULL)"),
            ("_CTFontCopyDisplayName", shim_ct_font_copy_full_name as *const () as u64, "CTFontCopyDisplayName (stub → NULL)"),
            ("_CTFontGetGlyphsForCharacters", shim_ct_font_get_glyphs_for_chars as *const () as u64, "CTFontGetGlyphsForCharacters (stub → identity map)"),
            ("_CTFontGetAdvancesForGlyphs", shim_ct_font_get_advances as *const () as u64, "CTFontGetAdvancesForGlyphs (stub → size*0.6)"),
            ("_CTFontGetSymbolicTraits", shim_ct_zero_stub as *const () as u64, "CTFontGetSymbolicTraits (stub → 0)"),
            ("_CTFontCopyCharacterSet", shim_ct_null_stub as *const () as u64, "CTFontCopyCharacterSet (stub → NULL)"),
            ("_CTFontCopyTraits", shim_ct_null_stub as *const () as u64, "CTFontCopyTraits (stub → NULL)"),
            ("_CTFontGetMatrix", shim_ct_font_get_matrix as *const () as u64, "CTFontGetMatrix (sret → identity)"),
            ("_CTFontGetUnderlinePosition", shim_ct_font_get_underline_pos as *const () as u64, "CTFontGetUnderlinePosition"),
            ("_CTFontGetUnderlineThickness", shim_ct_font_get_underline_thickness as *const () as u64, "CTFontGetUnderlineThickness"),

            // -- CTFontManager extended --
            ("_CTFontManagerRegisterFontsForURL", shim_ct_bool_true as *const () as u64, "CTFontManagerRegisterFontsForURL (stub → true)"),
            ("_CTFontManagerRegisterFontsForURLs", shim_ct_bool_true as *const () as u64, "CTFontManagerRegisterFontsForURLs (stub → true)"),

            // -- CTStringAttributes constants (these are data symbols) --
            ("_kCTFontAttributeName", &KCTFONT_ATTR_NAME as *const u64 as u64, "kCTFontAttributeName"),
            ("_kCTForegroundColorAttributeName", &KCTFOREGROUND_COLOR_ATTR as *const u64 as u64, "kCTForegroundColorAttributeName"),
            ("_kCTParagraphStyleAttributeName", &KCTPARAGRAPH_STYLE_ATTR as *const u64 as u64, "kCTParagraphStyleAttributeName"),
            ("_kCTForegroundColorFromContextAttributeName", &KCTFG_FROM_CTX_ATTR as *const u64 as u64, "kCTForegroundColorFromContextAttributeName")
        );
        log::debug!("CoreTextStubs: registered {} shims", 21 + 41);
    }
}

// ---------------------------------------------------------------------------
// Generic null / zero stubs (kept for manager + glyph-count)
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_null_stub(_arg0: u64, _arg1: u64, _arg2: u64) -> u64 { 0 }
pub extern "C" fn shim_ct_zero_stub(_arg0: u64) -> u64 { 0 }

// ---------------------------------------------------------------------------
// CTFont creation
// ---------------------------------------------------------------------------

/// `CTFontCreateWithName(name: CFStringRef, size: f64, matrix: u64) -> CTFontRef`
///
/// `name` is treated as a C-string pointer.  If NULL we default to "Helvetica".
/// `size` arrives as a raw f64 in a float register — the caller passes it via
/// the second integer-register slot so we receive the bit pattern in `size_bits`.
pub extern "C" fn shim_ct_font_create_with_name(name_ptr: u64, size_bits: u64, _matrix: u64) -> u64 {
    let name = if name_ptr == 0 {
        "Helvetica".to_string()
    } else {
        let cstr = unsafe { std::ffi::CStr::from_ptr(name_ptr as *const std::ffi::c_char) };
        cstr.to_string_lossy().into_owned()
    };

    let size = f64::from_bits(size_bits);
    let size = if size <= 0.0 { 12.0 } else { size };

    let handle = new_handle();
    with_registry(|map| {
        map.insert(handle, CtFont::new(name, size));
    });
    handle
}

/// `CTFontCopyFullName(font: u64) -> CFStringRef`  — stub returns NULL.
pub extern "C" fn shim_ct_font_copy_full_name(_font: u64) -> u64 { 0 }

// ---------------------------------------------------------------------------
// CTFont metrics
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_font_get_ascent(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.ascent, 0.0);
    v.to_bits()
}

pub extern "C" fn shim_ct_font_get_descent(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.descent, 0.0);
    v.to_bits()
}

pub extern "C" fn shim_ct_font_get_leading(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.leading, 0.0);
    v.to_bits()
}

pub extern "C" fn shim_ct_font_get_x_height(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.x_height, 0.0);
    v.to_bits()
}

pub extern "C" fn shim_ct_font_get_cap_height(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.cap_height, 0.0);
    v.to_bits()
}

pub extern "C" fn shim_ct_font_get_units_per_em(font: u64) -> u32 {
    lookup_font(font, |f| f.units_per_em, 0)
}

pub extern "C" fn shim_ct_font_get_size(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.size, 0.0);
    v.to_bits()
}

/// `CTFontGetBoundingBox(font, rect_out) -> rect_out`
///
/// Writes a CGRect `{x: 0, y: -descent, width: size*0.6*units_per_em, height: ascent+descent}`
/// to `rect_out` and returns `rect_out`.
pub extern "C" fn shim_ct_font_get_bounding_box(font: u64, rect_out: u64) -> u64 {
    if rect_out == 0 {
        return 0;
    }
    let (x, y, w, h) = lookup_font(font, |f| {
        (
            0.0_f64,
            -f.descent,
            f.size * 0.6 * f.units_per_em as f64,
            f.ascent + f.descent,
        )
    }, (0.0, 0.0, 0.0, 0.0));

    let ptr = rect_out as *mut f64;
    unsafe {
        ptr.write(x);
        ptr.add(1).write(y);
        ptr.add(2).write(w);
        ptr.add(3).write(h);
    }
    rect_out
}

// ---------------------------------------------------------------------------
// CTFontDescriptor
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_descriptor_create(_attrs: u64) -> u64 {
    new_handle()
}

pub extern "C" fn shim_ct_descriptor_match(descriptor: u64, _mandatory_attrs: u64) -> u64 {
    descriptor
}

pub extern "C" fn shim_ct_descriptor_copy_attr(_descriptor: u64, _attr_name: u64) -> u64 {
    0
}

// ---------------------------------------------------------------------------
// CTParagraphStyle
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_paragraph_style_create(_settings: u64, _count: usize) -> u64 {
    new_handle()
}

pub extern "C" fn shim_ct_paragraph_style_get_value(_style: u64, _spec: u32, _value_size: usize, _value_out: u64) -> u8 {
    0 // false — value not available
}

// ---------------------------------------------------------------------------
// CTLine registry — stores the text and font size for CTLineDraw
// ---------------------------------------------------------------------------

struct CtLine {
    text: String,
    font_size: f64,
}

static LINE_REGISTRY: Mutex<Option<HashMap<u64, CtLine>>> = Mutex::new(None);

fn with_line_registry<R>(f: impl FnOnce(&mut HashMap<u64, CtLine>) -> R) -> R {
    let mut guard = LINE_REGISTRY.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

// ---------------------------------------------------------------------------
// CTLine
// ---------------------------------------------------------------------------

/// CTLineCreateWithAttributedString(attrStr) → CTLineRef
///
/// Extracts the plain text from the NSAttributedString (offset 8 = NSString ptr)
/// and stores it for CTLineDraw.
pub extern "C" fn shim_ct_line_create(attr_str: u64) -> u64 {
    let handle = new_handle();
    let mut text = String::new();
    let mut font_size = 12.0;

    if attr_str != 0 {
        // NSAttributedString layout: [0]=isa, [8]=string_ptr (NSString)
        let string_ptr = unsafe { *((attr_str as usize + 8) as *const u64) };
        if string_ptr != 0 {
            // Try to read the NSString as a C string.
            // NSString layout: [0]=isa, [8]=c_buf_ptr or [16]=c_buf_ptr depending on class.
            // Use the cfstring helper pattern: try offset 16 first (common for our NSString),
            // then offset 8.
            let buf_ptr = unsafe { *((string_ptr as usize + 16) as *const u64) };
            if buf_ptr > 0x1000 {
                if let Ok(cstr) = unsafe { std::ffi::CStr::from_ptr(buf_ptr as *const std::ffi::c_char) }.to_str() {
                    text = cstr.to_string();
                }
            }
        }
        // Try to read font from attributes: [16]=attrs_count, [24]=attrs_ptr
        // AttrRun[0] at attrs_ptr: [16]=dict_ptr → NSDictionary with font
        // This is best-effort; just use 12.0 default if we can't read it.
        let attrs_count = unsafe { *((attr_str as usize + 16) as *const u64) };
        if attrs_count > 0 {
            let attrs_ptr = unsafe { *((attr_str as usize + 24) as *const u64) };
            if attrs_ptr > 0x1000 {
                // AttrRun[0].dict_ptr at offset 16
                let dict_ptr = unsafe { *((attrs_ptr as usize + 16) as *const u64) };
                if dict_ptr > 0x1000 {
                    // Try to find CTFont in dictionary — for now just use any font handle
                    // that exists in the font registry. This is best-effort.
                    let size = with_registry(|map| {
                        map.values().next().map(|f| f.size)
                    });
                    if let Some(s) = size {
                        font_size = s;
                    }
                }
            }
        }
    }

    with_line_registry(|map| {
        map.insert(handle, CtLine { text, font_size });
    });
    handle
}

// ---------------------------------------------------------------------------
// CTRun
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_run_get_glyph_count(_run: u64) -> i64 {
    0
}

pub extern "C" fn shim_ct_run_get_positions(_run: u64, _range_loc: i64, _range_len: i64, _positions: u64) {
    // no-op
}

pub extern "C" fn shim_ct_run_get_glyphs(_run: u64, _range_loc: i64, _range_len: i64, _glyphs: u64) {
    // no-op
}

/// CTRunGetStringRange(run) → CFRange (sret: {location, length}).
pub extern "C" fn shim_ct_run_get_string_range(sret: u64, _run: u64) {
    if sret != 0 {
        unsafe {
            *(sret as *mut i64) = 0;
            *((sret as usize + 8) as *mut i64) = 0;
        }
    }
}

pub extern "C" fn shim_ct_run_get_advances(_run: u64, _range_loc: i64, _range_len: i64, _advances: u64) {
    // no-op
}

/// CTRunGetTypographicBounds(run, range, ascent_out, descent_out, leading_out) → f64 width.
pub extern "C" fn shim_ct_run_get_typo_bounds(_run: u64, _range_loc: i64, _range_len: i64,
    _ascent: u64, _descent: u64, _leading: u64) -> u64 {
    0.0_f64.to_bits()
}

// ---------------------------------------------------------------------------
// CTLine extended
// ---------------------------------------------------------------------------

/// CTLineDraw(line, context) — renders the stored text via CG context.
///
/// Uses the registered draw callback if available, otherwise no-op.
pub extern "C" fn shim_ct_line_draw(line: u64, context: u64) {
    let line_data = with_line_registry(|map| {
        map.get(&line).map(|l| (l.text.clone(), l.font_size))
    });
    if let Some((text, font_size)) = line_data {
        if !text.is_empty() && context != 0 {
            let cb = CT_LINE_DRAW_CB.load(std::sync::atomic::Ordering::Relaxed);
            if cb != 0 {
                // Call: fn(ctx: u64, text_ptr: *const u8, text_len: usize, font_size: f64)
                let f: extern "C" fn(u64, *const u8, usize, f64) =
                    unsafe { std::mem::transmute(cb) };
                f(context, text.as_ptr(), text.len(), font_size);
            }
        }
    }
}

/// Callback for CTLineDraw to render text through CG context.
/// Set by cg-shims at initialization time.
static CT_LINE_DRAW_CB: AtomicU64 = AtomicU64::new(0);

/// Register the CTLineDraw rendering callback.
/// Called by cg-shims to wire CTLineDraw → CGContext text rendering.
pub fn set_ct_line_draw_callback(f: extern "C" fn(u64, *const u8, usize, f64)) {
    CT_LINE_DRAW_CB.store(f as *const () as u64, Ordering::Relaxed);
}

/// CTLineGetStringRange(line) → CFRange (sret).
pub extern "C" fn shim_ct_line_get_string_range(sret: u64, _line: u64) {
    if sret != 0 {
        unsafe {
            *(sret as *mut i64) = 0;
            *((sret as usize + 8) as *mut i64) = 0;
        }
    }
}

/// CTLineGetTypographicBounds(line, ascent_out, descent_out, leading_out) → f64 width.
pub extern "C" fn shim_ct_line_get_typo_bounds(line: u64, ascent: u64, descent: u64, leading: u64) -> u64 {
    // Try to get real metrics from the line's stored font size and text
    let data = with_line_registry(|map| {
        map.get(&line).map(|l| (l.text.len(), l.font_size))
    });
    let (text_len, font_size) = data.unwrap_or((0, 12.0));
    let fs = if font_size > 0.0 { font_size } else { 12.0 };

    if ascent != 0 { unsafe { *(ascent as *mut f64) = fs * 0.8; } }
    if descent != 0 { unsafe { *(descent as *mut f64) = fs * 0.2; } }
    if leading != 0 { unsafe { *(leading as *mut f64) = fs * 0.1; } }

    // Approximate width: ~0.6 * font_size per character
    let width = text_len as f64 * fs * 0.6;
    width.to_bits()
}

/// CTLineGetBoundsWithOptions(line, options) → CGRect (sret, 32 bytes via x8).
pub extern "C" fn shim_ct_line_get_bounds(sret: u64, _line: u64, _options: u64) {
    if sret != 0 {
        unsafe {
            let p = sret as *mut f64;
            *p = 0.0; *p.add(1) = 0.0; *p.add(2) = 0.0; *p.add(3) = 0.0;
        }
    }
}

/// CTLineGetOffsetForStringIndex(line, index, secondary_offset_out) → f64.
pub extern "C" fn shim_ct_line_get_offset(_line: u64, _idx: i64, _secondary: u64) -> u64 {
    0.0_f64.to_bits()
}

/// CTLineCreateTruncatedLine / CTLineCreateJustifiedLine — returns the same line.
pub extern "C" fn shim_ct_line_create_truncated(line: u64, _width: u64, _type: u32, _token: u64) -> u64 {
    if line != 0 { line } else { new_handle() }
}

// ---------------------------------------------------------------------------
// CTFramesetter / CTFrame
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_framesetter_create(_attr_str: u64) -> u64 {
    new_handle()
}

/// CTFramesetterCreateFrame(framesetter, string_range, path, frame_attrs) → CTFrame.
pub extern "C" fn shim_ct_framesetter_create_frame(
    _framesetter: u64, _range_loc: i64, _range_len: i64, _path: u64, _attrs: u64
) -> u64 {
    new_handle()
}

/// CTFramesetterSuggestFrameSizeWithConstraints(fs, range, attrs, constraints, fit_range_out) → CGSize (sret).
pub extern "C" fn shim_ct_framesetter_suggest_size(
    sret: u64, _fs: u64, _range_loc: i64, _range_len: i64, _attrs: u64,
    _cw: f64, _ch: f64, _fit_range: u64,
) {
    if sret != 0 {
        unsafe {
            let p = sret as *mut f64;
            *p = 0.0; *p.add(1) = 0.0;
        }
    }
}

/// CTFrameDraw(frame, context) — no-op.
pub extern "C" fn shim_ct_frame_draw(_frame: u64, _context: u64) {
    // no-op
}

pub extern "C" fn shim_ct_frame_get_line_origins(
    _frame: u64, _range_loc: i64, _range_len: i64, _origins: u64
) {
    // no-op
}

/// CTFrameGetVisibleStringRange(frame) → CFRange (sret).
pub extern "C" fn shim_ct_frame_get_visible_range(sret: u64, _frame: u64) {
    if sret != 0 {
        unsafe {
            *(sret as *mut i64) = 0;
            *((sret as usize + 8) as *mut i64) = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// CTFont extended
// ---------------------------------------------------------------------------

/// CTFontCreateWithFontDescriptor(descriptor, size, matrix) → CTFont.
pub extern "C" fn shim_ct_font_create_with_descriptor(_desc: u64, size_bits: u64, _matrix: u64) -> u64 {
    let size = f64::from_bits(size_bits);
    let size = if size <= 0.0 { 12.0 } else { size };
    let handle = new_handle();
    with_registry(|map| { map.insert(handle, CtFont::new("System".into(), size)); });
    handle
}

/// CTFontCreateCopyWithAttributes(font, size, matrix, attrs) → CTFont.
pub extern "C" fn shim_ct_font_create_copy(font: u64, size_bits: u64, _matrix: u64, _attrs: u64) -> u64 {
    let size = f64::from_bits(size_bits);
    let actual_size = if size <= 0.0 {
        lookup_font(font, |f| f.size, 12.0)
    } else {
        size
    };
    let handle = new_handle();
    with_registry(|map| { map.insert(handle, CtFont::new("System".into(), actual_size)); });
    handle
}

/// CTFontCreateCopyWithSymbolicTraits(font, size, matrix, desired, mask) → CTFont.
pub extern "C" fn shim_ct_font_create_copy_traits(
    font: u64, size_bits: u64, _matrix: u64, _desired: u32, _mask: u32
) -> u64 {
    shim_ct_font_create_copy(font, size_bits, 0, 0)
}

/// CTFontGetGlyphsForCharacters(font, chars, glyphs, count) → bool.
/// Simplified: map each character code to itself as a "glyph index".
pub extern "C" fn shim_ct_font_get_glyphs_for_chars(
    _font: u64, chars: u64, glyphs: u64, count: i64,
) -> u8 {
    if chars == 0 || glyphs == 0 || count <= 0 { return 0; }
    let c = count as usize;
    unsafe {
        let src = chars as *const u16;
        let dst = glyphs as *mut u16;
        for i in 0..c {
            *dst.add(i) = *src.add(i);
        }
    }
    1 // true — all mapped
}

/// CTFontGetAdvancesForGlyphs(font, orientation, glyphs, advances, count) → f64 total.
pub extern "C" fn shim_ct_font_get_advances(
    font: u64, _orientation: u32, _glyphs: u64, advances: u64, count: i64,
) -> u64 {
    let size = lookup_font(font, |f| f.size, 12.0);
    let advance = size * 0.6; // approximate monospace advance
    let c = count.max(0) as usize;
    if advances != 0 && c > 0 {
        unsafe {
            // Each advance is CGSize = {width: f64, height: f64}
            let p = advances as *mut f64;
            for i in 0..c {
                *p.add(i * 2) = advance;
                *p.add(i * 2 + 1) = 0.0;
            }
        }
    }
    (advance * c as f64).to_bits()
}

/// CTFontGetMatrix(font) → CGAffineTransform (48 bytes → sret via x8).
pub extern "C" fn shim_ct_font_get_matrix(sret: u64, _font: u64) {
    if sret != 0 {
        unsafe {
            let p = sret as *mut f64;
            *p = 1.0; *p.add(1) = 0.0;
            *p.add(2) = 0.0; *p.add(3) = 1.0;
            *p.add(4) = 0.0; *p.add(5) = 0.0;
        }
    }
}

/// CTFontGetUnderlinePosition(font) → f64.
pub extern "C" fn shim_ct_font_get_underline_pos(font: u64) -> u64 {
    let v = lookup_font(font, |f| -f.size * 0.1, -1.2);
    v.to_bits()
}

/// CTFontGetUnderlineThickness(font) → f64.
pub extern "C" fn shim_ct_font_get_underline_thickness(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.size * 0.05, 0.6);
    v.to_bits()
}

pub extern "C" fn shim_ct_bool_true(_a: u64, _b: u64, _c: u64) -> u8 { 1 }

// ---------------------------------------------------------------------------
// Data symbol sentinels for CoreText attribute name keys
// ---------------------------------------------------------------------------

static KCTFONT_ATTR_NAME: u64 = 0x4354_464F_4E54_0001;
static KCTFOREGROUND_COLOR_ATTR: u64 = 0x4354_464F_4E54_0002;
static KCTPARAGRAPH_STYLE_ATTR: u64 = 0x4354_464F_4E54_0003;
static KCTFG_FROM_CTX_ATTR: u64 = 0x4354_464F_4E54_0004;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_create_returns_handle() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(12.0), 0);
        assert_ne!(h, 0, "CTFontCreateWithName should return a non-zero handle");
    }

    #[test]
    fn test_font_metrics_proportional() {
        let size = 20.0_f64;
        let h = shim_ct_font_create_with_name(0, size.to_bits(), 0);

        let ascent = f64::from_bits(shim_ct_font_get_ascent(h));
        let descent = f64::from_bits(shim_ct_font_get_descent(h));
        let leading = f64::from_bits(shim_ct_font_get_leading(h));

        assert!((ascent - 16.0).abs() < 1e-9, "ascent should be 16.0, got {ascent}");
        assert!((descent - 4.0).abs() < 1e-9, "descent should be 4.0, got {descent}");
        assert!((leading - 2.0).abs() < 1e-9, "leading should be 2.0, got {leading}");
    }

    #[test]
    fn test_font_get_size() {
        let size = 14.0_f64;
        let h = shim_ct_font_create_with_name(0, size.to_bits(), 0);
        let got = f64::from_bits(shim_ct_font_get_size(h));
        assert!((got - 14.0).abs() < 1e-9, "size should be 14.0, got {got}");
    }

    #[test]
    fn test_font_get_units_per_em() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(12.0), 0);
        assert_eq!(shim_ct_font_get_units_per_em(h), 2048);
    }

    #[test]
    fn test_descriptor_create() {
        let h = shim_ct_descriptor_create(0);
        assert_ne!(h, 0, "descriptor handle should be non-zero");
    }

    #[test]
    fn test_descriptor_copy_attribute_null() {
        let h = shim_ct_descriptor_create(0);
        assert_eq!(shim_ct_descriptor_copy_attr(h, 0), 0);
    }

    #[test]
    fn test_paragraph_style_create() {
        let h = shim_ct_paragraph_style_create(0, 0);
        assert_ne!(h, 0, "paragraph style handle should be non-zero");
    }

    #[test]
    fn test_line_create() {
        let h = shim_ct_line_create(0);
        assert_ne!(h, 0, "line handle should be non-zero");
    }

    #[test]
    fn test_run_glyph_count_zero() {
        assert_eq!(shim_ct_run_get_glyph_count(0), 0);
    }

    #[test]
    fn test_font_manager_available() {
        let result = shim_ct_null_stub(0, 0, 0);
        assert_eq!(result, 0);
    }
}
