//! Menu system — NSMenu and NSMenuItem.
//!
//! Provides a hierarchical menu model compatible with AppKit conventions.
//! Menus store items; items can reference submenus, key equivalents
//! (keyboard shortcuts), and action callbacks.

use std::fmt;

use window_backend::KeyCode;

/// Modifier flags for key equivalents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModifierFlags {
    pub command: bool,
    pub shift: bool,
    pub option: bool,
    pub control: bool,
}

impl ModifierFlags {
    pub const NONE: Self = Self {
        command: false,
        shift: false,
        option: false,
        control: false,
    };

    pub const COMMAND: Self = Self {
        command: true,
        shift: false,
        option: false,
        control: false,
    };

    pub const fn command() -> Self {
        Self::COMMAND
    }

    pub const fn shift_command() -> Self {
        Self {
            command: true,
            shift: true,
            option: false,
            control: false,
        }
    }
}

/// A keyboard shortcut associated with a menu item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEquivalent {
    pub key: KeyCode,
    pub modifiers: ModifierFlags,
}

impl KeyEquivalent {
    pub fn new(key: KeyCode, modifiers: ModifierFlags) -> Self {
        Self { key, modifiers }
    }

    /// Convenience: Cmd+key.
    pub fn command(key: KeyCode) -> Self {
        Self::new(key, ModifierFlags::COMMAND)
    }
}

impl fmt::Display for KeyEquivalent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.control {
            write!(f, "^")?;
        }
        if self.modifiers.option {
            write!(f, "\u{2325}")?; // ⌥
        }
        if self.modifiers.shift {
            write!(f, "\u{21E7}")?; // ⇧
        }
        if self.modifiers.command {
            write!(f, "\u{2318}")?; // ⌘
        }
        write!(f, "{:?}", self.key)
    }
}

/// Action callback for a menu item.
type MenuAction = Box<dyn FnMut() + Send>;

/// A menu item — either a regular action item, a separator, or a submenu container.
pub struct NSMenuItem {
    title: String,
    enabled: bool,
    key_equivalent: Option<KeyEquivalent>,
    action: Option<MenuAction>,
    submenu: Option<NSMenu>,
    tag: i64,
    separator: bool,
}

impl NSMenuItem {
    /// Create a new menu item with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            enabled: true,
            key_equivalent: None,
            action: None,
            submenu: None,
            tag: 0,
            separator: false,
        }
    }

    /// Create a menu item with title, action, and key equivalent.
    pub fn with_action(
        title: impl Into<String>,
        key_equiv: Option<KeyEquivalent>,
        action: impl FnMut() + Send + 'static,
    ) -> Self {
        Self {
            title: title.into(),
            enabled: true,
            key_equivalent: key_equiv,
            action: Some(Box::new(action)),
            submenu: None,
            tag: 0,
            separator: false,
        }
    }

    /// Create a separator item.
    pub fn separator() -> Self {
        Self {
            title: String::new(),
            enabled: false,
            key_equivalent: None,
            action: None,
            submenu: None,
            tag: 0,
            separator: true,
        }
    }

    /// The item's title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Whether this is a separator.
    pub fn is_separator(&self) -> bool {
        self.separator
    }

    /// Whether the item is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable the item.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// The key equivalent, if any.
    pub fn key_equivalent(&self) -> Option<&KeyEquivalent> {
        self.key_equivalent.as_ref()
    }

    /// Set the key equivalent.
    pub fn set_key_equivalent(&mut self, key_equiv: KeyEquivalent) {
        self.key_equivalent = Some(key_equiv);
    }

    /// The item's tag for identification.
    pub fn tag(&self) -> i64 {
        self.tag
    }

    /// Set the tag.
    pub fn set_tag(&mut self, tag: i64) {
        self.tag = tag;
    }

    /// Attach a submenu to this item.
    pub fn set_submenu(&mut self, menu: NSMenu) {
        self.submenu = Some(menu);
    }

    /// Access the submenu, if any.
    pub fn submenu(&self) -> Option<&NSMenu> {
        self.submenu.as_ref()
    }

    /// Mutable access to the submenu.
    pub fn submenu_mut(&mut self) -> Option<&mut NSMenu> {
        self.submenu.as_mut()
    }

    /// Whether this item has a submenu.
    pub fn has_submenu(&self) -> bool {
        self.submenu.is_some()
    }

    /// Perform the action (if enabled and action is set).
    pub fn perform_action(&mut self) {
        if self.enabled {
            if let Some(ref mut action) = self.action {
                action();
            }
        }
    }
}

impl fmt::Debug for NSMenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NSMenuItem")
            .field("title", &self.title)
            .field("enabled", &self.enabled)
            .field("separator", &self.separator)
            .field("key_equivalent", &self.key_equivalent)
            .field("has_action", &self.action.is_some())
            .field("has_submenu", &self.submenu.is_some())
            .field("tag", &self.tag)
            .finish()
    }
}

/// A menu containing menu items.
pub struct NSMenu {
    title: String,
    items: Vec<NSMenuItem>,
}

impl NSMenu {
    /// Create a new menu with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
        }
    }

    /// The menu title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Add an item to the end of the menu.
    pub fn add_item(&mut self, item: NSMenuItem) {
        self.items.push(item);
    }

    /// Insert an item at a specific index.
    pub fn insert_item(&mut self, item: NSMenuItem, index: usize) {
        let idx = index.min(self.items.len());
        self.items.insert(idx, item);
    }

    /// Remove the item at an index. Returns it if in bounds.
    pub fn remove_item(&mut self, index: usize) -> Option<NSMenuItem> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    /// The number of items.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Access the items slice.
    pub fn items(&self) -> &[NSMenuItem] {
        &self.items
    }

    /// Mutable access to the items.
    pub fn items_mut(&mut self) -> &mut Vec<NSMenuItem> {
        &mut self.items
    }

    /// Find an item by tag (non-recursive).
    pub fn item_with_tag(&self, tag: i64) -> Option<&NSMenuItem> {
        self.items.iter().find(|item| item.tag() == tag)
    }

    /// Mutable item by tag (non-recursive).
    pub fn item_with_tag_mut(&mut self, tag: i64) -> Option<&mut NSMenuItem> {
        self.items.iter_mut().find(|item| item.tag() == tag)
    }

    /// Try to perform the action matching a key equivalent.
    ///
    /// Walks all items and submenus recursively. Returns `true` if a
    /// matching shortcut was found and its action performed.
    pub fn perform_key_equivalent(
        &mut self,
        key: KeyCode,
        modifiers: ModifierFlags,
    ) -> bool {
        for item in &mut self.items {
            if item.is_enabled() {
                if let Some(ke) = &item.key_equivalent {
                    if ke.key == key && ke.modifiers == modifiers {
                        item.perform_action();
                        return true;
                    }
                }
            }
            if let Some(ref mut submenu) = item.submenu {
                if submenu.perform_key_equivalent(key, modifiers) {
                    return true;
                }
            }
        }
        false
    }
}

impl fmt::Debug for NSMenu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NSMenu")
            .field("title", &self.title)
            .field("items", &self.items)
            .finish()
    }
}

#[cfg(test)]
mod tests;
