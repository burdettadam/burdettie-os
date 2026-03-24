//! Tests for macho-diff: comparison engine.
//!
//! Validates that the diff engine correctly compares planned vs observed behavior.

#[cfg(test)]
mod tests {
    use macho_diff::{DiffReport, HostObservation, Mismatch};

    // ── Exact match ─────────────────────────────────────────────────

    #[test]
    fn exact_dylib_match() {
        let planned = vec!["/usr/lib/libSystem.B.dylib".to_string()];
        let observed = vec!["/usr/lib/libSystem.B.dylib".to_string()];
        let report = DiffReport::compare_dylibs(&planned, &observed);
        assert!(report.is_clean());
        assert_eq!(report.matches.len(), 1);
        assert!(report.mismatches.is_empty());
    }

    // ── Missing from observation ────────────────────────────────────

    #[test]
    fn missing_dylib_in_observation() {
        let planned = vec![
            "/usr/lib/libSystem.B.dylib".to_string(),
            "/usr/lib/libfoo.dylib".to_string(),
        ];
        let observed = vec!["/usr/lib/libSystem.B.dylib".to_string()];
        let report = DiffReport::compare_dylibs(&planned, &observed);
        assert!(!report.is_clean());
        assert_eq!(report.mismatches.len(), 1);
        assert!(report.mismatches[0].expected.contains("libfoo"));
    }

    // ── Extra in observation (host loaded more than planned) ────────

    #[test]
    fn extra_dylib_in_observation() {
        let planned = vec!["/usr/lib/libSystem.B.dylib".to_string()];
        let observed = vec![
            "/usr/lib/libSystem.B.dylib".to_string(),
            "/usr/lib/libc++.1.dylib".to_string(),
        ];
        let report = DiffReport::compare_dylibs(&planned, &observed);
        assert!(report.is_clean(), "extras are warnings, not mismatches");
        assert!(!report.warnings.is_empty());
        assert!(report.warnings[0].contains("libc++"));
    }

    // ── Empty planned vs empty observed ─────────────────────────────

    #[test]
    fn empty_comparison() {
        let report = DiffReport::compare_dylibs(&[], &[]);
        assert!(report.is_clean());
        assert!(report.matches.is_empty());
        assert!(report.mismatches.is_empty());
        assert!(report.warnings.is_empty());
    }

    // ── Partial name matching (e.g., basename match) ────────────────

    #[test]
    fn partial_name_match() {
        let planned = vec![
            "libSystem.B.dylib".to_string(), // basename only
        ];
        let observed = vec![
            "/usr/lib/libSystem.B.dylib".to_string(), // full path
        ];
        let report = DiffReport::compare_dylibs(&planned, &observed);
        // Should match because one contains the other
        assert!(report.is_clean());
    }

    // ── Multiple mismatches ─────────────────────────────────────────

    #[test]
    fn multiple_mismatches() {
        let planned = vec![
            "/usr/lib/libA.dylib".to_string(),
            "/usr/lib/libB.dylib".to_string(),
            "/usr/lib/libC.dylib".to_string(),
        ];
        let observed = vec![];
        let report = DiffReport::compare_dylibs(&planned, &observed);
        assert_eq!(report.mismatches.len(), 3);
    }

    // ── Serde roundtrip ─────────────────────────────────────────────

    #[test]
    fn diff_report_serializes() {
        let report = DiffReport {
            matches: vec!["libSystem found".into()],
            mismatches: vec![Mismatch {
                category: "dylib".into(),
                expected: "libFoo.dylib".into(),
                observed: "(not found)".into(),
            }],
            warnings: vec!["extra lib loaded".into()],
        };
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("libFoo"));
        assert!(json.contains("category"));
    }

    // ── HostObservation construction ────────────────────────────────

    #[test]
    fn host_observation_serde_roundtrip() {
        let obs = HostObservation {
            loaded_dylibs: vec!["/usr/lib/libSystem.B.dylib".into()],
            segments: vec![],
            entry_offset: Some(0x1000),
        };
        let json = serde_json::to_string(&obs).unwrap();
        let restored: HostObservation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.loaded_dylibs.len(), 1);
        assert_eq!(restored.entry_offset, Some(0x1000));
    }
}
