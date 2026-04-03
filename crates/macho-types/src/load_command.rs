use serde::{Deserialize, Serialize};

/// Known load command types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum LoadCommandType {
    Segment = 0x1,
    Symtab = 0x2,
    UnixThread = 0x5,
    Dysymtab = 0xB,
    LoadDylib = 0xC,
    IdDylib = 0xD,
    LoadDylinker = 0xE,
    IdDylinker = 0xF,
    Segment64 = 0x19,
    Uuid = 0x1B,
    CodeSignature = 0x1D,
    SegmentSplitInfo = 0x1E,
    DyldInfo = 0x22,
    VersionMinMacOS = 0x24,
    VersionMinIOS = 0x25,
    FunctionStarts = 0x26,
    DyldEnvironment = 0x27,
    DataInCode = 0x29,
    SourceVersion = 0x2A,
    LinkerOption = 0x2D,
    VersionMinTvOS = 0x2F,
    VersionMinWatchOS = 0x30,
    Note = 0x31,
    BuildVersion = 0x32,
    LazyLoadDylib = 0x20,
    EncryptionInfo = 0x21,
    EncryptionInfo64 = 0x2C,
    LoadWeakDylib = 0x8000_0018,
    Rpath = 0x8000_001C,
    ReexportDylib = 0x8000_001F,
    DyldInfoOnly = 0x8000_0022,
    LoadUpwardDylib = 0x8000_0023,
    Main = 0x8000_0028,
    DyldExportsTrie = 0x8000_0033,
    DyldChainedFixups = 0x8000_0034,
    Unknown(u32),
}

impl From<u32> for LoadCommandType {
    fn from(v: u32) -> Self {
        match v {
            0x1 => Self::Segment,
            0x19 => Self::Segment64,
            0x2 => Self::Symtab,
            0xB => Self::Dysymtab,
            0xC => Self::LoadDylib,
            0xD => Self::IdDylib,
            0xE => Self::LoadDylinker,
            0x8000_0018 => Self::LoadWeakDylib,
            0xF => Self::IdDylinker,
            0x1B => Self::Uuid,
            0x8000_001C => Self::Rpath,
            0x1D => Self::CodeSignature,
            0x1E => Self::SegmentSplitInfo,
            0x8000_0028 => Self::Main,
            0x2A => Self::SourceVersion,
            0x22 => Self::DyldInfo,
            0x20 => Self::LazyLoadDylib,
            0x21 => Self::EncryptionInfo,
            0x2C => Self::EncryptionInfo64,
            0x8000_001F => Self::ReexportDylib,
            0x8000_0022 => Self::DyldInfoOnly,
            0x8000_0023 => Self::LoadUpwardDylib,
            0x26 => Self::FunctionStarts,
            0x29 => Self::DataInCode,
            0x32 => Self::BuildVersion,
            0x8000_0033 => Self::DyldExportsTrie,
            0x8000_0034 => Self::DyldChainedFixups,
            0x5 => Self::UnixThread,
            0x24 => Self::VersionMinMacOS,
            0x25 => Self::VersionMinIOS,
            0x27 => Self::DyldEnvironment,
            0x2D => Self::LinkerOption,
            0x2F => Self::VersionMinTvOS,
            0x30 => Self::VersionMinWatchOS,
            0x31 => Self::Note,
            other => Self::Unknown(other),
        }
    }
}

/// A parsed load command with its type-specific payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadCommand {
    pub cmd: LoadCommandType,
    pub cmd_size: u32,
    pub payload: LoadCommandPayload,
}

/// Type-specific data for each load command variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadCommandPayload {
    Segment(super::segment::Segment),
    Symtab {
        sym_off: u32,
        nsyms: u32,
        str_off: u32,
        str_size: u32,
    },
    LoadDylib(super::dylib::DylibRef),
    LoadWeakDylib(super::dylib::DylibRef),
    ReexportDylib(super::dylib::DylibRef),
    LazyLoadDylib(super::dylib::DylibRef),
    LoadUpwardDylib(super::dylib::DylibRef),
    IdDylib(super::dylib::DylibRef),
    EncryptionInfo {
        crypt_off: u32,
        crypt_size: u32,
        crypt_id: u32,
    },
    LoadDylinker {
        name: String,
    },
    Rpath {
        path: String,
    },
    Main {
        entry_offset: u64,
        stack_size: u64,
    },
    Uuid {
        uuid: [u8; 16],
    },
    BuildVersion {
        platform: u32,
        minos: u32,
        sdk: u32,
        ntools: u32,
    },
    DyldInfo {
        rebase_off: u32,
        rebase_size: u32,
        bind_off: u32,
        bind_size: u32,
        weak_bind_off: u32,
        weak_bind_size: u32,
        lazy_bind_off: u32,
        lazy_bind_size: u32,
        export_off: u32,
        export_size: u32,
    },
    LinkEdit {
        data_off: u32,
        data_size: u32,
    },
    UnixThread {
        /// The instruction pointer / program counter from the thread state.
        entry_address: u64,
    },
    VersionMin {
        version: u32,
        sdk: u32,
    },
    DyldEnvironment {
        name: String,
    },
    SourceVersion {
        version: u64,
    },
    Dysymtab {
        ilocalsym: u32,
        nlocalsym: u32,
        iextdefsym: u32,
        nextdefsym: u32,
        iundefsym: u32,
        nundefsym: u32,
        indirectsymoff: u32,
        nindirectsyms: u32,
    },
    /// Catch-all for unrecognised commands — stores raw bytes.
    Unknown(Vec<u8>),
}

/// Decode a packed macOS/iOS version (e.g. from LC_BUILD_VERSION minos/sdk
/// or LC_VERSION_MIN_*) into "major.minor.patch" string.
pub fn decode_version(v: u32) -> String {
    let major = (v >> 16) & 0xFFFF;
    let minor = (v >> 8) & 0xFF;
    let patch = v & 0xFF;
    format!("{major}.{minor}.{patch}")
}

/// Decode platform integer from LC_BUILD_VERSION to a human-readable name.
pub fn platform_name(platform: u32) -> &'static str {
    match platform {
        1 => "macOS",
        2 => "iOS",
        3 => "tvOS",
        4 => "watchOS",
        5 => "bridgeOS",
        6 => "macCatalyst",
        7 => "iOSSimulator",
        8 => "tvOSSimulator",
        9 => "watchOSSimulator",
        10 => "driverKit",
        11 => "visionOS",
        12 => "visionOSSimulator",
        _ => "unknown",
    }
}

/// Decode LC_SOURCE_VERSION packed u64 into "a.b.c.d.e" string.
pub fn decode_source_version(v: u64) -> String {
    let a = (v >> 40) & 0xFFFFFF;
    let b = (v >> 30) & 0x3FF;
    let c = (v >> 20) & 0x3FF;
    let d = (v >> 10) & 0x3FF;
    let e = v & 0x3FF;
    format!("{a}.{b}.{c}.{d}.{e}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── decode_version ──────────────────────────────────────────────

    #[test]
    fn decode_version_10_6_0() {
        assert_eq!(decode_version(0x000A_0600), "10.6.0");
    }

    #[test]
    fn decode_version_14_0_0() {
        assert_eq!(decode_version(0x000E_0000), "14.0.0");
    }

    #[test]
    fn decode_version_12_3_1() {
        // major=12, minor=3, patch=1 → (12<<16)|(3<<8)|1
        let packed = (12u32 << 16) | (3 << 8) | 1;
        assert_eq!(decode_version(packed), "12.3.1");
    }

    #[test]
    fn decode_version_zero() {
        assert_eq!(decode_version(0x0000_0000), "0.0.0");
    }

    #[test]
    fn decode_version_max() {
        assert_eq!(decode_version(0xFFFF_FFFF), "65535.255.255");
    }

    #[test]
    fn decode_version_11_2_0() {
        assert_eq!(decode_version(0x000B_0200), "11.2.0");
    }

    // ── platform_name ───────────────────────────────────────────────

    #[test]
    fn platform_macos() {
        assert_eq!(platform_name(1), "macOS");
    }

    #[test]
    fn platform_ios() {
        assert_eq!(platform_name(2), "iOS");
    }

    #[test]
    fn platform_tvos() {
        assert_eq!(platform_name(3), "tvOS");
    }

    #[test]
    fn platform_watchos() {
        assert_eq!(platform_name(4), "watchOS");
    }

    #[test]
    fn platform_unknown() {
        assert_eq!(platform_name(999), "unknown");
    }

    #[test]
    fn platform_mac_catalyst() {
        assert_eq!(platform_name(6), "macCatalyst");
    }

    #[test]
    fn platform_bridge_os() {
        assert_eq!(platform_name(5), "bridgeOS");
    }

    #[test]
    fn platform_vision_os() {
        assert_eq!(platform_name(11), "visionOS");
    }

    #[test]
    fn platform_ios_simulator() {
        assert_eq!(platform_name(7), "iOSSimulator");
    }

    #[test]
    fn platform_driver_kit() {
        assert_eq!(platform_name(10), "driverKit");
    }

    // ── decode_source_version ───────────────────────────────────────

    #[test]
    fn source_version_zero() {
        assert_eq!(decode_source_version(0), "0.0.0.0.0");
    }

    #[test]
    fn source_version_simple() {
        // a=1, b=2, c=3, d=4, e=5
        let packed = (1u64 << 40) | (2u64 << 30) | (3u64 << 20) | (4u64 << 10) | 5;
        assert_eq!(decode_source_version(packed), "1.2.3.4.5");
    }

    #[test]
    fn source_version_large_a() {
        // a occupies 24 bits, so test a large a value
        let packed = (1000u64 << 40) | (0u64 << 30) | (0u64 << 20) | (0u64 << 10) | 0;
        assert_eq!(decode_source_version(packed), "1000.0.0.0.0");
    }

    #[test]
    fn source_version_max_components() {
        // b,c,d,e are 10-bit → max 1023
        let packed = (0u64 << 40) | (1023u64 << 30) | (1023u64 << 20) | (1023u64 << 10) | 1023;
        assert_eq!(decode_source_version(packed), "0.1023.1023.1023.1023");
    }

    // ── LoadCommandType From<u32> ───────────────────────────────────

    #[test]
    fn load_command_type_segment() {
        assert_eq!(LoadCommandType::from(0x1), LoadCommandType::Segment);
    }

    #[test]
    fn load_command_type_segment64() {
        assert_eq!(LoadCommandType::from(0x19), LoadCommandType::Segment64);
    }

    #[test]
    fn load_command_type_load_dylib() {
        assert_eq!(LoadCommandType::from(0xC), LoadCommandType::LoadDylib);
    }

    #[test]
    fn load_command_type_build_version() {
        assert_eq!(LoadCommandType::from(0x32), LoadCommandType::BuildVersion);
    }

    #[test]
    fn load_command_type_main() {
        assert_eq!(
            LoadCommandType::from(0x8000_0028),
            LoadCommandType::Main
        );
    }

    #[test]
    fn load_command_type_chained_fixups() {
        assert_eq!(
            LoadCommandType::from(0x8000_0034),
            LoadCommandType::DyldChainedFixups
        );
    }

    #[test]
    fn load_command_type_unknown() {
        assert_eq!(
            LoadCommandType::from(0x9999),
            LoadCommandType::Unknown(0x9999)
        );
    }
}
