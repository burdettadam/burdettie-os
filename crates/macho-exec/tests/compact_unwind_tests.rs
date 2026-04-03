//! Tests for compact unwind → .eh_frame conversion on real fixture binaries.

use macho_exec::compact_to_ehframe::{generate_eh_frame, parse_compact_unwind};
use macho_parse::parse;

fn read_fixture(name: &str) -> Option<Vec<u8>> {
    macho_fixtures::read_fixture(name)
}

/// Extract the raw `__unwind_info` section bytes from a parsed Mach-O.
fn extract_unwind_info(data: &[u8]) -> Option<Vec<u8>> {
    let parsed = parse(data).ok()?;
    for seg in &parsed.segments {
        if seg.name == "__TEXT" {
            for sect in &seg.sections {
                if sect.sectname == "__unwind_info" {
                    let start = sect.offset as usize;
                    let end = start + sect.size as usize;
                    if end <= data.len() {
                        return Some(data[start..end].to_vec());
                    }
                }
            }
        }
    }
    None
}

#[test]
fn simple_cli_compact_to_ehframe_roundtrip() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let Some(cu_data) = extract_unwind_info(&data) else {
        panic!("simple_cli/app should have __unwind_info");
    };

    let entries = parse_compact_unwind(&cu_data);
    assert!(!entries.is_empty(), "should parse at least one compact entry");

    // All entries should have non-zero encodings.
    for e in &entries {
        assert_ne!(e.encoding, 0, "entry at offset {:#x} has zero encoding", e.func_offset);
    }

    // Generate .eh_frame.
    let eh = generate_eh_frame(&entries, 0x1_0000_0000);
    assert!(!eh.is_empty());

    // Validate structure: CIE at start, then FDE(s), then zero terminator.
    let cie_len = u32::from_ne_bytes([eh[0], eh[1], eh[2], eh[3]]);
    assert!(cie_len > 0, "CIE length should be non-zero");

    let cie_id = u32::from_ne_bytes([eh[4], eh[5], eh[6], eh[7]]);
    assert_eq!(cie_id, 0, "CIE identifier should be 0");

    // At least one FDE should follow the CIE.
    let fde_off = 4 + cie_len as usize;
    assert!(fde_off + 8 <= eh.len(), "should have at least one FDE after CIE");
    let fde_len = u32::from_ne_bytes([eh[fde_off], eh[fde_off+1], eh[fde_off+2], eh[fde_off+3]]);
    assert!(fde_len > 0, "first FDE should have non-zero length");

    // Terminator at the end.
    let last4 = &eh[eh.len()-4..];
    let term = u32::from_ne_bytes([last4[0], last4[1], last4[2], last4[3]]);
    assert_eq!(term, 0, ".eh_frame should end with zero terminator");
}

#[test]
fn exception_basics_compact_to_ehframe() {
    let Some(data) = read_fixture("exception_basics/app") else {
        return;
    };
    let Some(cu_data) = extract_unwind_info(&data) else {
        return; // skip if no unwind info
    };

    let entries = parse_compact_unwind(&cu_data);
    assert!(!entries.is_empty());

    let eh = generate_eh_frame(&entries, 0x2_0000_0000);
    assert!(!eh.is_empty());

    // Count FDEs: walk the .eh_frame, each record is length + content.
    let mut fde_count = 0;
    let mut pos = 0;
    while pos + 4 <= eh.len() {
        let len = u32::from_ne_bytes([eh[pos], eh[pos+1], eh[pos+2], eh[pos+3]]);
        if len == 0 {
            break; // terminator
        }
        let record_end = pos + 4 + len as usize;
        if record_end > eh.len() {
            break;
        }
        // Check if this is an FDE (CIE pointer != 0).
        if pos + 8 <= eh.len() {
            let id = u32::from_ne_bytes([eh[pos+4], eh[pos+5], eh[pos+6], eh[pos+7]]);
            if id != 0 {
                fde_count += 1;
            }
        }
        pos = record_end;
    }

    assert!(
        fde_count >= 1,
        "should have generated at least 1 FDE, got {fde_count}"
    );
}

#[test]
fn autorelease_has_multiple_entries() {
    let Some(data) = read_fixture("autorelease/app") else {
        return;
    };
    let Some(cu_data) = extract_unwind_info(&data) else {
        return;
    };

    let entries = parse_compact_unwind(&cu_data);
    assert!(
        !entries.is_empty(),
        "autorelease should have unwind entries"
    );

    // Function lengths should be non-zero (except possibly the last).
    for (i, e) in entries.iter().enumerate() {
        if i < entries.len() - 1 {
            assert!(e.func_length > 0, "entry {i} at {:#x} has zero length", e.func_offset);
        }
    }

    let eh = generate_eh_frame(&entries, 0x1_0000_4000);
    assert!(!eh.is_empty());
}

#[test]
fn minimal_throw_compact_unwind_carries_exception_metadata() {
    let Some(data) = read_fixture("minimal_throw/app") else {
        return;
    };
    let Some(cu_data) = extract_unwind_info(&data) else {
        panic!("minimal_throw/app should have __unwind_info");
    };

    let entries = parse_compact_unwind(&cu_data);
    assert!(!entries.is_empty(), "minimal_throw should parse compact unwind entries");
    assert!(
        entries.iter().any(|entry| entry.personality_slot_offset.is_some()),
        "minimal_throw should expose an exception personality slot"
    );
    assert!(
        entries.iter().any(|entry| entry.lsda_offset.is_some()),
        "minimal_throw should expose LSDA data for catch matching"
    );

    let eh = generate_eh_frame(&entries, 0x3_0000_0000);
    assert!(!eh.is_empty(), "minimal_throw should synthesize a non-empty eh_frame");
}

#[test]
fn try_catch_test_compact_unwind_carries_exception_metadata() {
    let Some(data) = read_fixture("try_catch_test/app") else {
        return;
    };
    let Some(cu_data) = extract_unwind_info(&data) else {
        panic!("try_catch_test/app should have __unwind_info");
    };

    let entries = parse_compact_unwind(&cu_data);
    assert!(!entries.is_empty(), "try_catch_test should parse compact unwind entries");
    assert!(
        entries.iter().any(|entry| entry.personality_slot_offset.is_some()),
        "try_catch_test should expose an exception personality slot"
    );
    assert!(
        entries.iter().any(|entry| entry.lsda_offset.is_some()),
        "try_catch_test should expose LSDA data for catch matching"
    );

    let eh = generate_eh_frame(&entries, 0x4_0000_0000);
    assert!(!eh.is_empty(), "try_catch_test should synthesize a non-empty eh_frame");
}
