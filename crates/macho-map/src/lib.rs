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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_region(name: &str, vmaddr: u64, vmsize: u64, fileoff: u64, filesize: u64) -> MapRegion {
        MapRegion {
            segment_name: name.to_string(),
            vmaddr,
            vmsize,
            fileoff,
            filesize,
            protection: VmProt::READ | VmProt::EXECUTE,
            max_protection: VmProt::READ | VmProt::WRITE | VmProt::EXECUTE,
            zerofill_size: vmsize.saturating_sub(filesize),
        }
    }

    #[test]
    fn empty_plan_display() {
        let plan = MappingPlan { regions: vec![] };
        assert_eq!(plan.display(), "");
    }

    #[test]
    fn single_region_display() {
        let plan = MappingPlan {
            regions: vec![make_region("__TEXT", 0x100000, 0x4000, 0, 0x4000)],
        };
        let out = plan.display();
        assert!(out.contains("__TEXT"), "should contain segment name");
        assert!(out.contains("0x00100000"), "should contain vmaddr");
        assert!(out.contains("0x4000"), "should contain vmsize");
        assert!(out.contains("r-x"), "should contain protection");
    }

    #[test]
    fn zerofill_shown_when_nonzero() {
        let plan = MappingPlan {
            regions: vec![make_region("__DATA", 0x200000, 0x8000, 0x4000, 0x2000)],
        };
        let out = plan.display();
        assert!(out.contains("zerofill"), "should show zerofill line");
        assert!(out.contains("0x6000"), "zerofill should be 0x6000");
    }

    #[test]
    fn zerofill_not_shown_when_zero() {
        let plan = MappingPlan {
            regions: vec![make_region("__TEXT", 0x100000, 0x4000, 0, 0x4000)],
        };
        let out = plan.display();
        assert!(!out.contains("zerofill"), "should not show zerofill when 0");
    }

    #[test]
    fn multiple_regions_display() {
        let plan = MappingPlan {
            regions: vec![
                make_region("__TEXT", 0x100000, 0x4000, 0, 0x4000),
                make_region("__DATA", 0x104000, 0x1000, 0x4000, 0x1000),
            ],
        };
        let out = plan.display();
        assert!(out.contains("__TEXT"));
        assert!(out.contains("__DATA"));
    }

    #[test]
    fn map_region_debug() {
        let r = make_region("__LINKEDIT", 0x300000, 0x1000, 0x8000, 0x1000);
        let dbg = format!("{:?}", r);
        assert!(dbg.contains("__LINKEDIT"));
        assert!(dbg.contains("LINKEDIT")); // segment name present
    }

    #[test]
    fn mapping_plan_clone() {
        let plan = MappingPlan {
            regions: vec![make_region("__TEXT", 0x100000, 0x4000, 0, 0x4000)],
        };
        let cloned = plan.clone();
        assert_eq!(cloned.regions.len(), 1);
        assert_eq!(cloned.regions[0].segment_name, "__TEXT");
    }

    #[test]
    fn display_data_segment_shows_rw_protection() {
        let plan = MappingPlan {
            regions: vec![MapRegion {
                segment_name: "__DATA".to_string(),
                vmaddr: 0x200000,
                vmsize: 0x1000,
                fileoff: 0x4000,
                filesize: 0x1000,
                protection: VmProt::READ | VmProt::WRITE,
                max_protection: VmProt::READ | VmProt::WRITE,
                zerofill_size: 0,
            }],
        };
        let out = plan.display();
        assert!(out.contains("rw-"), "expected rw- protection, got: {out}");
    }

    #[test]
    fn display_readonly_segment() {
        let plan = MappingPlan {
            regions: vec![MapRegion {
                segment_name: "__DATA_CONST".to_string(),
                vmaddr: 0x300000,
                vmsize: 0x1000,
                fileoff: 0x5000,
                filesize: 0x1000,
                protection: VmProt::READ,
                max_protection: VmProt::READ | VmProt::WRITE,
                zerofill_size: 0,
            }],
        };
        let out = plan.display();
        assert!(out.contains("r--"), "expected r-- protection, got: {out}");
    }

    #[test]
    fn display_no_protection() {
        let plan = MappingPlan {
            regions: vec![MapRegion {
                segment_name: "__PAGEZERO".to_string(),
                vmaddr: 0,
                vmsize: 0x1000,
                fileoff: 0,
                filesize: 0,
                protection: VmProt::empty(),
                max_protection: VmProt::empty(),
                zerofill_size: 0x1000,
            }],
        };
        let out = plan.display();
        assert!(out.contains("---"), "expected --- protection, got: {out}");
    }
}
