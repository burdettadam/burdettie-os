//! Private parsing helpers for ObjC metadata extraction.

use macho_parse::{vmaddr_to_file_offset, ParsedMachO};

use crate::types::*;

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
            ) = if section_name == "__objc_classlist" {
                let name = extract_class_name(parsed, data, ptr);
                let (methods, ivars, properties, isz, flags) =
                    extract_class_details(parsed, data, ptr);
                let cm = extract_metaclass_methods(parsed, data, ptr);
                let super_addr = extract_superclass_addr(parsed, data, ptr);
                let meta_addr = extract_metaclass_addr(parsed, data, ptr);
                (
                    name, methods, cm, ivars, properties, isz, flags, super_addr, meta_addr,
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
                instance_size,
                ro_flags,
                superclass_addr,
                metaclass_addr,
            });
        }
    }
    refs
}

pub fn count_pointer_list(parsed: &ParsedMachO, _data: &[u8], section_name: &str) -> usize {
    parsed
        .sections
        .iter()
        .find(|s| s.sectname == section_name)
        .map(|s| (s.size / 8) as usize)
        .unwrap_or(0)
}

// ── Category parsing ───────────────────────────────────────────────

pub fn parse_catlist(parsed: &ParsedMachO, data: &[u8]) -> Vec<ObjcCategoryRef> {
    let mut cats = Vec::new();

    for section in &parsed.sections {
        if section.sectname != "__objc_catlist" {
            continue;
        }
        let offset = section.offset as usize;
        let count = (section.size / 8) as usize;

        for i in 0..count {
            let pos = offset + i * 8;
            if pos + 8 > data.len() {
                break;
            }
            let cat_ptr = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());

            let Some(cat_off) = resolve_on_disk_pointer(parsed, data, cat_ptr) else {
                continue;
            };
            if cat_off + 32 > data.len() {
                continue;
            }

            let name_raw = u64::from_le_bytes(data[cat_off..cat_off + 8].try_into().unwrap());
            let name = resolve_on_disk_pointer(parsed, data, name_raw)
                .and_then(|off| read_cstring(data, off));

            let cls_raw = u64::from_le_bytes(data[cat_off + 8..cat_off + 16].try_into().unwrap());
            let class_vmaddr = resolve_to_vmaddr(parsed, cls_raw);

            let inst_raw = u64::from_le_bytes(data[cat_off + 16..cat_off + 24].try_into().unwrap());
            let instance_methods = if inst_raw != 0 {
                parse_method_list(parsed, data, inst_raw)
            } else {
                Vec::new()
            };

            let cls_m_raw = if cat_off + 32 <= data.len() {
                u64::from_le_bytes(data[cat_off + 24..cat_off + 32].try_into().unwrap())
            } else {
                0
            };
            let class_methods = if cls_m_raw != 0 {
                parse_method_list(parsed, data, cls_m_raw)
            } else {
                Vec::new()
            };

            let cat_struct_vmaddr = resolve_to_vmaddr(parsed, cat_ptr);
            cats.push(ObjcCategoryRef {
                name,
                class_vmaddr,
                cat_struct_vmaddr,
                instance_methods,
                class_methods,
            });
        }
    }
    cats
}

// ── Initializer parsing ────────────────────────────────────────────

pub fn parse_function_pointer_array(data: &[u8], offset: usize, size: usize) -> Vec<u64> {
    let count = size / 8;
    let mut addrs = Vec::with_capacity(count);
    for i in 0..count {
        let pos = offset + i * 8;
        if pos + 8 > data.len() {
            break;
        }
        addrs.push(u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap()));
    }
    addrs
}

pub fn parse_init_offsets(data: &[u8], offset: usize, size: usize, text_vmaddr: u64) -> Vec<u64> {
    let count = size / 4;
    let mut addrs = Vec::with_capacity(count);
    for i in 0..count {
        let pos = offset + i * 4;
        if pos + 4 > data.len() {
            break;
        }
        let off = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        addrs.push(text_vmaddr + off as u64);
    }
    addrs
}

pub fn parse_image_info(parsed: &ParsedMachO, data: &[u8]) -> Option<ObjcImageInfo> {
    for section in &parsed.sections {
        if section.sectname == "__objc_imageinfo" {
            let offset = section.offset as usize;
            if offset + 8 > data.len() {
                return None;
            }
            let version = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let flags = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            return Some(ObjcImageInfo { version, flags });
        }
    }
    None
}

// ── Class detail extraction ────────────────────────────────────────

fn extract_class_name(parsed: &ParsedMachO, data: &[u8], class_ptr_raw: u64) -> Option<String> {
    let class_off = resolve_on_disk_pointer(parsed, data, class_ptr_raw)?;
    if class_off + 40 > data.len() {
        return None;
    }
    let data_raw = u64::from_le_bytes(data[class_off + 32..class_off + 40].try_into().unwrap());

    let ro_off = resolve_on_disk_pointer(parsed, data, data_raw)?;
    if ro_off + 32 > data.len() {
        return None;
    }
    let name_raw = u64::from_le_bytes(data[ro_off + 24..ro_off + 32].try_into().unwrap());

    let name_off = resolve_on_disk_pointer(parsed, data, name_raw)?;
    read_cstring(data, name_off)
}

fn extract_class_details(
    parsed: &ParsedMachO,
    data: &[u8],
    class_ptr_raw: u64,
) -> (
    Vec<ObjcMethodInfo>,
    Vec<ObjcIvarInfo>,
    Vec<ObjcPropertyInfo>,
    u32,
    u32,
) {
    let empty = (Vec::new(), Vec::new(), Vec::new(), 0, 0);
    let Some(class_off) = resolve_on_disk_pointer(parsed, data, class_ptr_raw) else {
        return empty;
    };
    if class_off + 40 > data.len() {
        return empty;
    }
    let data_raw = u64::from_le_bytes(data[class_off + 32..class_off + 40].try_into().unwrap());

    let Some(ro_off) = resolve_on_disk_pointer(parsed, data, data_raw) else {
        return empty;
    };
    if ro_off + 40 > data.len() {
        return empty;
    }

    let ro_flags = u32::from_le_bytes(data[ro_off..ro_off + 4].try_into().unwrap());
    let instance_size = u32::from_le_bytes(data[ro_off + 8..ro_off + 12].try_into().unwrap());
    let method_list_raw = u64::from_le_bytes(data[ro_off + 32..ro_off + 40].try_into().unwrap());

    let methods = if method_list_raw != 0 {
        parse_method_list(parsed, data, method_list_raw)
    } else {
        Vec::new()
    };

    // ivars at class_ro_t offset 48, properties at offset 64
    let ivars = if ro_off + 56 <= data.len() {
        let ivars_raw = u64::from_le_bytes(data[ro_off + 48..ro_off + 56].try_into().unwrap());
        if ivars_raw != 0 {
            parse_ivar_list(parsed, data, ivars_raw)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let properties = if ro_off + 72 <= data.len() {
        let props_raw = u64::from_le_bytes(data[ro_off + 64..ro_off + 72].try_into().unwrap());
        if props_raw != 0 {
            parse_property_list(parsed, data, props_raw)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    (methods, ivars, properties, instance_size, ro_flags)
}

fn extract_metaclass_methods(
    parsed: &ParsedMachO,
    data: &[u8],
    class_ptr_raw: u64,
) -> Vec<ObjcMethodInfo> {
    let Some(class_off) = resolve_on_disk_pointer(parsed, data, class_ptr_raw) else {
        return Vec::new();
    };
    if class_off + 8 > data.len() {
        return Vec::new();
    }
    let isa_raw = u64::from_le_bytes(data[class_off..class_off + 8].try_into().unwrap());

    let Some(meta_off) = resolve_on_disk_pointer(parsed, data, isa_raw) else {
        return Vec::new();
    };
    if meta_off + 40 > data.len() {
        return Vec::new();
    }
    let data_raw = u64::from_le_bytes(data[meta_off + 32..meta_off + 40].try_into().unwrap());

    let Some(ro_off) = resolve_on_disk_pointer(parsed, data, data_raw) else {
        return Vec::new();
    };
    if ro_off + 40 > data.len() {
        return Vec::new();
    }
    let method_list_raw = u64::from_le_bytes(data[ro_off + 32..ro_off + 40].try_into().unwrap());

    if method_list_raw != 0 {
        parse_method_list(parsed, data, method_list_raw)
    } else {
        Vec::new()
    }
}

fn extract_superclass_addr(parsed: &ParsedMachO, data: &[u8], class_ptr_raw: u64) -> u64 {
    let Some(class_off) = resolve_on_disk_pointer(parsed, data, class_ptr_raw) else {
        return 0;
    };
    if class_off + 16 > data.len() {
        return 0;
    }
    let super_raw = u64::from_le_bytes(data[class_off + 8..class_off + 16].try_into().unwrap());
    if super_raw == 0 {
        return 0;
    }
    resolve_to_vmaddr(parsed, super_raw)
}

fn extract_metaclass_addr(parsed: &ParsedMachO, data: &[u8], class_ptr_raw: u64) -> u64 {
    let Some(class_off) = resolve_on_disk_pointer(parsed, data, class_ptr_raw) else {
        return 0;
    };
    if class_off + 8 > data.len() {
        return 0;
    }
    let isa_raw = u64::from_le_bytes(data[class_off..class_off + 8].try_into().unwrap());
    if isa_raw == 0 {
        return 0;
    }
    resolve_to_vmaddr(parsed, isa_raw)
}

// ── Ivar list parsing ──────────────────────────────────────────────

/// Parse an ivar_list_t from the binary.
///
/// Layout: u32 entsize_and_flags, u32 count, then N × ivar_t entries.
/// Each ivar_t (64-bit): offset_ptr(8) + name_ptr(8) + type_ptr(8) + alignment_raw(u32) + size(u32) = 32 bytes
fn parse_ivar_list(parsed: &ParsedMachO, data: &[u8], ivar_list_raw: u64) -> Vec<ObjcIvarInfo> {
    let Some(il_off) = resolve_on_disk_pointer(parsed, data, ivar_list_raw) else {
        return Vec::new();
    };
    if il_off + 8 > data.len() {
        return Vec::new();
    }

    let _entsize = u32::from_le_bytes(data[il_off..il_off + 4].try_into().unwrap());
    let count = u32::from_le_bytes(data[il_off + 4..il_off + 8].try_into().unwrap());
    if count == 0 || count > 10000 {
        return Vec::new();
    }

    let entries_start = il_off + 8;
    let mut ivars = Vec::with_capacity(count as usize);

    for i in 0..count as usize {
        let entry_off = entries_start + i * 32;
        if entry_off + 32 > data.len() {
            break;
        }

        let offset_ptr = u64::from_le_bytes(data[entry_off..entry_off + 8].try_into().unwrap());
        let name_raw = u64::from_le_bytes(data[entry_off + 8..entry_off + 16].try_into().unwrap());
        let type_raw = u64::from_le_bytes(data[entry_off + 16..entry_off + 24].try_into().unwrap());
        let alignment_raw =
            u32::from_le_bytes(data[entry_off + 24..entry_off + 28].try_into().unwrap());
        let size = u32::from_le_bytes(data[entry_off + 28..entry_off + 32].try_into().unwrap());

        let name = resolve_on_disk_pointer(parsed, data, name_raw)
            .and_then(|off| read_cstring(data, off))
            .unwrap_or_default();

        let type_encoding = resolve_on_disk_pointer(parsed, data, type_raw)
            .and_then(|off| read_cstring(data, off))
            .unwrap_or_default();

        // The offset_ptr points to a u32 containing the ivar's byte offset within the instance.
        let offset = resolve_on_disk_pointer(parsed, data, offset_ptr)
            .and_then(|off| {
                if off + 4 <= data.len() {
                    Some(u32::from_le_bytes(data[off..off + 4].try_into().unwrap()))
                } else {
                    None
                }
            })
            .unwrap_or(0);

        if !name.is_empty() {
            ivars.push(ObjcIvarInfo {
                name,
                type_encoding,
                offset,
                size,
                alignment: alignment_raw,
            });
        }
    }
    ivars
}

// ── Property list parsing ──────────────────────────────────────────

/// Parse a property_list_t from the binary.
///
/// Layout: u32 entsize, u32 count, then N × property_t entries.
/// Each property_t (64-bit): name_ptr(8) + attributes_ptr(8) = 16 bytes
fn parse_property_list(
    parsed: &ParsedMachO,
    data: &[u8],
    prop_list_raw: u64,
) -> Vec<ObjcPropertyInfo> {
    let Some(pl_off) = resolve_on_disk_pointer(parsed, data, prop_list_raw) else {
        return Vec::new();
    };
    if pl_off + 8 > data.len() {
        return Vec::new();
    }

    let _entsize = u32::from_le_bytes(data[pl_off..pl_off + 4].try_into().unwrap());
    let count = u32::from_le_bytes(data[pl_off + 4..pl_off + 8].try_into().unwrap());
    if count == 0 || count > 10000 {
        return Vec::new();
    }

    let entries_start = pl_off + 8;
    let mut properties = Vec::with_capacity(count as usize);

    for i in 0..count as usize {
        let entry_off = entries_start + i * 16;
        if entry_off + 16 > data.len() {
            break;
        }

        let name_raw = u64::from_le_bytes(data[entry_off..entry_off + 8].try_into().unwrap());
        let attr_raw = u64::from_le_bytes(data[entry_off + 8..entry_off + 16].try_into().unwrap());

        let name = resolve_on_disk_pointer(parsed, data, name_raw)
            .and_then(|off| read_cstring(data, off))
            .unwrap_or_default();

        let attributes = resolve_on_disk_pointer(parsed, data, attr_raw)
            .and_then(|off| read_cstring(data, off))
            .unwrap_or_default();

        if !name.is_empty() {
            properties.push(ObjcPropertyInfo { name, attributes });
        }
    }
    properties
}

// ── Method list parsing ────────────────────────────────────────────

fn parse_method_list(
    parsed: &ParsedMachO,
    data: &[u8],
    method_list_raw: u64,
) -> Vec<ObjcMethodInfo> {
    let Some(ml_off) = resolve_on_disk_pointer(parsed, data, method_list_raw) else {
        return Vec::new();
    };
    if ml_off + 8 > data.len() {
        return Vec::new();
    }

    let entsize_and_flags = u32::from_le_bytes(data[ml_off..ml_off + 4].try_into().unwrap());
    let count = u32::from_le_bytes(data[ml_off + 4..ml_off + 8].try_into().unwrap());
    if count == 0 || count > 10000 {
        return Vec::new();
    }

    let is_relative = (entsize_and_flags & 0x8000_0000) != 0;
    let entries_start = ml_off + 8;
    let mut methods = Vec::with_capacity(count as usize);

    if is_relative {
        for i in 0..count as usize {
            let entry_off = entries_start + i * 12;
            if entry_off + 12 > data.len() {
                break;
            }
            let sel_rel = i32::from_le_bytes(data[entry_off..entry_off + 4].try_into().unwrap());
            let types_rel =
                i32::from_le_bytes(data[entry_off + 4..entry_off + 8].try_into().unwrap());
            let imp_rel =
                i32::from_le_bytes(data[entry_off + 8..entry_off + 12].try_into().unwrap());

            let entry_vmaddr = file_offset_to_vmaddr(parsed, entry_off as u64);

            let selref_vmaddr = (entry_vmaddr as i64 + sel_rel as i64) as u64;
            let selector = vmaddr_to_file_offset(parsed, selref_vmaddr)
                .and_then(|selref_off| {
                    if selref_off + 8 > data.len() {
                        return None;
                    }
                    let sel_ptr =
                        u64::from_le_bytes(data[selref_off..selref_off + 8].try_into().unwrap());
                    resolve_on_disk_pointer(parsed, data, sel_ptr)
                        .and_then(|str_off| read_cstring(data, str_off))
                })
                .unwrap_or_default();

            let types_vmaddr = (entry_vmaddr as i64 + 4 + types_rel as i64) as u64;
            let type_encoding = vmaddr_to_file_offset(parsed, types_vmaddr)
                .and_then(|off| read_cstring(data, off))
                .unwrap_or_default();

            let imp_vmaddr = (entry_vmaddr as i64 + 8 + imp_rel as i64) as u64;

            if !selector.is_empty() {
                methods.push(ObjcMethodInfo {
                    selector,
                    type_encoding,
                    imp_vmaddr,
                });
            }
        }
    } else {
        for i in 0..count as usize {
            let entry_off = entries_start + i * 24;
            if entry_off + 24 > data.len() {
                break;
            }
            let sel_raw = u64::from_le_bytes(data[entry_off..entry_off + 8].try_into().unwrap());
            let types_raw =
                u64::from_le_bytes(data[entry_off + 8..entry_off + 16].try_into().unwrap());
            let imp_raw =
                u64::from_le_bytes(data[entry_off + 16..entry_off + 24].try_into().unwrap());

            let selector = resolve_on_disk_pointer(parsed, data, sel_raw)
                .and_then(|off| read_cstring(data, off))
                .unwrap_or_default();
            let type_encoding = resolve_on_disk_pointer(parsed, data, types_raw)
                .and_then(|off| read_cstring(data, off))
                .unwrap_or_default();

            let target = imp_raw & 0xFF_FFFF_FFFF;
            let text_vmaddr = parsed
                .segments
                .iter()
                .find(|s| s.name == "__TEXT")
                .map(|s| s.vmaddr)
                .unwrap_or(0);
            let imp_vmaddr = text_vmaddr + target;

            if !selector.is_empty() {
                methods.push(ObjcMethodInfo {
                    selector,
                    type_encoding,
                    imp_vmaddr,
                });
            }
        }
    }
    methods
}

// ── Selector parsing ───────────────────────────────────────────────

pub fn parse_selrefs(parsed: &ParsedMachO, data: &[u8]) -> (Vec<String>, Vec<u64>) {
    let mut selectors = Vec::new();
    let mut addrs = Vec::new();

    for section in &parsed.sections {
        if section.sectname != "__objc_selrefs" {
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
            let slot_vmaddr = section.addr + (i as u64 * 8);

            if let Some(off) = resolve_on_disk_pointer(parsed, data, ptr) {
                if let Some(name) = read_cstring(data, off) {
                    selectors.push(name);
                    addrs.push(slot_vmaddr);
                }
            }
        }
    }
    (selectors, addrs)
}

// ── Pointer utilities ──────────────────────────────────────────────

fn file_offset_to_vmaddr(parsed: &ParsedMachO, file_offset: u64) -> u64 {
    for seg in &parsed.segments {
        if seg.fileoff == 0 && seg.filesize == 0 {
            continue;
        }
        if file_offset >= seg.fileoff && file_offset < seg.fileoff + seg.filesize {
            return seg.vmaddr + (file_offset - seg.fileoff);
        }
    }
    file_offset
}

fn resolve_to_vmaddr(parsed: &ParsedMachO, raw: u64) -> u64 {
    let target = raw & 0xFF_FFFF_FFFF;
    let text_vmaddr = parsed
        .segments
        .iter()
        .find(|s| s.name == "__TEXT")
        .map(|s| s.vmaddr)
        .unwrap_or(0);
    text_vmaddr + target
}

fn resolve_on_disk_pointer(parsed: &ParsedMachO, data: &[u8], raw: u64) -> Option<usize> {
    let target = raw & 0xFF_FFFF_FFFF;
    let text_vmaddr = parsed
        .segments
        .iter()
        .find(|s| s.name == "__TEXT")
        .map(|s| s.vmaddr)
        .unwrap_or(0);

    let vmaddr = text_vmaddr + target;
    if let Some(off) = vmaddr_to_file_offset(parsed, vmaddr) {
        if off < data.len() {
            return Some(off);
        }
    }
    let off = target as usize;
    if off < data.len() {
        Some(off)
    } else {
        None
    }
}

fn read_cstring(data: &[u8], offset: usize) -> Option<String> {
    if offset >= data.len() {
        return None;
    }
    let end = data[offset..]
        .iter()
        .position(|&b| b == 0)
        .map(|p| offset + p)
        .unwrap_or(data.len());
    let s = std::str::from_utf8(&data[offset..end]).ok()?;
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
