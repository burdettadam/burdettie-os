//! Tests for macho-types: enum conversions, bitflags, version encoding.
//!
//! Ported from:
//!   - XNU `<mach-o/loader.h>` constant definitions
//!   - LLVM lld Mach-O tests (segment protection values)
//!   - goblin crate type coverage
#![allow(clippy::identity_op)] // binary format clarity

#[cfg(test)]
mod tests {
    use macho_types::dylib::*;
    use macho_types::entry::*;
    use macho_types::header::*;
    use macho_types::load_command::*;
    use macho_types::segment::*;
    use macho_types::symbol::*;

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

    // ── LoadCommandType (from loader.h LC_* constants) ──────────────

    #[test]
    fn load_command_type_known_values() {
        assert_eq!(LoadCommandType::from(0x01), LoadCommandType::Segment);
        assert_eq!(LoadCommandType::from(0x19), LoadCommandType::Segment64);
        assert_eq!(LoadCommandType::from(0x02), LoadCommandType::Symtab);
        assert_eq!(LoadCommandType::from(0x0B), LoadCommandType::Dysymtab);
        assert_eq!(LoadCommandType::from(0x0C), LoadCommandType::LoadDylib);
        assert_eq!(LoadCommandType::from(0x0D), LoadCommandType::IdDylib);
        assert_eq!(LoadCommandType::from(0x0E), LoadCommandType::LoadDylinker);
        assert_eq!(LoadCommandType::from(0x1B), LoadCommandType::Uuid);
    }

    #[test]
    fn load_command_type_req_dyld_flag() {
        // LC_REQ_DYLD = 0x80000000 — load commands with this bit require dyld
        assert_eq!(LoadCommandType::from(0x8000_001C), LoadCommandType::Rpath);
        assert_eq!(LoadCommandType::from(0x8000_0028), LoadCommandType::Main);
        assert_eq!(
            LoadCommandType::from(0x8000_0022),
            LoadCommandType::DyldInfoOnly
        );
        assert_eq!(
            LoadCommandType::from(0x8000_0033),
            LoadCommandType::DyldExportsTrie
        );
        assert_eq!(
            LoadCommandType::from(0x8000_0034),
            LoadCommandType::DyldChainedFixups
        );
    }

    #[test]
    fn load_command_type_unknown() {
        assert_eq!(
            LoadCommandType::from(0xFFFF),
            LoadCommandType::Unknown(0xFFFF)
        );
    }

    // ── VmProt (from mach/vm_prot.h) ────────────────────────────────

    #[test]
    fn vmprot_read_write_execute_values() {
        assert_eq!(VmProt::READ.bits(), 0x01); // VM_PROT_READ
        assert_eq!(VmProt::WRITE.bits(), 0x02); // VM_PROT_WRITE
        assert_eq!(VmProt::EXECUTE.bits(), 0x04); // VM_PROT_EXECUTE
    }

    #[test]
    fn vmprot_text_segment() {
        // __TEXT is typically r-x
        let prot = VmProt::READ | VmProt::EXECUTE;
        assert!(prot.contains(VmProt::READ));
        assert!(!prot.contains(VmProt::WRITE));
        assert!(prot.contains(VmProt::EXECUTE));
        assert_eq!(format!("{prot}"), "r-x");
    }

    #[test]
    fn vmprot_data_segment() {
        // __DATA is typically rw-
        let prot = VmProt::READ | VmProt::WRITE;
        assert_eq!(format!("{prot}"), "rw-");
    }

    #[test]
    fn vmprot_no_access() {
        // __PAGEZERO has no protections
        let prot = VmProt::empty();
        assert_eq!(format!("{prot}"), "---");
    }

    #[test]
    fn vmprot_linkedit_segment() {
        // __LINKEDIT is typically r--
        let prot = VmProt::READ;
        assert_eq!(format!("{prot}"), "r--");
    }

    // ── Section flags ───────────────────────────────────────────────

    #[test]
    fn section_type_extraction() {
        let s = Section {
            sectname: "__bss".into(),
            segname: "__DATA".into(),
            addr: 0,
            size: 0,
            offset: 0,
            align: 0,
            reloff: 0,
            nreloc: 0,
            flags: 0x01, // S_ZEROFILL
            reserved1: 0,
            reserved2: 0,
        };
        assert_eq!(s.section_type(), 0x01);
        assert!(s.is_zerofill());
    }

    #[test]
    fn section_regular_not_zerofill() {
        let s = Section {
            sectname: "__text".into(),
            segname: "__TEXT".into(),
            addr: 0,
            size: 0,
            offset: 0,
            align: 0,
            reloff: 0,
            nreloc: 0,
            flags: 0x00, // S_REGULAR
            reserved1: 0,
            reserved2: 0,
        };
        assert_eq!(s.section_type(), 0x00);
        assert!(!s.is_zerofill());
    }

    #[test]
    fn section_gb_zerofill() {
        let s = Section {
            sectname: "__huge".into(),
            segname: "__DATA".into(),
            addr: 0,
            size: 0,
            offset: 0,
            align: 0,
            reloff: 0,
            nreloc: 0,
            flags: 0x0C, // S_GB_ZEROFILL
            reserved1: 0,
            reserved2: 0,
        };
        assert!(s.is_zerofill());
    }

    // ── SymbolType (from nlist.h N_TYPE mask) ───────────────────────

    #[test]
    fn symbol_type_from_ntype_undefined() {
        assert_eq!(SymbolType::from_ntype(0x00), SymbolType::Undefined);
        // External undefined: N_EXT set
        assert_eq!(SymbolType::from_ntype(0x01), SymbolType::Undefined);
        assert!(SymbolType::is_external(0x01));
    }

    #[test]
    fn symbol_type_from_ntype_defined() {
        // N_SECT = 0x0E
        assert_eq!(SymbolType::from_ntype(0x0E), SymbolType::Defined);
        // External defined: N_SECT | N_EXT = 0x0F
        assert_eq!(SymbolType::from_ntype(0x0F), SymbolType::Defined);
        assert!(SymbolType::is_external(0x0F));
    }

    #[test]
    fn symbol_type_from_ntype_absolute() {
        // N_ABS = 0x02
        assert_eq!(SymbolType::from_ntype(0x02), SymbolType::Absolute);
    }

    #[test]
    fn symbol_type_from_ntype_indirect() {
        // N_INDR = 0x0A
        assert_eq!(SymbolType::from_ntype(0x0A), SymbolType::Indirect);
    }

    #[test]
    fn symbol_type_from_ntype_prebound() {
        // N_PBUD = 0x0C
        assert_eq!(SymbolType::from_ntype(0x0C), SymbolType::PreboundUndefined);
    }

    #[test]
    fn symbol_type_external_flag() {
        assert!(!SymbolType::is_external(0x00)); // plain undefined
        assert!(SymbolType::is_external(0x01)); // N_EXT
        assert!(!SymbolType::is_external(0x0E)); // defined, not external
        assert!(SymbolType::is_external(0x0F)); // defined + external
    }

    // ── DylibRef version encoding (Apple packed version format) ─────

    #[test]
    fn dylib_version_decode_1_0_0() {
        // 1.0.0 = (1 << 16) | (0 << 8) | 0 = 0x00010000
        let (maj, min, pat) = DylibRef::decode_version(0x0001_0000);
        assert_eq!((maj, min, pat), (1, 0, 0));
    }

    #[test]
    fn dylib_version_decode_complex() {
        // 1311.100.2 = (1311 << 16) | (100 << 8) | 2
        let encoded = (1311u32 << 16) | (100u32 << 8) | 2;
        let (maj, min, pat) = DylibRef::decode_version(encoded);
        assert_eq!((maj, min, pat), (1311, 100, 2));
    }

    #[test]
    fn dylib_version_string() {
        let dref = DylibRef {
            name: "/usr/lib/libSystem.B.dylib".into(),
            compatibility_version: 0x0001_0000, // 1.0.0
            current_version: (1356u32 << 16),   // 1356.0.0
            timestamp: 2,
        };
        assert_eq!(dref.compat_version_string(), "1.0.0");
        assert_eq!(dref.current_version_string(), "1356.0.0");
    }

    #[test]
    fn dylib_version_decode_zero() {
        let (maj, min, pat) = DylibRef::decode_version(0);
        assert_eq!((maj, min, pat), (0, 0, 0));
    }

    // ── EntryPoint ──────────────────────────────────────────────────

    #[test]
    fn entry_point_lc_main() {
        let e = EntryPoint::Main {
            entry_offset: 0x1000,
            stack_size: 0,
        };
        match e {
            EntryPoint::Main {
                entry_offset,
                stack_size,
            } => {
                assert_eq!(entry_offset, 0x1000);
                assert_eq!(stack_size, 0);
            }
            _ => panic!("expected Main variant"),
        }
    }

    #[test]
    fn entry_point_unix_thread() {
        let e = EntryPoint::UnixThread {
            entry_address: 0x100000000,
        };
        match e {
            EntryPoint::UnixThread { entry_address } => {
                assert_eq!(entry_address, 0x100000000);
            }
            _ => panic!("expected UnixThread variant"),
        }
    }

    // ── Serde roundtrip ─────────────────────────────────────────────

    #[test]
    fn mach_flags_serde_roundtrip() {
        let original = MachFlags::NO_UNDEFS | MachFlags::PIE | MachFlags::TWO_LEVEL;
        let json = serde_json::to_string(&original).unwrap();
        let restored: MachFlags = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn vmprot_serde_roundtrip() {
        let original = VmProt::READ | VmProt::EXECUTE;
        let json = serde_json::to_string(&original).unwrap();
        let restored: VmProt = serde_json::from_str(&json).unwrap();
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
}

// ── SectionType tests ───────────────────────────────────────────────

#[cfg(test)]
mod section_type_tests {
    use macho_types::segment::SectionType;

    #[test]
    fn section_type_regular() {
        assert_eq!(SectionType::from(0x00), SectionType::Regular);
    }

    #[test]
    fn section_type_zerofill() {
        assert_eq!(SectionType::from(0x01), SectionType::ZeroFill);
    }

    #[test]
    fn section_type_mod_init_func() {
        assert_eq!(SectionType::from(0x09), SectionType::ModInitFuncPointers);
    }

    #[test]
    fn section_type_thread_local_variables() {
        assert_eq!(SectionType::from(0x13), SectionType::ThreadLocalVariables);
    }

    #[test]
    fn section_type_init_offsets() {
        assert_eq!(SectionType::from(0x16), SectionType::InitOffsets);
    }

    #[test]
    fn section_type_unknown() {
        assert_eq!(SectionType::from(0xFF), SectionType::Unknown(0xFF));
    }

    #[test]
    fn section_type_name_on_section() {
        let section = macho_types::Section {
            sectname: "__text".into(),
            segname: "__TEXT".into(),
            addr: 0,
            size: 0,
            offset: 0,
            align: 0,
            reloff: 0,
            nreloc: 0,
            flags: 0x00, // S_REGULAR
            reserved1: 0,
            reserved2: 0,
        };
        assert_eq!(section.section_type_name(), SectionType::Regular);
    }
}

// ── LoadCommandType new variant tests ───────────────────────────────

#[cfg(test)]
mod load_command_type_tests {
    use macho_types::load_command::LoadCommandType;

    #[test]
    fn unix_thread_from_u32() {
        assert_eq!(LoadCommandType::from(0x5), LoadCommandType::UnixThread);
    }

    #[test]
    fn version_min_macos_from_u32() {
        assert_eq!(
            LoadCommandType::from(0x24),
            LoadCommandType::VersionMinMacOS
        );
    }

    #[test]
    fn version_min_ios_from_u32() {
        assert_eq!(LoadCommandType::from(0x25), LoadCommandType::VersionMinIOS);
    }

    #[test]
    fn dyld_environment_from_u32() {
        assert_eq!(
            LoadCommandType::from(0x27),
            LoadCommandType::DyldEnvironment
        );
    }

    #[test]
    fn linker_option_from_u32() {
        assert_eq!(LoadCommandType::from(0x2D), LoadCommandType::LinkerOption);
    }

    #[test]
    fn note_from_u32() {
        assert_eq!(LoadCommandType::from(0x31), LoadCommandType::Note);
    }

    #[test]
    fn version_min_tvos_from_u32() {
        assert_eq!(LoadCommandType::from(0x2F), LoadCommandType::VersionMinTvOS);
    }

    #[test]
    fn version_min_watchos_from_u32() {
        assert_eq!(
            LoadCommandType::from(0x30),
            LoadCommandType::VersionMinWatchOS
        );
    }
}

// ── Phase 13: Version decode, platform name, source version ─────

#[cfg(test)]
mod version_decode_tests {
    use macho_types::load_command::{decode_source_version, decode_version, platform_name};

    #[test]
    fn decode_version_16_0_0() {
        // macOS 16.0.0 → major=16, minor=0, patch=0, packed: 0x00100000
        let v = (16 << 16) | (0 << 8) | 0;
        assert_eq!(decode_version(v), "16.0.0");
    }

    #[test]
    fn decode_version_14_5_1() {
        let v = (14 << 16) | (5 << 8) | 1;
        assert_eq!(decode_version(v), "14.5.1");
    }

    #[test]
    fn decode_version_zero() {
        assert_eq!(decode_version(0), "0.0.0");
    }

    #[test]
    fn platform_name_macos() {
        assert_eq!(platform_name(1), "macOS");
    }

    #[test]
    fn platform_name_ios() {
        assert_eq!(platform_name(2), "iOS");
    }

    #[test]
    fn platform_name_tvos() {
        assert_eq!(platform_name(3), "tvOS");
    }

    #[test]
    fn platform_name_watchos() {
        assert_eq!(platform_name(4), "watchOS");
    }

    #[test]
    fn platform_name_mac_catalyst() {
        assert_eq!(platform_name(6), "macCatalyst");
    }

    #[test]
    fn platform_name_vision_os() {
        assert_eq!(platform_name(11), "visionOS");
    }

    #[test]
    fn platform_name_unknown() {
        assert_eq!(platform_name(255), "unknown");
    }

    #[test]
    fn decode_source_version_zero() {
        assert_eq!(decode_source_version(0), "0.0.0.0.0");
    }

    #[test]
    fn decode_source_version_1_2_3_4_5() {
        // a=1 (bits 40..63), b=2 (bits 30..39), c=3 (bits 20..29), d=4 (bits 10..19), e=5 (bits 0..9)
        let v: u64 = (1 << 40) | (2 << 30) | (3 << 20) | (4 << 10) | 5;
        assert_eq!(decode_source_version(v), "1.2.3.4.5");
    }

    #[test]
    fn decode_source_version_large_a() {
        // a occupies 24 bits so can be up to 16777215
        let v: u64 = (1000u64 << 40) | (0 << 30) | (0 << 20) | (0 << 10) | 0;
        assert_eq!(decode_source_version(v), "1000.0.0.0.0");
    }
}

// ── Phase 14: Symbol desc accessors, section reserved fields ────

#[cfg(test)]
mod symbol_desc_tests {
    use macho_types::symbol::{is_stab, Symbol, SymbolType};

    fn make_sym(desc: u16) -> Symbol {
        Symbol {
            name: "test".into(),
            sym_type: SymbolType::Defined,
            section_index: 1,
            desc,
            value: 0,
        }
    }

    #[test]
    fn weak_ref_flag() {
        assert!(make_sym(0x0040).is_weak_ref());
        assert!(!make_sym(0x0000).is_weak_ref());
    }

    #[test]
    fn weak_def_flag() {
        assert!(make_sym(0x0080).is_weak_def());
        assert!(!make_sym(0x0000).is_weak_def());
    }

    #[test]
    fn no_dead_strip_flag() {
        assert!(make_sym(0x0020).is_no_dead_strip());
        assert!(!make_sym(0x0000).is_no_dead_strip());
    }

    #[test]
    fn referenced_dynamically_flag() {
        assert!(make_sym(0x0010).is_referenced_dynamically());
        assert!(!make_sym(0x0000).is_referenced_dynamically());
    }

    #[test]
    fn combined_flags() {
        let sym = make_sym(0x00C0); // WEAK_REF | WEAK_DEF
        assert!(sym.is_weak_ref());
        assert!(sym.is_weak_def());
        assert!(!sym.is_no_dead_strip());
    }

    #[test]
    fn library_ordinal_positive() {
        let sym = make_sym(0x0300); // ordinal = 3
        assert_eq!(sym.library_ordinal(), 3);
    }

    #[test]
    fn library_ordinal_negative() {
        // Ordinal 0xFF = -1 (main executable)
        let sym = make_sym(0xFF00);
        assert_eq!(sym.library_ordinal(), -1);
    }

    #[test]
    fn library_ordinal_flat_lookup() {
        // Ordinal 0xFE = -2 (flat lookup)
        let sym = make_sym(0xFE00);
        assert_eq!(sym.library_ordinal(), -2);
    }

    #[test]
    fn stab_detection() {
        assert!(is_stab(0x20)); // N_GSYM
        assert!(is_stab(0x64)); // N_SO
        assert!(is_stab(0xE0)); // high bits
        assert!(!is_stab(0x0E)); // N_SECT
        assert!(!is_stab(0x01)); // N_EXT
        assert!(!is_stab(0x0F)); // N_SECT | N_EXT
        assert!(!is_stab(0x00)); // undefined
    }
}

#[cfg(test)]
mod section_reserved_tests {
    use macho_types::segment::{Section, SectionType};

    #[test]
    fn section_has_reserved_fields() {
        let s = Section {
            sectname: "__stubs".into(),
            segname: "__TEXT".into(),
            addr: 0x1000,
            size: 24,
            offset: 0x1000,
            align: 2,
            reloff: 0,
            nreloc: 0,
            flags: 0x80000408, // S_SYMBOL_STUBS | S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS
            reserved1: 5,      // indirect sym table index
            reserved2: 12,     // stub size
        };
        assert_eq!(s.reserved1, 5);
        assert_eq!(s.reserved2, 12);
    }

    #[test]
    fn section_reserved_defaults_zero() {
        let s = Section {
            sectname: "__text".into(),
            segname: "__TEXT".into(),
            addr: 0,
            size: 0,
            offset: 0,
            align: 0,
            reloff: 0,
            nreloc: 0,
            flags: 0,
            reserved1: 0,
            reserved2: 0,
        };
        assert_eq!(s.reserved1, 0);
        assert_eq!(s.reserved2, 0);
    }

    // ── TlvDescriptor struct ────────────────────────────────────────

    #[test]
    fn tlv_descriptor_default_values() {
        let d = macho_types::TlvDescriptor {
            address: 0x1000,
            thunk: 0x2000,
            key: 0,
            offset: 48,
        };
        assert_eq!(d.address, 0x1000);
        assert_eq!(d.thunk, 0x2000);
        assert_eq!(d.key, 0);
        assert_eq!(d.offset, 48);
    }

    #[test]
    fn tlv_descriptor_serializes_to_json() {
        let d = macho_types::TlvDescriptor {
            address: 0x100,
            thunk: 0x200,
            key: 1,
            offset: 8,
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("\"address\""));
        assert!(json.contains("\"thunk\""));
        assert!(json.contains("\"key\""));
        assert!(json.contains("\"offset\""));
    }

    // ── Entitlement key extraction ──────────────────────────────────

    #[test]
    fn entitlement_keys_empty_when_no_plist() {
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: None,
            has_cms_signature: false,
        };
        assert!(cs.entitlement_keys().is_empty());
    }

    #[test]
    fn entitlement_keys_extracts_true() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
</dict>
</plist>"#;
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: Some(xml.to_string()),
            has_cms_signature: false,
        };
        let keys = cs.entitlement_keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].0, "com.apple.security.cs.allow-jit");
        assert!(keys[0].1);
    }

    #[test]
    fn entitlement_keys_extracts_false() {
        let xml = r#"<plist version="1.0"><dict>
    <key>com.apple.security.cs.disable-library-validation</key>
    <false/>
</dict></plist>"#;
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: Some(xml.to_string()),
            has_cms_signature: false,
        };
        let keys = cs.entitlement_keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(
            keys[0].0,
            "com.apple.security.cs.disable-library-validation"
        );
        assert!(!keys[0].1);
    }

    #[test]
    fn entitlement_keys_multiple() {
        let xml = r#"<plist version="1.0"><dict>
    <key>com.apple.security.get-task-allow</key>
    <true/>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <false/>
</dict></plist>"#;
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: Some(xml.to_string()),
            has_cms_signature: false,
        };
        let keys = cs.entitlement_keys();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].0, "com.apple.security.get-task-allow");
        assert!(keys[0].1);
        assert_eq!(keys[1].0, "com.apple.security.cs.allow-jit");
        assert!(keys[1].1);
        assert_eq!(
            keys[2].0,
            "com.apple.security.cs.disable-library-validation"
        );
        assert!(!keys[2].1);
    }

    #[test]
    fn entitlement_keys_non_boolean_treated_as_true() {
        let xml = r#"<plist version="1.0"><dict>
    <key>com.apple.developer.team-identifier</key>
    <string>ABCDE12345</string>
</dict></plist>"#;
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: Some(xml.to_string()),
            has_cms_signature: false,
        };
        let keys = cs.entitlement_keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].0, "com.apple.developer.team-identifier");
        assert!(keys[0].1); // non-boolean defaults to true (present)
    }

    #[test]
    fn entitlement_keys_empty_dict() {
        let xml = r#"<plist version="1.0"><dict></dict></plist>"#;
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: Some(xml.to_string()),
            has_cms_signature: false,
        };
        assert!(cs.entitlement_keys().is_empty());
    }

    // ── SectionType TLV variants ────────────────────────────────────

    #[test]
    fn section_type_thread_local_regular() {
        assert_eq!(SectionType::from(0x11), SectionType::ThreadLocalRegular);
    }

    #[test]
    fn section_type_thread_local_zerofill() {
        assert_eq!(SectionType::from(0x12), SectionType::ThreadLocalZeroFill);
    }

    #[test]
    fn section_type_thread_local_variables() {
        assert_eq!(SectionType::from(0x13), SectionType::ThreadLocalVariables);
    }

    #[test]
    fn section_type_thread_local_variable_pointers() {
        assert_eq!(
            SectionType::from(0x14),
            SectionType::ThreadLocalVariablePointers
        );
    }

    #[test]
    fn section_type_thread_local_init_function_pointers() {
        assert_eq!(
            SectionType::from(0x15),
            SectionType::ThreadLocalInitFunctionPointers
        );
    }

    #[test]
    fn section_flags_detect_tlv_type() {
        let s = Section {
            sectname: "__thread_vars".into(),
            segname: "__DATA".into(),
            addr: 0,
            size: 48,
            offset: 0,
            align: 3,
            reloff: 0,
            nreloc: 0,
            flags: 0x13, // S_THREAD_LOCAL_VARIABLES
            reserved1: 0,
            reserved2: 0,
        };
        assert_eq!(s.section_type_name(), SectionType::ThreadLocalVariables);
    }
}

// ── Phase 16: RelocationType arm64 mapping & is_zerofill fix ────────

#[cfg(test)]
mod phase16_types_tests {
    use macho_types::relocation::RelocationType;
    use macho_types::segment::Section;

    // ── RelocationType arm64 codes ──────────────────────────────────

    #[test]
    fn reloc_type_vanilla() {
        assert_eq!(RelocationType::from(0), RelocationType::Vanilla);
    }

    #[test]
    fn reloc_type_pair() {
        assert_eq!(RelocationType::from(1), RelocationType::Pair);
    }

    #[test]
    fn reloc_type_sector_diff() {
        assert_eq!(RelocationType::from(2), RelocationType::SectorDiff);
    }

    #[test]
    fn reloc_type_local_sector_diff() {
        assert_eq!(RelocationType::from(3), RelocationType::LocalSectorDiff);
    }

    #[test]
    fn reloc_type_pb_lazy_ptr() {
        assert_eq!(RelocationType::from(4), RelocationType::PbLazyPtr);
    }

    #[test]
    fn reloc_type_branch24() {
        assert_eq!(RelocationType::from(5), RelocationType::Branch24);
    }

    #[test]
    fn reloc_type_arm64_branch26() {
        assert_eq!(RelocationType::from(6), RelocationType::Arm64Branch26);
    }

    #[test]
    fn reloc_type_arm64_page21() {
        assert_eq!(RelocationType::from(7), RelocationType::Arm64Page21);
    }

    #[test]
    fn reloc_type_arm64_pageoff12() {
        assert_eq!(RelocationType::from(8), RelocationType::Arm64Pageoff12);
    }

    #[test]
    fn reloc_type_arm64_got_load_page21() {
        assert_eq!(RelocationType::from(9), RelocationType::Arm64GotLoadPage21);
    }

    #[test]
    fn reloc_type_arm64_got_load_pageoff12() {
        assert_eq!(
            RelocationType::from(10),
            RelocationType::Arm64GotLoadPageoff12
        );
    }

    #[test]
    fn reloc_type_arm64_subtractor() {
        assert_eq!(RelocationType::from(11), RelocationType::Arm64Subtractor);
    }

    #[test]
    fn reloc_type_arm64_addend() {
        assert_eq!(RelocationType::from(12), RelocationType::Arm64Addend);
    }

    #[test]
    fn reloc_type_arm64_pointer() {
        assert_eq!(RelocationType::from(13), RelocationType::Arm64Pointer);
    }

    #[test]
    fn reloc_type_unknown_14() {
        assert_eq!(RelocationType::from(14), RelocationType::Unknown(14));
    }

    #[test]
    fn reloc_type_unknown_255() {
        assert_eq!(RelocationType::from(255), RelocationType::Unknown(255));
    }

    // ── is_zerofill includes ThreadLocalZeroFill ────────────────────

    #[test]
    fn is_zerofill_regular_zerofill() {
        let s = Section {
            sectname: "".into(),
            segname: "".into(),
            addr: 0,
            size: 0,
            offset: 0,
            align: 0,
            reloff: 0,
            nreloc: 0,
            flags: 0x01, // S_ZEROFILL
            reserved1: 0,
            reserved2: 0,
        };
        assert!(s.is_zerofill());
    }

    #[test]
    fn is_zerofill_gb_zerofill() {
        let s = Section {
            sectname: "".into(),
            segname: "".into(),
            addr: 0,
            size: 0,
            offset: 0,
            align: 0,
            reloff: 0,
            nreloc: 0,
            flags: 0x0C, // S_GB_ZEROFILL
            reserved1: 0,
            reserved2: 0,
        };
        assert!(s.is_zerofill());
    }

    #[test]
    fn is_zerofill_thread_local_zerofill() {
        let s = Section {
            sectname: "__thread_bss".into(),
            segname: "__DATA".into(),
            addr: 0,
            size: 4,
            offset: 0,
            align: 0,
            reloff: 0,
            nreloc: 0,
            flags: 0x12, // S_THREAD_LOCAL_ZEROFILL
            reserved1: 0,
            reserved2: 0,
        };
        assert!(
            s.is_zerofill(),
            "ThreadLocalZeroFill should be treated as zerofill"
        );
    }

    #[test]
    fn is_zerofill_false_for_regular() {
        let s = Section {
            sectname: "".into(),
            segname: "".into(),
            addr: 0,
            size: 0,
            offset: 0,
            align: 0,
            reloff: 0,
            nreloc: 0,
            flags: 0x00, // S_REGULAR
            reserved1: 0,
            reserved2: 0,
        };
        assert!(!s.is_zerofill());
    }

    // ── ParsedMachO Deserialize roundtrip ───────────────────────────

    #[test]
    fn relocation_serializes_roundtrip() {
        let r = macho_types::Relocation {
            address: 0x100,
            symbol_index: 5,
            pc_relative: true,
            length: 2,
            extern_sym: true,
            reloc_type: RelocationType::Arm64Branch26,
        };
        let json = serde_json::to_string(&r).unwrap();
        let restored: macho_types::Relocation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.address, 0x100);
        assert_eq!(restored.symbol_index, 5);
        assert!(restored.pc_relative);
        assert_eq!(restored.length, 2);
        assert!(restored.extern_sym);
        assert_eq!(restored.reloc_type, RelocationType::Arm64Branch26);
    }
}

// ── Phase 17: Load command types, MachFlags, CpuSubtype, DataInCodeKind ──

#[cfg(test)]
mod phase17_types_tests {
    use macho_types::data_in_code::*;
    use macho_types::header::*;
    use macho_types::load_command::*;

    // ── LoadCommandType additions ──

    #[test]
    fn lc_reexport_dylib_roundtrip() {
        assert_eq!(
            LoadCommandType::from(0x8000_001F),
            LoadCommandType::ReexportDylib
        );
    }

    #[test]
    fn lc_lazy_load_dylib_roundtrip() {
        assert_eq!(LoadCommandType::from(0x20), LoadCommandType::LazyLoadDylib);
    }

    #[test]
    fn lc_load_upward_dylib_roundtrip() {
        assert_eq!(
            LoadCommandType::from(0x8000_0023),
            LoadCommandType::LoadUpwardDylib
        );
    }

    #[test]
    fn lc_encryption_info_roundtrip() {
        assert_eq!(LoadCommandType::from(0x21), LoadCommandType::EncryptionInfo);
    }

    #[test]
    fn lc_encryption_info_64_roundtrip() {
        assert_eq!(
            LoadCommandType::from(0x2C),
            LoadCommandType::EncryptionInfo64
        );
    }

    #[test]
    fn lc_existing_types_still_valid() {
        assert_eq!(LoadCommandType::from(0xC), LoadCommandType::LoadDylib);
        assert_eq!(
            LoadCommandType::from(0x8000_0018),
            LoadCommandType::LoadWeakDylib
        );
        assert_eq!(
            LoadCommandType::from(0x8000_0034),
            LoadCommandType::DyldChainedFixups
        );
    }

    #[test]
    fn lc_unknown_unchanged() {
        assert_eq!(
            LoadCommandType::from(0xDEAD),
            LoadCommandType::Unknown(0xDEAD)
        );
    }

    // ── MachFlags additions ──

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

    // ── CpuSubtype ──

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

    // ── DataInCodeKind ──

    #[test]
    fn data_in_code_kind_data() {
        assert_eq!(DataInCodeKind::from(1), DataInCodeKind::Data);
    }

    #[test]
    fn data_in_code_kind_jump_table8() {
        assert_eq!(DataInCodeKind::from(2), DataInCodeKind::JumpTable8);
    }

    #[test]
    fn data_in_code_kind_jump_table16() {
        assert_eq!(DataInCodeKind::from(3), DataInCodeKind::JumpTable16);
    }

    #[test]
    fn data_in_code_kind_jump_table32() {
        assert_eq!(DataInCodeKind::from(4), DataInCodeKind::JumpTable32);
    }

    #[test]
    fn data_in_code_kind_abs_jump_table32() {
        assert_eq!(DataInCodeKind::from(5), DataInCodeKind::AbsJumpTable32);
    }

    #[test]
    fn data_in_code_kind_unknown() {
        assert_eq!(DataInCodeKind::from(99), DataInCodeKind::Unknown(99));
    }
}
