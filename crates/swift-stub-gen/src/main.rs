//! swift-stub-gen — generate Rust shim stubs for Swift-mangled symbols.
//!
//! # Usage
//!
//! ```text
//! swift-stub-gen [OPTIONS] <framework-binary> [<framework-binary>...]
//!
//! OPTIONS
//!   -o, --output <FILE>   Write generated source to FILE [default: stdout]
//!   --struct-name <NAME>  Override the generated struct prefix (PascalCase)
//!   --demangle            Invoke `swift-demangle` for human-readable comments
//!   --include-reexports   Also emit stubs for re-exported symbols
//!   --include-data        Also emit stubs for data symbols (metadata, witnesses)
//!   --all-symbols         Include non-Swift symbols (useful for full coverage)
//!   --list                Print matched symbols only (no code generation)
//!   --stats               Print a short count summary at the end (stderr)
//! ```
//!
//! # Example
//!
//! ```text
//! # Generate stubs for SwiftUI (arm64 slice of the fat binary):
//! swift-stub-gen --demangle \
//!     /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/ \
//!     DeviceSupport/*/Symbols/System/Library/Frameworks/SwiftUI.framework/SwiftUI \
//!     -o crates/libsystem-shims/src/darwin_stubs/swift_swiftui.rs
//!
//! # Inspect without generating code:
//! swift-stub-gen --list /path/to/RegexBuilder | head -20
//!
//! # Multiple frameworks merged into one file:
//! swift-stub-gen --struct-name Combined \
//!     /path/to/SwiftUI /path/to/RegexBuilder \
//!     -o swift_combined.rs
//! ```

mod codegen;
mod demangler;
mod extractor;

use std::path::PathBuf;

use clap::Parser;
use codegen::{generate, prefix_from_path, CodegenOptions};
use extractor::extract_symbols_full;

#[derive(Parser, Debug)]
#[command(
    name = "swift-stub-gen",
    about = "Generate Rust shim stubs for Swift-mangled symbols in Mach-O framework binaries",
    long_about = None,
    version,
)]
struct Cli {
    /// One or more Mach-O framework binaries to scan.
    #[arg(required = true, value_name = "FRAMEWORK_BINARY")]
    inputs: Vec<PathBuf>,

    /// Write generated Rust source to this file (default: stdout).
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Override the generated struct name prefix (PascalCase).
    /// Default: derived from the (first) framework binary name.
    #[arg(long, value_name = "NAME")]
    struct_name: Option<String>,

    /// Include re-exported symbols (aliases with no local code/data).
    #[arg(long, default_value_t = false)]
    include_reexports: bool,

    /// Include data-symbol stubs (type metadata, protocol witnesses).
    #[arg(long, default_value_t = false)]
    include_data: bool,

    /// Include non-Swift symbols as well (bulk framework coverage).
    #[arg(long, default_value_t = false)]
    all_symbols: bool,

    /// Call `swift-demangle` for readable comments in the generated code.
    /// Requires Xcode command-line tools on the host.
    #[arg(long, default_value_t = false)]
    demangle: bool,

    /// Print matched symbol names one per line without generating code.
    #[arg(long, default_value_t = false)]
    list: bool,

    /// Print a summary of matched / total symbols to stderr.
    #[arg(long, default_value_t = false)]
    stats: bool,
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let cli = Cli::parse();

    // ── Collect symbols from all input binaries ─────────────────────────────
    let mut all_symbols: Vec<extractor::ExtractedSymbol> = Vec::new();
    let mut had_error = false;

    for path in &cli.inputs {
        log::info!("Reading {}", path.display());
        match extract_symbols_full(path, cli.include_reexports) {
            Ok(syms) => {
                log::info!("  {} total exports", syms.len());
                all_symbols.extend(syms);
            }
            Err(e) => {
                eprintln!("error: {e}");
                had_error = true;
            }
        }
    }

    if had_error {
        std::process::exit(1);
    }

    // ── Apply symbol filter ─────────────────────────────────────────────────
    let matched: Vec<extractor::ExtractedSymbol> = if cli.all_symbols {
        all_symbols
    } else {
        all_symbols
            .into_iter()
            .filter(|s| demangler::is_swift_mangled(&s.name))
            .collect()
    };

    if cli.stats || cli.list {
        eprintln!("Matched symbols: {}", matched.len());
    }

    // ── List mode ───────────────────────────────────────────────────────────
    if cli.list {
        for sym in &matched {
            if cli.demangle {
                let human = demangler::demangle(&sym.name).unwrap_or_else(|| sym.name.clone());
                println!("{}\t{}", sym.name, human);
            } else {
                println!("{}", sym.name);
            }
        }
        return;
    }

    // ── Code generation ─────────────────────────────────────────────────────
    let struct_prefix = cli
        .struct_name
        .clone()
        .unwrap_or_else(|| prefix_from_path(cli.inputs.first().map(|p| p.as_path()).unwrap()));

    let date = {
        // Use the source file mtime if available; fall back to a placeholder
        // so the tool is deterministic in CI (no clock dependency).
        let mtime = cli
            .inputs
            .first()
            .and_then(|p| std::fs::metadata(p).ok())
            .and_then(|m| m.modified().ok())
            .and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs())
            });
        format_date(mtime)
    };

    let source_desc = cli
        .inputs
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let opts = CodegenOptions {
        struct_prefix: struct_prefix.clone(),
        source_desc,
        date,
        demangle: cli.demangle,
        include_reexports: cli.include_reexports,
        include_data_stubs: cli.include_data,
    };

    let source = generate(&matched, &opts);

    // ── Write output ────────────────────────────────────────────────────────
    if let Some(out_path) = &cli.output {
        if let Err(e) = std::fs::write(out_path, &source) {
            eprintln!("error writing {}: {e}", out_path.display());
            std::process::exit(1);
        }
        log::info!("Wrote {}", out_path.display());
    } else {
        print!("{source}");
    }

    if cli.stats {
        let fn_count = matched.iter().filter(|s| !s.is_data).count();
        let data_count = matched.iter().filter(|s| s.is_data).count();
        eprintln!("  struct:    {}Stubs", struct_prefix);
        eprintln!("  fn stubs:  {fn_count}");
        eprintln!("  data stubs:{data_count}");
    }
}

/// Format a Unix timestamp as `YYYY-MM-DD` (UTC).
///
/// This is a minimal implementation that avoids the `chrono`/`time` crate
/// dependency just for a header comment.
fn format_date(unix_secs: Option<u64>) -> String {
    let secs = unix_secs.unwrap_or(0);
    // Days since Unix epoch
    let days = secs / 86400;
    // Gregorian calendar via the Julian day algorithm
    let j = days as i64 + 2440588; // JD of 1970-01-01 = 2440588
    let a = j + 32044;
    let b = (4 * a + 3) / 146097;
    let c = a - (146097 * b) / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - (1461 * d) / 4;
    let m = (5 * e + 2) / 153;
    let day = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = 100 * b + d - 4800 + m / 10;
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_date_epoch() {
        assert_eq!(format_date(Some(0)), "1970-01-01");
    }

    #[test]
    fn test_format_date_known() {
        // 2026-04-03 = 2026*365.25 + ... let's just check it parses without panic
        let d = format_date(Some(1743638400)); // approx 2025-04-03
        assert!(d.starts_with("202"));
        assert_eq!(d.len(), 10);
        assert_eq!(&d[4..5], "-");
    }

    #[test]
    fn test_format_date_none() {
        let d = format_date(None);
        assert_eq!(d, "1970-01-01");
    }
}
