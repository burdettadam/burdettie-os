//! NSOpenPanel / NSSavePanel — file chooser panels (headless-friendly).

/// Result of running a panel modally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelReturn {
    OK,
    Cancel,
}

/// File-open chooser panel (headless-friendly).
pub struct NSOpenPanel {
    title: String,
    allowed_types: Vec<String>,
    allows_multiple: bool,
    can_choose_directories: bool,
    can_choose_files: bool,
    urls: Vec<String>,
}

impl NSOpenPanel {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            allowed_types: Vec::new(),
            allows_multiple: false,
            can_choose_directories: false,
            can_choose_files: true,
            urls: Vec::new(),
        }
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_allowed_file_types(&mut self, types: Vec<String>) {
        self.allowed_types = types;
    }

    pub fn allowed_file_types(&self) -> &[String] {
        &self.allowed_types
    }

    pub fn set_allows_multiple_selection(&mut self, allows: bool) {
        self.allows_multiple = allows;
    }

    pub fn allows_multiple_selection(&self) -> bool {
        self.allows_multiple
    }

    pub fn set_can_choose_directories(&mut self, can: bool) {
        self.can_choose_directories = can;
    }

    pub fn can_choose_directories(&self) -> bool {
        self.can_choose_directories
    }

    pub fn set_can_choose_files(&mut self, can: bool) {
        self.can_choose_files = can;
    }

    pub fn can_choose_files(&self) -> bool {
        self.can_choose_files
    }

    pub fn urls(&self) -> &[String] {
        &self.urls
    }

    /// Pre-set URLs for headless testing.
    pub fn set_test_urls(&mut self, urls: Vec<String>) {
        self.urls = urls;
    }

    /// Run modally. In headless mode, returns OK if test_urls are set.
    pub fn run_modal(&self) -> PanelReturn {
        if self.urls.is_empty() {
            PanelReturn::Cancel
        } else {
            PanelReturn::OK
        }
    }
}

impl Default for NSOpenPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// File-save chooser panel (headless-friendly).
pub struct NSSavePanel {
    title: String,
    allowed_types: Vec<String>,
    url: Option<String>,
}

impl NSSavePanel {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            allowed_types: Vec::new(),
            url: None,
        }
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_allowed_file_types(&mut self, types: Vec<String>) {
        self.allowed_types = types;
    }

    pub fn allowed_file_types(&self) -> &[String] {
        &self.allowed_types
    }

    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Pre-set the save URL for headless testing.
    pub fn set_test_url(&mut self, url: impl Into<String>) {
        self.url = Some(url.into());
    }

    /// Run modally. In headless mode, returns OK if a test URL is set.
    pub fn run_modal(&self) -> PanelReturn {
        if self.url.is_some() {
            PanelReturn::OK
        } else {
            PanelReturn::Cancel
        }
    }
}

impl Default for NSSavePanel {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_panel_default() {
        let panel = NSOpenPanel::new();
        assert!(panel.title().is_empty());
        assert!(panel.allowed_file_types().is_empty());
        assert!(!panel.allows_multiple_selection());
        assert!(!panel.can_choose_directories());
        assert!(panel.can_choose_files());
        assert!(panel.urls().is_empty());
    }

    #[test]
    fn open_panel_set_allowed_types() {
        let mut panel = NSOpenPanel::new();
        panel.set_allowed_file_types(vec!["png".into(), "jpg".into()]);
        assert_eq!(panel.allowed_file_types(), &["png", "jpg"]);
    }

    #[test]
    fn open_panel_test_urls() {
        let mut panel = NSOpenPanel::new();
        panel.set_test_urls(vec!["/tmp/a.txt".into(), "/tmp/b.txt".into()]);
        assert_eq!(panel.urls().len(), 2);
        assert_eq!(panel.urls()[0], "/tmp/a.txt");
    }

    #[test]
    fn open_panel_run_modal_with_urls() {
        let mut panel = NSOpenPanel::new();
        assert_eq!(panel.run_modal(), PanelReturn::Cancel);
        panel.set_test_urls(vec!["/tmp/file.txt".into()]);
        assert_eq!(panel.run_modal(), PanelReturn::OK);
    }

    #[test]
    fn save_panel_set_title() {
        let mut panel = NSSavePanel::new();
        panel.set_title("Save As");
        assert_eq!(panel.title(), "Save As");
        assert_eq!(panel.run_modal(), PanelReturn::Cancel);
        panel.set_test_url("/tmp/out.txt");
        assert_eq!(panel.run_modal(), PanelReturn::OK);
        assert_eq!(panel.url(), Some("/tmp/out.txt"));
    }
}
