//! Tests for macho-map: segment mapping plan generation.
//!
//! Ported from:
//!   - LLVM lld: segment-layout, pagezero, data-const tests
//!   - XNU mach_loader.c: slide computation, protection validation
//!   - dyld: zerofill handling

#[cfg(test)]
mod tests {
    use macho_map::{MapRegion, MappingPlan};
    use macho_parse::parse;
    use macho_types::segment::VmProt;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
    }

    fn read_fixture(name: &str) -> Option<Vec<u8>> {
        std::fs::read(fixtures_dir().join(name)).ok()
    }

    // ── Mapping plan from simple_cli ────────────────────────────────

    #[test]
    fn simple_cli_mapping_plan_has_regions() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);
        assert!(!plan.regions.is_empty());
    }

    #[test]
    fn simple_cli_pagezero_is_largest_region() {
        // PAGEZERO on arm64 is 4GB — should be the largest by vmsize
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);
        let pz = plan.regions.iter().find(|r| r.segment_name == "__PAGEZERO");
        assert!(pz.is_some());
        let pz = pz.unwrap();
        assert_eq!(pz.vmaddr, 0);
        assert!(
            pz.vmsize >= 0x1_0000_0000,
            "PAGEZERO should be at least 4GB on arm64"
        );
        assert_eq!(pz.filesize, 0);
        assert_eq!(pz.protection, VmProt::empty());
    }

    #[test]
    fn simple_cli_text_is_read_execute() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);
        let text = plan.regions.iter().find(|r| r.segment_name == "__TEXT");
        assert!(text.is_some());
        let text = text.unwrap();
        assert!(text.protection.contains(VmProt::READ));
        assert!(text.protection.contains(VmProt::EXECUTE));
        assert!(!text.protection.contains(VmProt::WRITE));
    }

    #[test]
    fn simple_cli_linkedit_is_readonly() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);
        let le = plan.regions.iter().find(|r| r.segment_name == "__LINKEDIT");
        assert!(le.is_some());
        let le = le.unwrap();
        assert_eq!(le.protection, VmProt::READ);
    }

    // ── Zerofill computation (from dyld segment mapping behavior) ───

    #[test]
    fn zerofill_computed_from_vmsize_minus_filesize() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);

        for region in &plan.regions {
            if region.vmsize > region.filesize {
                assert_eq!(
                    region.zerofill_size,
                    region.vmsize - region.filesize,
                    "zerofill should be vmsize - filesize for segment {}",
                    region.segment_name
                );
            } else {
                assert_eq!(region.zerofill_size, 0);
            }
        }
    }

    // ── Segments with vmsize=0 are excluded (from lld behavior) ─────

    #[test]
    fn zero_vmsize_segments_excluded() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);

        for region in &plan.regions {
            assert!(
                region.vmsize > 0,
                "mapping plan should not include zero-size segments"
            );
        }
    }

    // ── Segment address ordering (from XNU: segments must not overlap) ──

    #[test]
    fn segments_do_not_overlap() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);

        // Filter out PAGEZERO which is special
        let mut mapped: Vec<&MapRegion> = plan
            .regions
            .iter()
            .filter(|r| r.segment_name != "__PAGEZERO")
            .collect();
        mapped.sort_by_key(|r| r.vmaddr);

        for pair in mapped.windows(2) {
            let end_prev = pair[0].vmaddr + pair[0].vmsize;
            assert!(
                end_prev <= pair[1].vmaddr,
                "segments {} and {} overlap: {:#x}+{:#x} vs {:#x}",
                pair[0].segment_name,
                pair[1].segment_name,
                pair[0].vmaddr,
                pair[0].vmsize,
                pair[1].vmaddr
            );
        }
    }

    // ── Display output format ───────────────────────────────────────

    #[test]
    fn display_contains_segment_names() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);
        let display = plan.display();
        assert!(display.contains("__TEXT"));
        assert!(display.contains("vmaddr:"));
        assert!(display.contains("prot:"));
    }

    // ── JSON serialization (invariant: every stage emits JSON) ──────

    #[test]
    fn mapping_plan_serializes_to_json() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("__TEXT"));
        assert!(json.contains("vmaddr"));
    }

    // ── dylib_chain mapping plan ────────────────────────────────────

    #[test]
    fn dylib_chain_app_mapping() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);
        assert!(!plan.regions.is_empty());

        let has_text = plan.regions.iter().any(|r| r.segment_name == "__TEXT");
        assert!(has_text);
    }

    #[test]
    fn dylib_chain_dylib_mapping() {
        let Some(data) = read_fixture("dylib_chain/libchaina.dylib") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);

        // Dylibs don't have PAGEZERO
        let has_pagezero = plan.regions.iter().any(|r| r.segment_name == "__PAGEZERO");
        assert!(!has_pagezero, "dylibs should not have __PAGEZERO");
    }
}

// ── Phase 17: max_protection field ──

#[cfg(test)]
mod phase17_map_tests {
    use macho_map::MappingPlan;
    use macho_parse::parse;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
    }

    fn read_fixture(name: &str) -> Option<Vec<u8>> {
        std::fs::read(fixtures_dir().join(name)).ok()
    }

    #[test]
    fn mapping_plan_has_max_protection() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);

        for region in &plan.regions {
            if region.segment_name == "__PAGEZERO" {
                continue;
            }
            // max_protection should be at least as permissive as protection
            assert!(
                region.max_protection.contains(region.protection),
                "segment {} maxprot {} should contain initprot {}",
                region.segment_name,
                region.max_protection,
                region.protection
            );
        }
    }

    #[test]
    fn text_segment_maxprot_includes_execute() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);

        let text = plan.regions.iter().find(|r| r.segment_name == "__TEXT");
        assert!(text.is_some(), "should have __TEXT");
        let text = text.unwrap();
        assert!(
            text.max_protection
                .contains(macho_types::segment::VmProt::EXECUTE),
            "__TEXT maxprot should include EXECUTE"
        );
    }

    #[test]
    fn display_includes_maxprot() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);
        let display = plan.display();
        assert!(
            display.contains("maxprot"),
            "display should show maxprot field"
        );
    }
}
