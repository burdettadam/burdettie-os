use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolTable {
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub sym_type: SymbolType,
    pub section_index: u8,
    pub desc: u16,
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolType {
    /// Undefined — needs to be resolved from a dylib.
    Undefined,
    /// Absolute symbol — not relative to any section.
    Absolute,
    /// Defined in a section.
    Defined,
    /// Pre-bound undefined (rare).
    PreboundUndefined,
    /// Indirect symbol.
    Indirect,
    /// Unknown n_type value.
    Unknown(u8),
}

impl SymbolType {
    pub fn from_ntype(n_type: u8) -> Self {
        let masked = n_type & 0x0E; // N_TYPE mask
        match masked {
            0x00 => Self::Undefined,
            0x02 => Self::Absolute,
            0x0E => Self::Defined,
            0x0C => Self::PreboundUndefined,
            0x0A => Self::Indirect,
            _ => Self::Unknown(masked),
        }
    }

    pub fn is_external(n_type: u8) -> bool {
        n_type & 0x01 != 0 // N_EXT
    }
}

impl Symbol {
    /// The dylib ordinal from n_desc (two-level namespace).
    /// Ordinal 0 = self, -1 = main executable, -2 = flat lookup, -3 = weak lookup.
    pub fn library_ordinal(&self) -> i16 {
        ((self.desc >> 8) & 0xFF) as i8 as i16
    }

    /// True if this is a stab debugging symbol (n_type has high bits set).
    /// Note: stab type is stored in the raw n_type, but we only have SymbolType here.
    /// Callers should check the raw n_type with `is_stab()`.
    pub fn is_weak_ref(&self) -> bool {
        self.desc & 0x0040 != 0
    }

    pub fn is_weak_def(&self) -> bool {
        self.desc & 0x0080 != 0
    }

    pub fn is_no_dead_strip(&self) -> bool {
        self.desc & 0x0020 != 0
    }

    pub fn is_referenced_dynamically(&self) -> bool {
        self.desc & 0x0010 != 0
    }
}

/// Check if a raw n_type byte represents a stab debugging symbol.
pub fn is_stab(n_type: u8) -> bool {
    n_type & 0xE0 != 0
}
