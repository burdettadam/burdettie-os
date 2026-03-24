use bitflags::bitflags;
use serde::{Deserialize, Serialize};

/// Mach-O magic constants.
pub const MH_MAGIC_64: u32 = 0xFEED_FACF;
pub const MH_CIGAM_64: u32 = 0xCFFAEDFE;
pub const MH_MAGIC: u32 = 0xFEED_FACE;
pub const MH_CIGAM: u32 = 0xCEFAEDFE;
pub const FAT_MAGIC: u32 = 0xCAFE_BABE;
pub const FAT_CIGAM: u32 = 0xBEBA_FECA;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachHeader {
    pub magic: u32,
    pub cpu_type: CpuType,
    pub cpu_subtype: u32,
    pub file_type: FileType,
    pub num_commands: u32,
    pub size_of_commands: u32,
    pub flags: MachFlags,
    /// Only present in 64-bit Mach-O.
    pub reserved: Option<u32>,
}

impl MachHeader {
    pub fn is_64bit(&self) -> bool {
        self.magic == MH_MAGIC_64 || self.magic == MH_CIGAM_64
    }

    pub fn is_swapped(&self) -> bool {
        self.magic == MH_CIGAM || self.magic == MH_CIGAM_64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum CpuType {
    X86 = 7,
    X86_64 = 0x0100_0007,
    Arm = 12,
    Arm64 = 0x0100_000C,
    Unknown(u32),
}

impl From<u32> for CpuType {
    fn from(v: u32) -> Self {
        match v {
            7 => Self::X86,
            0x0100_0007 => Self::X86_64,
            12 => Self::Arm,
            0x0100_000C => Self::Arm64,
            other => Self::Unknown(other),
        }
    }
}

impl From<CpuType> for u32 {
    fn from(ct: CpuType) -> u32 {
        match ct {
            CpuType::X86 => 7,
            CpuType::X86_64 => 0x0100_0007,
            CpuType::Arm => 12,
            CpuType::Arm64 => 0x0100_000C,
            CpuType::Unknown(v) => v,
        }
    }
}

/// Named ARM64/x86_64 CPU subtypes.
///
/// The high bit (0x8000_0000) is `CPU_SUBTYPE_LIB64` — a capability flag.
/// We mask it out for matching then preserve it in the raw value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuSubtype {
    Arm64All,
    Arm64E,
    ArmV7,
    ArmV7S,
    X86_64All,
    X86_64Haswell,
    /// Any CPU subtype not explicitly listed.
    Unknown(u32),
}

impl CpuSubtype {
    /// Decode a raw cpu_subtype u32. The high bit is a capability flag that
    /// we strip for matching but preserve in `Unknown`.
    pub fn from_pair(cpu_type: CpuType, raw: u32) -> Self {
        let sub = raw & !0x8000_0000; // mask off CPU_SUBTYPE_LIB64
        match cpu_type {
            CpuType::Arm64 => match sub {
                0 | 0xFF => Self::Arm64All,
                2 => Self::Arm64E,
                _ => Self::Unknown(raw),
            },
            CpuType::Arm => match sub {
                9 => Self::ArmV7,
                11 => Self::ArmV7S,
                _ => Self::Unknown(raw),
            },
            CpuType::X86_64 => match sub {
                3 | 0xFF => Self::X86_64All,
                8 => Self::X86_64Haswell,
                _ => Self::Unknown(raw),
            },
            _ => Self::Unknown(raw),
        }
    }

    /// Is this subtype ARM64E (pointer authentication)?
    pub fn is_arm64e(&self) -> bool {
        matches!(self, Self::Arm64E)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum FileType {
    Object = 1,
    Execute = 2,
    Dylib = 6,
    Bundle = 8,
    DylibStub = 9,
    Dsym = 10,
    Unknown(u32),
}

impl From<u32> for FileType {
    fn from(v: u32) -> Self {
        match v {
            1 => Self::Object,
            2 => Self::Execute,
            6 => Self::Dylib,
            8 => Self::Bundle,
            9 => Self::DylibStub,
            10 => Self::Dsym,
            other => Self::Unknown(other),
        }
    }
}

impl From<FileType> for u32 {
    fn from(ft: FileType) -> u32 {
        match ft {
            FileType::Object => 1,
            FileType::Execute => 2,
            FileType::Dylib => 6,
            FileType::Bundle => 8,
            FileType::DylibStub => 9,
            FileType::Dsym => 10,
            FileType::Unknown(v) => v,
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MachFlags: u32 {
        const NO_UNDEFS                = 0x0000_0001;
        const INCRLINK                 = 0x0000_0002;
        const DYLDLINK                 = 0x0000_0004;
        const TWO_LEVEL                = 0x0000_0080;
        const FORCE_FLAT               = 0x0000_0100;
        const SUBSECTIONS_VIA_SYMBOLS  = 0x0000_2000;
        const WEAK_DEFINES             = 0x0000_8000;
        const BINDS_TO_WEAK            = 0x0001_0000;
        const ALLOW_STACK_EXECUTION    = 0x0002_0000;
        const NO_REEXPORTED_DYLIBS     = 0x0010_0000;
        const PIE                      = 0x0020_0000;
        const ROOT_SAFE                = 0x0040_0000;
        const HAS_TLV_DESCRIPTORS      = 0x0080_0000;
        const NO_HEAP_EXECUTION        = 0x0100_0000;
        const APP_EXTENSION_SAFE       = 0x0200_0000;
        const CANONICAL                = 0x0000_4000;
        const DYLIB_IN_CACHE           = 0x8000_0000u32;
    }
}

impl Serialize for MachFlags {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MachFlags {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u32::deserialize(deserializer)?;
        Ok(Self::from_bits_truncate(bits))
    }
}

/// Header for a fat (universal) Mach-O binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatHeader {
    pub magic: u32,
    pub nfat_arch: u32,
}

/// Architecture descriptor within a fat binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatArch {
    pub cpu_type: CpuType,
    pub cpu_subtype: u32,
    /// File offset to the start of this architecture's Mach-O data.
    pub offset: u64,
    /// Size of this architecture's Mach-O data.
    pub size: u64,
    /// Alignment as a power of 2.
    pub align: u32,
}

/// A complete parsed fat binary with its architecture slices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatBinary {
    pub header: FatHeader,
    pub arches: Vec<FatArch>,
}
