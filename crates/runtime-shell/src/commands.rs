//! Shell command implementations.

use runtime_demo::{demo_binary_path, demos, find_demo, DemoCategory};

use crate::scorecard;
use crate::shell::{PolicyMode, Session};

/// Display help text.
pub fn help() {
    println!("Commands:");
    println!("  list                List all available demos");
    println!("  run <name>          Run a demo by name");
    println!("  score               Display compatibility scorecard");
    println!("  gui [app]           Launch a GUI demo app");
    println!("  trace on|off        Toggle runtime event tracing");
    println!("  policy perm|secure  Toggle security policy mode");
    println!("  info                Show current session settings");
    println!("  banner              Reprint the boot banner");
    println!("  help                Show this help");
    println!("  exit                Quit the shell");
}

/// List all available demos, grouped by category.
pub fn list() {
    let all = demos();
    let categories = [
        DemoCategory::Basic,
        DemoCategory::DynamicLinking,
        DemoCategory::ObjC,
        DemoCategory::Foundation,
        DemoCategory::Exceptions,
        DemoCategory::Integration,
    ];

    for cat in &categories {
        let in_cat: Vec<_> = all.iter().filter(|d| d.category == *cat).collect();
        if in_cat.is_empty() {
            continue;
        }

        println!("  ── {} ──", cat);
        for demo in &in_cat {
            let exit_note = if demo.expected_exit != 0 {
                format!(" (exit {})", demo.expected_exit)
            } else {
                String::new()
            };
            println!("    {:<16} {}{}", demo.name, demo.description, exit_note);
        }
        println!();
    }

    println!("  Run a demo: run <name>");
}

/// Run a demo by name.
pub fn run_demo(args: &[&str], session: &mut Session) {
    if args.is_empty() {
        println!("Usage: run <demo-name>");
        println!("  Use 'list' to see available demos.");
        return;
    }

    let name = args[0];

    let Some(demo) = find_demo(name) else {
        println!("Unknown demo: {name}");
        println!("Use 'list' to see available demos.");
        return;
    };

    let binary = demo_binary_path(&demo, &session.fixtures_root);
    if !binary.is_file() {
        println!("Fixture binary not found: {}", binary.display());
        println!("Rebuild fixtures with: ./scripts/build_fixtures.sh");
        return;
    }

    println!("▶ Running: {} ({})", demo.name, demo.description);
    println!("  Binary:  {}", binary.display());
    println!("  Policy:  {}", session.policy_mode);
    if session.trace_enabled {
        println!("  Tracing: enabled");
    }
    println!();

    // Build config
    let mut config = match session.policy_mode {
        PolicyMode::Permissive => macho_runtime::RuntimeConfig::permissive(&binary),
        PolicyMode::Secure => macho_runtime::RuntimeConfig::secure(&binary),
    };

    if session.trace_enabled {
        config.tracer = Some(macho_runtime::RuntimeTracer::new());
    }

    // Run under lock
    let _lock = session
        .runtime_lock
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    match macho_runtime::Runtime::load_and_run(&binary, &config) {
        Ok(result) => {
            println!("─── Output ───────────────────────────────────────────");
            println!();
            println!("─── Result ───────────────────────────────────────────");
            println!("  Exit code:     {}", result.exit_code);
            println!("  Expected:      {}", demo.expected_exit);
            println!(
                "  Status:        {}",
                if result.exit_code == demo.expected_exit {
                    "✓ PASS"
                } else {
                    "✗ FAIL (unexpected exit code)"
                }
            );
            println!("  Images loaded: {}", result.images_loaded);
            println!("  Shims:         {}", result.shims_installed);
            println!("  Initializers:  {}", result.initializers_run);

            if let Some(report) = &result.security_report {
                println!(
                    "  Security:      {}",
                    if report.allowed { "allowed" } else { "denied" }
                );
            }

            // Print trace summary if enabled
            if let Some(tracer) = &result.tracer {
                let events = tracer.events();
                println!("  Trace events:  {}", events.len());
                // Show last few events
                let tail: Vec<_> = events.iter().rev().take(5).collect();
                for evt in tail.iter().rev() {
                    println!("    [{:>14}] {}", evt.kind, evt.message);
                }
                if events.len() > 5 {
                    println!("    ... ({} more events)", events.len() - 5);
                }
            }
        }
        Err(e) => {
            println!("─── Error ────────────────────────────────────────────");
            println!("  {e}");
        }
    }
}

/// Display the compatibility scorecard.
pub fn scorecard(session: &Session) {
    scorecard::print_scorecard(&session.fixtures_root, &session.runtime_lock);
}

/// Available GUI demo apps.
const GUI_APPS: &[(&str, &str)] = &[
    (
        "hello-darwin",
        "Minimal lifecycle demo with fixture scorecard",
    ),
    (
        "control-gallery",
        "Widget exercise — text input, buttons, labels",
    ),
    (
        "foundation-explorer",
        "Browse registered Foundation classes and methods",
    ),
];

/// Launch a GUI demo app.
pub fn gui(args: &[&str], session: &Session) {
    if args.is_empty() {
        println!("GUI demo apps:");
        for (name, desc) in GUI_APPS {
            println!("  {name:<24} {desc}");
        }
        println!();
        println!("Launch: gui <app-name>");
        return;
    }

    let name = args[0];
    let Some((_, desc)) = GUI_APPS.iter().find(|(n, _)| *n == name) else {
        println!("Unknown GUI app: {name}");
        println!("Available apps:");
        for (n, d) in GUI_APPS {
            println!("  {n:<24} {d}");
        }
        return;
    };

    // Look for the binary relative to our own executable, then in PATH-like locations
    let binary = find_gui_binary(name, session);
    let Some(binary) = binary else {
        println!("GUI binary not found: {name}");
        println!("Build with: cargo build --workspace");
        return;
    };

    println!("▶ Launching: {name} — {desc}");
    println!("  Binary: {}", binary.display());
    println!();

    match std::process::Command::new(&binary).spawn() {
        Ok(mut child) => {
            println!("  GUI app launched (pid {})", child.id());
            println!("  Close the window or press Ctrl+C to return to shell.");
            match child.wait() {
                Ok(status) => {
                    println!("  App exited with: {status}");
                }
                Err(e) => {
                    println!("  Error waiting for app: {e}");
                }
            }
        }
        Err(e) => {
            println!("  Failed to launch: {e}");
        }
    }
}

/// Find the GUI binary by searching likely locations.
fn find_gui_binary(name: &str, session: &Session) -> Option<std::path::PathBuf> {
    // 1. Sibling of our own executable (deployment layout: bin/runtime-shell + bin/hello-darwin)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    // 2. In the dist/bin directory relative to fixtures root
    let dist_bin = session
        .fixtures_root
        .parent()
        .map(|p| p.join("bin").join(name));
    if let Some(ref p) = dist_bin {
        if p.is_file() {
            return Some(p.clone());
        }
    }

    // 3. Cargo target/debug (development)
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(workspace) = manifest.parent().and_then(|p| p.parent()) {
        let candidate = workspace.join("target/debug").join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        let candidate = workspace.join("target/release").join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

/// Toggle trace mode.
pub fn trace(args: &[&str], session: &mut Session) {
    match args.first().copied() {
        Some("on" | "1" | "true" | "yes") => {
            session.trace_enabled = true;
            println!("Tracing enabled. Events will be shown after each demo run.");
        }
        Some("off" | "0" | "false" | "no") => {
            session.trace_enabled = false;
            println!("Tracing disabled.");
        }
        _ => {
            println!(
                "Tracing is currently: {}",
                if session.trace_enabled { "on" } else { "off" }
            );
            println!("Usage: trace on|off");
        }
    }
}

/// Toggle security policy mode.
pub fn policy(args: &[&str], session: &mut Session) {
    match args.first().copied() {
        Some("perm" | "permissive") => {
            session.policy_mode = PolicyMode::Permissive;
            println!("Policy: permissive (no security checks)");
        }
        Some("secure" | "apple" | "strict") => {
            session.policy_mode = PolicyMode::Secure;
            println!("Policy: secure (Apple-default code signing checks)");
        }
        _ => {
            println!("Policy is currently: {}", session.policy_mode);
            println!("Usage: policy permissive|secure");
        }
    }
}

/// Show current session info.
pub fn info(session: &Session) {
    println!("Session:");
    println!("  Fixtures:  {}", session.fixtures_root.display());
    println!("  Policy:    {}", session.policy_mode);
    println!(
        "  Tracing:   {}",
        if session.trace_enabled { "on" } else { "off" }
    );

    let fixture_count = std::fs::read_dir(&session.fixtures_root)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().is_dir())
                .count()
        })
        .unwrap_or(0);
    println!("  Fixture dirs: {fixture_count}");
    println!("  Demos:     {}", runtime_demo::demos().len());
}
