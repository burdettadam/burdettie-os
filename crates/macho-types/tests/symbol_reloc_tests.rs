//! Tests for SymbolType, DylibRef version encoding, EntryPoint,
//! symbol descriptor flags, and entitlement key extraction.
#![allow(clippy::identity_op)]

use macho_types::symbol::*;

// ── SymbolType (from nlist.h N_TYPE mask) ───────────────────────

#[test]
fn symbol_type_from_ntype_undefined() {
    assert_eq!(SymbolType::from_ntype(0x00), SymbolType::Undefined);
    // External undefined: N_EXT set
    assert_eq!(SymbolType::from_ntype(0x01), SymbolType::Undefined);
    assert!(SymbolType::is_external(0x01));
}

#[test]
fn symbol_type_from_ntype_defined() {
    // N_SECT = 0x0E
    assert_eq!(SymbolType::from_ntype(0x0E), SymbolType::Defined);
    // External defined: N_SECT | N_EXT = 0x0F
    assert_eq!(SymbolType::from_ntype(0x0F), SymbolType::Defined);
    assert!(SymbolType::is_external(0x0F));
}

#[test]
fn symbol_type_from_ntype_absolute() {
    // N_ABS = 0x02
    assert_eq!(SymbolType::from_ntype(0x02), SymbolType::Absolute);
}

#[test]
fn symbol_type_from_ntype_indirect() {
    // N_INDR = 0x0A
    assert_eq!(SymbolType::from_ntype(0x0A), SymbolType::Indirect);
}

#[test]
fn symbol_type_from_ntype_prebound() {
    // N_PBUD = 0x0C
    assert_eq!(SymbolType::from_ntype(0x0C), SymbolType::PreboundUndefined);
}

#[test]
fn symbol_type_external_flag() {
    assert!(!SymbolType::is_external(0x00)); // plain undefined
    assert!(SymbolType::is_external(0x01)); // N_EXT
    assert!(!SymbolType::is_external(0x0E)); // defined, not external
    assert!(SymbolType::is_external(0x0F)); // defined + external
}

// ── DylibRef version encoding (Apple packed version format) ─────

mod dylib_tests {
    use macho_types::dylib::*;

    #[test]
    fn dylib_version_decode_1_0_0() {
        // 1.0.0 = (1 << 16) | (0 << 8) | 0 = 0x00010000
        let (maj, min, pat) = DylibRef::decode_version(0x0001_0000);
        assert_eq!((maj, min, pat), (1, 0, 0));
    }

    #[test]
    fn dylib_version_decode_complex() {
        // 1311.100.2 = (1311 << 16) | (100 << 8) | 2
        let encoded = (1311u32 << 16) | (100u32 << 8) | 2;
        let (maj, min, pat) = DylibRef::decode_version(encoded);
        assert_eq!((maj, min, pat), (1311, 100, 2));
    }

    #[test]
    fn dylib_version_string() {
        let dref = DylibRef {
            name: "/usr/lib/libSystem.B.dylib".into(),
            compatibility_version: 0x0001_0000, // 1.0.0
            current_version: (1356u32 << 16),   // 1356.0.0
            timestamp: 2,
        };
        assert_eq!(dref.compat_version_string(), "1.0.0");
        assert_eq!(dref.current_version_string(), "1356.0.0");
    }

    #[test]
    fn dylib_version_decode_zero() {
        let (maj, min, pat) = DylibRef::decode_version(0);
        assert_eq!((maj, min, pat), (0, 0, 0));
    }
}

// ── EntryPoint ──────────────────────────────────────────────────

mod entry_tests {
    use macho_types::entry::*;

    #[test]
    fn entry_point_lc_main() {
        let e = EntryPoint::Main {
            entry_offset: 0x1000,
            stack_size: 0,
        };
        match e {
            EntryPoint::Main {
                entry_offset,
                stack_size,
            } => {
                assert_eq!(entry_offset, 0x1000);
                assert_eq!(stack_size, 0);
            }
            _ => panic!("expected Main variant"),
        }
    }

    #[test]
    fn entry_point_unix_thread() {
        let e = EntryPoint::UnixThread {
            entry_address: 0x100000000,
        };
        match e {
            EntryPoint::UnixThread { entry_address } => {
                assert_eq!(entry_address, 0x100000000);
            }
            _ => panic!("expected UnixThread variant"),
        }
    }
}

// ── Symbol desc accessors ───────────────────────────────────────

mod symbol_desc_tests {
    use macho_types::symbol::{is_stab, Symbol, SymbolType};

    fn make_sym(desc: u16) -> Symbol {
        Symbol {
            name: "test".into(),
            sym_type: SymbolType::Defined,
            section_index: 1,
            desc,
            value: 0,
        }
    }

    #[test]
    fn weak_ref_flag() {
        assert!(make_sym(0x0040).is_weak_ref());
        assert!(!make_sym(0x0000).is_weak_ref());
    }

    #[test]
    fn weak_def_flag() {
        assert!(make_sym(0x0080).is_weak_def());
        assert!(!make_sym(0x0000).is_weak_def());
    }

    #[test]
    fn no_dead_strip_flag() {
        assert!(make_sym(0x0020).is_no_dead_strip());
        assert!(!make_sym(0x0000).is_no_dead_strip());
    }

    #[test]
    fn referenced_dynamically_flag() {
        assert!(make_sym(0x0010).is_referenced_dynamically());
        assert!(!make_sym(0x0000).is_referenced_dynamically());
    }

    #[test]
    fn combined_flags() {
        let sym = make_sym(0x00C0); // WEAK_REF | WEAK_DEF
        assert!(sym.is_weak_ref());
        assert!(sym.is_weak_def());
        assert!(!sym.is_no_dead_strip());
    }

    #[test]
    fn library_ordinal_positive() {
        let sym = make_sym(0x0300); // ordinal = 3
        assert_eq!(sym.library_ordinal(), 3);
    }

    #[test]
    fn library_ordinal_negative() {
        // Ordinal 0xFF = -1 (main executable)
        let sym = make_sym(0xFF00);
        assert_eq!(sym.library_ordinal(), -1);
    }

    #[test]
    fn library_ordinal_flat_lookup() {
        // Ordinal 0xFE = -2 (flat lookup)
        let sym = make_sym(0xFE00);
        assert_eq!(sym.library_ordinal(), -2);
    }

    #[test]
    fn stab_detection() {
        assert!(is_stab(0x20)); // N_GSYM
        assert!(is_stab(0x64)); // N_SO
        assert!(is_stab(0xE0)); // high bits
        assert!(!is_stab(0x0E)); // N_SECT
        assert!(!is_stab(0x01)); // N_EXT
        assert!(!is_stab(0x0F)); // N_SECT | N_EXT
        assert!(!is_stab(0x00)); // undefined
    }
}

// ── Entitlement key extraction ──────────────────────────────────

mod entitlement_tests {
    #[test]
    fn entitlement_keys_empty_when_no_plist() {
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: None,
            has_cms_signature: false,
        };
        assert!(cs.entitlement_keys().is_empty());
    }

    #[test]
    fn entitlement_keys_extracts_true() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
</dict>
</plist>"#;
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: Some(xml.to_string()),
            has_cms_signature: false,
        };
        let keys = cs.entitlement_keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].0, "com.apple.security.cs.allow-jit");
        assert!(keys[0].1);
    }

    #[test]
    fn entitlement_keys_extracts_false() {
        let xml = r#"<plist version="1.0"><dict>
    <key>com.apple.security.cs.disable-library-validation</key>
    <false/>
</dict></plist>"#;
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: Some(xml.to_string()),
            has_cms_signature: false,
        };
        let keys = cs.entitlement_keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(
            keys[0].0,
            "com.apple.security.cs.disable-library-validation"
        );
        assert!(!keys[0].1);
    }

    #[test]
    fn entitlement_keys_multiple() {
        let xml = r#"<plist version="1.0"><dict>
    <key>com.apple.security.get-task-allow</key>
    <true/>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <false/>
</dict></plist>"#;
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: Some(xml.to_string()),
            has_cms_signature: false,
        };
        let keys = cs.entitlement_keys();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].0, "com.apple.security.get-task-allow");
        assert!(keys[0].1);
        assert_eq!(keys[1].0, "com.apple.security.cs.allow-jit");
        assert!(keys[1].1);
        assert_eq!(
            keys[2].0,
            "com.apple.security.cs.disable-library-validation"
        );
        assert!(!keys[2].1);
    }

    #[test]
    fn entitlement_keys_non_boolean_treated_as_true() {
        let xml = r#"<plist version="1.0"><dict>
    <key>com.apple.developer.team-identifier</key>
    <string>ABCDE12345</string>
</dict></plist>"#;
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: Some(xml.to_string()),
            has_cms_signature: false,
        };
        let keys = cs.entitlement_keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].0, "com.apple.developer.team-identifier");
        assert!(keys[0].1); // non-boolean defaults to true (present)
    }

    #[test]
    fn entitlement_keys_empty_dict() {
        let xml = r#"<plist version="1.0"><dict></dict></plist>"#;
        let cs = macho_types::CodeSignature {
            super_blob: macho_types::SuperBlob {
                magic: 0,
                length: 0,
                count: 0,
                index: vec![],
            },
            code_directory: None,
            entitlements_plist: Some(xml.to_string()),
            has_cms_signature: false,
        };
        assert!(cs.entitlement_keys().is_empty());
    }
}
