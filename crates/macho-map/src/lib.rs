use macho_parse::ParsedMachO;
use macho_types::segment::VmProt;
use serde::{Deserialize, Serialize};

/// A planned memory mapping region derived from Mach-O segment commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapRegion {
    pub segment_name: String,
    pub vmaddr: u64,
    pub vmsize: u64,
    pub fileoff: u64,
    pub filesize: u64,
    pub protection: VmProt,
    /// Maximum allowed protection (e.g. __DATA_CONST: maxprot=RW, initprot=R).
    pub max_protection: VmProt,
    /// Bytes beyond filesize that should be zero-filled.
    pub zerofill_size: u64,
}

/// Complete mapping plan for a Mach-O binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingPlan {
    pub regions: Vec<MapRegion>,
}

impl MappingPlan {
    /// Compute a mapping plan from a parsed Mach-O.
    pub fn from_parsed(parsed: &ParsedMachO) -> Self {
        let regions = parsed
            .segments
            .iter()
            .filter(|seg| seg.vmsize > 0)
            .map(|seg| {
                let zerofill_size = seg.vmsize.saturating_sub(seg.filesize);
                MapRegion {
                    segment_name: seg.name.clone(),
                    vmaddr: seg.vmaddr,
                    vmsize: seg.vmsize,
                    fileoff: seg.fileoff,
                    filesize: seg.filesize,
                    protection: seg.initprot,
                    max_protection: seg.maxprot,
                    zerofill_size,
                }
            })
            .collect();

        Self { regions }
    }

    /// Human-readable display of the mapping plan.
    pub fn display(&self) -> String {
        let mut out = String::new();
        for r in &self.regions {
            out.push_str(&format!(
                "Segment {}\n  vmaddr: {:#010x}\n  vmsize: {:#x}\n  fileoff: {:#x}\n  filesize: {:#x}\n  prot: {}\n  maxprot: {}\n",
                r.segment_name, r.vmaddr, r.vmsize, r.fileoff, r.filesize, r.protection, r.max_protection
            ));
            if r.zerofill_size > 0 {
                out.push_str(&format!("  zerofill: {:#x}\n", r.zerofill_size));
            }
            out.push('\n');
        }
        out
    }
}
