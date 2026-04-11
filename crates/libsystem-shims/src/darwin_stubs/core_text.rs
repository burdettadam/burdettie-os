//! CoreText framework — real font metrics via callback to graphics-core.
//!
//! macOS text rendering API. On Linux there is no CoreText, so we provide
//! handle-based stubs that store `CtFont` records in a global registry.
//!
//! Font metrics are obtained from real TrueType font data via a callback
//! registered by `cg-shims` at initialization time. When no callback is
//! registered, proportional estimates are used as a fallback.

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
// Callback infrastructure — wired by cg-shims at init time
// ---------------------------------------------------------------------------

/// Font metrics callback: `fn(name_ptr, name_len, size) -> FontMetricsResult`.
///
/// Returns real ascent/descent/leading/x_height/cap_height/advance_per_char
/// from the `graphics-core` FontManager.  Registered by `cg-shims`.
#[repr(C)]
pub struct FontMetricsResult {
    pub ascent: f64,
    pub descent: f64,   // negative
    pub leading: f64,   // line_gap
    pub x_height: f64,
    pub cap_height: f64,
    pub advance_width: f64, // average char advance
    pub units_per_em: u32,
}

type FontMetricsFn = extern "C" fn(*const u8, usize, f64) -> FontMetricsResult;
static CT_FONT_METRICS_CB: AtomicU64 = AtomicU64::new(0);

/// Register the font metrics callback. Called by cg-shims at init.
pub fn set_ct_font_metrics_callback(f: FontMetricsFn) {
    CT_FONT_METRICS_CB.store(f as *const () as u64, Ordering::Relaxed);
}

fn get_real_metrics(name: &str, size: f64) -> Option<FontMetricsResult> {
    let cb = CT_FONT_METRICS_CB.load(Ordering::Relaxed);
    if cb == 0 { return None; }
    let f: FontMetricsFn = unsafe { std::mem::transmute(cb) };
    Some(f(name.as_ptr(), name.len(), size))
}

/// Text measurement callback: `fn(text_ptr, text_len, font_name_ptr, font_name_len, size) -> f64`.
///
/// Returns the advance width of the text string using real font metrics.
type TextMeasureFn = extern "C" fn(*const u8, usize, *const u8, usize, f64) -> f64;
static CT_TEXT_MEASURE_CB: AtomicU64 = AtomicU64::new(0);

/// Font family enumeration callback: `fn(out_ptr, out_cap) -> count`.
/// Writes up to `out_cap` nul-terminated C string pointers to `out_ptr`.
/// Each pointer is `malloc`-allocated and must be freed by the caller.
/// Returns actual number of families.
type FontFamiliesFn = extern "C" fn(*mut *mut u8, usize) -> usize;
static CT_FONT_FAMILIES_CB: AtomicU64 = AtomicU64::new(0);

/// Register the font families callback. Called by cg-shims at init.
pub fn set_ct_font_families_callback(f: FontFamiliesFn) {
    CT_FONT_FAMILIES_CB.store(f as *const () as u64, Ordering::Relaxed);
}

/// Register the text measurement callback. Called by cg-shims at init.
pub fn set_ct_text_measure_callback(f: TextMeasureFn) {
    CT_TEXT_MEASURE_CB.store(f as *const () as u64, Ordering::Relaxed);
}

fn measure_text_real(text: &str, font_name: &str, size: f64) -> Option<f64> {
    let cb = CT_TEXT_MEASURE_CB.load(Ordering::Relaxed);
    if cb == 0 { return None; }
    let f: TextMeasureFn = unsafe { std::mem::transmute(cb) };
    Some(f(text.as_ptr(), text.len(), font_name.as_ptr(), font_name.len(), size))
}

/// Char advance callback: `fn(ch: u32, font_name_ptr, font_name_len, size) -> f64`.
type CharAdvanceFn = extern "C" fn(u32, *const u8, usize, f64) -> f64;
static CT_CHAR_ADVANCE_CB: AtomicU64 = AtomicU64::new(0);

/// Register the per-character advance callback. Called by cg-shims at init.
pub fn set_ct_char_advance_callback(f: CharAdvanceFn) {
    CT_CHAR_ADVANCE_CB.store(f as *const () as u64, Ordering::Relaxed);
}

fn char_advance_real(ch: u32, font_name: &str, size: f64) -> Option<f64> {
    let cb = CT_CHAR_ADVANCE_CB.load(Ordering::Relaxed);
    if cb == 0 { return None; }
    let f: CharAdvanceFn = unsafe { std::mem::transmute(cb) };
    Some(f(ch, font_name.as_ptr(), font_name.len(), size))
}

// ---------------------------------------------------------------------------
// CtFont registry
// ---------------------------------------------------------------------------

struct CtFont {
    name: String,
    size: f64,
    ascent: f64,
    descent: f64,    // positive value (magnitude)
    leading: f64,
    x_height: f64,
    cap_height: f64,
    units_per_em: u32,
    avg_advance: f64, // average character advance width
}

impl CtFont {
    fn new(name: String, size: f64) -> Self {
        // Try real metrics from graphics-core via callback
        if let Some(m) = get_real_metrics(&name, size) {
            return Self {
                name,
                size,
                ascent: m.ascent,
                descent: m.descent.abs(), // store as positive
                leading: m.leading,
                x_height: m.x_height,
                cap_height: m.cap_height,
                units_per_em: m.units_per_em,
                avg_advance: m.advance_width,
            };
        }
        // Fallback: proportional estimates
        Self {
            name,
            size,
            ascent: size * 0.8,
            descent: size * 0.2,
            leading: size * 0.1,
            x_height: size * 0.5,
            cap_height: size * 0.7,
            units_per_em: 2048,
            avg_advance: size * 0.6,
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
            // -- CTFontManager --
            ("_CTFontManagerCopyAvailableFontFamilyNames", shim_ct_font_manager_copy_family_names as *const () as u64, "CTFontManagerCopyAvailableFontFamilyNames (real → CFArray)"),
            ("_CTFontManagerCopyAvailablePostScriptNames", shim_ct_font_manager_copy_family_names as *const () as u64, "CTFontManagerCopyAvailablePostScriptNames (real → CFArray)"),

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
            ("_CTRunGetGlyphCount", shim_ct_run_get_glyph_count as *const () as u64, "CTRunGetGlyphCount (real → character count)"),
            ("_CTRunGetPositions", shim_ct_run_get_positions as *const () as u64, "CTRunGetPositions (real → cursor positions)"),
            ("_CTRunGetGlyphs", shim_ct_run_get_glyphs as *const () as u64, "CTRunGetGlyphs (real → glyph indices)"),
            ("_CTRunGetStringRange", shim_ct_run_get_string_range as *const () as u64, "CTRunGetStringRange (real → {location, length})"),
            ("_CTRunGetAdvances", shim_ct_run_get_advances as *const () as u64, "CTRunGetAdvances (real → per-glyph advances)"),
            ("_CTRunGetAttributes", shim_ct_run_get_attributes as *const () as u64, "CTRunGetAttributes (real → attribute dict)"),
            ("_CTRunGetStringIndices", shim_ct_run_get_glyphs as *const () as u64, "CTRunGetStringIndices (no-op)"),
            ("_CTRunGetStatus", shim_ct_zero_stub as *const () as u64, "CTRunGetStatus (stub → 0)"),
            ("_CTRunGetTypographicBounds", shim_ct_run_get_typo_bounds as *const () as u64, "CTRunGetTypographicBounds (real → width/ascent/descent/leading)"),

            // -- CTLine extended --
            ("_CTLineDraw", shim_ct_line_draw as *const () as u64, "CTLineDraw (no-op)"),
            ("_CTLineGetGlyphRuns", shim_ct_line_get_glyph_runs as *const () as u64, "CTLineGetGlyphRuns (real → CFArray of CTRun)"),
            ("_CTLineGetGlyphCount", shim_ct_line_get_glyph_count as *const () as u64, "CTLineGetGlyphCount (real → text len)"),
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
            ("_CTFrameGetLines", shim_ct_frame_get_lines as *const () as u64, "CTFrameGetLines (real → CFArray of CTLine)"),
            ("_CTFrameGetLineOrigins", shim_ct_frame_get_line_origins as *const () as u64, "CTFrameGetLineOrigins (no-op)"),
            ("_CTFrameGetPath", shim_ct_null_stub as *const () as u64, "CTFrameGetPath (stub → NULL)"),
            ("_CTFrameGetVisibleStringRange", shim_ct_frame_get_visible_range as *const () as u64, "CTFrameGetVisibleStringRange (sret → {0,0})"),

            // -- CTFont extended --
            ("_CTFontCreateWithFontDescriptor", shim_ct_font_create_with_descriptor as *const () as u64, "CTFontCreateWithFontDescriptor (handle)"),
            ("_CTFontCreateCopyWithAttributes", shim_ct_font_create_copy as *const () as u64, "CTFontCreateCopyWithAttributes (handle)"),
            ("_CTFontCreateCopyWithSymbolicTraits", shim_ct_font_create_copy_traits as *const () as u64, "CTFontCreateCopyWithSymbolicTraits (handle)"),
            ("_CTFontCopyFamilyName", shim_ct_font_copy_family_name as *const () as u64, "CTFontCopyFamilyName (real → CFString)"),
            ("_CTFontCopyPostScriptName", shim_ct_font_copy_postscript_name as *const () as u64, "CTFontCopyPostScriptName (real → CFString)"),
            ("_CTFontCopyDisplayName", shim_ct_font_copy_display_name as *const () as u64, "CTFontCopyDisplayName (real → CFString)"),
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

/// CTFontManagerCopyAvailableFontFamilyNames() → CFArrayRef.
///
/// Returns all available font family names. Uses the font families callback
/// if registered (from cg-shims), otherwise falls back to hardcoded names.
pub extern "C" fn shim_ct_font_manager_copy_family_names() -> u64 {
    // Try dynamic enumeration via callback.
    let cb_val = CT_FONT_FAMILIES_CB.load(Ordering::Relaxed);
    if cb_val != 0 {
        let cb: FontFamiliesFn = unsafe { std::mem::transmute(cb_val) };
        // First call to get count.
        let count = cb(std::ptr::null_mut(), 0);
        if count > 0 {
            let mut ptrs = vec![std::ptr::null_mut::<u8>(); count];
            let actual = cb(ptrs.as_mut_ptr(), count);
            let array = unsafe { libc::calloc(1, 24) as u64 };
            if array == 0 { return 0; }
            let items = unsafe { libc::calloc(actual, 8) as *mut u64 };
            if items.is_null() { return array; }
            for (i, p) in ptrs.iter().take(actual).enumerate() {
                let str_obj = if p.is_null() {
                    0
                } else {
                    let c_str = *p as *const std::ffi::c_char;
                    let obj = alloc_cfstring(unsafe { std::ffi::CStr::from_ptr(c_str) }.to_str().unwrap_or(""));
                    unsafe { libc::free(*p as *mut std::ffi::c_void); }
                    obj
                };
                unsafe { items.add(i).write(str_obj); }
            }
            unsafe {
                *((array as usize + 8) as *mut u64) = actual as u64;
                *((array as usize + 16) as *mut u64) = items as u64;
            }
            return array;
        }
    }

    // Fallback: hardcoded embedded font names.
    static FAMILY_NAMES: &[&str] = &[
        "System", "System-Bold", "Monospace", "System-Italic", "System-BoldItalic",
        "DejaVu Sans", "DejaVu Sans Mono", "Helvetica", "Courier", "Times",
    ];

    let count = FAMILY_NAMES.len();
    let array = unsafe { libc::calloc(1, 24) as u64 };
    if array == 0 { return 0; }

    let items = unsafe { libc::calloc(count, 8) as *mut u64 };
    if items.is_null() { return array; }

    for (i, name) in FAMILY_NAMES.iter().enumerate() {
        let str_obj = alloc_cfstring(name);
        unsafe { items.add(i).write(str_obj); }
    }
    unsafe {
        *((array as usize + 8) as *mut u64) = count as u64;
        *((array as usize + 16) as *mut u64) = items as u64;
    }
    array
}

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

/// `CTFontCopyFullName(font: u64) -> CFStringRef`  — returns the font name as a CFString.
pub extern "C" fn shim_ct_font_copy_full_name(font: u64) -> u64 {
    let name = with_registry(|map| map.get(&font).map(|f| f.name.clone()));
    match name {
        Some(n) => alloc_cfstring(&n),
        None => 0,
    }
}

/// Allocate a minimal CFString-like object: [isa:8][flags:8][cstr_ptr:8][len:8].
fn alloc_cfstring(s: &str) -> u64 {
    let cstr = match std::ffi::CString::new(s) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let len = s.len();
    let obj = unsafe { libc::calloc(1, 32) as u64 };
    if obj != 0 {
        let ptr = cstr.into_raw();
        unsafe {
            *((obj as usize + 16) as *mut u64) = ptr as u64;
            *((obj as usize + 24) as *mut u64) = len as u64;
        }
    }
    obj
}

/// `CTFontCopyFamilyName(font: u64) -> CFStringRef`
pub extern "C" fn shim_ct_font_copy_family_name(font: u64) -> u64 {
    shim_ct_font_copy_full_name(font)
}

/// `CTFontCopyPostScriptName(font: u64) -> CFStringRef`
pub extern "C" fn shim_ct_font_copy_postscript_name(font: u64) -> u64 {
    shim_ct_font_copy_full_name(font)
}

/// `CTFontCopyDisplayName(font: u64) -> CFStringRef`
pub extern "C" fn shim_ct_font_copy_display_name(font: u64) -> u64 {
    shim_ct_font_copy_full_name(font)
}

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
    font_name: String,
    font_size: f64,
    run_handles: Vec<u64>,      // CTRun handles for this line
    runs_cfarray: u64,          // cached CFArray for CTLineGetGlyphRuns
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
/// and stores it for CTLineDraw. Also builds a single CTRun for the entire text.
pub extern "C" fn shim_ct_line_create(attr_str: u64) -> u64 {
    let handle = new_handle();
    let mut text = String::new();
    let mut font_size = 12.0;
    let mut font_name = "System".to_string();

    if attr_str != 0 {
        // NSAttributedString layout: [0]=isa, [8]=string_ptr (NSString)
        let string_ptr = unsafe { *((attr_str as usize + 8) as *const u64) };
        if string_ptr != 0 {
            let buf_ptr = unsafe { *((string_ptr as usize + 16) as *const u64) };
            if buf_ptr > 0x1000 {
                if let Ok(cstr) = unsafe { std::ffi::CStr::from_ptr(buf_ptr as *const std::ffi::c_char) }.to_str() {
                    text = cstr.to_string();
                }
            }
        }
        // Try to read font from attributes
        let attrs_count = unsafe { *((attr_str as usize + 16) as *const u64) };
        if attrs_count > 0 {
            let attrs_ptr = unsafe { *((attr_str as usize + 24) as *const u64) };
            if attrs_ptr > 0x1000 {
                let dict_ptr = unsafe { *((attrs_ptr as usize + 16) as *const u64) };
                if dict_ptr > 0x1000 {
                    let info = with_registry(|map| {
                        map.values().next().map(|f| (f.name.clone(), f.size))
                    });
                    if let Some((name, size)) = info {
                        font_name = name;
                        font_size = size;
                    }
                }
            }
        }
    }

    // Build a single CTRun for the entire line text.
    let mut run_handles = Vec::new();
    let runs_cfarray;
    if !text.is_empty() {
        let (rh, run) = build_run(&text, &font_name, font_size, 0);
        with_run_registry(|map| { map.insert(rh, run); });
        run_handles.push(rh);
        runs_cfarray = make_cfarray_of_handles(&run_handles);
    } else {
        runs_cfarray = make_cfarray_of_handles(&[]);
    }

    with_line_registry(|map| {
        map.insert(handle, CtLine { text, font_name, font_size, run_handles, runs_cfarray });
    });
    handle
}

/// Build a minimal CFArray holding the given handles.
/// Layout: [isa:8][count:8][items_ptr:8].
fn make_cfarray_of_handles(handles: &[u64]) -> u64 {
    let array = unsafe { libc::calloc(1, 24) as u64 };
    if array == 0 { return 0; }
    let count = handles.len();
    let items = if count > 0 {
        let p = unsafe { libc::calloc(count, 8) as *mut u64 };
        if p.is_null() { return array; }
        for (i, &h) in handles.iter().enumerate() {
            unsafe { p.add(i).write(h); }
        }
        p as u64
    } else {
        0
    };
    unsafe {
        *((array as usize + 8) as *mut u64) = count as u64;
        *((array as usize + 16) as *mut u64) = items;
    }
    array
}

// ---------------------------------------------------------------------------
// CTRun registry — stores per-run glyph data
// ---------------------------------------------------------------------------

struct CtRun {
    text: String,            // the substring for this run
    font_name: String,
    font_size: f64,
    location: i64,           // start offset in the parent string
    glyphs: Vec<u16>,        // one glyph per character (identity-mapped)
    advances: Vec<f64>,      // per-glyph advance width
    positions: Vec<(f64, f64)>, // per-glyph (x, y) positions
    total_width: f64,
    ascent: f64,
    descent: f64,
    leading: f64,
}

static RUN_REGISTRY: Mutex<Option<HashMap<u64, CtRun>>> = Mutex::new(None);

fn with_run_registry<R>(f: impl FnOnce(&mut HashMap<u64, CtRun>) -> R) -> R {
    let mut guard = RUN_REGISTRY.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

/// Build a CTRun from a text segment with a given font.
fn build_run(text: &str, font_name: &str, font_size: f64, location: i64) -> (u64, CtRun) {
    let handle = new_handle();
    let chars: Vec<char> = text.chars().collect();
    let mut glyphs = Vec::with_capacity(chars.len());
    let mut advances = Vec::with_capacity(chars.len());
    let mut positions = Vec::with_capacity(chars.len());
    let mut x_cursor = 0.0_f64;

    let (ascent, descent, leading) = if let Some(m) = get_real_metrics(font_name, font_size) {
        (m.ascent, m.descent.abs(), m.leading)
    } else {
        (font_size * 0.8, font_size * 0.2, font_size * 0.1)
    };

    for ch in &chars {
        let glyph = *ch as u16;
        glyphs.push(glyph);

        let adv = char_advance_real(*ch as u32, font_name, font_size)
            .unwrap_or(font_size * 0.6);
        advances.push(adv);
        positions.push((x_cursor, 0.0));
        x_cursor += adv;
    }

    let run = CtRun {
        text: text.to_string(),
        font_name: font_name.to_string(),
        font_size,
        location,
        glyphs,
        advances,
        positions,
        total_width: x_cursor,
        ascent,
        descent,
        leading,
    };
    (handle, run)
}

// ---------------------------------------------------------------------------
// CTRun
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_run_get_glyph_count(run: u64) -> i64 {
    with_run_registry(|map| {
        map.get(&run).map(|r| r.glyphs.len() as i64).unwrap_or(0)
    })
}

pub extern "C" fn shim_ct_run_get_positions(run: u64, range_loc: i64, range_len: i64, positions: u64) {
    if positions == 0 { return; }
    with_run_registry(|map| {
        if let Some(r) = map.get(&run) {
            let (start, count) = resolve_range(range_loc, range_len, r.positions.len());
            for i in 0..count {
                let (x, y) = r.positions[start + i];
                unsafe {
                    let p = (positions as *mut f64).add(i * 2);
                    *p = x;
                    *p.add(1) = y;
                }
            }
        }
    });
}

pub extern "C" fn shim_ct_run_get_glyphs(run: u64, range_loc: i64, range_len: i64, glyphs: u64) {
    if glyphs == 0 { return; }
    with_run_registry(|map| {
        if let Some(r) = map.get(&run) {
            let (start, count) = resolve_range(range_loc, range_len, r.glyphs.len());
            for i in 0..count {
                unsafe { *((glyphs as *mut u16).add(i)) = r.glyphs[start + i]; }
            }
        }
    });
}

/// CTRunGetStringRange(run) → CFRange (sret: {location, length}).
pub extern "C" fn shim_ct_run_get_string_range(sret: u64, run: u64) {
    if sret == 0 { return; }
    with_run_registry(|map| {
        if let Some(r) = map.get(&run) {
            unsafe {
                *(sret as *mut i64) = r.location;
                *((sret as usize + 8) as *mut i64) = r.glyphs.len() as i64;
            }
        } else {
            unsafe {
                *(sret as *mut i64) = 0;
                *((sret as usize + 8) as *mut i64) = 0;
            }
        }
    });
}

pub extern "C" fn shim_ct_run_get_advances(run: u64, range_loc: i64, range_len: i64, advances: u64) {
    if advances == 0 { return; }
    with_run_registry(|map| {
        if let Some(r) = map.get(&run) {
            let (start, count) = resolve_range(range_loc, range_len, r.advances.len());
            for i in 0..count {
                unsafe {
                    let p = (advances as *mut f64).add(i * 2);
                    *p = r.advances[start + i];
                    *p.add(1) = 0.0; // CGSize height = 0
                }
            }
        }
    });
}

/// CTRunGetAttributes(run) → CFDictionaryRef.
/// Returns a minimal dictionary with the font handle.
pub extern "C" fn shim_ct_run_get_attributes(run: u64) -> u64 {
    with_run_registry(|map| {
        if let Some(r) = map.get(&run) {
            // Find or create a font handle for this run's font
            let font_handle = with_registry(|fmap| {
                for (&h, f) in fmap.iter() {
                    if f.name == r.font_name && (f.size - r.font_size).abs() < 0.01 {
                        return h;
                    }
                }
                let h = new_handle();
                fmap.insert(h, CtFont::new(r.font_name.clone(), r.font_size));
                h
            });
            // Return a tiny dictionary: just the font attribute
            let dict = unsafe { libc::calloc(1, 24) as u64 };
            if dict != 0 {
                let items = unsafe { libc::calloc(2, 8) as *mut u64 };
                if !items.is_null() {
                    unsafe {
                        *items = KCTFONT_ATTR_NAME;
                        *items.add(1) = font_handle;
                        *((dict as usize + 8) as *mut u64) = 1; // count
                        *((dict as usize + 16) as *mut u64) = items as u64;
                    }
                }
            }
            dict
        } else {
            0
        }
    })
}

/// CTRunGetTypographicBounds(run, range, ascent_out, descent_out, leading_out) → f64 width.
pub extern "C" fn shim_ct_run_get_typo_bounds(run: u64, range_loc: i64, range_len: i64,
    ascent: u64, descent: u64, leading: u64) -> u64 {
    with_run_registry(|map| {
        if let Some(r) = map.get(&run) {
            let (start, count) = resolve_range(range_loc, range_len, r.advances.len());
            let width: f64 = r.advances[start..start + count].iter().sum();
            if ascent != 0 { unsafe { *(ascent as *mut f64) = r.ascent; } }
            if descent != 0 { unsafe { *(descent as *mut f64) = r.descent; } }
            if leading != 0 { unsafe { *(leading as *mut f64) = r.leading; } }
            width.to_bits()
        } else {
            if ascent != 0 { unsafe { *(ascent as *mut f64) = 0.0; } }
            if descent != 0 { unsafe { *(descent as *mut f64) = 0.0; } }
            if leading != 0 { unsafe { *(leading as *mut f64) = 0.0; } }
            0.0_f64.to_bits()
        }
    })
}

/// Resolve a CFRange {loc, len} against a total count.
/// If loc==0 && len==0, that means "entire range" in CoreText convention.
fn resolve_range(loc: i64, len: i64, total: usize) -> (usize, usize) {
    if loc == 0 && len == 0 {
        (0, total)
    } else {
        let start = loc.max(0) as usize;
        let count = if len <= 0 { total.saturating_sub(start) } else { len as usize };
        let count = count.min(total.saturating_sub(start));
        (start, count)
    }
}

// ---------------------------------------------------------------------------
// CTLine extended
// ---------------------------------------------------------------------------

/// CTLineDraw(line, context) — renders the stored text via CG context.
///
/// Uses the registered draw callback if available, otherwise no-op.
/// Passes font name to the callback for proper font resolution.
pub extern "C" fn shim_ct_line_draw(line: u64, context: u64) {
    let line_data = with_line_registry(|map| {
        map.get(&line).map(|l| (l.text.clone(), l.font_name.clone(), l.font_size))
    });
    if let Some((text, font_name, font_size)) = line_data {
        if !text.is_empty() && context != 0 {
            let cb = CT_LINE_DRAW_CB.load(std::sync::atomic::Ordering::Relaxed);
            if cb != 0 {
                // Call: fn(ctx, text_ptr, text_len, font_name_ptr, font_name_len, font_size)
                let f: extern "C" fn(u64, *const u8, usize, *const u8, usize, f64) =
                    unsafe { std::mem::transmute(cb) };
                f(context, text.as_ptr(), text.len(), font_name.as_ptr(), font_name.len(), font_size);
            }
        }
    }
}

/// Callback for CTLineDraw to render text through CG context.
/// Set by cg-shims at initialization time.
/// Signature: fn(ctx, text_ptr, text_len, font_name_ptr, font_name_len, font_size)
static CT_LINE_DRAW_CB: AtomicU64 = AtomicU64::new(0);

/// Register the CTLineDraw rendering callback.
/// Called by cg-shims to wire CTLineDraw → CGContext text rendering.
pub fn set_ct_line_draw_callback(f: extern "C" fn(u64, *const u8, usize, *const u8, usize, f64)) {
    CT_LINE_DRAW_CB.store(f as *const () as u64, Ordering::Relaxed);
}

/// CTLineGetStringRange(line) → CFRange (sret).
pub extern "C" fn shim_ct_line_get_string_range(sret: u64, line: u64) {
    if sret == 0 { return; }
    let len = with_line_registry(|map| {
        map.get(&line).map(|l| l.text.len() as i64).unwrap_or(0)
    });
    unsafe {
        *(sret as *mut i64) = 0;
        *((sret as usize + 8) as *mut i64) = len;
    }
}

/// CTLineGetGlyphRuns(line) → CFArrayRef of CTRunRef handles.
pub extern "C" fn shim_ct_line_get_glyph_runs(line: u64) -> u64 {
    with_line_registry(|map| {
        map.get(&line).map(|l| l.runs_cfarray).unwrap_or(0)
    })
}

/// CTLineGetTypographicBounds(line, ascent_out, descent_out, leading_out) → f64 width.
///
/// Uses real text measurement when the callback is available.
pub extern "C" fn shim_ct_line_get_typo_bounds(line: u64, ascent: u64, descent: u64, leading: u64) -> u64 {
    let data = with_line_registry(|map| {
        map.get(&line).map(|l| (l.text.clone(), l.font_name.clone(), l.font_size))
    });
    let (text, font_name, font_size) = data.unwrap_or((String::new(), "System".into(), 12.0));
    let fs = if font_size > 0.0 { font_size } else { 12.0 };

    // Get metrics for ascent/descent/leading
    if let Some(m) = get_real_metrics(&font_name, fs) {
        if ascent != 0 { unsafe { *(ascent as *mut f64) = m.ascent; } }
        if descent != 0 { unsafe { *(descent as *mut f64) = m.descent.abs(); } }
        if leading != 0 { unsafe { *(leading as *mut f64) = m.leading; } }
    } else {
        if ascent != 0 { unsafe { *(ascent as *mut f64) = fs * 0.8; } }
        if descent != 0 { unsafe { *(descent as *mut f64) = fs * 0.2; } }
        if leading != 0 { unsafe { *(leading as *mut f64) = fs * 0.1; } }
    }

    // Measure text width
    let width = measure_text_real(&text, &font_name, fs)
        .unwrap_or_else(|| text.len() as f64 * fs * 0.6);
    width.to_bits()
}

/// CTLineGetBoundsWithOptions(line, options) → CGRect (sret, 32 bytes via x8).
pub extern "C" fn shim_ct_line_get_bounds(sret: u64, line: u64, _options: u64) {
    if sret == 0 { return; }
    let data = with_line_registry(|map| {
        map.get(&line).map(|l| (l.text.clone(), l.font_name.clone(), l.font_size))
    });
    let (text, font_name, font_size) = data.unwrap_or((String::new(), "System".into(), 12.0));
    let fs = if font_size > 0.0 { font_size } else { 12.0 };

    let width = measure_text_real(&text, &font_name, fs)
        .unwrap_or_else(|| text.len() as f64 * fs * 0.6);

    let (asc, desc) = if let Some(m) = get_real_metrics(&font_name, fs) {
        (m.ascent, m.descent.abs())
    } else {
        (fs * 0.8, fs * 0.2)
    };

    unsafe {
        let p = sret as *mut f64;
        *p = 0.0;           // x
        *p.add(1) = -desc;  // y (below baseline)
        *p.add(2) = width;  // width
        *p.add(3) = asc + desc; // height
    }
}

/// CTLineGetOffsetForStringIndex(line, index, secondary_offset_out) → f64.
pub extern "C" fn shim_ct_line_get_offset(line: u64, idx: i64, _secondary: u64) -> u64 {
    // Measure text from start up to idx
    if idx <= 0 { return 0.0_f64.to_bits(); }
    let data = with_line_registry(|map| {
        map.get(&line).map(|l| (l.text.clone(), l.font_name.clone(), l.font_size))
    });
    if let Some((text, font_name, font_size)) = data {
        let prefix: String = text.chars().take(idx as usize).collect();
        let offset = measure_text_real(&prefix, &font_name, font_size)
            .unwrap_or_else(|| prefix.len() as f64 * font_size * 0.6);
        return offset.to_bits();
    }
    0.0_f64.to_bits()
}

/// CTLineGetGlyphCount(line) → CFIndex.
pub extern "C" fn shim_ct_line_get_glyph_count(line: u64) -> i64 {
    with_line_registry(|map| {
        map.get(&line).map(|l| l.text.chars().count() as i64).unwrap_or(0)
    })
}

/// CTLineCreateTruncatedLine / CTLineCreateJustifiedLine — returns the same line.
pub extern "C" fn shim_ct_line_create_truncated(line: u64, _width: u64, _type: u32, _token: u64) -> u64 {
    if line != 0 { line } else { new_handle() }
}

// ---------------------------------------------------------------------------
// CTFramesetter / CTFrame
// ---------------------------------------------------------------------------

/// Framesetter registry: stores the attributed string text + font info.
struct CtFramesetter {
    text: String,
    font_name: String,
    font_size: f64,
}

static FRAMESETTER_REGISTRY: Mutex<Option<HashMap<u64, CtFramesetter>>> = Mutex::new(None);

fn with_framesetter_registry<R>(f: impl FnOnce(&mut HashMap<u64, CtFramesetter>) -> R) -> R {
    let mut guard = FRAMESETTER_REGISTRY.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

/// CTFramesetterCreateWithAttributedString(attrStr) → CTFramesetterRef.
pub extern "C" fn shim_ct_framesetter_create(attr_str: u64) -> u64 {
    let handle = new_handle();
    // Reuse the same text extraction as CTLine
    let mut text = String::new();
    let mut font_name = "System".to_string();
    let mut font_size = 12.0;
    if attr_str != 0 {
        let string_ptr = unsafe { *((attr_str as usize + 8) as *const u64) };
        if string_ptr != 0 {
            let buf_ptr = unsafe { *((string_ptr as usize + 16) as *const u64) };
            if buf_ptr > 0x1000 {
                if let Ok(cstr) = unsafe { std::ffi::CStr::from_ptr(buf_ptr as *const std::ffi::c_char) }.to_str() {
                    text = cstr.to_string();
                }
            }
        }
        let info = with_registry(|map| {
            map.values().next().map(|f| (f.name.clone(), f.size))
        });
        if let Some((name, size)) = info {
            font_name = name;
            font_size = size;
        }
    }
    with_framesetter_registry(|map| {
        map.insert(handle, CtFramesetter { text, font_name, font_size });
    });
    handle
}

// ---------------------------------------------------------------------------
// CTFrame registry — stores line handles for multi-line layout
// ---------------------------------------------------------------------------

struct CtFrame {
    line_handles: Vec<u64>,     // CTLine handles
    line_origins: Vec<(f64, f64)>, // (x, y) origin for each line
    lines_cfarray: u64,         // cached CFArray
    text: String,
    font_name: String,
    font_size: f64,
}

static FRAME_REGISTRY: Mutex<Option<HashMap<u64, CtFrame>>> = Mutex::new(None);

fn with_frame_registry<R>(f: impl FnOnce(&mut HashMap<u64, CtFrame>) -> R) -> R {
    let mut guard = FRAME_REGISTRY.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

/// CTFramesetterCreateFrame(framesetter, string_range, path, frame_attrs) → CTFrame.
///
/// Performs simple line-breaking: splits text on newlines and produces one
/// CTLine per visual line. Each CTLine has its own CTRun with glyph data.
pub extern "C" fn shim_ct_framesetter_create_frame(
    framesetter: u64, _range_loc: i64, _range_len: i64, _path: u64, _attrs: u64
) -> u64 {
    let handle = new_handle();
    let data = with_framesetter_registry(|map| {
        map.get(&framesetter).map(|f| (f.text.clone(), f.font_name.clone(), f.font_size))
    });
    let (text, font_name, font_size) = data.unwrap_or((String::new(), "System".into(), 12.0));

    let line_height = if let Some(m) = get_real_metrics(&font_name, font_size) {
        m.ascent + m.descent.abs() + m.leading
    } else {
        font_size * 1.2
    };

    // Split on newlines to create lines.
    let lines: Vec<&str> = if text.is_empty() {
        vec![]
    } else {
        text.split('\n').collect()
    };

    let mut line_handles = Vec::with_capacity(lines.len());
    let mut line_origins = Vec::with_capacity(lines.len());
    let mut char_offset: i64 = 0;

    for (i, line_text) in lines.iter().enumerate() {
        let line_handle = new_handle();

        // Build runs for this line
        let mut run_handles = Vec::new();
        let runs_cfarray;
        if !line_text.is_empty() {
            let (rh, run) = build_run(line_text, &font_name, font_size, char_offset);
            with_run_registry(|map| { map.insert(rh, run); });
            run_handles.push(rh);
            runs_cfarray = make_cfarray_of_handles(&run_handles);
        } else {
            runs_cfarray = make_cfarray_of_handles(&[]);
        }

        with_line_registry(|map| {
            map.insert(line_handle, CtLine {
                text: line_text.to_string(),
                font_name: font_name.clone(),
                font_size,
                run_handles,
                runs_cfarray,
            });
        });

        line_handles.push(line_handle);
        // Origin: x=0, y = top - line_height * (i+1) + ascent
        // (CoreText uses flipped coordinates with origin at bottom-left)
        line_origins.push((0.0, -(i as f64) * line_height));

        char_offset += line_text.len() as i64 + 1; // +1 for the newline
    }

    let lines_cfarray = make_cfarray_of_handles(&line_handles);

    with_frame_registry(|map| {
        map.insert(handle, CtFrame {
            line_handles,
            line_origins,
            lines_cfarray,
            text,
            font_name,
            font_size,
        });
    });
    handle
}

/// CTFramesetterSuggestFrameSizeWithConstraints(fs, range, attrs, constraints, fit_range_out) → CGSize (sret).
///
/// Measures the text and returns a real suggested size based on font metrics.
pub extern "C" fn shim_ct_framesetter_suggest_size(
    sret: u64, fs: u64, _range_loc: i64, _range_len: i64, _attrs: u64,
    _cw: f64, _ch: f64, _fit_range: u64,
) {
    if sret == 0 { return; }
    let data = with_framesetter_registry(|map| {
        map.get(&fs).map(|f| (f.text.clone(), f.font_name.clone(), f.font_size))
    });
    let (text, font_name, font_size) = data.unwrap_or((String::new(), "System".into(), 12.0));

    let width = measure_text_real(&text, &font_name, font_size)
        .unwrap_or_else(|| text.len() as f64 * font_size * 0.6);
    let height = if let Some(m) = get_real_metrics(&font_name, font_size) {
        m.ascent + m.descent.abs() + m.leading
    } else {
        font_size * 1.2
    };
    unsafe {
        let p = sret as *mut f64;
        *p = width;
        *p.add(1) = height;
    }
}

/// CTFrameDraw(frame, context) — draws all lines via CTLineDraw.
pub extern "C" fn shim_ct_frame_draw(frame: u64, context: u64) {
    if context == 0 { return; }
    let line_handles = with_frame_registry(|map| {
        map.get(&frame).map(|f| f.line_handles.clone()).unwrap_or_default()
    });
    for lh in line_handles {
        shim_ct_line_draw(lh, context);
    }
}

/// CTFrameGetLines(frame) → CFArrayRef of CTLineRef handles.
pub extern "C" fn shim_ct_frame_get_lines(frame: u64) -> u64 {
    with_frame_registry(|map| {
        map.get(&frame).map(|f| f.lines_cfarray).unwrap_or(0)
    })
}

pub extern "C" fn shim_ct_frame_get_line_origins(
    frame: u64, range_loc: i64, range_len: i64, origins: u64
) {
    if origins == 0 { return; }
    with_frame_registry(|map| {
        if let Some(f) = map.get(&frame) {
            let (start, count) = resolve_range(range_loc, range_len, f.line_origins.len());
            for i in 0..count {
                let (x, y) = f.line_origins[start + i];
                unsafe {
                    let p = (origins as *mut f64).add(i * 2);
                    *p = x;
                    *p.add(1) = y;
                }
            }
        }
    });
}

/// CTFrameGetVisibleStringRange(frame) → CFRange (sret).
pub extern "C" fn shim_ct_frame_get_visible_range(sret: u64, frame: u64) {
    if sret == 0 { return; }
    let len = with_frame_registry(|map| {
        map.get(&frame).map(|f| f.text.len() as i64).unwrap_or(0)
    });
    unsafe {
        *(sret as *mut i64) = 0;
        *((sret as usize + 8) as *mut i64) = len;
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
///
/// Uses real per-character advance widths when the char advance callback is available.
/// Falls back to `avg_advance` from the font registry otherwise.
pub extern "C" fn shim_ct_font_get_advances(
    font: u64, _orientation: u32, glyphs: u64, advances: u64, count: i64,
) -> u64 {
    let c = count.max(0) as usize;
    let (font_name, font_size, fallback_advance) = with_registry(|map| {
        map.get(&font).map_or(
            ("System".to_string(), 12.0, 7.2),
            |f| (f.name.clone(), f.size, f.avg_advance),
        )
    });

    let mut total = 0.0_f64;
    for i in 0..c {
        let advance = if glyphs != 0 {
            let glyph_code = unsafe { *(glyphs as *const u16).add(i) } as u32;
            // Try real char advance if callback available
            char_advance_real(glyph_code, &font_name, font_size)
                .unwrap_or(fallback_advance)
        } else {
            fallback_advance
        };
        total += advance;
        if advances != 0 {
            unsafe {
                // Each advance is CGSize = {width: f64, height: f64}
                let p = (advances as *mut f64).add(i * 2);
                *p = advance;
                *p.add(1) = 0.0;
            }
        }
    }
    total.to_bits()
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
    fn test_font_manager_copy_family_names() {
        let arr = shim_ct_font_manager_copy_family_names();
        assert_ne!(arr, 0, "should return non-null CFArray");
        let count = unsafe { *((arr as usize + 8) as *const u64) };
        assert!(count >= 5, "should have at least 5 font family names, got {count}");
    }

    // ── Font name tests ─────────────────────────────────────────

    #[test]
    fn test_font_copy_full_name_returns_cfstring() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(12.0), 0);
        let name_obj = shim_ct_font_copy_full_name(h);
        // No callback → name should be "Helvetica" (the default when name_ptr=0)
        assert_ne!(name_obj, 0, "should return a non-null CFString");
        // Read the string from offset 16
        let cstr_ptr = unsafe { *((name_obj as usize + 16) as *const u64) };
        assert_ne!(cstr_ptr, 0);
        let cstr = unsafe { std::ffi::CStr::from_ptr(cstr_ptr as *const std::ffi::c_char) };
        let name = cstr.to_str().unwrap();
        assert_eq!(name, "Helvetica");
    }

    #[test]
    fn test_font_copy_full_name_null_font_returns_null() {
        // Non-existent handle
        assert_eq!(shim_ct_font_copy_full_name(0xDEAD_BEEF), 0);
    }

    #[test]
    fn test_font_copy_family_name() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(12.0), 0);
        let name_obj = shim_ct_font_copy_family_name(h);
        assert_ne!(name_obj, 0);
    }

    // ── Font creation variants ──────────────────────────────────

    #[test]
    fn test_font_create_with_descriptor() {
        let desc = shim_ct_descriptor_create(0);
        let h = shim_ct_font_create_with_descriptor(desc, f64::to_bits(18.0), 0);
        assert_ne!(h, 0);
        let size = f64::from_bits(shim_ct_font_get_size(h));
        assert!((size - 18.0).abs() < 1e-9);
    }

    #[test]
    fn test_font_create_copy_preserves_size() {
        let h1 = shim_ct_font_create_with_name(0, f64::to_bits(16.0), 0);
        // Copy with size=0 should preserve the original size
        let h2 = shim_ct_font_create_copy(h1, f64::to_bits(0.0), 0, 0);
        let s2 = f64::from_bits(shim_ct_font_get_size(h2));
        assert!((s2 - 16.0).abs() < 1e-9, "copy should preserve size, got {s2}");
    }

    #[test]
    fn test_font_create_copy_with_new_size() {
        let h1 = shim_ct_font_create_with_name(0, f64::to_bits(16.0), 0);
        let h2 = shim_ct_font_create_copy(h1, f64::to_bits(24.0), 0, 0);
        let s2 = f64::from_bits(shim_ct_font_get_size(h2));
        assert!((s2 - 24.0).abs() < 1e-9, "copy should use new size 24, got {s2}");
    }

    #[test]
    fn test_font_create_copy_with_traits() {
        let h1 = shim_ct_font_create_with_name(0, f64::to_bits(14.0), 0);
        let h2 = shim_ct_font_create_copy_traits(h1, f64::to_bits(0.0), 0, 0x0002, 0x0002);
        assert_ne!(h2, 0);
    }

    // ── Font metrics (fallback) ─────────────────────────────────

    #[test]
    fn test_font_metrics_all_positive() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(16.0), 0);
        let ascent = f64::from_bits(shim_ct_font_get_ascent(h));
        let descent = f64::from_bits(shim_ct_font_get_descent(h));
        let leading = f64::from_bits(shim_ct_font_get_leading(h));
        let x_height = f64::from_bits(shim_ct_font_get_x_height(h));
        let cap_height = f64::from_bits(shim_ct_font_get_cap_height(h));
        assert!(ascent > 0.0, "ascent should be positive");
        assert!(descent > 0.0, "descent magnitude should be positive");
        assert!(leading >= 0.0, "leading should be non-negative");
        assert!(x_height > 0.0, "x_height should be positive");
        assert!(cap_height > 0.0, "cap_height should be positive");
        assert!(ascent > descent, "ascent > descent");
        assert!(cap_height > x_height, "cap_height > x_height");
    }

    #[test]
    fn test_font_bounding_box() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(12.0), 0);
        let mut rect = [0.0_f64; 4];
        shim_ct_font_get_bounding_box(h, rect.as_mut_ptr() as u64);
        let [x, y, w, height] = rect;
        assert!((x - 0.0).abs() < 1e-9);
        assert!(y < 0.0, "y should be negative (below baseline)");
        assert!(w > 0.0, "width should be positive");
        assert!(height > 0.0, "height should be positive");
    }

    #[test]
    fn test_font_get_matrix_identity() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(12.0), 0);
        let mut m = [0.0_f64; 6];
        shim_ct_font_get_matrix(m.as_mut_ptr() as u64, h);
        assert!((m[0] - 1.0).abs() < 1e-9, "a=1");
        assert!((m[1] - 0.0).abs() < 1e-9, "b=0");
        assert!((m[2] - 0.0).abs() < 1e-9, "c=0");
        assert!((m[3] - 1.0).abs() < 1e-9, "d=1");
    }

    #[test]
    fn test_font_underline_position() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(20.0), 0);
        let pos = f64::from_bits(shim_ct_font_get_underline_pos(h));
        let thickness = f64::from_bits(shim_ct_font_get_underline_thickness(h));
        assert!(pos < 0.0, "underline position below baseline: {pos}");
        assert!(thickness > 0.0, "underline thickness positive: {thickness}");
    }

    // ── Font advances ───────────────────────────────────────────

    #[test]
    fn test_font_get_advances_fills_array() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(12.0), 0);
        let glyphs: [u16; 3] = [65, 66, 67]; // A, B, C
        let mut advances = [0.0_f64; 6]; // 3 CGSize = 6 f64
        let total = f64::from_bits(shim_ct_font_get_advances(
            h, 0, glyphs.as_ptr() as u64, advances.as_mut_ptr() as u64, 3
        ));
        assert!(total > 0.0, "total advance should be positive: {total}");
        assert!(advances[0] > 0.0, "first advance width should be positive");
        assert!((advances[1] - 0.0).abs() < 1e-9, "first advance height should be 0");
    }

    #[test]
    fn test_font_get_glyphs_for_chars() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(12.0), 0);
        let chars: [u16; 3] = [0x41, 0x42, 0x43]; // A, B, C
        let mut glyphs = [0u16; 3];
        let ok = shim_ct_font_get_glyphs_for_chars(
            h, chars.as_ptr() as u64, glyphs.as_mut_ptr() as u64, 3
        );
        assert_eq!(ok, 1, "should return true");
        // Identity mapping: glyph = char code
        assert_eq!(glyphs[0], 0x41);
        assert_eq!(glyphs[1], 0x42);
    }

    // ── CTLine tests ────────────────────────────────────────────

    #[test]
    fn test_line_get_glyph_count() {
        // Can't easily set up a real attributed string in unit tests,
        // so test with a NULL attributed string → 0 glyphs
        let h = shim_ct_line_create(0);
        let count = shim_ct_line_get_glyph_count(h);
        assert_eq!(count, 0, "line from null attr string should have 0 glyphs");
    }

    #[test]
    fn test_line_get_string_range_empty() {
        let h = shim_ct_line_create(0);
        let mut range = [0i64; 2];
        shim_ct_line_get_string_range(range.as_mut_ptr() as u64, h);
        assert_eq!(range[0], 0, "location should be 0");
        assert_eq!(range[1], 0, "length should be 0 for empty line");
    }

    #[test]
    fn test_line_get_typo_bounds_empty() {
        let h = shim_ct_line_create(0);
        let mut asc = 0.0_f64;
        let mut desc = 0.0_f64;
        let mut lead = 0.0_f64;
        let width_bits = shim_ct_line_get_typo_bounds(
            h, &mut asc as *mut f64 as u64, &mut desc as *mut f64 as u64, &mut lead as *mut f64 as u64,
        );
        let width = f64::from_bits(width_bits);
        // Empty text → width should be 0
        assert!((width - 0.0).abs() < 1e-9, "empty line width should be 0, got {width}");
        assert!(asc > 0.0, "ascent should be positive even for empty line");
    }

    #[test]
    fn test_line_get_offset_zero() {
        let h = shim_ct_line_create(0);
        let offset = f64::from_bits(shim_ct_line_get_offset(h, 0, 0));
        assert!((offset - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_line_create_truncated_returns_same() {
        let h = shim_ct_line_create(0);
        let h2 = shim_ct_line_create_truncated(h, f64::to_bits(100.0), 0, 0);
        assert_eq!(h, h2, "truncated should return same handle");
    }

    // ── CTRun tests ─────────────────────────────────────────────

    #[test]
    fn test_run_get_string_range_zero() {
        let mut range = [0i64; 2];
        shim_ct_run_get_string_range(range.as_mut_ptr() as u64, 0);
        assert_eq!(range[0], 0);
        assert_eq!(range[1], 0);
    }

    #[test]
    fn test_run_get_typo_bounds_zero() {
        let width = f64::from_bits(shim_ct_run_get_typo_bounds(0, 0, 0, 0, 0, 0));
        assert!((width - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_build_run_produces_glyph_data() {
        let (h, run) = build_run("ABC", "System", 12.0, 0);
        assert_ne!(h, 0);
        assert_eq!(run.glyphs.len(), 3);
        assert_eq!(run.advances.len(), 3);
        assert_eq!(run.positions.len(), 3);
        assert_eq!(run.glyphs[0], 'A' as u16);
        assert_eq!(run.glyphs[1], 'B' as u16);
        assert_eq!(run.glyphs[2], 'C' as u16);
        // Positions should be monotonically increasing
        assert!((run.positions[0].0 - 0.0).abs() < 1e-9, "first position x=0");
        assert!(run.positions[1].0 > 0.0, "second position x > 0");
        assert!(run.positions[2].0 > run.positions[1].0, "third position > second");
        assert!(run.total_width > 0.0, "total width > 0");
        assert!(run.ascent > 0.0);
        assert!(run.descent > 0.0);
    }

    #[test]
    fn test_run_registry_glyph_count() {
        let (h, run) = build_run("Hello", "System", 14.0, 5);
        with_run_registry(|map| { map.insert(h, run); });
        let count = shim_ct_run_get_glyph_count(h);
        assert_eq!(count, 5);
    }

    #[test]
    fn test_run_get_string_range_real() {
        let (h, run) = build_run("Test", "System", 12.0, 10);
        with_run_registry(|map| { map.insert(h, run); });
        let mut range = [0i64; 2];
        shim_ct_run_get_string_range(range.as_mut_ptr() as u64, h);
        assert_eq!(range[0], 10, "location should be 10");
        assert_eq!(range[1], 4, "length should be 4");
    }

    #[test]
    fn test_run_get_glyphs_real() {
        let (h, run) = build_run("XY", "System", 12.0, 0);
        with_run_registry(|map| { map.insert(h, run); });
        let mut glyphs = [0u16; 2];
        shim_ct_run_get_glyphs(h, 0, 0, glyphs.as_mut_ptr() as u64);
        assert_eq!(glyphs[0], 'X' as u16);
        assert_eq!(glyphs[1], 'Y' as u16);
    }

    #[test]
    fn test_run_get_positions_real() {
        let (h, run) = build_run("AB", "System", 12.0, 0);
        with_run_registry(|map| { map.insert(h, run); });
        let mut positions = [0.0_f64; 4]; // 2 CGPoints
        shim_ct_run_get_positions(h, 0, 0, positions.as_mut_ptr() as u64);
        assert!((positions[0] - 0.0).abs() < 1e-9, "first x = 0");
        assert!((positions[1] - 0.0).abs() < 1e-9, "first y = 0");
        assert!(positions[2] > 0.0, "second x > 0");
    }

    #[test]
    fn test_run_get_advances_real() {
        let (h, run) = build_run("AB", "System", 12.0, 0);
        with_run_registry(|map| { map.insert(h, run); });
        let mut advances = [0.0_f64; 4]; // 2 CGSize
        shim_ct_run_get_advances(h, 0, 0, advances.as_mut_ptr() as u64);
        assert!(advances[0] > 0.0, "first advance width > 0");
        assert!((advances[1] - 0.0).abs() < 1e-9, "first advance height = 0");
    }

    #[test]
    fn test_run_get_typo_bounds_real() {
        let (h, run) = build_run("Hello", "System", 16.0, 0);
        with_run_registry(|map| { map.insert(h, run); });
        let mut asc = 0.0_f64;
        let mut desc = 0.0_f64;
        let mut lead = 0.0_f64;
        let width = f64::from_bits(shim_ct_run_get_typo_bounds(
            h, 0, 0,
            &mut asc as *mut f64 as u64,
            &mut desc as *mut f64 as u64,
            &mut lead as *mut f64 as u64,
        ));
        assert!(width > 0.0, "width should be > 0");
        assert!(asc > 0.0, "ascent > 0");
        assert!(desc > 0.0, "descent > 0");
    }

    #[test]
    fn test_run_get_attributes_returns_dict() {
        let (h, run) = build_run("X", "Helvetica", 12.0, 0);
        with_run_registry(|map| { map.insert(h, run); });
        let dict = shim_ct_run_get_attributes(h);
        assert_ne!(dict, 0, "attributes should return non-null dict");
        let count = unsafe { *((dict as usize + 8) as *const u64) };
        assert_eq!(count, 1, "should have 1 attribute (font)");
    }

    #[test]
    fn test_resolve_range_full() {
        assert_eq!(resolve_range(0, 0, 10), (0, 10));
    }

    #[test]
    fn test_resolve_range_subset() {
        assert_eq!(resolve_range(2, 3, 10), (2, 3));
    }

    #[test]
    fn test_resolve_range_clamp() {
        assert_eq!(resolve_range(8, 5, 10), (8, 2));
    }

    // ── CTLine → CTRun pipeline tests ───────────────────────────

    #[test]
    fn test_line_get_glyph_runs_empty() {
        let h = shim_ct_line_create(0);
        let arr = shim_ct_line_get_glyph_runs(h);
        assert_ne!(arr, 0, "should return non-null CFArray");
        let count = unsafe { *((arr as usize + 8) as *const u64) };
        assert_eq!(count, 0, "empty line should have 0 runs");
    }

    // ── CTFrame pipeline tests ──────────────────────────────────

    #[test]
    fn test_frame_get_lines_returns_array() {
        // Create a framesetter with empty text
        let fs = shim_ct_framesetter_create(0);
        let frame = shim_ct_framesetter_create_frame(fs, 0, 0, 0, 0);
        let lines = shim_ct_frame_get_lines(frame);
        // Empty text → no lines
        assert_ne!(lines, 0, "should return non-null CFArray");
    }

    #[test]
    fn test_frame_draw_no_crash_valid() {
        let fs = shim_ct_framesetter_create(0);
        let frame = shim_ct_framesetter_create_frame(fs, 0, 0, 0, 0);
        shim_ct_frame_draw(frame, 0); // context=0 → early return
    }

    #[test]
    fn test_frame_get_visible_range_valid() {
        let fs = shim_ct_framesetter_create(0);
        let frame = shim_ct_framesetter_create_frame(fs, 0, 0, 0, 0);
        let mut range = [0i64; 2];
        shim_ct_frame_get_visible_range(range.as_mut_ptr() as u64, frame);
        // Empty text → range = {0, 0}
        assert_eq!(range[0], 0);
    }

    #[test]
    fn test_make_cfarray_of_handles_empty() {
        let arr = make_cfarray_of_handles(&[]);
        assert_ne!(arr, 0);
        let count = unsafe { *((arr as usize + 8) as *const u64) };
        assert_eq!(count, 0);
    }

    #[test]
    fn test_make_cfarray_of_handles_with_items() {
        let arr = make_cfarray_of_handles(&[42, 43, 44]);
        assert_ne!(arr, 0);
        let count = unsafe { *((arr as usize + 8) as *const u64) };
        assert_eq!(count, 3);
        let items_ptr = unsafe { *((arr as usize + 16) as *const u64) };
        assert_ne!(items_ptr, 0);
        let item0 = unsafe { *(items_ptr as *const u64) };
        assert_eq!(item0, 42);
    }

    // ── CTParagraphStyle tests ──────────────────────────────────

    #[test]
    fn test_paragraph_style_get_value_false() {
        let h = shim_ct_paragraph_style_create(0, 0);
        let result = shim_ct_paragraph_style_get_value(h, 0, 0, 0);
        assert_eq!(result, 0, "should return false (0)");
    }

    // ── Data symbol tests ───────────────────────────────────────

    #[test]
    fn test_data_symbols_non_null() {
        assert_ne!(&KCTFONT_ATTR_NAME as *const u64 as u64, 0);
        assert_ne!(&KCTFOREGROUND_COLOR_ATTR as *const u64 as u64, 0);
        assert_ne!(&KCTPARAGRAPH_STYLE_ATTR as *const u64 as u64, 0);
        assert_ne!(&KCTFG_FROM_CTX_ATTR as *const u64 as u64, 0);
    }

    #[test]
    fn test_data_symbols_all_different() {
        let a = KCTFONT_ATTR_NAME;
        let b = KCTFOREGROUND_COLOR_ATTR;
        let c = KCTPARAGRAPH_STYLE_ATTR;
        let d = KCTFG_FROM_CTX_ATTR;
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_ne!(b, c);
    }

    // ── Registration test ───────────────────────────────────────

    #[test]
    fn test_all_shims_registered() {
        let mut table = ShimTable::new();
        CoreTextStubs::install(&mut table);
        let expected = [
            "_CTFontCreateWithName", "_CTFontCopyFullName", "_CTFontGetAscent",
            "_CTFontGetDescent", "_CTFontGetLeading", "_CTFontGetSize",
            "_CTFontGetBoundingBox", "_CTFontGetGlyphsForCharacters",
            "_CTFontGetAdvancesForGlyphs", "_CTFontCopyFamilyName",
            "_CTFontCopyPostScriptName", "_CTFontCopyDisplayName",
            "_CTLineCreateWithAttributedString", "_CTLineDraw",
            "_CTLineGetGlyphRuns", "_CTLineGetGlyphCount",
            "_CTLineGetStringRange", "_CTLineGetTypographicBounds",
            "_CTLineGetBoundsWithOptions", "_CTLineGetOffsetForStringIndex",
            "_CTRunGetGlyphCount", "_CTRunGetPositions", "_CTRunGetGlyphs",
            "_CTRunGetStringRange", "_CTRunGetTypographicBounds",
            "_CTFramesetterCreateWithAttributedString",
            "_CTFramesetterCreateFrame",
            "_CTFramesetterSuggestFrameSizeWithConstraints",
            "_CTFrameDraw", "_CTFrameGetLines",
            "_CTFontDescriptorCreateWithAttributes",
            "_CTParagraphStyleCreate",
            "_CTFontManagerCopyAvailableFontFamilyNames",
            "_CTFontManagerRegisterFontsForURL",
            "_kCTFontAttributeName",
        ];
        for sym in expected {
            assert!(table.resolve(sym).is_some(), "{sym} should be registered");
        }
    }
}
