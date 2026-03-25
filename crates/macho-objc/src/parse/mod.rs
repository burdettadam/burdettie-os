//! Private parsing helpers for ObjC metadata extraction.

use macho_parse::ParsedMachO;

use crate::types::*;

mod category;
mod class;
mod protocol;
mod util;

pub use category::*;
pub use protocol::parse_protolist;
use class::{
    extract_class_details, extract_class_name, extract_metaclass_addr, extract_metaclass_methods,
    extract_superclass_addr,
};
use util::resolve_to_vmaddr;

// ── Pointer list parsing ───────────────────────────────────────────

pub fn parse_pointer_list(
    parsed: &ParsedMachO,
    data: &[u8],
    section_name: &str,
) -> Vec<ObjcClassRef> {
    let mut refs = Vec::new();

    for section in &parsed.sections {
        if section.sectname != section_name {
            continue;
        }
        let offset = section.offset as usize;
        let count = (section.size / 8) as usize;

        for i in 0..count {
            let pos = offset + i * 8;
            if pos + 8 > data.len() {
                break;
            }
            let ptr = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            let class_vmaddr = resolve_to_vmaddr(parsed, ptr);

            let (
                name,
                methods,
                class_methods,
                ivars,
                properties,
                instance_size,
                ro_flags,
                superclass_addr,
                metaclass_addr,
                protocol_names,
            ) = if section_name == "__objc_classlist" {
                let name = extract_class_name(parsed, data, ptr);
                let (methods, ivars, properties, isz, flags, proto_names) =
                    extract_class_details(parsed, data, ptr);
                let cm = extract_metaclass_methods(parsed, data, ptr);
                let super_addr = extract_superclass_addr(parsed, data, ptr);
                let meta_addr = extract_metaclass_addr(parsed, data, ptr);
                (
                    name, methods, cm, ivars, properties, isz, flags, super_addr, meta_addr,
                    proto_names,
                )
            } else {
                (
                    None,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    0,
                    0,
                    0,
                    0,
                    Vec::new(),
                )
            };

            refs.push(ObjcClassRef {
                pointer_addr: section.addr + (i as u64 * 8),
                class_data_addr: class_vmaddr,
                name,
                methods,
                class_methods,
                ivars,
                properties,
                protocol_names,
                instance_size,
                ro_flags,
                superclass_addr,
                metaclass_addr,
            });
        }
    }
    refs
}
