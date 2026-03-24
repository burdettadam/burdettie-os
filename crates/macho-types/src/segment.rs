use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub name: String,
    pub vmaddr: u64,
    pub vmsize: u64,
    pub fileoff: u64,
    pub filesize: u64,
    pub maxprot: VmProt,
    pub initprot: VmProt,
    pub nsects: u32,
    pub flags: u32,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub sectname: String,
    pub segname: String,
    pub addr: u64,
    pub size: u64,
    pub offset: u32,
    pub align: u32,
    pub reloff: u32,
    pub nreloc: u32,
    pub flags: u32,
    /// For sections with indirect symbols (__stubs, __got, __la_symbol_ptr),
    /// this is the index into the indirect symbol table.
    pub reserved1: u32,
    /// For __stubs sections, this is the stub size in bytes.
    pub reserved2: u32,
}

impl Section {
    /// Returns the section type from the low 8 bits of flags.
    pub fn section_type(&self) -> u8 {
        (self.flags & 0xFF) as u8
    }

    /// Returns the section type as a named enum variant.
    pub fn section_type_name(&self) -> SectionType {
        SectionType::from(self.section_type())
    }

    /// True if this section contains zero-fill data (no file backing).
    pub fn is_zerofill(&self) -> bool {
        let t = self.section_type();
        t == 0x01 || t == 0x0C || t == 0x12 // S_ZEROFILL, S_GB_ZEROFILL, S_THREAD_LOCAL_ZEROFILL
    }
}

/// Named section types (low 8 bits of section flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionType {
    Regular,
    ZeroFill,
    CstringLiterals,
    FourByteLiterals,
    EightByteLiterals,
    LiteralPointers,
    NonLazySymbolPointers,
    LazySymbolPointers,
    SymbolStubs,
    ModInitFuncPointers,
    ModTermFuncPointers,
    Coalesced,
    GbZeroFill,
    Interposing,
    SixteenByteLiterals,
    DtraceDof,
    LazyDylibSymbolPointers,
    ThreadLocalRegular,
    ThreadLocalZeroFill,
    ThreadLocalVariables,
    ThreadLocalVariablePointers,
    ThreadLocalInitFunctionPointers,
    InitOffsets,
    Unknown(u8),
}

impl From<u8> for SectionType {
    fn from(v: u8) -> Self {
        match v {
            0x00 => Self::Regular,
            0x01 => Self::ZeroFill,
            0x02 => Self::CstringLiterals,
            0x03 => Self::FourByteLiterals,
            0x04 => Self::EightByteLiterals,
            0x05 => Self::LiteralPointers,
            0x06 => Self::NonLazySymbolPointers,
            0x07 => Self::LazySymbolPointers,
            0x08 => Self::SymbolStubs,
            0x09 => Self::ModInitFuncPointers,
            0x0A => Self::ModTermFuncPointers,
            0x0B => Self::Coalesced,
            0x0C => Self::GbZeroFill,
            0x0D => Self::Interposing,
            0x0E => Self::SixteenByteLiterals,
            0x0F => Self::DtraceDof,
            0x10 => Self::LazyDylibSymbolPointers,
            0x11 => Self::ThreadLocalRegular,
            0x12 => Self::ThreadLocalZeroFill,
            0x13 => Self::ThreadLocalVariables,
            0x14 => Self::ThreadLocalVariablePointers,
            0x15 => Self::ThreadLocalInitFunctionPointers,
            0x16 => Self::InitOffsets,
            other => Self::Unknown(other),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VmProt: u32 {
        const READ    = 0x01;
        const WRITE   = 0x02;
        const EXECUTE = 0x04;
    }
}

impl Serialize for VmProt {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VmProt {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u32::deserialize(deserializer)?;
        Ok(Self::from_bits_truncate(bits))
    }
}

impl std::fmt::Display for VmProt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = if self.contains(Self::READ) { 'r' } else { '-' };
        let w = if self.contains(Self::WRITE) { 'w' } else { '-' };
        let x = if self.contains(Self::EXECUTE) {
            'x'
        } else {
            '-'
        };
        write!(f, "{r}{w}{x}")
    }
}

/// A Thread-Local Variable (TLV) descriptor from `__thread_vars` sections.
///
/// Each descriptor is 3 pointers (24 bytes on 64-bit):
///   - `thunk`: function pointer to the TLV allocator (typically `_tlv_bootstrap`)
///   - `key`: pthread key for this TLV slot
///   - `offset`: offset into the thread-local storage block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlvDescriptor {
    /// Address of the descriptor in the `__thread_vars` section.
    pub address: u64,
    /// Raw thunk function pointer (allocator, e.g. `_tlv_bootstrap`).
    pub thunk: u64,
    /// Pthread key value.
    pub key: u64,
    /// Offset into the TLV storage block.
    pub offset: u64,
}
