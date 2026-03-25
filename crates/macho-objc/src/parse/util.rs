//! Pointer resolution and string utilities.

use macho_parse::{vmaddr_to_file_offset, ParsedMachO};

pub(super) fn file_offset_to_vmaddr(parsed: &ParsedMachO, file_offset: u64) -> u64 {
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

pub fn resolve_to_vmaddr(parsed: &ParsedMachO, raw: u64) -> u64 {
    let target = raw & 0xFF_FFFF_FFFF;
    let text_vmaddr = parsed
        .segments
        .iter()
        .find(|s| s.name == "__TEXT")
        .map(|s| s.vmaddr)
        .unwrap_or(0);
    text_vmaddr + target
}

pub fn resolve_on_disk_pointer(parsed: &ParsedMachO, data: &[u8], raw: u64) -> Option<usize> {
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

pub fn read_cstring(data: &[u8], offset: usize) -> Option<String> {
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
