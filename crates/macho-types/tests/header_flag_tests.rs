//! Tests for magic constants, CPU types, file types, MachFlags, MachHeader,
//! CpuSubtype, and bitflag serde roundtrips.
#![allow(clippy::identity_op)]

use macho_types::header::*;

// ── Magic constants (from loader.h) ──────────────────────────────

#[test]
fn magic_64_matches_loader_h() {
    assert_eq!(MH_MAGIC_64, 0xFEED_FACF);
    assert_eq!(MH_CIGAM_64, 0xCFFAEDFE);
}

#[test]
fn magic_32_matches_loader_h() {
    assert_eq!(MH_MAGIC, 0xFEED_FACE);
    assert_eq!(MH_CIGAM, 0xCEFAEDFE);
}

#[test]
fn fat_magic_matches_loader_h() {
    assert_eq!(FAT_MAGIC, 0xCAFE_BABE);
    assert_eq!(FAT_CIGAM, 0xBEBA_FECA);
}

// ── CpuType roundtrip (from loader.h CPU_TYPE_* constants) ──────

#[test]
fn cpu_type_x86_roundtrip() {
    assert_eq!(CpuType::from(7u32), CpuType::X86);
    assert_eq!(u32::from(CpuType::X86), 7);
}

#[test]
fn cpu_type_x86_64_roundtrip() {
    // CPU_TYPE_X86_64 = CPU_TYPE_X86 | CPU_ARCH_ABI64
    assert_eq!(CpuType::from(0x0100_0007u32), CpuType::X86_64);
    assert_eq!(u32::from(CpuType::X86_64), 0x0100_0007);
}

#[test]
fn cpu_type_arm_roundtrip() {
    assert_eq!(CpuType::from(12u32), CpuType::Arm);
    assert_eq!(u32::from(CpuType::Arm), 12);
}

#[test]
fn cpu_type_arm64_roundtrip() {
    // CPU_TYPE_ARM64 = CPU_TYPE_ARM | CPU_ARCH_ABI64
    assert_eq!(CpuType::from(0x0100_000Cu32), CpuType::Arm64);
    assert_eq!(u32::from(CpuType::Arm64), 0x0100_000C);
}

#[test]
fn cpu_type_unknown_preserves_value() {
    let unknown = CpuType::from(0xDEAD_BEEFu32);
    assert_eq!(unknown, CpuType::Unknown(0xDEAD_BEEF));
    assert_eq!(u32::from(unknown), 0xDEAD_BEEF);
}

// ── FileType roundtrip (from loader.h MH_* constants) ───────────

#[test]
fn file_type_all_known_variants() {
    assert_eq!(FileType::from(1u32), FileType::Object); // MH_OBJECT
    assert_eq!(FileType::from(2u32), FileType::Execute); // MH_EXECUTE
    assert_eq!(FileType::from(6u32), FileType::Dylib); // MH_DYLIB
    assert_eq!(FileType::from(8u32), FileType::Bundle); // MH_BUNDLE
    assert_eq!(FileType::from(9u32), FileType::DylibStub); // MH_DYLIB_STUB
    assert_eq!(FileType::from(10u32), FileType::Dsym); // MH_DSYM
}

#[test]
fn file_type_unknown_preserves() {
    assert_eq!(FileType::from(99u32), FileType::Unknown(99));
    assert_eq!(u32::from(FileType::Unknown(99)), 99);
}

// ── MachFlags (from loader.h MH_* flag constants) ───────────────

#[test]
fn mach_flags_no_undefs() {
    let f = MachFlags::NO_UNDEFS;
    assert_eq!(f.bits(), 0x1); // MH_NOUNDEFS
}

#[test]
fn mach_flags_pie() {
    let f = MachFlags::PIE;
    assert_eq!(f.bits(), 0x0020_0000); // MH_PIE
}

#[test]
fn mach_flags_two_level() {
    let f = MachFlags::TWO_LEVEL;
    assert_eq!(f.bits(), 0x80); // MH_TWOLEVEL
}

#[test]
fn mach_flags_composite() {
    let f = MachFlags::NO_UNDEFS | MachFlags::DYLDLINK | MachFlags::PIE | MachFlags::TWO_LEVEL;
    assert!(f.contains(MachFlags::NO_UNDEFS));
    assert!(f.contains(MachFlags::PIE));
    assert!(f.contains(MachFlags::TWO_LEVEL));
    assert!(!f.contains(MachFlags::HAS_TLV_DESCRIPTORS));
}

#[test]
fn mach_flags_from_bits_truncate() {
    // Typical flags for a modern arm64 executable
    let typical = 0x0020_0085u32; // PIE | TWO_LEVEL | DYLDLINK | NO_UNDEFS
    let f = MachFlags::from_bits_truncate(typical);
    assert!(f.contains(MachFlags::PIE));
    assert!(f.contains(MachFlags::TWO_LEVEL));
    assert!(f.contains(MachFlags::DYLDLINK));
    assert!(f.contains(MachFlags::NO_UNDEFS));
}

// ── MachHeader ──────────────────────────────────────────────────

#[test]
fn header_is_64bit() {
    let h = MachHeader {
        magic: MH_MAGIC_64,
        cpu_type: CpuType::Arm64,
        cpu_subtype: 0,
        file_type: FileType::Execute,
        num_commands: 0,
        size_of_commands: 0,
        flags: MachFlags::empty(),
        reserved: Some(0),
    };
    assert!(h.is_64bit());
    assert!(!h.is_swapped());
}

#[test]
fn header_is_swapped_64() {
    let h = MachHeader {
        magic: MH_CIGAM_64,
        cpu_type: CpuType::X86_64,
        cpu_subtype: 0,
        file_type: FileType::Execute,
        num_commands: 0,
        size_of_commands: 0,
        flags: MachFlags::empty(),
        reserved: Some(0),
    };
    assert!(h.is_64bit());
    assert!(h.is_swapped());
}

#[test]
fn header_32bit() {
    let h = MachHeader {
        magic: MH_MAGIC,
        cpu_type: CpuType::X86,
        cpu_subtype: 0,
        file_type: FileType::Object,
        num_commands: 0,
        size_of_commands: 0,
        flags: MachFlags::empty(),
        reserved: None,
    };
    assert!(!h.is_64bit());
    assert!(!h.is_swapped());
}

// ── Serde roundtrip ─────────────────────────────────────────────

#[test]
fn mach_flags_serde_roundtrip() {
    let original = MachFlags::NO_UNDEFS | MachFlags::PIE | MachFlags::TWO_LEVEL;
    let json = serde_json::to_string(&original).unwrap();
    let restored: MachFlags = serde_json::from_str(&json).unwrap();
    assert_eq!(original, restored);
}

// ── New MachFlags values (from loader.h) ────────────────────────

#[test]
fn mach_flags_weak_defines() {
    assert_eq!(MachFlags::WEAK_DEFINES.bits(), 0x8000);
}

#[test]
fn mach_flags_binds_to_weak() {
    assert_eq!(MachFlags::BINDS_TO_WEAK.bits(), 0x10000);
}

#[test]
fn mach_flags_allow_stack_execution() {
    assert_eq!(MachFlags::ALLOW_STACK_EXECUTION.bits(), 0x20000);
}

#[test]
fn mach_flags_no_reexported_dylibs() {
    assert_eq!(MachFlags::NO_REEXPORTED_DYLIBS.bits(), 0x100000);
}

#[test]
fn mach_flags_no_heap_execution() {
    assert_eq!(MachFlags::NO_HEAP_EXECUTION.bits(), 0x1000000);
}

#[test]
fn mach_flags_app_extension_safe() {
    assert_eq!(MachFlags::APP_EXTENSION_SAFE.bits(), 0x2000000);
}

#[test]
fn mach_flags_dylib_in_cache() {
    assert_eq!(MachFlags::DYLIB_IN_CACHE.bits(), 0x80000000);
}

#[test]
fn mach_flags_from_bits_truncate_with_new_flags() {
    // Typical flags for a system framework dylib in the shared cache
    let bits = 0x80200085u32; // DYLIB_IN_CACHE | PIE | TWO_LEVEL | DYLDLINK | NO_UNDEFS
    let f = MachFlags::from_bits_truncate(bits);
    assert!(f.contains(MachFlags::DYLIB_IN_CACHE));
    assert!(f.contains(MachFlags::PIE));
    assert!(f.contains(MachFlags::NO_UNDEFS));
}

#[test]
fn mach_flags_all_new_flags_are_distinct() {
    let all_new = MachFlags::WEAK_DEFINES
        | MachFlags::BINDS_TO_WEAK
        | MachFlags::ALLOW_STACK_EXECUTION
        | MachFlags::NO_REEXPORTED_DYLIBS
        | MachFlags::NO_HEAP_EXECUTION
        | MachFlags::APP_EXTENSION_SAFE
        | MachFlags::DYLIB_IN_CACHE;
    // Verify each flag bit is unique (OR of all = sum of individual bits)
    let sum = 0x8000u32 + 0x10000 + 0x20000 + 0x100000 + 0x1000000 + 0x2000000 + 0x80000000;
    assert_eq!(all_new.bits(), sum);
}

// ── MachFlags additions ─────────────────────────────────────────

#[test]
fn mach_flags_subsections_via_symbols() {
    let flags = MachFlags::from_bits_truncate(0x2000);
    assert!(flags.contains(MachFlags::SUBSECTIONS_VIA_SYMBOLS));
}

#[test]
fn mach_flags_force_flat() {
    let flags = MachFlags::from_bits_truncate(0x100);
    assert!(flags.contains(MachFlags::FORCE_FLAT));
}

#[test]
fn mach_flags_canonical() {
    let flags = MachFlags::from_bits_truncate(0x4000);
    assert!(flags.contains(MachFlags::CANONICAL));
}

#[test]
fn mach_flags_root_safe() {
    let flags = MachFlags::from_bits_truncate(0x0040_0000);
    assert!(flags.contains(MachFlags::ROOT_SAFE));
}

#[test]
fn mach_flags_incrlink() {
    let flags = MachFlags::from_bits_truncate(0x2);
    assert!(flags.contains(MachFlags::INCRLINK));
}

#[test]
fn mach_flags_combined_real_binary() {
    // Typical macOS binary: NO_UNDEFS | DYLDLINK | TWO_LEVEL | SUBSECTIONS_VIA_SYMBOLS | PIE
    let bits = 0x0000_0001 | 0x0000_0004 | 0x0000_0080 | 0x0000_2000 | 0x0020_0000;
    let flags = MachFlags::from_bits_truncate(bits);
    assert!(flags.contains(MachFlags::NO_UNDEFS));
    assert!(flags.contains(MachFlags::DYLDLINK));
    assert!(flags.contains(MachFlags::TWO_LEVEL));
    assert!(flags.contains(MachFlags::SUBSECTIONS_VIA_SYMBOLS));
    assert!(flags.contains(MachFlags::PIE));
}

// ── CpuSubtype ──────────────────────────────────────────────────

#[test]
fn cpu_subtype_arm64_all() {
    let sub = CpuSubtype::from_pair(CpuType::Arm64, 0);
    assert_eq!(sub, CpuSubtype::Arm64All);
    let sub_ff = CpuSubtype::from_pair(CpuType::Arm64, 0xFF);
    assert_eq!(sub_ff, CpuSubtype::Arm64All);
}

#[test]
fn cpu_subtype_arm64e() {
    let sub = CpuSubtype::from_pair(CpuType::Arm64, 2);
    assert_eq!(sub, CpuSubtype::Arm64E);
    assert!(sub.is_arm64e());
}

#[test]
fn cpu_subtype_arm64e_with_lib64_bit() {
    // CPU_SUBTYPE_LIB64 = 0x8000_0000
    let sub = CpuSubtype::from_pair(CpuType::Arm64, 0x8000_0002);
    assert_eq!(sub, CpuSubtype::Arm64E);
}

#[test]
fn cpu_subtype_x86_64_all() {
    let sub = CpuSubtype::from_pair(CpuType::X86_64, 3);
    assert_eq!(sub, CpuSubtype::X86_64All);
}

#[test]
fn cpu_subtype_x86_64_haswell() {
    let sub = CpuSubtype::from_pair(CpuType::X86_64, 8);
    assert_eq!(sub, CpuSubtype::X86_64Haswell);
}

#[test]
fn cpu_subtype_armv7() {
    let sub = CpuSubtype::from_pair(CpuType::Arm, 9);
    assert_eq!(sub, CpuSubtype::ArmV7);
}

#[test]
fn cpu_subtype_armv7s() {
    let sub = CpuSubtype::from_pair(CpuType::Arm, 11);
    assert_eq!(sub, CpuSubtype::ArmV7S);
}

#[test]
fn cpu_subtype_unknown() {
    let sub = CpuSubtype::from_pair(CpuType::X86, 99);
    assert_eq!(sub, CpuSubtype::Unknown(99));
    assert!(!sub.is_arm64e());
}
