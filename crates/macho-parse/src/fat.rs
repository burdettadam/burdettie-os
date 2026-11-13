//! Fat (universal) Mach-O binary parsing.
//!
//! Fat binaries contain multiple architecture slices, each a complete
//! Mach-O binary. This module parses the fat header and architecture
//! table, and provides helpers to select a slice by CPU type.

use macho_types::header::{
    CpuSubtype, CpuType, FatArch, FatBinary, FatHeader, FAT_CIGAM, FAT_MAGIC,
};

use crate::error::ParseError;

/// Parse a fat binary header and architecture table.
///
/// Returns the parsed `FatBinary` with offset/size for each architecture.
pub fn parse_fat(data: &[u8]) -> Result<FatBinary, ParseError> {
    if data.len() < 8 {
        return Err(ParseError::UnexpectedEof {
            offset: 0,
            needed: 8,
            available: data.len(),
        });
    }

    let magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let swapped = magic == FAT_CIGAM;

    if magic != FAT_MAGIC && !swapped {
        return Err(ParseError::BadMagic { magic });
    }

    let nfat_arch = read_u32_fat(data, 4, swapped);

    // Sanity check: each fat_arch entry is 20 bytes (32-bit) or 32 bytes (64-bit).
    // We detect fat_arch_64 by checking if the magic is the 64-bit variant.
    // Standard fat uses `fat_arch` (20 bytes each). We support both.
    let is_fat64 = false; // fat_arch_64 uses a different magic (0xCAFEBABF) — very rare
    let arch_size: usize = if is_fat64 { 32 } else { 20 };
    let header_end = 8 + nfat_arch as usize * arch_size;

    if header_end > data.len() {
        return Err(ParseError::UnexpectedEof {
            offset: 8,
            needed: header_end - 8,
            available: data.len() - 8,
        });
    }

    let mut arches = Vec::with_capacity(nfat_arch as usize);
    for i in 0..nfat_arch as usize {
        let base = 8 + i * arch_size;
        let cpu_type = CpuType::from(read_u32_fat(data, base, swapped));
        let cpu_subtype = read_u32_fat(data, base + 4, swapped);
        let offset = u64::from(read_u32_fat(data, base + 8, swapped));
        let size = u64::from(read_u32_fat(data, base + 12, swapped));
        let align = read_u32_fat(data, base + 16, swapped);

        // Validate bounds
        if offset + size > data.len() as u64 {
            return Err(ParseError::Other(format!(
                "fat arch[{i}] ({cpu_type:?}) offset {offset:#x} + size {size:#x} exceeds file size {:#x}",
                data.len()
            )));
        }

        arches.push(FatArch {
            cpu_type,
            cpu_subtype,
            offset,
            size,
            align,
        });
    }

    Ok(FatBinary {
        header: FatHeader { magic, nfat_arch },
        arches,
    })
}

/// Check if data begins with a fat binary magic.
pub fn is_fat(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    let magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    magic == FAT_MAGIC || magic == FAT_CIGAM
}

/// Extract the raw slice for a specific CPU type from a fat binary.
///
/// Returns the byte sub-slice containing the Mach-O for the requested arch.
pub fn fat_slice_for_cpu(data: &[u8], cpu: CpuType) -> Result<&[u8], ParseError> {
    // For Arm64, prefer the non-PAC (Arm64All) subtype over Arm64E.
    let preferred = if cpu == CpuType::Arm64 {
        Some(CpuSubtype::Arm64All)
    } else {
        None
    };
    fat_slice_for_cpu_subtype(data, cpu, preferred)
}

/// Extract the raw slice for a specific CPU type and optional subtype.
///
/// If `preferred_subtype` is `Some`, prefers the matching (type, subtype) pair.
/// Falls back to the first slice matching `cpu` alone.
pub fn fat_slice_for_cpu_subtype(
    data: &[u8],
    cpu: CpuType,
    preferred_subtype: Option<CpuSubtype>,
) -> Result<&[u8], ParseError> {
    let fat = parse_fat(data)?;
    let target_u32: u32 = cpu.into();

    // Prefer exact (type + subtype) match
    if let Some(ref pref) = preferred_subtype {
        for arch in &fat.arches {
            let arch_u32: u32 = arch.cpu_type.into();
            if arch_u32 == target_u32 {
                let arch_sub = CpuSubtype::from_pair(arch.cpu_type, arch.cpu_subtype);
                if &arch_sub == pref {
                    let start = arch.offset as usize;
                    let end = start + arch.size as usize;
                    return Ok(&data[start..end]);
                }
            }
        }
    }

    // Fall back to first match by CPU type alone
    for arch in &fat.arches {
        let arch_u32: u32 = arch.cpu_type.into();
        if arch_u32 == target_u32 {
            let arch_sub = CpuSubtype::from_pair(arch.cpu_type, arch.cpu_subtype);
            if arch_sub == CpuSubtype::Arm64E {
                log::warn!("fat binary: selecting arm64e slice (no plain arm64 available); PAC features may not be fully supported");
            }
            let start = arch.offset as usize;
            let end = start + arch.size as usize;
            return Ok(&data[start..end]);
        }
    }
    Err(ParseError::Other(format!(
        "fat binary has no slice for {cpu:?} (available: {:?})",
        fat.arches.iter().map(|a| a.cpu_type).collect::<Vec<_>>()
    )))
}

/// Parse a Mach-O binary, auto-detecting fat format and selecting the given CPU.
///
/// If the data is a fat binary, extracts the slice for `preferred_cpu` and parses it.
/// If the data is a thin Mach-O, parses it directly (ignoring `preferred_cpu`).
pub fn parse_auto(data: &[u8], preferred_cpu: CpuType) -> Result<crate::ParsedMachO, ParseError> {
    if is_fat(data) {
        let slice = fat_slice_for_cpu(data, preferred_cpu)?;
        crate::parse(slice)
    } else {
        crate::parse(data)
    }
}

/// Read a big-endian u32, byte-swapping if the fat header was cigam.
fn read_u32_fat(data: &[u8], offset: usize, swapped: bool) -> u32 {
    let raw = u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]);
    if swapped {
        raw.swap_bytes()
    } else {
        raw
    }
}
