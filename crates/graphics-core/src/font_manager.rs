//! Font loading and management using `ab_glyph` for TrueType/OpenType rasterization.
//!
//! Provides a `FontManager` that holds loaded fonts keyed by name, with an
//! embedded DejaVu Sans font as the default system font.

use ab_glyph::{Font as AbFont, FontVec, ScaleFont};

use super::unicode::{is_combining_mark, is_zero_width, normalize_nfc};

/// Embedded DejaVu Sans Regular — the default system font.
pub(crate) const DEJAVU_SANS: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");
/// Embedded DejaVu Sans Bold.
pub(crate) const DEJAVU_SANS_BOLD: &[u8] = include_bytes!("../fonts/DejaVuSans-Bold.ttf");
/// Embedded DejaVu Sans Mono.
pub(crate) const DEJAVU_SANS_MONO: &[u8] = include_bytes!("../fonts/DejaVuSansMono.ttf");
/// Embedded DejaVu Sans Oblique (italic).
pub(crate) const DEJAVU_SANS_OBLIQUE: &[u8] = include_bytes!("../fonts/DejaVuSans-Oblique.ttf");
/// Embedded DejaVu Sans Bold Oblique (bold italic).
pub(crate) const DEJAVU_SANS_BOLD_OBLIQUE: &[u8] = include_bytes!("../fonts/DejaVuSans-BoldOblique.ttf");

/// A loaded font face backed by `ab_glyph::FontVec`.
pub struct FontFace {
    inner: FontVec,
    name: String,
}

impl FontFace {
    /// Load a font from raw TTF/OTF bytes.
    pub fn from_bytes(name: impl Into<String>, data: &[u8]) -> Option<Self> {
        let inner = FontVec::try_from_vec(data.to_vec()).ok()?;
        Some(Self {
            inner,
            name: name.into(),
        })
    }

    /// The font name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Font metrics at a given point size: (ascent, descent, line_gap).
    /// Ascent is positive (above baseline), descent is negative (below baseline).
    pub fn metrics(&self, size: f64) -> FontMetrics {
        let scaled = self.inner.as_scaled(size as f32);
        FontMetrics {
            ascent: scaled.ascent() as f64,
            descent: scaled.descent() as f64,
            line_gap: scaled.line_gap() as f64,
        }
    }

    /// Line height = ascent - descent + line_gap.
    pub fn line_height(&self, size: f64) -> f64 {
        let m = self.metrics(size);
        m.ascent - m.descent + m.line_gap
    }

    /// Measure the advance width of a single character.
    pub fn char_width(&self, ch: char, size: f64) -> f64 {
        let scaled = self.inner.as_scaled(size as f32);
        let glyph_id = self.inner.glyph_id(ch);
        scaled.h_advance(glyph_id) as f64
    }

    /// Measure the total advance width of a string (no line breaking).
    ///
    /// NFC-normalizes the input and skips combining marks and zero-width
    /// characters for correct measurement of accented text and emoji sequences.
    pub fn measure_text(&self, text: &str, size: f64) -> f64 {
        let normalized = normalize_nfc(text);
        let scaled = self.inner.as_scaled(size as f32);
        let mut width = 0.0f32;
        let mut prev_glyph: Option<ab_glyph::GlyphId> = None;

        for ch in normalized.chars() {
            if is_zero_width(ch) || is_combining_mark(ch) {
                continue;
            }
            let glyph_id = self.inner.glyph_id(ch);
            if let Some(prev) = prev_glyph {
                width += scaled.kern(prev, glyph_id);
            }
            width += scaled.h_advance(glyph_id);
            prev_glyph = Some(glyph_id);
        }
        width as f64
    }

    /// Returns `true` if the font has a glyph for the given character.
    ///
    /// A character is considered present if its glyph ID is non-zero
    /// (glyph ID 0 is the `.notdef` glyph).
    pub fn has_glyph(&self, ch: char) -> bool {
        self.inner.glyph_id(ch) != ab_glyph::GlyphId(0)
    }

    /// Access the underlying ab_glyph font for rasterization.
    pub(crate) fn inner(&self) -> &FontVec {
        &self.inner
    }
}

/// Font metrics at a specific point size.
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    /// Distance from baseline to top of tallest glyph (positive).
    pub ascent: f64,
    /// Distance from baseline to bottom of lowest glyph (negative).
    pub descent: f64,
    /// Extra spacing between lines beyond ascent+descent.
    pub line_gap: f64,
}

impl FontMetrics {
    /// Total line height = ascent - descent + line_gap.
    pub fn line_height(&self) -> f64 {
        self.ascent - self.descent + self.line_gap
    }
}

/// Manages a collection of loaded fonts with a built-in default set.
pub struct FontManager {
    fonts: Vec<FontFace>,
}

impl FontManager {
    /// Create a new font manager with the embedded DejaVu fonts pre-loaded.
    pub fn new() -> Self {
        let mut mgr = Self { fonts: Vec::new() };

        // Load embedded fonts — these should always succeed.
        if let Some(face) = FontFace::from_bytes("System", DEJAVU_SANS) {
            mgr.fonts.push(face);
        }
        if let Some(face) = FontFace::from_bytes("System-Bold", DEJAVU_SANS_BOLD) {
            mgr.fonts.push(face);
        }
        if let Some(face) = FontFace::from_bytes("Monospace", DEJAVU_SANS_MONO) {
            mgr.fonts.push(face);
        }
        if let Some(face) = FontFace::from_bytes("System-Italic", DEJAVU_SANS_OBLIQUE) {
            mgr.fonts.push(face);
        }
        if let Some(face) = FontFace::from_bytes("System-BoldItalic", DEJAVU_SANS_BOLD_OBLIQUE) {
            mgr.fonts.push(face);
        }

        mgr
    }

    /// Register a font from raw TTF/OTF bytes.
    pub fn register(&mut self, name: impl Into<String>, data: &[u8]) -> bool {
        if let Some(face) = FontFace::from_bytes(name, data) {
            self.fonts.push(face);
            true
        } else {
            false
        }
    }

    /// Look up a font by name. Falls back to the default system font.
    pub fn get(&self, name: &str) -> &FontFace {
        self.fonts
            .iter()
            .find(|f| f.name == name)
            .or_else(|| self.fonts.first())
            .expect("FontManager must have at least one font loaded")
    }

    /// The default system font.
    pub fn system(&self) -> &FontFace {
        self.get("System")
    }

    /// The default bold font.
    pub fn bold(&self) -> &FontFace {
        self.get("System-Bold")
    }

    /// The default monospace font.
    pub fn monospace(&self) -> &FontFace {
        self.get("Monospace")
    }

    /// The default italic font.
    pub fn italic(&self) -> &FontFace {
        self.get("System-Italic")
    }

    /// The default bold italic font.
    pub fn bold_italic(&self) -> &FontFace {
        self.get("System-BoldItalic")
    }

    /// Access a font face by index. Returns None if out of range.
    pub fn face_by_index(&self, index: usize) -> Option<&FontFace> {
        self.fonts.get(index)
    }

    /// Resolve a `Font` descriptor to a concrete `FontFace`, respecting bold/italic flags.
    pub fn resolve_font(&self, font: &super::Font) -> &FontFace {
        if font.bold && font.italic {
            self.get("System-BoldItalic")
        } else if font.bold {
            self.get("System-Bold")
        } else if font.italic {
            self.get("System-Italic")
        } else {
            self.get(&font.name)
        }
    }
}

impl Default for FontManager {
    fn default() -> Self {
        Self::new()
    }
}
