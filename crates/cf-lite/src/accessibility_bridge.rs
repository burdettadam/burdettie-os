//! Accessibility bridge: converts the NSView tree into AccessKit node trees.
//!
//! This module walks the guest view hierarchy and produces an
//! `accesskit::TreeUpdate` that screen readers consume via the
//! `accesskit_winit` adapter in the windowing backend.

use accesskit::{Node, NodeId, Role, Tree, TreeUpdate};

use crate::nsobj;
use crate::nsview;

// ── ISA → AccessKit role mapping ───────────────────────────────────

/// Map a guest view's ISA class address to an AccessKit Role.
fn role_for_isa(isa: u64) -> Role {
    if isa == nsobj::nswindow_class_addr() {
        Role::Window
    } else if isa == nsobj::nsbutton_class_addr() || isa == nsobj::nspopupbutton_class_addr() {
        Role::Button
    } else if isa == nsobj::nstextfield_class_addr() {
        Role::TextInput
    } else if isa == nsobj::nstextview_class_addr() {
        Role::MultilineTextInput
    } else if isa == nsobj::nsimageview_class_addr() {
        Role::Image
    } else if isa == nsobj::nsslider_class_addr() {
        Role::Slider
    } else if isa == nsobj::nsprogressindicator_class_addr() {
        Role::ProgressIndicator
    } else if isa == nsobj::nsscrollview_class_addr() {
        Role::ScrollView
    } else if isa == nsobj::nstableview_class_addr() {
        Role::Table
    } else if isa == nsobj::nsbox_class_addr() {
        Role::Group
    } else if isa == nsobj::nssplitview_class_addr() {
        Role::Splitter
    } else if isa == nsobj::nsstackview_class_addr() || isa == nsobj::nsview_class_addr() {
        Role::Group
    } else if isa == nsobj::nstabview_class_addr() {
        Role::TabList
    } else if isa == nsobj::nssegmentedcontrol_class_addr() {
        Role::RadioGroup
    } else if isa == nsobj::nscombobox_class_addr() {
        Role::ComboBox
    } else if isa == nsobj::nscolorwell_class_addr() {
        Role::ColorWell
    } else if isa == nsobj::nsstepper_class_addr() {
        Role::SpinButton
    } else if isa == nsobj::nscontrol_class_addr() {
        Role::GenericContainer
    } else {
        Role::Unknown
    }
}

// ── Node IDs ───────────────────────────────────────────────────────

/// Convert a guest view pointer to a stable AccessKit NodeId.
fn node_id(view: u64) -> NodeId {
    NodeId(view)
}

// ── Label extraction ───────────────────────────────────────────────

/// Try to extract a human-readable label from a view.
///
/// Checks (in order): NSButton title, NSTextField/NSControl stringValue,
/// NSWindow title. Returns `None` if no text is available.
pub(crate) fn label_for_view(view: u64) -> Option<String> {
    if view == 0 {
        return None;
    }
    let isa = unsafe { *(view as *const u64) };

    // NSButton → title at offset 80
    if isa == nsobj::nsbutton_class_addr() || isa == nsobj::nspopupbutton_class_addr() {
        let title_ptr = unsafe { *((view as usize + 80) as *const u64) };
        if let Some(s) = crate::cfstring::extract_cfstring_cstr(title_ptr) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }

    // NSTextField → stringValue at offset 80
    if isa == nsobj::nstextfield_class_addr()
        || isa == nsobj::nscontrol_class_addr()
        || isa == nsobj::nscombobox_class_addr()
    {
        let sv_ptr = unsafe { *((view as usize + 80) as *const u64) };
        if let Some(s) = crate::cfstring::extract_cfstring_cstr(sv_ptr) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }

    None
}

// ── Tree builder ───────────────────────────────────────────────────

/// Build an AccessKit node for a single view, without recursing.
fn build_node(view: u64) -> Node {
    let isa = if view != 0 {
        unsafe { *(view as *const u64) }
    } else {
        0
    };
    let role = role_for_isa(isa);
    let children = nsview::snapshot_subviews_pub(view);

    let mut node = Node::new(role);

    // Set the label.
    if let Some(label) = label_for_view(view) {
        node.set_label(label);
    }

    // Set bounding rect from frame.
    let frame = nsview::view_frame(view);
    node.set_bounds(accesskit::Rect {
        x0: frame.origin.x,
        y0: frame.origin.y,
        x1: frame.origin.x + frame.size.width,
        y1: frame.origin.y + frame.size.height,
    });

    // Add children.
    let child_ids: Vec<NodeId> = children.iter().map(|&c| node_id(c)).collect();
    node.set_children(child_ids);

    node
}

/// Recursively collect nodes from a view hierarchy.
fn collect_nodes(view: u64, out: &mut Vec<(NodeId, Node)>) {
    if view == 0 {
        return;
    }
    out.push((node_id(view), build_node(view)));
    for child in nsview::snapshot_subviews_pub(view) {
        collect_nodes(child, out);
    }
}

/// Build a full `TreeUpdate` for a guest window's view hierarchy.
///
/// The tree root is the window's content view. Called by the windowing
/// backend after window creation and on view hierarchy changes.
pub fn build_tree_update(content_view: u64) -> TreeUpdate {
    if content_view == 0 {
        return TreeUpdate {
            nodes: vec![],
            tree: Some(Tree::new(NodeId(1))),
            focus: NodeId(1),
        };
    }

    let mut nodes = Vec::new();
    collect_nodes(content_view, &mut nodes);

    let root_id = node_id(content_view);
    TreeUpdate {
        nodes,
        tree: Some(Tree::new(root_id)),
        focus: root_id,
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nsview::VIEW_SIZE;
    use crate::shim_test_helpers::*;

    fn alloc_view() -> u64 {
        alloc_obj(VIEW_SIZE, nsobj::nsview_class_addr())
    }

    fn alloc_button(title: &str) -> u64 {
        let btn = alloc_obj(112, nsobj::nsbutton_class_addr());
        let ns_title = make_string(title);
        write_field(btn, 80, ns_title); // TITLE_OFF
        btn
    }

    #[test]
    fn node_id_from_address() {
        let id = node_id(0x1000);
        assert_eq!(id.0, 0x1000u64);
    }

    #[test]
    fn role_mapping_button() {
        let role = role_for_isa(nsobj::nsbutton_class_addr());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn role_mapping_textfield() {
        let role = role_for_isa(nsobj::nstextfield_class_addr());
        assert_eq!(role, Role::TextInput);
    }

    #[test]
    fn role_mapping_view() {
        let role = role_for_isa(nsobj::nsview_class_addr());
        assert_eq!(role, Role::Group);
    }

    #[test]
    fn role_mapping_unknown() {
        let role = role_for_isa(0xDEAD);
        assert_eq!(role, Role::Unknown);
    }

    #[test]
    fn empty_tree_update() {
        let update = build_tree_update(0);
        assert!(update.nodes.is_empty());
        assert!(update.tree.is_some());
    }

    #[test]
    fn single_view_tree() {
        let view = alloc_view();
        let update = build_tree_update(view);
        assert_eq!(update.nodes.len(), 1);
        assert_eq!(update.nodes[0].0, node_id(view));
        assert_eq!(update.nodes[0].1.role(), Role::Group);
        free_obj(view);
    }

    #[test]
    fn view_tree_with_children() {
        let parent = alloc_view();
        let child1 = alloc_view();
        let child2 = alloc_view();
        crate::nsview::shim_nsview_add_subview(parent, 0, child1);
        crate::nsview::shim_nsview_add_subview(parent, 0, child2);

        let update = build_tree_update(parent);
        // parent + 2 children
        assert_eq!(update.nodes.len(), 3);
        free_obj(child2);
        free_obj(child1);
        free_obj(parent);
    }

    #[test]
    fn button_label_extraction() {
        let btn = alloc_button("Click Me");
        let label = label_for_view(btn);
        assert_eq!(label.as_deref(), Some("Click Me"));
        free_obj(btn);
    }

    #[test]
    fn button_node_has_name() {
        let btn = alloc_button("Press");
        let node = build_node(btn);
        assert_eq!(node.role(), Role::Button);
        assert_eq!(node.label(), Some("Press"));
        free_obj(btn);
    }
}
