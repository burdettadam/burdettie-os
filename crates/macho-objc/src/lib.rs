mod parse;
mod types;

pub use types::*;

use macho_parse::ParsedMachO;
use parse::{
    parse_catlist, parse_function_pointer_array, parse_image_info,
    parse_init_offsets, parse_pointer_list, parse_protolist, parse_selrefs,
};
use types::OBJC_SECTIONS;

/// Detect whether a parsed Mach-O contains Objective-C metadata.
pub fn detect_objc(parsed: &ParsedMachO) -> ObjcPresence {
    let mut found = Vec::new();

    for section in &parsed.sections {
        if OBJC_SECTIONS.contains(&section.sectname.as_str()) {
            found.push(section.sectname.clone());
        }
    }

    ObjcPresence {
        has_objc: !found.is_empty(),
        sections_found: found,
    }
}

/// Parse ObjC metadata from a parsed Mach-O and its raw data.
pub fn parse_objc_metadata(parsed: &ParsedMachO, data: &[u8]) -> ObjcMetadata {
    let presence = detect_objc(parsed);

    let class_refs = parse_pointer_list(parsed, data, "__objc_classlist");
    let categories = parse_catlist(parsed, data);
    let category_count = categories.len();
    let protocols = parse_protolist(parsed, data);
    let protocol_count = protocols.len();
    let (selectors, selref_addrs) = parse_selrefs(parsed, data);
    let image_info = parse_image_info(parsed, data);

    ObjcMetadata {
        presence,
        class_refs,
        categories,
        protocols,
        category_count,
        protocol_count,
        selectors,
        selref_addrs,
        image_info,
    }
}

/// Extract constructors and model initialization order.
pub fn extract_initializers(parsed: &ParsedMachO, data: &[u8]) -> InitializerInfo {
    let mut constructors = Vec::new();
    let mut init_steps = Vec::new();
    let presence = detect_objc(parsed);

    let text_vmaddr = parsed
        .segments
        .iter()
        .find(|s| s.name == "__TEXT")
        .map(|s| s.vmaddr)
        .unwrap_or(0);

    let has_objc_load = presence.has_objc;
    if has_objc_load {
        init_steps.push("ObjC +load methods".to_string());
    }

    for section in &parsed.sections {
        if section.sectname == "__mod_init_func" {
            let entries =
                parse_function_pointer_array(data, section.offset as usize, section.size as usize);
            for addr in entries {
                constructors.push(ConstructorEntry {
                    address: addr,
                    source: ConstructorSource::ModInitFunc,
                });
            }
        }
    }

    for section in &parsed.sections {
        if section.sectname == "__init_offsets" {
            let entries = parse_init_offsets(
                data,
                section.offset as usize,
                section.size as usize,
                text_vmaddr,
            );
            for addr in entries {
                constructors.push(ConstructorEntry {
                    address: addr,
                    source: ConstructorSource::InitOffsets,
                });
            }
        }
    }

    for (i, ctor) in constructors.iter().enumerate() {
        init_steps.push(format!(
            "constructor[{}] at {:#x} ({:?})",
            i, ctor.address, ctor.source
        ));
    }

    InitializerInfo {
        constructors,
        has_objc_load,
        init_steps,
    }
}
