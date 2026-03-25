//! Tests for LoadCommandType variants, version decoding, platform names,
//! source version encoding, RelocationType arm64 mapping, and DataInCodeKind.
#![allow(clippy::identity_op)]

use macho_types::load_command::*;

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

// ── LoadCommandType new variant tests ───────────────────────────

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

// ── LoadCommandType additions ───────────────────────────────────

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

// ── Version decode, platform name, source version ───────────────

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

// ── RelocationType arm64 codes ──────────────────────────────────

mod relocation_tests {
    use macho_types::relocation::RelocationType;

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

// ── DataInCodeKind ──────────────────────────────────────────────

mod data_in_code_tests {
    use macho_types::data_in_code::*;

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
