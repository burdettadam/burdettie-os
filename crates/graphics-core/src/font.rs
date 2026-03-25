// ── Font ───────────────────────────────────────────────────────────

/// A font descriptor.
#[derive(Debug, Clone)]
pub struct Font {
    pub name: String,
    pub size: f64,
}

impl Font {
    pub fn new(name: impl Into<String>, size: f64) -> Self {
        Self {
            name: name.into(),
            size,
        }
    }

    pub fn system(size: f64) -> Self {
        Self::new("System", size)
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::system(14.0)
    }
}
