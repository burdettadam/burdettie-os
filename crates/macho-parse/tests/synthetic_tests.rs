//! Tests for macho-parse: synthetic byte-level parsing and malformed input.
//!
//! Ported from:
//!   - goblin crate: bad magic, truncated input, fuzz-found panics
//!   - LLVM llvm-objdump: malformed load commands, truncated sections
//!   - XNU mach_loader.c: header bounds checking
//!   - Apple dyld: segment protection and entry point validation
#![allow(clippy::vec_init_then_push)] // binary format construction

use macho_parse::{parse, ParseError};
use macho_types::header::*;
use macho_types::segment::VmProt;

/// Build a minimal valid 64-bit Mach-O with no load commands.
fn minimal_macho64() -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&MH_MAGIC_64.to_le_bytes()); // magic
    buf.extend_from_slice(&0x0100_000Cu32.to_le_bytes()); // cpu_type: ARM64
    buf.extend_from_slice(&0u32.to_le_bytes()); // cpu_subtype
    buf.extend_from_slice(&2u32.to_le_bytes()); // file_type: MH_EXECUTE
    buf.extend_from_slice(&0u32.to_le_bytes()); // ncmds
    buf.extend_from_slice(&0u32.to_le_bytes()); // sizeofcmds
    buf.extend_from_slice(&0u32.to_le_bytes()); // flags
    buf.extend_from_slice(&0u32.to_le_bytes()); // reserved
    buf
}

/// Append a minimal LC_SEGMENT_64 with no sections to a buffer.
fn append_segment64(buf: &mut Vec<u8>, name: &str, vmaddr: u64, vmsize: u64, prot: u32) {
    let cmd_size: u32 = 72; // sizeof(segment_command_64)
    buf.extend_from_slice(&0x19u32.to_le_bytes()); // LC_SEGMENT_64
    buf.extend_from_slice(&cmd_size.to_le_bytes());
    // segname: 16 bytes, null-padded
    let mut segname = [0u8; 16];
    let name_bytes = name.as_bytes();
    segname[..name_bytes.len().min(16)].copy_from_slice(&name_bytes[..name_bytes.len().min(16)]);
    buf.extend_from_slice(&segname);
    buf.extend_from_slice(&vmaddr.to_le_bytes()); // vmaddr
    buf.extend_from_slice(&vmsize.to_le_bytes()); // vmsize
    buf.extend_from_slice(&0u64.to_le_bytes()); // fileoff
    buf.extend_from_slice(&0u64.to_le_bytes()); // filesize
    buf.extend_from_slice(&prot.to_le_bytes()); // maxprot
    buf.extend_from_slice(&prot.to_le_bytes()); // initprot
    buf.extend_from_slice(&0u32.to_le_bytes()); // nsects
    buf.extend_from_slice(&0u32.to_le_bytes()); // flags
}

/// Append LC_MAIN to a buffer.
fn append_lc_main(buf: &mut Vec<u8>, entry_offset: u64, stack_size: u64) {
    let cmd_size: u32 = 24;
    buf.extend_from_slice(&0x8000_0028u32.to_le_bytes()); // LC_MAIN
    buf.extend_from_slice(&cmd_size.to_le_bytes());
    buf.extend_from_slice(&entry_offset.to_le_bytes());
    buf.extend_from_slice(&stack_size.to_le_bytes());
}

/// Build a complete minimal Mach-O with specified load commands.
fn build_macho64(cmds: &[(&str, u64, u64, u32)], entry: Option<(u64, u64)>) -> Vec<u8> {
    // Calculate sizes
    let segment_cmd_size: u32 = 72;
    let main_cmd_size: u32 = 24;
    let ncmds = cmds.len() as u32 + if entry.is_some() { 1 } else { 0 };
    let sizeofcmds =
        (cmds.len() as u32 * segment_cmd_size) + if entry.is_some() { main_cmd_size } else { 0 };

    let mut buf = Vec::new();
    buf.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
    buf.extend_from_slice(&0x0100_000Cu32.to_le_bytes()); // ARM64
    buf.extend_from_slice(&0u32.to_le_bytes()); // subtype
    buf.extend_from_slice(&2u32.to_le_bytes()); // MH_EXECUTE
    buf.extend_from_slice(&ncmds.to_le_bytes());
    buf.extend_from_slice(&sizeofcmds.to_le_bytes());
    buf.extend_from_slice(&0x0020_0085u32.to_le_bytes()); // PIE|TWO_LEVEL|DYLDLINK|NO_UNDEFS
    buf.extend_from_slice(&0u32.to_le_bytes()); // reserved

    for (name, vmaddr, vmsize, prot) in cmds {
        append_segment64(&mut buf, name, *vmaddr, *vmsize, *prot);
    }

    if let Some((offset, stack)) = entry {
        append_lc_main(&mut buf, offset, stack);
    }

    buf
}

// ── Bad magic (from goblin: DEADBEEF test) ──────────────────────

#[test]
fn reject_bad_magic() {
    let data = 0xDEAD_BEEFu32.to_le_bytes();
    let err = parse(&data).unwrap_err();
    match err {
        ParseError::BadMagic { magic } => {
            assert_eq!(magic, 0xDEAD_BEEF);
        }
        _ => panic!("expected BadMagic, got: {err}"),
    }
}

// ── Empty input (from goblin edge cases) ────────────────────────

#[test]
fn reject_empty_input() {
    let err = parse(&[]).unwrap_err();
    match err {
        ParseError::UnexpectedEof { offset: 0, .. } => {}
        _ => panic!("expected UnexpectedEof at 0, got: {err}"),
    }
}

// ── Truncated header (from XNU mach_loader bounds check) ────────

#[test]
fn reject_truncated_header_3_bytes() {
    let err = parse(&[0xCF, 0xFA, 0xED]).unwrap_err();
    match err {
        ParseError::UnexpectedEof { .. } => {}
        _ => panic!("expected UnexpectedEof, got: {err}"),
    }
}

#[test]
fn reject_truncated_header_16_bytes() {
    // Valid magic + partial header (only cpu_type, cpu_subtype, file_type)
    let mut data = Vec::new();
    data.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
    data.extend_from_slice(&0x0100_000Cu32.to_le_bytes()); // ARM64
    data.extend_from_slice(&0u32.to_le_bytes()); // cpu_subtype
    data.extend_from_slice(&2u32.to_le_bytes()); // file_type
                                                  // Missing: ncmds, sizeofcmds, flags, reserved
    let err = parse(&data).unwrap_err();
    match err {
        ParseError::UnexpectedEof { .. } => {}
        _ => panic!("expected UnexpectedEof, got: {err}"),
    }
}

// ── Minimal valid Mach-O with no commands ───────────────────────

#[test]
fn parse_minimal_no_commands() {
    let data = minimal_macho64();
    let parsed = parse(&data).unwrap();
    assert!(parsed.header.is_64bit());
    assert_eq!(parsed.header.cpu_type, CpuType::Arm64);
    assert_eq!(parsed.header.file_type, FileType::Execute);
    assert_eq!(parsed.commands.len(), 0);
    assert_eq!(parsed.segments.len(), 0);
    assert!(parsed.entry.is_none());
}

// ── Segment parsing (from LLVM lld MachO segment tests) ────────

#[test]
fn parse_pagezero_segment() {
    let data = build_macho64(&[("__PAGEZERO", 0, 0x1_0000_0000, 0)], None);
    let parsed = parse(&data).unwrap();
    assert_eq!(parsed.segments.len(), 1);
    let seg = &parsed.segments[0];
    assert_eq!(seg.name, "__PAGEZERO");
    assert_eq!(seg.vmaddr, 0);
    assert_eq!(seg.vmsize, 0x1_0000_0000);
    assert_eq!(seg.initprot, VmProt::empty());
}

#[test]
fn parse_text_segment() {
    let data = build_macho64(
        &[("__TEXT", 0x1_0000_0000, 0x4000, 5)], // r-x
        None,
    );
    let parsed = parse(&data).unwrap();
    let seg = &parsed.segments[0];
    assert_eq!(seg.name, "__TEXT");
    assert_eq!(seg.vmaddr, 0x1_0000_0000);
    assert_eq!(seg.initprot, VmProt::READ | VmProt::EXECUTE);
}

#[test]
fn parse_data_segment() {
    let data = build_macho64(
        &[("__DATA", 0x1_0000_4000, 0x4000, 3)], // rw-
        None,
    );
    let parsed = parse(&data).unwrap();
    let seg = &parsed.segments[0];
    assert_eq!(seg.name, "__DATA");
    assert_eq!(seg.initprot, VmProt::READ | VmProt::WRITE);
}

#[test]
fn parse_linkedit_segment() {
    let data = build_macho64(
        &[("__LINKEDIT", 0x1_0000_8000, 0x4000, 1)], // r--
        None,
    );
    let parsed = parse(&data).unwrap();
    let seg = &parsed.segments[0];
    assert_eq!(seg.name, "__LINKEDIT");
    assert_eq!(seg.initprot, VmProt::READ);
}

// ── Standard 4-segment layout (from lld: basic-layout test) ─────

#[test]
fn parse_standard_segment_layout() {
    let data = build_macho64(
        &[
            ("__PAGEZERO", 0, 0x1_0000_0000, 0),      // ---
            ("__TEXT", 0x1_0000_0000, 0x4000, 5),      // r-x
            ("__DATA", 0x1_0000_4000, 0x4000, 3),      // rw-
            ("__LINKEDIT", 0x1_0000_8000, 0x4000, 1),  // r--
        ],
        Some((0x1000, 0)),
    );
    let parsed = parse(&data).unwrap();
    assert_eq!(parsed.segments.len(), 4);

    // Verify ordering
    assert_eq!(parsed.segments[0].name, "__PAGEZERO");
    assert_eq!(parsed.segments[1].name, "__TEXT");
    assert_eq!(parsed.segments[2].name, "__DATA");
    assert_eq!(parsed.segments[3].name, "__LINKEDIT");

    // Verify entry point
    assert!(parsed.entry.is_some());
    match &parsed.entry {
        Some(macho_types::EntryPoint::Main { entry_offset, .. }) => {
            assert_eq!(*entry_offset, 0x1000);
        }
        _ => panic!("expected LC_MAIN entry point"),
    }
}

// ── Load command too small (from LLVM: malformed-lc-size) ───────

#[test]
fn reject_load_command_too_small() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
    buf.extend_from_slice(&0x0100_000Cu32.to_le_bytes()); // ARM64
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&2u32.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes()); // ncmds = 1
    buf.extend_from_slice(&8u32.to_le_bytes()); // sizeofcmds
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes()); // reserved

    // Load command with size < 8
    buf.extend_from_slice(&0x19u32.to_le_bytes()); // LC_SEGMENT_64
    buf.extend_from_slice(&4u32.to_le_bytes()); // size = 4 (invalid, min is 8)

    let err = parse(&buf).unwrap_err();
    match err {
        ParseError::LoadCommandTooSmall { size: 4, min: 8 } => {}
        _ => panic!("expected LoadCommandTooSmall, got: {err}"),
    }
}

// ── Header flags preserved (from XNU: flag validation) ──────────

#[test]
fn parse_preserves_flags() {
    let data = build_macho64(&[], None);
    let parsed = parse(&data).unwrap();
    assert!(parsed.header.flags.contains(MachFlags::PIE));
    assert!(parsed.header.flags.contains(MachFlags::TWO_LEVEL));
    assert!(parsed.header.flags.contains(MachFlags::DYLDLINK));
    assert!(parsed.header.flags.contains(MachFlags::NO_UNDEFS));
}

// ── Multiple segments have correct count ────────────────────────

#[test]
fn parse_multiple_segments_correct_count() {
    let data = build_macho64(
        &[
            ("__PAGEZERO", 0, 0x1_0000_0000, 0),
            ("__TEXT", 0x1_0000_0000, 0x4000, 5),
            ("__DATA", 0x1_0000_4000, 0x2000, 3),
            ("__LINKEDIT", 0x1_0000_8000, 0x1000, 1),
        ],
        None,
    );
    let parsed = parse(&data).unwrap();
    assert_eq!(parsed.header.num_commands, 4);
    assert_eq!(parsed.commands.len(), 4);
    assert_eq!(parsed.segments.len(), 4);
}

// ── Truncated load command data ─────────────────────────────────

#[test]
fn reject_truncated_segment_command() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
    buf.extend_from_slice(&0x0100_000Cu32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&2u32.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes()); // ncmds = 1
    buf.extend_from_slice(&72u32.to_le_bytes()); // sizeofcmds = 72
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes()); // reserved

    // Truncated LC_SEGMENT_64: only cmd + cmdsize (8 bytes), missing rest
    buf.extend_from_slice(&0x19u32.to_le_bytes());
    buf.extend_from_slice(&72u32.to_le_bytes());
    // Only 16 more bytes of segment name, not the full 64 remaining
    buf.extend_from_slice(&[0u8; 16]);

    let err = parse(&buf).unwrap_err();
    match err {
        ParseError::UnexpectedEof { .. } => {}
        _ => panic!("expected UnexpectedEof, got: {err}"),
    }
}

// ── JSON serialization roundtrip (invariant: every stage emits JSON) ──

#[test]
fn parsed_macho_serializes_to_json() {
    let data = build_macho64(
        &[
            ("__PAGEZERO", 0, 0x1_0000_0000, 0),
            ("__TEXT", 0x1_0000_0000, 0x4000, 5),
        ],
        Some((0x1000, 0)),
    );
    let parsed = parse(&data).unwrap();
    let json = serde_json::to_string_pretty(&parsed).unwrap();
    assert!(json.contains("__PAGEZERO"));
    assert!(json.contains("__TEXT"));
    assert!(json.contains("Arm64"));
}
