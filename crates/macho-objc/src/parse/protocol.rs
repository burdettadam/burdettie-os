//! Protocol parsing from __objc_protolist section.
//!
//! Layout of `protocol_t` (arm64):
//!   +0   isa              (u64, always NULL for on-disk protocols)
//!   +8   mangledName      (u64 → char*)
//!   +16  protocols        (u64 → protocol_list_t*)
//!   +24  instanceMethods  (u64 → method_list_t*)
//!   +32  classMethods     (u64 → method_list_t*)
//!   +40  optionalInstanceMethods (u64 → method_list_t*)
//!   +48  optionalClassMethods   (u64 → method_list_t*)
//!   +56  instanceProperties     (u64 → property_list_t*)

use macho_parse::ParsedMachO;

use super::class::{parse_method_list, parse_property_list};
use super::util::{read_cstring, resolve_on_disk_pointer};
use crate::types::ObjcProtocolRef;

/// Parse the `__objc_protolist` section and return all protocol definitions.
pub fn parse_protolist(parsed: &ParsedMachO, data: &[u8]) -> Vec<ObjcProtocolRef> {
    let mut protocols = Vec::new();

    for section in &parsed.sections {
        if section.sectname != "__objc_protolist" {
            continue;
        }
        let offset = section.offset as usize;
        let count = (section.size / 8) as usize;

        for i in 0..count {
            let pos = offset + i * 8;
            if pos + 8 > data.len() {
                break;
            }
            let proto_ptr = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            if let Some(proto) = parse_protocol_t(parsed, data, proto_ptr) {
                protocols.push(proto);
            }
        }
    }
    protocols
}

/// Parse a single `protocol_t` structure from a raw pointer.
fn parse_protocol_t(
    parsed: &ParsedMachO,
    data: &[u8],
    proto_ptr: u64,
) -> Option<ObjcProtocolRef> {
    let proto_off = resolve_on_disk_pointer(parsed, data, proto_ptr)?;
    // Need at least 64 bytes for the protocol_t struct
    if proto_off + 64 > data.len() {
        return None;
    }

    // +8: mangledName
    let name_raw = u64::from_le_bytes(data[proto_off + 8..proto_off + 16].try_into().unwrap());
    let name = resolve_on_disk_pointer(parsed, data, name_raw)
        .and_then(|off| read_cstring(data, off))?;

    // +16: protocols (adopted protocols list)
    let proto_list_raw =
        u64::from_le_bytes(data[proto_off + 16..proto_off + 24].try_into().unwrap());
    let adopted_protocols = if proto_list_raw != 0 {
        parse_protocol_name_list(parsed, data, proto_list_raw)
    } else {
        Vec::new()
    };

    // +24: instanceMethods
    let inst_methods_raw =
        u64::from_le_bytes(data[proto_off + 24..proto_off + 32].try_into().unwrap());
    let instance_methods = if inst_methods_raw != 0 {
        parse_method_list(parsed, data, inst_methods_raw)
    } else {
        Vec::new()
    };

    // +32: classMethods
    let class_methods_raw =
        u64::from_le_bytes(data[proto_off + 32..proto_off + 40].try_into().unwrap());
    let class_methods = if class_methods_raw != 0 {
        parse_method_list(parsed, data, class_methods_raw)
    } else {
        Vec::new()
    };

    // +40: optionalInstanceMethods
    let opt_inst_raw =
        u64::from_le_bytes(data[proto_off + 40..proto_off + 48].try_into().unwrap());
    let optional_instance_methods = if opt_inst_raw != 0 {
        parse_method_list(parsed, data, opt_inst_raw)
    } else {
        Vec::new()
    };

    // +48: optionalClassMethods
    let opt_class_raw =
        u64::from_le_bytes(data[proto_off + 48..proto_off + 56].try_into().unwrap());
    let optional_class_methods = if opt_class_raw != 0 {
        parse_method_list(parsed, data, opt_class_raw)
    } else {
        Vec::new()
    };

    // +56: instanceProperties
    let props_raw =
        u64::from_le_bytes(data[proto_off + 56..proto_off + 64].try_into().unwrap());
    let properties = if props_raw != 0 {
        parse_property_list(parsed, data, props_raw)
    } else {
        Vec::new()
    };

    Some(ObjcProtocolRef {
        name,
        instance_methods,
        class_methods,
        optional_instance_methods,
        optional_class_methods,
        properties,
        adopted_protocols,
    })
}

/// Parse a `protocol_list_t` extracting just the protocol names.
///
/// Layout: u64 count, then count × u64 protocol pointers.
/// Each protocol pointer → protocol_t, from which we read the name.
fn parse_protocol_name_list(
    parsed: &ParsedMachO,
    data: &[u8],
    list_raw: u64,
) -> Vec<String> {
    let Some(list_off) = resolve_on_disk_pointer(parsed, data, list_raw) else {
        return Vec::new();
    };
    if list_off + 8 > data.len() {
        return Vec::new();
    }

    let count = u64::from_le_bytes(data[list_off..list_off + 8].try_into().unwrap());
    if count == 0 || count > 10000 {
        return Vec::new();
    }

    let mut names = Vec::with_capacity(count as usize);
    let entries_start = list_off + 8;

    for i in 0..count as usize {
        let entry_off = entries_start + i * 8;
        if entry_off + 8 > data.len() {
            break;
        }
        let proto_ptr = u64::from_le_bytes(data[entry_off..entry_off + 8].try_into().unwrap());
        if proto_ptr == 0 {
            continue;
        }
        // Read the protocol name from the protocol_t at proto_ptr (+8 = mangledName)
        if let Some(proto_off) = resolve_on_disk_pointer(parsed, data, proto_ptr) {
            if proto_off + 16 <= data.len() {
                let name_raw =
                    u64::from_le_bytes(data[proto_off + 8..proto_off + 16].try_into().unwrap());
                if let Some(name) = resolve_on_disk_pointer(parsed, data, name_raw)
                    .and_then(|off| read_cstring(data, off))
                {
                    names.push(name);
                }
            }
        }
    }
    names
}

/// Parse the baseProtocols field from `class_ro_t` at offset +40.
///
/// Returns a list of protocol names that the class conforms to.
/// Layout of protocol_list_t: u64 count, then count × u64 protocol pointers.
pub(super) fn parse_base_protocols(
    parsed: &ParsedMachO,
    data: &[u8],
    base_protocols_raw: u64,
) -> Vec<String> {
    parse_protocol_name_list(parsed, data, base_protocols_raw)
}
