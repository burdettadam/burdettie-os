//! Category, initializer, and selector parsing.

use macho_parse::ParsedMachO;

use crate::types::*;
use super::class::parse_method_list;
use super::util::{read_cstring, resolve_on_disk_pointer, resolve_to_vmaddr};

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
