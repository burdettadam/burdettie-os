use super::*;

#[test]
fn menu_item_basic() {
    let item = NSMenuItem::new("Copy");
    assert_eq!(item.title(), "Copy");
    assert!(item.is_enabled());
    assert!(!item.is_separator());
}

#[test]
fn menu_item_separator() {
    let sep = NSMenuItem::separator();
    assert!(sep.is_separator());
    assert!(!sep.is_enabled());
    assert_eq!(sep.title(), "");
}

#[test]
fn menu_item_with_action() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let fired = Arc::new(AtomicBool::new(false));
    let f = fired.clone();
    let mut item = NSMenuItem::with_action(
        "Quit",
        Some(KeyEquivalent::command(KeyCode::Q)),
        move || {
            f.store(true, Ordering::SeqCst);
        },
    );

    assert!(!fired.load(Ordering::SeqCst));
    item.perform_action();
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn menu_item_disabled_no_action() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let fired = Arc::new(AtomicBool::new(false));
    let f = fired.clone();
    let mut item = NSMenuItem::with_action("Disabled", None, move || {
        f.store(true, Ordering::SeqCst);
    });
    item.set_enabled(false);
    item.perform_action();
    assert!(!fired.load(Ordering::SeqCst));
}

#[test]
fn menu_add_items() {
    let mut menu = NSMenu::new("Edit");
    menu.add_item(NSMenuItem::new("Cut"));
    menu.add_item(NSMenuItem::separator());
    menu.add_item(NSMenuItem::new("Paste"));
    assert_eq!(menu.item_count(), 3);
    assert_eq!(menu.items()[0].title(), "Cut");
    assert!(menu.items()[1].is_separator());
    assert_eq!(menu.items()[2].title(), "Paste");
}

#[test]
fn menu_insert_item() {
    let mut menu = NSMenu::new("File");
    menu.add_item(NSMenuItem::new("Open"));
    menu.add_item(NSMenuItem::new("Close"));
    menu.insert_item(NSMenuItem::new("New"), 0);
    assert_eq!(menu.items()[0].title(), "New");
    assert_eq!(menu.items()[1].title(), "Open");
}

#[test]
fn menu_remove_item() {
    let mut menu = NSMenu::new("Test");
    menu.add_item(NSMenuItem::new("A"));
    menu.add_item(NSMenuItem::new("B"));
    let removed = menu.remove_item(0);
    assert_eq!(removed.unwrap().title(), "A");
    assert_eq!(menu.item_count(), 1);
}

#[test]
fn menu_item_tag() {
    let mut menu = NSMenu::new("Test");
    let mut item = NSMenuItem::new("Special");
    item.set_tag(42);
    menu.add_item(item);

    let found = menu.item_with_tag(42);
    assert!(found.is_some());
    assert_eq!(found.unwrap().title(), "Special");
    assert!(menu.item_with_tag(99).is_none());
}

#[test]
fn menu_item_submenu() {
    let mut file_menu = NSMenu::new("File");
    let mut recent = NSMenuItem::new("Open Recent");
    let mut submenu = NSMenu::new("Recent Files");
    submenu.add_item(NSMenuItem::new("doc1.txt"));
    submenu.add_item(NSMenuItem::new("doc2.txt"));
    recent.set_submenu(submenu);
    file_menu.add_item(recent);

    assert!(file_menu.items()[0].has_submenu());
    assert_eq!(file_menu.items()[0].submenu().unwrap().item_count(), 2);
}

#[test]
fn perform_key_equivalent_direct() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let fired = Arc::new(AtomicBool::new(false));
    let f = fired.clone();

    let mut menu = NSMenu::new("Edit");
    menu.add_item(NSMenuItem::with_action(
        "Copy",
        Some(KeyEquivalent::command(KeyCode::C)),
        move || {
            f.store(true, Ordering::SeqCst);
        },
    ));

    let hit = menu.perform_key_equivalent(KeyCode::C, ModifierFlags::COMMAND);
    assert!(hit);
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn perform_key_equivalent_submenu() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let fired = Arc::new(AtomicBool::new(false));
    let f = fired.clone();

    let mut main_menu = NSMenu::new("Main");
    let mut edit_item = NSMenuItem::new("Edit");
    let mut edit_menu = NSMenu::new("Edit");
    edit_menu.add_item(NSMenuItem::with_action(
        "Paste",
        Some(KeyEquivalent::command(KeyCode::V)),
        move || {
            f.store(true, Ordering::SeqCst);
        },
    ));
    edit_item.set_submenu(edit_menu);
    main_menu.add_item(edit_item);

    let hit = main_menu.perform_key_equivalent(KeyCode::V, ModifierFlags::COMMAND);
    assert!(hit);
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn perform_key_equivalent_no_match() {
    let mut menu = NSMenu::new("File");
    menu.add_item(NSMenuItem::with_action(
        "New",
        Some(KeyEquivalent::command(KeyCode::N)),
        || {},
    ));

    let hit = menu.perform_key_equivalent(KeyCode::X, ModifierFlags::COMMAND);
    assert!(!hit);
}

#[test]
fn key_equivalent_display() {
    let ke = KeyEquivalent::command(KeyCode::S);
    let s = format!("{ke}");
    assert!(s.contains('\u{2318}')); // ⌘
}

#[test]
fn modifier_flags_default() {
    let flags = ModifierFlags::default();
    assert_eq!(flags, ModifierFlags::NONE);
}

#[test]
fn menu_item_set_title_changes_title() {
    let mut item = NSMenuItem::new("Old");
    assert_eq!(item.title(), "Old");
    item.set_title("New");
    assert_eq!(item.title(), "New");
}

#[test]
fn menu_item_set_key_equivalent_after_construction() {
    let mut item = NSMenuItem::new("Save");
    assert!(item.key_equivalent().is_none());
    item.set_key_equivalent(KeyEquivalent::command(KeyCode::S));
    let ke = item.key_equivalent().unwrap();
    assert_eq!(ke.key, KeyCode::S);
    assert!(ke.modifiers.command);
}

#[test]
fn menu_set_title_mutates() {
    let mut menu = NSMenu::new("A");
    assert_eq!(menu.title(), "A");
    menu.set_title("B");
    assert_eq!(menu.title(), "B");
}

#[test]
fn modifier_flags_from_backend_modifiers_all_set() {
    let m = window_backend::Modifiers {
        command: true,
        shift: true,
        option: true,
        control: true,
    };
    let flags = ModifierFlags::from_backend_modifiers(&m);
    assert!(flags.command);
    assert!(flags.shift);
    assert!(flags.option);
    assert!(flags.control);
}

#[test]
fn modifier_flags_from_backend_modifiers_none_set() {
    let m = window_backend::Modifiers {
        command: false,
        shift: false,
        option: false,
        control: false,
    };
    let flags = ModifierFlags::from_backend_modifiers(&m);
    assert_eq!(flags, ModifierFlags::NONE);
}

#[test]
fn key_equivalent_display_shift_command() {
    let ke = KeyEquivalent::new(KeyCode::N, ModifierFlags::shift_command());
    let s = format!("{ke}");
    assert!(s.contains('\u{21E7}')); // ⇧
    assert!(s.contains('\u{2318}')); // ⌘
}

#[test]
fn key_equivalent_display_option() {
    let ke = KeyEquivalent::new(
        KeyCode::O,
        ModifierFlags {
            option: true,
            ..ModifierFlags::NONE
        },
    );
    let s = format!("{ke}");
    assert!(s.contains('\u{2325}')); // ⌥
}

#[test]
fn menu_item_disabled() {
    let mut item = NSMenuItem::new("Action");
    assert!(item.is_enabled());
    item.set_enabled(false);
    assert!(!item.is_enabled());
    item.set_enabled(true);
    assert!(item.is_enabled());
}
