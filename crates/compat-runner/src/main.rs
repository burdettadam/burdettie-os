//! `darwin-compat` CLI — compatibility testing tool.
//!
//! Commands:
//!   scan <dir>       Scan a corpus directory and produce a report.
//!   run <binary>     Run a single binary and print its result.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use compat_runner::{discover_corpus, generate_report, run_binary, CorpusEntry};

#[derive(Parser)]
#[command(name = "darwin-compat", about = "Darwin runtime compatibility testing")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan a corpus directory and produce a compatibility report.
    Scan {
        /// Path to the corpus directory.
        dir: PathBuf,
        /// Write JSON report to this file (default: stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Run a single binary and print its result.
    Run {
        /// Path to the Mach-O binary.
        binary: PathBuf,
        /// Category tag for the binary.
        #[arg(short, long, default_value = "unknown")]
        category: String,
        /// Expected exit code.
        #[arg(short, long, default_value = "0")]
        expected_exit: i32,
    },
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Command::Scan { dir, output } => {
            let entries = discover_corpus(&dir);
            if entries.is_empty() {
                eprintln!("No corpus entries found in {}", dir.display());
                std::process::exit(1);
            }

            eprintln!("Scanning {} binaries...", entries.len());
            let report = generate_report(&entries);

            // Print summary table
            print_summary_table(&report);

            // Write JSON
            let json = serde_json::to_string_pretty(&report).expect("JSON serialization failed");
            if let Some(out_path) = output {
                std::fs::write(&out_path, &json).expect("Failed to write report");
                eprintln!("\nReport written to {}", out_path.display());
            } else {
                println!("{json}");
            }
        }
        Command::Run {
            binary,
            category,
            expected_exit,
        } => {
            let name = binary
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let entry = CorpusEntry {
                name,
                path: binary,
                category,
                expected_exit,
            };

            let result = run_binary(&entry);
            let json =
                serde_json::to_string_pretty(&result).expect("JSON serialization failed");
            println!("{json}");

            if !result.success {
                std::process::exit(1);
            }
        }
    }
}

fn print_summary_table(report: &compat_runner::CompatibilityReport) {
    eprintln!();
    eprintln!(
        "╔═══════════════════════════════╦═════════╦══════════╦════════╗"
    );
    eprintln!(
        "║ Binary                        ║ Status  ║ Exit     ║ Gaps   ║"
    );
    eprintln!(
        "╠═══════════════════════════════╬═════════╬══════════╬════════╣"
    );

    for b in &report.binaries {
        let status = if b.success { "  PASS " } else { "  FAIL " };
        let exit = if let Some(code) = b.exit_code {
            format!(" {:>3}/{:<3} ", code, b.expected_exit)
        } else {
            "  -/- ".to_string() + "  "
        };
        let gaps = b.missing_symbols.len() + b.missing_classes.len() + b.missing_selectors.len();
        let gap_str = if gaps > 0 {
            format!(" {:>4} ", gaps)
        } else {
            "    0 ".to_string()
        };

        eprintln!(
            "║ {:<29} ║{}║{}║{}║",
            truncate(&b.name, 29),
            status,
            exit,
            gap_str,
        );
    }

    eprintln!(
        "╠═══════════════════════════════╬═════════╬══════════╬════════╣"
    );
    eprintln!(
        "║ TOTAL: {:>3}/{:<3} ({:.0}%)            ║         ║          ║        ║",
        report.summary.passed,
        report.summary.total,
        report.summary.pass_rate,
    );
    eprintln!(
        "╚═══════════════════════════════╩═════════╩══════════╩════════╝"
    );
    eprintln!();
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
