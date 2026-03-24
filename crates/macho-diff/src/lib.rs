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
