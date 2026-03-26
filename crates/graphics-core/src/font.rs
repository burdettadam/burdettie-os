// ── Font ───────────────────────────────────────────────────────────

/// A font descriptor.
#[derive(Debug, Clone)]
pub struct Font {
    pub name: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
}

impl Font {
    pub fn new(name: impl Into<String>, size: f64) -> Self {
        Self {
            name: name.into(),
            size,
            bold: false,
            italic: false,
        }
    }

    pub fn system(size: f64) -> Self {
        Self::new("System", size)
    }

    /// Returns a copy with bold set.
    pub fn with_bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Returns a copy with italic set.
    pub fn with_italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::system(14.0)
    }
}

/// Style attributes for a range of text in an attributed string.
#[derive(Debug, Clone)]
pub struct TextRun {
    /// Byte range in the source string.
    pub range: std::ops::Range<usize>,
    /// Font name override (None = use default).
    pub font_name: Option<String>,
    /// Font size override (None = use default).
    pub size: Option<f64>,
    /// Text color override (None = use default).
    pub color: Option<super::Color>,
    /// Whether this run is bold.
    pub bold: bool,
    /// Whether this run is italic.
    pub italic: bool,
}
