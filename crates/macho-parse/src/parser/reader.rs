//! Low-level byte reader for Mach-O binary parsing.

use crate::ParseError;
use macho_types::header::{MachFlags, MH_CIGAM, MH_CIGAM_64, MH_MAGIC, MH_MAGIC_64};
use macho_types::segment::VmProt;
use macho_types::*;

pub(super) struct Reader<'a> {
    pub(super) data: &'a [u8],
    pub(super) pos: usize,
}

impl<'a> Reader<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub(super) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub(super) fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        if self.remaining() < n {
            return Err(ParseError::UnexpectedEof {
                offset: self.pos,
                needed: n,
                available: self.remaining(),
            });
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub(super) fn read_u8(&mut self) -> Result<u8, ParseError> {
        Ok(self.read_bytes(1)?[0])
    }

    pub(super) fn read_u16(&mut self) -> Result<u16, ParseError> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub(super) fn read_u32(&mut self) -> Result<u32, ParseError> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub(super) fn read_u64(&mut self) -> Result<u64, ParseError> {
        let b = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub(super) fn read_fixed_string(&mut self, len: usize) -> Result<String, ParseError> {
        let bytes = self.read_bytes(len)?;
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(len);
        std::str::from_utf8(&bytes[..end])
            .map(std::string::ToString::to_string)
            .map_err(|e| ParseError::InvalidUtf8 {
                context: "fixed string",
                source: e,
            })
    }

    pub(super) fn read_header(&mut self) -> Result<MachHeader, ParseError> {
        let magic = self.read_u32()?;
        match magic {
            MH_MAGIC_64 | MH_CIGAM_64 | MH_MAGIC | MH_CIGAM => {}
            _ => return Err(ParseError::BadMagic { magic }),
        }

        let cpu_type = CpuType::from(self.read_u32()?);
        let cpu_subtype = self.read_u32()?;
        let file_type = FileType::from(self.read_u32()?);
        let num_commands = self.read_u32()?;
        let size_of_commands = self.read_u32()?;
        let flags = MachFlags::from_bits_truncate(self.read_u32()?);
        let reserved = if magic == MH_MAGIC_64 || magic == MH_CIGAM_64 {
            Some(self.read_u32()?)
        } else {
            None
        };

        Ok(MachHeader {
            magic,
            cpu_type,
            cpu_subtype,
            file_type,
            num_commands,
            size_of_commands,
            flags,
            reserved,
        })
    }

    pub(super) fn read_segment_64(&mut self) -> Result<Segment, ParseError> {
        let name = self.read_fixed_string(16)?;
        let vmaddr = self.read_u64()?;
        let vmsize = self.read_u64()?;
        let fileoff = self.read_u64()?;
        let filesize = self.read_u64()?;
        let maxprot = VmProt::from_bits_truncate(self.read_u32()?);
        let initprot = VmProt::from_bits_truncate(self.read_u32()?);
        let nsects = self.read_u32()?;
        let flags = self.read_u32()?;

        let mut sections = Vec::with_capacity(nsects as usize);
        for _ in 0..nsects {
            sections.push(self.read_section_64()?);
        }

        Ok(Segment {
            name,
            vmaddr,
            vmsize,
            fileoff,
            filesize,
            maxprot,
            initprot,
            nsects,
            flags,
            sections,
        })
    }

    fn read_section_64(&mut self) -> Result<Section, ParseError> {
        let sectname = self.read_fixed_string(16)?;
        let segname = self.read_fixed_string(16)?;
        let addr = self.read_u64()?;
        let size = self.read_u64()?;
        let offset = self.read_u32()?;
        let align = self.read_u32()?;
        let reloff = self.read_u32()?;
        let nreloc = self.read_u32()?;
        let flags = self.read_u32()?;
        let reserved1 = self.read_u32()?;
        let reserved2 = self.read_u32()?;
        let _reserved3 = self.read_u32()?;

        Ok(Section {
            sectname,
            segname,
            addr,
            size,
            offset,
            align,
            reloff,
            nreloc,
            flags,
            reserved1,
            reserved2,
        })
    }

    pub(super) fn read_segment_32(&mut self) -> Result<Segment, ParseError> {
        let name = self.read_fixed_string(16)?;
        let vmaddr = u64::from(self.read_u32()?);
        let vmsize = u64::from(self.read_u32()?);
        let fileoff = u64::from(self.read_u32()?);
        let filesize = u64::from(self.read_u32()?);
        let maxprot = VmProt::from_bits_truncate(self.read_u32()?);
        let initprot = VmProt::from_bits_truncate(self.read_u32()?);
        let nsects = self.read_u32()?;
        let flags = self.read_u32()?;

        let mut sections = Vec::with_capacity(nsects as usize);
        for _ in 0..nsects {
            sections.push(self.read_section_32()?);
        }

        Ok(Segment {
            name,
            vmaddr,
            vmsize,
            fileoff,
            filesize,
            maxprot,
            initprot,
            nsects,
            flags,
            sections,
        })
    }

    fn read_section_32(&mut self) -> Result<Section, ParseError> {
        let sectname = self.read_fixed_string(16)?;
        let segname = self.read_fixed_string(16)?;
        let addr = u64::from(self.read_u32()?);
        let size = u64::from(self.read_u32()?);
        let offset = self.read_u32()?;
        let align = self.read_u32()?;
        let reloff = self.read_u32()?;
        let nreloc = self.read_u32()?;
        let flags = self.read_u32()?;
        let reserved1 = self.read_u32()?;
        let reserved2 = self.read_u32()?;

        Ok(Section {
            sectname,
            segname,
            addr,
            size,
            offset,
            align,
            reloff,
            nreloc,
            flags,
            reserved1,
            reserved2,
        })
    }

    pub(super) fn read_dylib_ref(&mut self, cmd_start: usize) -> Result<DylibRef, ParseError> {
        let name_offset = self.read_u32()? as usize;
        let timestamp = self.read_u32()?;
        let current_version = self.read_u32()?;
        let compatibility_version = self.read_u32()?;

        let abs_offset = cmd_start + name_offset;
        if abs_offset >= self.data.len() {
            return Err(ParseError::UnexpectedEof {
                offset: abs_offset,
                needed: 1,
                available: 0,
            });
        }
        let name_bytes = &self.data[abs_offset..];
        let end = name_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(name_bytes.len());
        let name = std::str::from_utf8(&name_bytes[..end])
            .map_err(|e| ParseError::InvalidUtf8 {
                context: "dylib name",
                source: e,
            })?
            .to_string();

        Ok(DylibRef {
            name,
            compatibility_version,
            current_version,
            timestamp,
        })
    }

    pub(super) fn read_lc_string(&mut self, cmd_start: usize) -> Result<String, ParseError> {
        let str_offset = self.read_u32()? as usize;
        let abs_offset = cmd_start + str_offset;
        if abs_offset >= self.data.len() {
            return Err(ParseError::UnexpectedEof {
                offset: abs_offset,
                needed: 1,
                available: 0,
            });
        }
        let bytes = &self.data[abs_offset..];
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..end])
            .map(std::string::ToString::to_string)
            .map_err(|e| ParseError::InvalidUtf8 {
                context: "lc_string",
                source: e,
            })
    }
}
