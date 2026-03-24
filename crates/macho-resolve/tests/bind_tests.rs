//! Tests for macho-resolve::bind — two-level namespace symbol binding.
//!
//! Validates that undefined symbols in Mach-O binaries are correctly
//! resolved against the export tries of their target dylibs.

#[cfg(test)]
mod tests {
    use macho_parse::parse;
    use macho_resolve::{bind_symbols, BindingReport, ResolutionPlan, SymbolBinding};
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

    // ── simple_cli binding ──────────────────────────────────────────

    #[test]
    fn simple_cli_binds_printf() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let resolution =
            ResolutionPlan::from_parsed_transitive(&parsed, fixtures_dir().join("simple_cli/app"));
        let report = bind_symbols(&data, &resolution);

        // simple_cli imports _printf from libSystem
        let printf_binding = report.bindings.iter().find(|b| b.name == "_printf");
        assert!(
            printf_binding.is_some(),
            "should have a binding for _printf"
        );
    }

    #[test]
    fn simple_cli_binding_report_has_bindings() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let resolution =
            ResolutionPlan::from_parsed_transitive(&parsed, fixtures_dir().join("simple_cli/app"));
        let report = bind_symbols(&data, &resolution);

        assert!(
            !report.bindings.is_empty(),
            "simple_cli should have at least one binding"
        );
    }

    #[test]
    fn simple_cli_printf_ordinal_is_positive() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let resolution =
            ResolutionPlan::from_parsed_transitive(&parsed, fixtures_dir().join("simple_cli/app"));
        let report = bind_symbols(&data, &resolution);

        if let Some(b) = report.bindings.iter().find(|b| b.name == "_printf") {
            assert!(
                b.dylib_ordinal >= 1,
                "printf ordinal should be >= 1 (two-level namespace)"
            );
        }
    }

    // ── dylib_chain binding ─────────────────────────────────────────

    #[test]
    fn dylib_chain_binds_greet() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let resolution =
            ResolutionPlan::from_parsed_transitive(&parsed, fixtures_dir().join("dylib_chain/app"));
        let report = bind_symbols(&data, &resolution);

        // dylib_chain imports _liba_greet from libchaina
        let greet = report.bindings.iter().find(|b| b.name == "_liba_greet");
        assert!(greet.is_some(), "should have a binding for _liba_greet");
    }

    #[test]
    fn dylib_chain_binding_count() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let resolution =
            ResolutionPlan::from_parsed_transitive(&parsed, fixtures_dir().join("dylib_chain/app"));
        let report = bind_symbols(&data, &resolution);

        // At minimum: _liba_greet and _printf
        assert!(
            report.bindings.len() >= 2,
            "dylib_chain should have at least 2 bindings, got {}",
            report.bindings.len()
        );
    }

    // ── BindingReport API ───────────────────────────────────────────

    #[test]
    fn binding_report_is_complete_when_no_failures() {
        let report = BindingReport {
            bindings: vec![SymbolBinding {
                name: "_test".to_string(),
                dylib_ordinal: 1,
                resolved_in: Some("/usr/lib/libSystem.B.dylib".to_string()),
                address: Some(0x1000),
                is_weak: false,
                bound: true,
            }],
            failed: vec![],
            bound_count: 1,
            weak_unbound_count: 0,
        };
        assert!(report.is_complete());
    }

    #[test]
    fn binding_report_not_complete_with_failures() {
        let report = BindingReport {
            bindings: vec![SymbolBinding {
                name: "_missing".to_string(),
                dylib_ordinal: 1,
                resolved_in: None,
                address: None,
                is_weak: false,
                bound: false,
            }],
            failed: vec!["_missing".to_string()],
            bound_count: 0,
            weak_unbound_count: 0,
        };
        assert!(!report.is_complete());
    }

    #[test]
    fn binding_report_complete_with_weak_unbound() {
        let report = BindingReport {
            bindings: vec![SymbolBinding {
                name: "_optional".to_string(),
                dylib_ordinal: 1,
                resolved_in: None,
                address: None,
                is_weak: true,
                bound: false,
            }],
            failed: vec![],
            bound_count: 0,
            weak_unbound_count: 1,
        };
        assert!(
            report.is_complete(),
            "weak unbound symbols should not cause incomplete"
        );
    }

    // ── Serde ───────────────────────────────────────────────────────

    #[test]
    fn binding_report_serializes_to_json() {
        let report = BindingReport {
            bindings: vec![SymbolBinding {
                name: "_foo".to_string(),
                dylib_ordinal: 1,
                resolved_in: Some("/usr/lib/libSystem.B.dylib".to_string()),
                address: Some(0x1234),
                is_weak: false,
                bound: true,
            }],
            failed: vec![],
            bound_count: 1,
            weak_unbound_count: 0,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("_foo"));
        assert!(json.contains("libSystem"));

        let roundtrip: BindingReport = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.bound_count, 1);
        assert!(roundtrip.is_complete());
    }

    #[test]
    fn symbol_binding_serializes_roundtrip() {
        let binding = SymbolBinding {
            name: "_bar".to_string(),
            dylib_ordinal: 2,
            resolved_in: Some("/usr/lib/libc++.1.dylib".to_string()),
            address: Some(0x5678),
            is_weak: false,
            bound: true,
        };
        let json = serde_json::to_string(&binding).unwrap();
        let back: SymbolBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "_bar");
        assert_eq!(back.dylib_ordinal, 2);
        assert!(back.bound);
    }

    // ── constructor_test binding ────────────────────────────────────

    #[test]
    fn constructor_test_binds_printf() {
        let Some(data) = read_fixture("constructor_test/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let resolution = ResolutionPlan::from_parsed_transitive(
            &parsed,
            fixtures_dir().join("constructor_test/app"),
        );
        let report = bind_symbols(&data, &resolution);

        let printf_binding = report.bindings.iter().find(|b| b.name == "_printf");
        assert!(
            printf_binding.is_some(),
            "constructor_test should bind _printf"
        );
    }

    // ── no bindings from malformed data ─────────────────────────────

    #[test]
    fn malformed_returns_empty_report() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF]; // bad magic
        let parsed = parse(&data);
        assert!(parsed.is_err(), "bad magic should fail to parse");

        // bind_symbols handles parse failure gracefully
        let resolution = ResolutionPlan {
            graph: macho_resolve::DependencyGraph {
                nodes: vec![],
                edges: vec![],
            },
            unresolved: vec![],
            rpaths: vec![],
            transitive: false,
        };
        let report = bind_symbols(&data, &resolution);
        assert!(report.bindings.is_empty());
        assert!(report.is_complete());
    }
}
