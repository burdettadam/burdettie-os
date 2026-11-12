//! Malformed metadata must not panic, invent records, or loop indefinitely.

use macho_parse::{
    extract_cstrings, parse_export_trie, parse_function_starts, parse_tlv_descriptors,
    vmaddr_to_file_offset, ParsedMachO,
};
use macho_types::load_command::{LoadCommandPayload, LoadCommandType};
use macho_types::{LoadCommand, Section, Segment};

fn parsed() -> ParsedMachO {
    let mut header = [0u8; 32];
    header[..4].copy_from_slice(&0xfeed_facfu32.to_le_bytes());
    header[4..8].copy_from_slice(&0x0100_000cu32.to_le_bytes());
    header[12..16].copy_from_slice(&2u32.to_le_bytes());
    macho_parse::parse(&header).expect("valid empty Mach-O header")
}

fn section(name: &str, flags: u32, offset: u32, size: u64) -> Section {
    Section {
        sectname: name.into(),
        segname: "__DATA".into(),
        addr: 0,
        size,
        offset,
        align: 0,
        reloff: 0,
        nreloc: 0,
        flags,
        reserved1: 0,
        reserved2: 0,
    }
}

fn segment(vmaddr: u64, vmsize: u64, fileoff: u64, filesize: u64) -> Segment {
    Segment {
        name: "__TEXT".into(),
        vmaddr,
        vmsize,
        fileoff,
        filesize,
        maxprot: macho_types::segment::VmProt::READ,
        initprot: macho_types::segment::VmProt::READ,
        nsects: 0,
        flags: 0,
        sections: Vec::new(),
    }
}

#[test]
fn export_trie_rejects_self_cycle() {
    assert!(parse_export_trie(&[0, 1, b'a', 0, 0]).is_empty());
}

#[test]
fn export_trie_rejects_ancestor_cycle() {
    // The node at offset 5 exports "a", then points back to the root.
    let exports = parse_export_trie(&[0, 1, b'a', 0, 5, 2, 0, 7, 1, b'b', 0, 0]);
    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].name, "a");
    assert_eq!(exports[0].address, 7);
}

#[test]
fn export_trie_preserves_shared_node_aliases() {
    let exports = parse_export_trie(&[0, 2, b'a', 0, 8, b'b', 0, 8, 2, 0, 42, 0]);
    let mut names: Vec<_> = exports.iter().map(|symbol| symbol.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["a", "b"]);
    assert!(exports.iter().all(|symbol| symbol.address == 42));
}

#[test]
fn export_trie_rejects_terminal_length_overflow() {
    let mut bytes = vec![0xff; 9];
    bytes.extend([1, 0]);
    assert!(parse_export_trie(&bytes).is_empty());
}

#[test]
fn export_trie_does_not_read_address_past_terminal() {
    // The terminal only contains flags; the following child-count byte is not
    // the missing export address.
    assert!(parse_export_trie(&[1, 0, 0]).is_empty());
}

#[test]
fn export_trie_rejects_truncated_and_overflowing_uleb128() {
    assert!(parse_export_trie(&[0x82]).is_empty());
    let mut bytes = vec![0x80; 9];
    bytes.extend([2, 0]);
    assert!(parse_export_trie(&bytes).is_empty());
}

#[test]
fn cstrings_reject_overflowing_section_range() {
    let mut parsed = parsed();
    parsed.sections.push(section("__cstring", 2, 1, u64::MAX));
    assert!(extract_cstrings(&parsed, b"valid\0").is_empty());
}

#[test]
fn tlv_rejects_overflowing_section_range() {
    let mut parsed = parsed();
    parsed
        .sections
        .push(section("__thread_vars", 0x13, 1, u64::MAX));
    assert!(parse_tlv_descriptors(&parsed, &[0; 24]).is_empty());
}

#[test]
fn tlv_stops_before_virtual_address_overflow() {
    let mut parsed = parsed();
    let mut section = section("__thread_vars", 0x13, 0, 48);
    section.addr = u64::MAX - 8;
    parsed.sections.push(section);
    let descriptors = parse_tlv_descriptors(&parsed, &[0; 48]);
    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].address, u64::MAX - 8);
}

#[test]
fn vmaddr_mapping_handles_segment_extent_overflow() {
    let mut parsed = parsed();
    parsed.segments.push(segment(u64::MAX - 8, 16, 0, 16));
    assert_eq!(vmaddr_to_file_offset(&parsed, u64::MAX - 4), Some(4));
    assert_eq!(vmaddr_to_file_offset(&parsed, 0), None);
}

#[test]
fn vmaddr_mapping_rejects_file_offset_overflow() {
    let mut parsed = parsed();
    parsed.segments.push(segment(0x1000, 16, u64::MAX - 1, 16));
    assert_eq!(vmaddr_to_file_offset(&parsed, 0x1004), None);
}

#[test]
fn function_starts_stop_at_address_overflow() {
    let mut parsed = parsed();
    parsed.segments.push(segment(u64::MAX - 1, 1, 0, 1));
    parsed.commands.push(LoadCommand {
        cmd: LoadCommandType::FunctionStarts,
        cmd_size: 16,
        payload: LoadCommandPayload::LinkEdit {
            data_off: 0,
            data_size: 2,
        },
    });
    assert_eq!(parse_function_starts(&parsed, &[1, 1]), [u64::MAX]);
}

#[test]
fn export_trie_bounds_acyclic_alias_expansion() {
    let mut bytes = Vec::new();
    // Every level has two distinct labels leading to the same next node.
    // This tiny acyclic graph describes 2^18 different symbol names.
    for depth in 0..18 {
        let next = (depth + 1) * 10;
        let offset = [0x80 | (next & 0x7f) as u8, (next >> 7) as u8];
        bytes.extend([
            0, 2, b'a', 0, offset[0], offset[1], b'b', 0, offset[0], offset[1],
        ]);
    }
    bytes.extend([2, 0, 42, 0]);
    let exports = parse_export_trie(&bytes);
    assert!(!exports.is_empty());
    assert!(exports.len() <= bytes.len() * 4);
    assert!(
        exports
            .iter()
            .map(|symbol| symbol.name.len())
            .sum::<usize>()
            <= bytes.len() * 64
    );
}

#[test]
fn function_starts_reject_truncated_and_overflowing_deltas() {
    let mut parsed = parsed();
    parsed.commands.push(LoadCommand {
        cmd: LoadCommandType::FunctionStarts,
        cmd_size: 16,
        payload: LoadCommandPayload::LinkEdit {
            data_off: 0,
            data_size: 1,
        },
    });
    assert!(parse_function_starts(&parsed, &[0x81]).is_empty());
    parsed.commands[0].payload = LoadCommandPayload::LinkEdit {
        data_off: 0,
        data_size: 10,
    };
    let mut bytes = [0x80; 10];
    bytes[9] = 2;
    assert!(parse_function_starts(&parsed, &bytes).is_empty());
}

fn unixthread(cpu: u32, flavor: u32, state: &[u8]) -> Vec<u8> {
    let is_64 = cpu & 0x0100_0000 != 0;
    let header_size = if is_64 { 32 } else { 28 };
    let command_size = 16 + state.len();
    let mut data = vec![0u8; header_size];
    let magic = if is_64 {
        0xfeed_facfu32
    } else {
        0xfeed_faceu32
    };
    data[..4].copy_from_slice(&magic.to_le_bytes());
    data[4..8].copy_from_slice(&cpu.to_le_bytes());
    data[12..16].copy_from_slice(&2u32.to_le_bytes());
    data[16..20].copy_from_slice(&1u32.to_le_bytes());
    data[20..24].copy_from_slice(&(command_size as u32).to_le_bytes());
    data.extend(5u32.to_le_bytes());
    data.extend((command_size as u32).to_le_bytes());
    data.extend(flavor.to_le_bytes());
    data.extend(((state.len() / 4) as u32).to_le_bytes());
    data.extend(state);
    data
}

fn entry_address(data: &[u8]) -> u64 {
    match macho_parse::parse(data).expect("valid thread state").entry {
        Some(macho_types::EntryPoint::UnixThread { entry_address }) => entry_address,
        entry => panic!("unexpected entry: {entry:?}"),
    }
}

#[test]
fn arm64_unixthread_reads_pc_instead_of_x16() {
    let mut state = [0u8; 272];
    state[128..136].copy_from_slice(&0x1111u64.to_le_bytes());
    state[256..264].copy_from_slice(&0x2222u64.to_le_bytes());
    assert_eq!(entry_address(&unixthread(0x0100_000c, 6, &state)), 0x2222);
}

#[test]
fn x86_64_unixthread_reads_rip() {
    let mut state = [0u8; 168];
    state[128..136].copy_from_slice(&0x3333u64.to_le_bytes());
    assert_eq!(entry_address(&unixthread(0x0100_0007, 4, &state)), 0x3333);
}

#[test]
fn i386_unixthread_reads_eip() {
    let mut state = [0u8; 64];
    state[40..44].copy_from_slice(&0x4444u32.to_le_bytes());
    assert_eq!(entry_address(&unixthread(7, 1, &state)), 0x4444);
}

#[test]
fn arm_unixthread_reads_pc_instead_of_r10() {
    let mut state = [0u8; 68];
    state[40..44].copy_from_slice(&0x5555u32.to_le_bytes());
    state[60..64].copy_from_slice(&0x6666u32.to_le_bytes());
    assert_eq!(entry_address(&unixthread(12, 1, &state)), 0x6666);
}

#[test]
fn unixthread_rejects_wrong_flavor_and_truncated_state() {
    assert!(macho_parse::parse(&unixthread(0x0100_000c, 4, &[0; 272])).is_err());
    assert!(macho_parse::parse(&unixthread(0x0100_000c, 6, &[0; 128])).is_err());
    let mut data = unixthread(0x0100_000c, 6, &[0; 272]);
    data[44..48].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(macho_parse::parse(&data).is_err());
}

#[test]
fn export_trie_bounds_shared_unterminated_reexport_names() {
    let mut bytes = Vec::new();
    // Empty edge labels keep prefix allocation tiny while both branches at
    // every level repeatedly reach the same malformed re-export terminal.
    for depth in 0..14 {
        let next = (depth + 1) * 8;
        let offset = [0x80 | (next & 0x7f) as u8, (next >> 7) as u8];
        bytes.extend([0, 2, 0, offset[0], offset[1], 0, offset[0], offset[1]]);
    }
    // 258 terminal bytes: flags, ordinal, and an unterminated 256-byte name.
    bytes.extend([0x82, 0x02, 8, 1]);
    bytes.extend([b'A'; 256]);
    bytes.push(0); // child count lies outside the terminal record
    assert!(parse_export_trie(&bytes).is_empty());
}
