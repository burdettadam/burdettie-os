use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relocation {
    pub address: u32,
    pub symbol_index: u32,
    pub pc_relative: bool,
    pub length: u8,
    pub extern_sym: bool,
    pub reloc_type: RelocationType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelocationType {
    Vanilla,
    Pair,
    SectorDiff,
    LocalSectorDiff,
    PbLazyPtr,
    Branch24,
    Arm64Addend,
    Arm64Subtractor,
    Arm64Branch26,
    Arm64Page21,
    Arm64Pageoff12,
    Arm64GotLoadPage21,
    Arm64GotLoadPageoff12,
    Arm64Pointer,
    Unknown(u8),
}

impl From<u8> for RelocationType {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Vanilla,
            1 => Self::Pair,
            2 => Self::SectorDiff,
            3 => Self::LocalSectorDiff,
            4 => Self::PbLazyPtr,
            5 => Self::Branch24,
            6 => Self::Arm64Branch26,
            7 => Self::Arm64Page21,
            8 => Self::Arm64Pageoff12,
            9 => Self::Arm64GotLoadPage21,
            10 => Self::Arm64GotLoadPageoff12,
            11 => Self::Arm64Subtractor,
            12 => Self::Arm64Addend,
            13 => Self::Arm64Pointer,
            _ => Self::Unknown(v),
        }
    }
}
