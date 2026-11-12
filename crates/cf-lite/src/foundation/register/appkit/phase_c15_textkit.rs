//! AppKit Phase C15: TextKit classes — NSTextStorage, NSLayoutManager, NSTextContainer.

use super::me;
use crate::{nslayoutmanager, nsobj, nstextcontainer, nstextstorage};
use objc_runtime::ClassInfo;
use std::collections::HashMap;

// ── NSTextStorage ──────────────────────────────────────────────

pub fn register_nstextstorage_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstextstorage_class_addr();
    let metaclass_addr = nsobj::nstextstorage_metaclass_addr();
    let info = ClassInfo {
        name: "NSTextStorage".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                nstextstorage::shim_nstextstorage_init as *const () as u64,
                "@@:",
            ),
            me(
                "string",
                nstextstorage::shim_nstextstorage_string as *const () as u64,
                "@@:",
            ),
            me(
                "setString:",
                nstextstorage::shim_nstextstorage_set_string as *const () as u64,
                "v@:@",
            ),
            me(
                "length",
                nstextstorage::shim_nstextstorage_length as *const () as u64,
                "Q@:",
            ),
            me(
                "replaceCharactersInRange:withString:",
                nstextstorage::shim_nstextstorage_replace_chars as *const () as u64,
                "v@:{NSRange=QQ}@",
            ),
            me(
                "mutableString",
                nstextstorage::shim_nstextstorage_mutable_string as *const () as u64,
                "@@:",
            ),
            me(
                "addAttribute:value:range:",
                nstextstorage::shim_nstextstorage_add_attribute as *const () as u64,
                "v@:@@{NSRange=QQ}",
            ),
            me(
                "addLayoutManager:",
                nstextstorage::shim_nstextstorage_add_layout_manager as *const () as u64,
                "v@:@",
            ),
            me(
                "removeLayoutManager:",
                nstextstorage::shim_nstextstorage_remove_layout_manager as *const () as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            nstextstorage::shim_nstextstorage_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTextStorage");
}

// ── NSLayoutManager ────────────────────────────────────────────

pub fn register_nslayoutmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nslayoutmanager_class_addr();
    let metaclass_addr = nsobj::nslayoutmanager_metaclass_addr();
    let info = ClassInfo {
        name: "NSLayoutManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                nslayoutmanager::shim_nslayoutmanager_init as *const () as u64,
                "@@:",
            ),
            me(
                "textStorage",
                nslayoutmanager::shim_nslayoutmanager_text_storage as *const () as u64,
                "@@:",
            ),
            me(
                "setTextStorage:",
                nslayoutmanager::shim_nslayoutmanager_set_text_storage as *const () as u64,
                "v@:@",
            ),
            me(
                "addTextContainer:",
                nslayoutmanager::shim_nslayoutmanager_add_text_container as *const () as u64,
                "v@:@",
            ),
            me(
                "removeTextContainerAtIndex:",
                nslayoutmanager::shim_nslayoutmanager_remove_text_container as *const () as u64,
                "v@:Q",
            ),
            me(
                "textContainers",
                nslayoutmanager::shim_nslayoutmanager_text_containers as *const () as u64,
                "@@:",
            ),
            me(
                "numberOfGlyphs",
                nslayoutmanager::shim_nslayoutmanager_number_of_glyphs as *const () as u64,
                "Q@:",
            ),
            me(
                "usedRectForTextContainer:",
                nslayoutmanager::shim_nslayoutmanager_used_rect as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:@",
            ),
            me(
                "ensureLayoutForTextContainer:",
                nslayoutmanager::shim_nslayoutmanager_ensure_layout as *const () as u64,
                "v@:@",
            ),
            me(
                "glyphRangeForTextContainer:",
                nslayoutmanager::shim_nslayoutmanager_glyph_range_for_tc as *const () as u64,
                "{NSRange=QQ}@:@",
            ),
            me(
                "boundingRectForGlyphRange:inTextContainer:",
                nslayoutmanager::shim_nslayoutmanager_bounding_rect as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:{NSRange=QQ}@",
            ),
            me(
                "lineFragmentRectForGlyphAtIndex:effectiveRange:",
                nslayoutmanager::shim_nslayoutmanager_line_fragment_rect as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:Q^{NSRange=QQ}",
            ),
            me(
                "invalidateLayoutForCharacterRange:actualCharacterRange:",
                nslayoutmanager::shim_nslayoutmanager_invalidate_layout as *const () as u64,
                "v@:{NSRange=QQ}^{NSRange=QQ}",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            nslayoutmanager::shim_nslayoutmanager_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSLayoutManager");
}

// ── NSTextContainer ────────────────────────────────────────────

pub fn register_nstextcontainer_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstextcontainer_class_addr();
    let metaclass_addr = nsobj::nstextcontainer_metaclass_addr();
    let info = ClassInfo {
        name: "NSTextContainer".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                nstextcontainer::shim_nstextcontainer_init as *const () as u64,
                "@@:",
            ),
            me(
                "initWithSize:",
                nstextcontainer::shim_nstextcontainer_init_with_size as *const () as u64,
                "@@:{CGSize=dd}",
            ),
            me(
                "size",
                nstextcontainer::shim_nstextcontainer_size_width as *const () as u64,
                "{CGSize=dd}@:",
            ),
            me(
                "setSize:",
                nstextcontainer::shim_nstextcontainer_set_size as *const () as u64,
                "v@:{CGSize=dd}",
            ),
            me(
                "layoutManager",
                nstextcontainer::shim_nstextcontainer_layout_manager as *const () as u64,
                "@@:",
            ),
            me(
                "setLayoutManager:",
                nstextcontainer::shim_nstextcontainer_set_layout_manager as *const () as u64,
                "v@:@",
            ),
            me(
                "lineFragmentPadding",
                nstextcontainer::shim_nstextcontainer_line_fragment_padding as *const () as u64,
                "d@:",
            ),
            me(
                "setLineFragmentPadding:",
                nstextcontainer::shim_nstextcontainer_set_line_fragment_padding as *const () as u64,
                "v@:d",
            ),
            me(
                "widthTracksTextView",
                nstextcontainer::shim_nstextcontainer_width_tracks_text_view as *const () as u64,
                "c@:",
            ),
            me(
                "setWidthTracksTextView:",
                nstextcontainer::shim_nstextcontainer_set_width_tracks as *const () as u64,
                "v@:c",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            nstextcontainer::shim_nstextcontainer_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTextContainer");
}
