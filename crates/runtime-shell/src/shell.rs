//! Shell session — REPL loop, banner, and prompt.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::commands;

/// Mutable session state shared across commands.
pub struct Session {
    /// Fixtures root directory.
    pub fixtures_root: PathBuf,
    /// Whether runtime tracing is enabled.
    pub trace_enabled: bool,
    /// Security policy mode.
    pub policy_mode: PolicyMode,
    /// Serialization lock for runtime calls (global ObjC state).
    pub runtime_lock: Mutex<()>,
}

/// Security policy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyMode {
    Permissive,
    Secure,
}

impl std::fmt::Display for PolicyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Permissive => write!(f, "permissive"),
            Self::Secure => write!(f, "secure"),
        }
    }
}

/// Resolve the fixtures directory.
fn find_fixtures_root() -> PathBuf {
    // Try env var first
    if let Ok(root) = std::env::var("DARWIN_LOADER_ROOT") {
        let p = PathBuf::from(root).join("fixtures");
        if p.is_dir() {
            return p;
        }
    }

    // Try relative to CARGO_MANIFEST_DIR (development)
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest.parent().and_then(|p| p.parent());
    if let Some(root) = workspace {
        let p = root.join("fixtures");
        if p.is_dir() {
            return p;
        }
    }

    // Try current directory
    let cwd = std::env::current_dir().unwrap_or_default();
    let p = cwd.join("fixtures");
    if p.is_dir() {
        return p;
    }

    // Fallback
    PathBuf::from("fixtures")
}

/// Print the boot banner.
fn print_banner(session: &Session) {
    let fixture_count = std::fs::read_dir(&session.fixtures_root)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().is_dir())
                .filter(|e| e.file_name().to_str().is_some_and(|n| n != "malformed"))
                .count()
        })
        .unwrap_or(0);

    let demo_count = runtime_demo::demos().len();

    println!();
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║           burdetty-os runtime shell            ║");
    println!("╠══════════════════════════════════════════════════════╣");
    println!("║                                                      ║");
    println!(
        "║  Version:    {}                                  ║",
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "║  Platform:   {} / {}          ║",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!(
        "║  Demos:      {:<3} scenarios                         ║",
        demo_count
    );
    println!(
        "║  Fixtures:   {:<3} binaries                          ║",
        fixture_count
    );
    println!(
        "║  Policy:     {:<11}                            ║",
        session.policy_mode
    );
    println!(
        "║  Tracing:    {:<3}                                    ║",
        if session.trace_enabled { "on" } else { "off" }
    );
    println!("║                                                      ║");
    println!("║  Foundation: 40 classes | Shims: 59 functions         ║");
    println!("║                                                      ║");
    println!("║  Type 'help' for commands, 'gui' for GUI demos       ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();
}

/// Run the interactive shell.
pub fn run() {
    let session = Session {
        fixtures_root: find_fixtures_root(),
        trace_enabled: false,
        policy_mode: PolicyMode::Permissive,
        runtime_lock: Mutex::new(()),
    };

    print_banner(&session);

    // Check fixtures exist
    if !session.fixtures_root.is_dir() {
        eprintln!(
            "Warning: fixtures directory not found at {}",
            session.fixtures_root.display()
        );
        eprintln!("Set DARWIN_LOADER_ROOT or run from the project root.\n");
    }

    let stdin = io::stdin();
    let mut session = session;

    loop {
        // Prompt
        print!("darwin> ");
        if io::stdout().flush().is_err() {
            break;
        }

        // Read line
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Err(_) => break,
            _ => {}
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse command
        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0].to_lowercase();
        let args = &parts[1..];

        match cmd.as_str() {
            "exit" | "quit" | "q" => {
                println!("Goodbye.");
                break;
            }
            "help" | "h" | "?" => commands::help(),
            "list" | "ls" => commands::list(),
            "run" | "r" => commands::run_demo(args, &mut session),
            "score" | "scorecard" => commands::scorecard(&session),
            "trace" => commands::trace(args, &mut session),
            "policy" => commands::policy(args, &mut session),
            "info" => commands::info(&session),
            "banner" => print_banner(&session),
            "gui" => commands::gui(args, &session),
            _ => {
                println!("Unknown command: {cmd}");
                println!("Type 'help' for available commands.");
            }
        }

        println!();
    }
}
