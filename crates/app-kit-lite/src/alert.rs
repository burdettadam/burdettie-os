//! NSAlert — modal dialog showing a message with buttons (headless-friendly).

/// Alert dialog style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertStyle {
    Warning,
    Informational,
    Critical,
}

/// Return value from running an alert modally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertReturn {
    FirstButton,
    SecondButton,
    ThirdButton,
}

/// A simple alert dialog (headless-friendly).
pub struct NSAlert {
    message_text: String,
    informative_text: String,
    buttons: Vec<String>,
    style: AlertStyle,
}

impl NSAlert {
    pub fn new() -> Self {
        Self {
            message_text: String::new(),
            informative_text: String::new(),
            buttons: Vec::new(),
            style: AlertStyle::Warning,
        }
    }

    pub fn set_message_text(&mut self, text: impl Into<String>) {
        self.message_text = text.into();
    }

    pub fn message_text(&self) -> &str {
        &self.message_text
    }

    pub fn set_informative_text(&mut self, text: impl Into<String>) {
        self.informative_text = text.into();
    }

    pub fn informative_text(&self) -> &str {
        &self.informative_text
    }

    pub fn add_button(&mut self, title: impl Into<String>) {
        self.buttons.push(title.into());
    }

    pub fn set_alert_style(&mut self, style: AlertStyle) {
        self.style = style;
    }

    pub fn alert_style(&self) -> &AlertStyle {
        &self.style
    }

    pub fn button_titles(&self) -> &[String] {
        &self.buttons
    }

    /// Run the alert modally. In headless mode, returns `FirstButton`.
    pub fn run_modal(&self) -> AlertReturn {
        AlertReturn::FirstButton
    }
}

impl Default for NSAlert {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_default_has_no_buttons() {
        let alert = NSAlert::new();
        assert!(alert.button_titles().is_empty());
    }

    #[test]
    fn alert_set_message_text() {
        let mut alert = NSAlert::new();
        alert.set_message_text("Error occurred");
        assert_eq!(alert.message_text(), "Error occurred");
        alert.set_informative_text("Please try again");
        assert_eq!(alert.informative_text(), "Please try again");
    }

    #[test]
    fn alert_add_buttons() {
        let mut alert = NSAlert::new();
        alert.add_button("OK");
        alert.add_button("Cancel");
        alert.add_button("Don't Save");
        assert_eq!(alert.button_titles(), &["OK", "Cancel", "Don't Save"]);
    }

    #[test]
    fn alert_run_modal_returns_first() {
        let alert = NSAlert::new();
        assert_eq!(alert.run_modal(), AlertReturn::FirstButton);
    }

    #[test]
    fn alert_style_default_warning() {
        let alert = NSAlert::new();
        assert_eq!(*alert.alert_style(), AlertStyle::Warning);
        let mut alert2 = NSAlert::new();
        alert2.set_alert_style(AlertStyle::Critical);
        assert_eq!(*alert2.alert_style(), AlertStyle::Critical);
    }
}
