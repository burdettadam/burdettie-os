use serde::{Deserialize, Serialize};

/// An observed fact from running the binary on the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostObservation {
    /// Dylibs actually loaded (from DYLD_PRINT_LIBRARIES or otool -L).
    pub loaded_dylibs: Vec<String>,
    /// Segment mappings observed (from vmmap or similar).
    pub segments: Vec<ObservedSegment>,
    /// Entry point observed (from otool -l LC_MAIN).
    pub entry_offset: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedSegment {
    pub name: String,
    pub vmaddr: u64,
    pub vmsize: u64,
}

/// A single mismatch between plan and observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mismatch {
    pub category: String,
    pub expected: String,
    pub observed: String,
}

/// Result of comparing a plan to host reality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub matches: Vec<String>,
    pub mismatches: Vec<Mismatch>,
    pub warnings: Vec<String>,
}

impl DiffReport {
    /// Compare loaded dylibs between plan and observation.
    pub fn compare_dylibs(planned: &[String], observed: &[String]) -> Self {
        let mut matches = Vec::new();
        let mut mismatches = Vec::new();
        let mut warnings = Vec::new();

        for name in planned {
            if observed
                .iter()
                .any(|o| o.contains(name.as_str()) || name.contains(o.as_str()))
            {
                matches.push(format!("dylib found: {name}"));
            } else {
                mismatches.push(Mismatch {
                    category: "dylib".into(),
                    expected: name.clone(),
                    observed: "(not found in host observation)".into(),
                });
            }
        }

        for obs in observed {
            if !planned
                .iter()
                .any(|p| obs.contains(p.as_str()) || p.contains(obs.as_str()))
            {
                warnings.push(format!("host loaded dylib not in plan: {obs}"));
            }
        }

        Self {
            matches,
            mismatches,
            warnings,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.mismatches.is_empty()
    }

    /// Compare segments between our plan and otool-observed segments.
    pub fn compare_segments(
        planned: &[(String, u64, u64)], // (name, vmaddr, vmsize)
        observed: &[ObservedSegment],
    ) -> Self {
        let mut matches = Vec::new();
        let mut mismatches = Vec::new();
        let mut warnings = Vec::new();

        for (name, vmaddr, vmsize) in planned {
            if let Some(obs) = observed.iter().find(|o| o.name == *name) {
                if obs.vmaddr == *vmaddr && obs.vmsize == *vmsize {
                    matches.push(format!(
                        "segment {name} matches: vmaddr={vmaddr:#x} vmsize={vmsize:#x}"
                    ));
                } else {
                    if obs.vmaddr != *vmaddr {
                        mismatches.push(Mismatch {
                            category: "segment_vmaddr".into(),
                            expected: format!("{name} vmaddr={vmaddr:#x}"),
                            observed: format!("{name} vmaddr={:#x}", obs.vmaddr),
                        });
                    }
                    if obs.vmsize != *vmsize {
                        mismatches.push(Mismatch {
                            category: "segment_vmsize".into(),
                            expected: format!("{name} vmsize={vmsize:#x}"),
                            observed: format!("{name} vmsize={:#x}", obs.vmsize),
                        });
                    }
                }
            } else {
                mismatches.push(Mismatch {
                    category: "segment".into(),
                    expected: format!("segment {name}"),
                    observed: "(not found in host)".into(),
                });
            }
        }

        for obs in observed {
            if !planned.iter().any(|(n, _, _)| n == &obs.name) {
                warnings.push(format!("host has segment not in plan: {}", obs.name));
            }
        }

        Self {
            matches,
            mismatches,
            warnings,
        }
    }

    /// Full comparison: dylibs + segments + entry point.
    pub fn full_compare(
        planned_dylibs: &[String],
        observed_dylibs: &[String],
        planned_segments: &[(String, u64, u64)],
        observed_segments: &[ObservedSegment],
        planned_entry: Option<u64>,
        observed_entry: Option<u64>,
    ) -> Self {
        let dylib_report = Self::compare_dylibs(planned_dylibs, observed_dylibs);
        let seg_report = Self::compare_segments(planned_segments, observed_segments);

        let mut matches = dylib_report.matches;
        matches.extend(seg_report.matches);

        let mut mismatches = dylib_report.mismatches;
        mismatches.extend(seg_report.mismatches);

        let mut warnings = dylib_report.warnings;
        warnings.extend(seg_report.warnings);

        // Compare entry point
        match (planned_entry, observed_entry) {
            (Some(pe), Some(oe)) if pe == oe => {
                matches.push(format!("entry point matches: {pe:#x}"));
            }
            (Some(pe), Some(oe)) => {
                mismatches.push(Mismatch {
                    category: "entry_point".into(),
                    expected: format!("{pe:#x}"),
                    observed: format!("{oe:#x}"),
                });
            }
            (Some(pe), None) => {
                warnings.push(format!("entry point {pe:#x} not observed from host"));
            }
            _ => {}
        }

        Self {
            matches,
            mismatches,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── compare_dylibs ─────────────────────────────────────────

    #[test]
    fn compare_dylibs_empty_inputs() {
        let report = DiffReport::compare_dylibs(&[], &[]);
        assert!(report.is_clean());
        assert!(report.matches.is_empty());
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn compare_dylibs_all_match() {
        let planned = vec!["libSystem".to_string(), "libc++".to_string()];
        let observed = vec![
            "/usr/lib/libSystem.B.dylib".to_string(),
            "/usr/lib/libc++.1.dylib".to_string(),
        ];
        let report = DiffReport::compare_dylibs(&planned, &observed);
        assert!(report.is_clean());
        assert_eq!(report.matches.len(), 2);
    }

    #[test]
    fn compare_dylibs_missing_from_host() {
        let planned = vec!["libFoo".to_string()];
        let observed: Vec<String> = Vec::new();
        let report = DiffReport::compare_dylibs(&planned, &observed);
        assert!(!report.is_clean());
        assert_eq!(report.mismatches.len(), 1);
        assert_eq!(report.mismatches[0].category, "dylib");
    }

    #[test]
    fn compare_dylibs_extra_on_host() {
        let planned: Vec<String> = Vec::new();
        let observed = vec!["/usr/lib/libExtra.dylib".to_string()];
        let report = DiffReport::compare_dylibs(&planned, &observed);
        assert!(report.is_clean()); // extras are warnings, not mismatches
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("libExtra"));
    }

    // ── compare_segments ───────────────────────────────────────

    #[test]
    fn compare_segments_empty_inputs() {
        let report = DiffReport::compare_segments(&[], &[]);
        assert!(report.is_clean());
        assert!(report.matches.is_empty());
    }

    #[test]
    fn compare_segments_exact_match() {
        let planned = vec![("__TEXT".to_string(), 0x1000u64, 0x4000u64)];
        let observed = vec![ObservedSegment {
            name: "__TEXT".to_string(),
            vmaddr: 0x1000,
            vmsize: 0x4000,
        }];
        let report = DiffReport::compare_segments(&planned, &observed);
        assert!(report.is_clean());
        assert_eq!(report.matches.len(), 1);
    }

    #[test]
    fn compare_segments_vmaddr_mismatch() {
        let planned = vec![("__TEXT".to_string(), 0x1000u64, 0x4000u64)];
        let observed = vec![ObservedSegment {
            name: "__TEXT".to_string(),
            vmaddr: 0x2000,
            vmsize: 0x4000,
        }];
        let report = DiffReport::compare_segments(&planned, &observed);
        assert!(!report.is_clean());
        assert!(report
            .mismatches
            .iter()
            .any(|m| m.category == "segment_vmaddr"));
    }

    #[test]
    fn compare_segments_vmsize_mismatch() {
        let planned = vec![("__DATA".to_string(), 0x5000u64, 0x1000u64)];
        let observed = vec![ObservedSegment {
            name: "__DATA".to_string(),
            vmaddr: 0x5000,
            vmsize: 0x2000,
        }];
        let report = DiffReport::compare_segments(&planned, &observed);
        assert!(!report.is_clean());
        assert!(report
            .mismatches
            .iter()
            .any(|m| m.category == "segment_vmsize"));
    }

    #[test]
    fn compare_segments_missing_from_host() {
        let planned = vec![("__LINKEDIT".to_string(), 0x8000u64, 0x1000u64)];
        let observed: Vec<ObservedSegment> = Vec::new();
        let report = DiffReport::compare_segments(&planned, &observed);
        assert!(!report.is_clean());
        assert_eq!(report.mismatches[0].category, "segment");
    }

    #[test]
    fn compare_segments_extra_on_host() {
        let planned: Vec<(String, u64, u64)> = Vec::new();
        let observed = vec![ObservedSegment {
            name: "__EXTRA".to_string(),
            vmaddr: 0xA000,
            vmsize: 0x1000,
        }];
        let report = DiffReport::compare_segments(&planned, &observed);
        assert!(report.is_clean());
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("__EXTRA"));
    }

    // ── full_compare ───────────────────────────────────────────

    #[test]
    fn full_compare_all_clean() {
        let planned_dylibs = vec!["libSystem".to_string()];
        let observed_dylibs = vec!["/usr/lib/libSystem.B.dylib".to_string()];
        let planned_segments = vec![("__TEXT".to_string(), 0x1000u64, 0x4000u64)];
        let observed_segments = vec![ObservedSegment {
            name: "__TEXT".to_string(),
            vmaddr: 0x1000,
            vmsize: 0x4000,
        }];
        let report = DiffReport::full_compare(
            &planned_dylibs,
            &observed_dylibs,
            &planned_segments,
            &observed_segments,
            Some(0x1234),
            Some(0x1234),
        );
        assert!(report.is_clean());
        assert!(report
            .matches
            .iter()
            .any(|m| m.contains("entry point matches")));
    }

    #[test]
    fn full_compare_entry_mismatch() {
        let report = DiffReport::full_compare(&[], &[], &[], &[], Some(0x100), Some(0x200));
        assert!(!report.is_clean());
        assert!(report
            .mismatches
            .iter()
            .any(|m| m.category == "entry_point"));
    }

    #[test]
    fn full_compare_entry_not_observed() {
        let report = DiffReport::full_compare(&[], &[], &[], &[], Some(0x100), None);
        assert!(report.is_clean()); // warning, not mismatch
        assert!(report.warnings.iter().any(|w| w.contains("not observed")));
    }

    #[test]
    fn full_compare_both_entries_none() {
        let report = DiffReport::full_compare(&[], &[], &[], &[], None, None);
        assert!(report.is_clean());
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn full_compare_composite_dylib_and_segment_issues() {
        let planned_dylibs = vec!["libFoo".to_string()];
        let observed_dylibs: Vec<String> = Vec::new();
        let planned_segments = vec![("__TEXT".to_string(), 0x1000u64, 0x4000u64)];
        let observed_segments = vec![ObservedSegment {
            name: "__TEXT".to_string(),
            vmaddr: 0x2000,
            vmsize: 0x4000,
        }];
        let report = DiffReport::full_compare(
            &planned_dylibs,
            &observed_dylibs,
            &planned_segments,
            &observed_segments,
            None,
            None,
        );
        assert!(!report.is_clean());
        // Should have both dylib and segment mismatches
        assert!(report.mismatches.iter().any(|m| m.category == "dylib"));
        assert!(report
            .mismatches
            .iter()
            .any(|m| m.category == "segment_vmaddr"));
    }

    // ── DiffReport helpers ─────────────────────────────────────

    #[test]
    fn is_clean_true_when_no_mismatches() {
        let report = DiffReport {
            matches: vec!["ok".into()],
            mismatches: Vec::new(),
            warnings: vec!["warn".into()],
        };
        assert!(report.is_clean());
    }

    #[test]
    fn is_clean_false_when_mismatches_present() {
        let report = DiffReport {
            matches: Vec::new(),
            mismatches: vec![Mismatch {
                category: "test".into(),
                expected: "a".into(),
                observed: "b".into(),
            }],
            warnings: Vec::new(),
        };
        assert!(!report.is_clean());
    }
}
