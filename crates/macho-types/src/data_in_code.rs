use serde::{Deserialize, Serialize};

/// A single entry from the LC_DATA_IN_CODE table.
///
/// Each entry describes a region within __TEXT that contains data
/// (not instructions), such as jump tables or constant pools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataInCodeEntry {
    /// Offset from the start of the __TEXT segment.
    pub offset: u32,
    /// Length of the data region in bytes.
    pub length: u16,
    /// Kind of data (jump table, constant, etc).
    pub kind: DataInCodeKind,
}

/// The kind of data embedded in a code section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataInCodeKind {
    /// Generic data (DICE_KIND_DATA = 1).
    Data,
    /// 8-bit jump table entries (DICE_KIND_JUMP_TABLE8 = 2).
    JumpTable8,
    /// 16-bit jump table entries (DICE_KIND_JUMP_TABLE16 = 3).
    JumpTable16,
    /// 32-bit jump table entries (DICE_KIND_JUMP_TABLE32 = 4).
    JumpTable32,
    /// Absolute 32-bit jump table (DICE_KIND_ABS_JUMP_TABLE32 = 5).
    AbsJumpTable32,
    /// Unknown kind value.
    Unknown(u16),
}

impl From<u16> for DataInCodeKind {
    fn from(v: u16) -> Self {
        match v {
            1 => Self::Data,
            2 => Self::JumpTable8,
            3 => Self::JumpTable16,
            4 => Self::JumpTable32,
            5 => Self::AbsJumpTable32,
            other => Self::Unknown(other),
        }
    }
}
