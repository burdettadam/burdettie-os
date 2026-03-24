//! Demo scenario descriptors for the runtime shell.
//!
//! Each demo maps a user-friendly name to a fixture binary, plus
//! a description and category for the `list` command.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A single runnable demo scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demo {
    /// Short identifier (used in `run <name>`).
    pub name: &'static str,
    /// Fixture directory name under `fixtures/`.
    pub fixture: &'static str,
    /// Human-readable one-line description.
    pub description: &'static str,
    /// Category for grouped listing.
    pub category: DemoCategory,
    /// Expected exit code.
    pub expected_exit: i32,
}

/// Demo categories for grouping in the `list` display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemoCategory {
    /// Basic C programs
    Basic,
    /// Dynamic library loading
    DynamicLinking,
    /// Objective-C runtime
    ObjC,
    /// Foundation framework
    Foundation,
    /// Exception handling
    Exceptions,
    /// Full integration
    Integration,
}

impl std::fmt::Display for DemoCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "Basic"),
            Self::DynamicLinking => write!(f, "Dynamic Linking"),
            Self::ObjC => write!(f, "Objective-C"),
            Self::Foundation => write!(f, "Foundation"),
            Self::Exceptions => write!(f, "Exceptions"),
            Self::Integration => write!(f, "Integration"),
        }
    }
}

/// All available demo scenarios.
pub fn demos() -> Vec<Demo> {
    vec![
        // ── Basic ──────────────────────────────────────────────
        Demo {
            name: "hello",
            fixture: "simple_cli",
            description: "Simple C program — prints a message and exits",
            category: DemoCategory::Basic,
            expected_exit: 0,
        },
        Demo {
            name: "return42",
            fixture: "return42",
            description: "Program that exits with code 42",
            category: DemoCategory::Basic,
            expected_exit: 42,
        },
        Demo {
            name: "constructor",
            fixture: "constructor_test",
            description: "Binary with __attribute__((constructor)) initializers",
            category: DemoCategory::Basic,
            expected_exit: 0,
        },
        Demo {
            name: "fat",
            fixture: "fat_binary",
            description: "Universal (fat) binary — arm64 slice extracted and run",
            category: DemoCategory::Basic,
            expected_exit: 0,
        },
        Demo {
            name: "process",
            fixture: "process_info",
            description: "Process identity — getpid, getprogname, argv[0]",
            category: DemoCategory::Basic,
            expected_exit: 0,
        },
        Demo {
            name: "tlv",
            fixture: "tlv_basic",
            description: "Thread-local variable storage",
            category: DemoCategory::Basic,
            expected_exit: 0,
        },
        // ── Dynamic Linking ────────────────────────────────────
        Demo {
            name: "dylib",
            fixture: "dylib_chain",
            description: "Three-level dylib dependency chain (app → liba → libb)",
            category: DemoCategory::DynamicLinking,
            expected_exit: 0,
        },
        Demo {
            name: "framework",
            fixture: "framework_style",
            description: "Framework bundle loading (.framework directory)",
            category: DemoCategory::DynamicLinking,
            expected_exit: 0,
        },
        Demo {
            name: "weak",
            fixture: "weak_dylib",
            description: "Weak-linked library (missing at runtime, handled gracefully)",
            category: DemoCategory::DynamicLinking,
            expected_exit: 0,
        },
        // ── Objective-C ────────────────────────────────────────
        Demo {
            name: "objc",
            fixture: "objc_classes",
            description: "ObjC class creation, alloc/init, ivar access, messaging",
            category: DemoCategory::ObjC,
            expected_exit: 0,
        },
        Demo {
            name: "objc-minimal",
            fixture: "objc_minimal",
            description: "Minimal ObjC — single class, single method",
            category: DemoCategory::ObjC,
            expected_exit: 0,
        },
        Demo {
            name: "objc-inherit",
            fixture: "objc_inherit",
            description: "ObjC class inheritance and method override",
            category: DemoCategory::ObjC,
            expected_exit: 0,
        },
        Demo {
            name: "objc-super",
            fixture: "objc_super",
            description: "ObjC super messaging — [super method] dispatch",
            category: DemoCategory::ObjC,
            expected_exit: 0,
        },
        Demo {
            name: "objc-category",
            fixture: "objc_category",
            description: "ObjC category methods merged into host class",
            category: DemoCategory::ObjC,
            expected_exit: 0,
        },
        // ── Foundation ─────────────────────────────────────────
        Demo {
            name: "foundation",
            fixture: "objc_foundation",
            description: "Foundation basics — NSString, NSArray, NSDictionary",
            category: DemoCategory::Foundation,
            expected_exit: 0,
        },
        Demo {
            name: "nsstring",
            fixture: "nsstring_ops",
            description: "NSString operations — length, compare, substring",
            category: DemoCategory::Foundation,
            expected_exit: 0,
        },
        Demo {
            name: "nsarray",
            fixture: "nsarray_basics",
            description: "NSArray — count, objectAtIndex:, containsObject:",
            category: DemoCategory::Foundation,
            expected_exit: 0,
        },
        Demo {
            name: "nsdict",
            fixture: "nsdictionary_basics",
            description: "NSDictionary — key lookup, allKeys, allValues",
            category: DemoCategory::Foundation,
            expected_exit: 0,
        },
        Demo {
            name: "nsnumber",
            fixture: "nsnumber_basics",
            description: "NSNumber — int/float boxing, comparisons",
            category: DemoCategory::Foundation,
            expected_exit: 0,
        },
        Demo {
            name: "nsdate",
            fixture: "nsdate_basics",
            description: "NSDate — time intervals, date comparison",
            category: DemoCategory::Foundation,
            expected_exit: 0,
        },
        Demo {
            name: "nsuuid",
            fixture: "nsuuid_basics",
            description: "NSUUID — generation and string formatting",
            category: DemoCategory::Foundation,
            expected_exit: 0,
        },
        // ── Exceptions ─────────────────────────────────────────
        Demo {
            name: "exceptions",
            fixture: "exception_basics",
            description: "@try/@catch/@finally exception handling",
            category: DemoCategory::Exceptions,
            expected_exit: 0,
        },
        Demo {
            name: "try-catch",
            fixture: "try_catch_test",
            description: "Exception throw + catch with NSException",
            category: DemoCategory::Exceptions,
            expected_exit: 0,
        },
        // ── Integration ────────────────────────────────────────
        Demo {
            name: "dispatch",
            fixture: "dispatch_test",
            description: "Grand Central Dispatch — dispatch_once, dispatch_once_f",
            category: DemoCategory::Integration,
            expected_exit: 0,
        },
        Demo {
            name: "blocks",
            fixture: "block_basics",
            description: "Block literals — creation, invocation, capture",
            category: DemoCategory::Integration,
            expected_exit: 0,
        },
        Demo {
            name: "autorelease",
            fixture: "autorelease",
            description: "Autorelease pool — @autoreleasepool { ... }",
            category: DemoCategory::Integration,
            expected_exit: 0,
        },
    ]
}

/// Look up a demo by name (case-insensitive).
pub fn find_demo(name: &str) -> Option<Demo> {
    let lower = name.to_lowercase();
    demos().into_iter().find(|d| d.name == lower)
}

/// Resolve the full binary path for a demo given the fixtures root.
/// Tries `app` first, then falls back to using the directory name as binary name.
pub fn demo_binary_path(demo: &Demo, fixtures_root: &Path) -> PathBuf {
    let app = fixtures_root.join(demo.fixture).join("app");
    if app.is_file() {
        return app;
    }
    // Fallback: binary named after the fixture directory
    let named = fixtures_root.join(demo.fixture).join(demo.fixture);
    if named.is_file() {
        return named;
    }
    // Return the `app` path regardless (caller checks existence)
    app
}

/// All fixture directory names for scorecard (excludes malformed).
pub fn all_fixture_names() -> Vec<&'static str> {
    vec![
        "autorelease",
        "block_basics",
        "blocks_foundation",
        "collection_enum",
        "constructor_test",
        "dispatch_test",
        "dylib_chain",
        "exception_basics",
        "fat_binary",
        "framework_style",
        "nsarray_basics",
        "nscalendar_basics",
        "nscharacterset_basics",
        "nsdata_basics",
        "nsdate_basics",
        "nsdateformatter_basics",
        "nsdictionary_basics",
        "nsindexset_basics",
        "nslocale_basics",
        "nslog_format_specifiers",
        "nsmutablearray_basics",
        "nsmutabledictionary_basics",
        "nsmutableset_basics",
        "nsnull_basics",
        "nsnumber_basics",
        "nsnumberformatter_basics",
        "nsobject_methods",
        "nsnotification_basics",
        "nsprocessinfo_basics",
        "nsset_basics",
        "nssortdescriptor_basics",
        "nsstring_dynamic",
        "nsstring_format",
        "nsstring_manip",
        "nsstring_mutable",
        "nsstring_ops",
        "nsstring_path",
        "nstimer_basics",
        "nstimezone_basics",
        "nsurl_basics",
        "nsuserdefaults_basics",
        "nsuuid_basics",
        "nsvalue_basics",
        "objc_category",
        "objc_classes",
        "objc_dylib_chain",
        "objc_foundation",
        "objc_inherit",
        "objc_minimal",
        "objc_super",
        "process_info",
        "property_introspection",
        "return42",
        "runtime_introspection",
        "simple_cli",
        "tlv_basic",
        "try_catch_test",
        "weak_dylib",
    ]
}
