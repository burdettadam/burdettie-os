//! perf-bench — Performance matrix comparing loader overhead against Apple's native stack.
//!
//! Usage:
//!   cargo run -p perf-bench --release                    # benchmark all corpus tools
//!   cargo run -p perf-bench --release -- --warmup 3      # with warmup iterations
//!   cargo run -p perf-bench --release -- --json          # JSON output for CI
//!   cargo run -p perf-bench --release -- --filter cache   # filter by name
//!   cargo run -p perf-bench --release -- --iterations 5  # average over N runs

#![allow(clippy::print_stderr)]  // CLI tool uses stderr for progress output

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use perf_bench::matrix::{self, ToolBenchmark};
use perf_bench::timing;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let warmup = parse_flag(&args, "--warmup").unwrap_or(1);
    let iterations = parse_flag(&args, "--iterations").unwrap_or(3);
    let json_output = args.iter().any(|a| a == "--json");
    let filter: Option<&str> = args
        .iter()
        .position(|a| a == "--filter")
        .and_then(|i| args.get(i + 1))
        .map(String::as_str);

    let corpus = discover_corpus();
    if corpus.is_empty() {
        eprintln!("No corpus tools found. Run corpus/build_corpus.sh first.");
        std::process::exit(1);
    }

    let tools: Vec<(String, PathBuf)> = if let Some(pat) = filter {
        corpus
            .into_iter()
            .filter(|(name, _)| name.contains(pat))
            .collect()
    } else {
        corpus
    };

    eprintln!(
        "Benchmarking {} corpus tools ({warmup} warmup + {iterations} measured iterations)...",
        tools.len()
    );

    let mut benchmarks: Vec<ToolBenchmark> = Vec::new();

    for (name, binary) in &tools {
        eprint!("  {name:<24} ");

        // ── Native baseline (run binary directly) ───────────────────
        // Warmup native
        for _ in 0..warmup {
            let _ = run_native(binary);
        }
        // Measured native
        let mut native_durations = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            native_durations.push(run_native(binary));
        }
        let native_avg = average_duration(&native_durations);

        // ── Loader execution (through our pipeline) ─────────────────
        // Warmup loader
        for _ in 0..warmup {
            let _ = timing::timed_load_and_run(binary);
        }
        // Measured loader
        let mut loader_timings = Vec::new();
        let mut last_result = None;
        for _ in 0..iterations {
            match timing::timed_load_and_run(binary) {
                Ok((result, t)) => {
                    last_result = Some(result);
                    loader_timings.push(t);
                }
                Err(e) => {
                    eprintln!("ERROR: {e}");
                    break;
                }
            }
        }

        if loader_timings.is_empty() {
            eprintln!("skipped (loader failed)");
            continue;
        }

        let avg_timing = average_timing(&loader_timings);
        let Some(result) = last_result else { continue };
        let ratio = avg_timing.overhead_ratio(native_avg);

        let ratio_str = if ratio.is_infinite() {
            "∞x".to_string()
        } else {
            format!("{ratio:.1}x")
        };
        eprintln!(
            "native={:>8}  loader={:>8}  overhead={ratio_str}",
            fmt_dur(native_avg),
            fmt_dur(avg_timing.total),
        );

        benchmarks.push(ToolBenchmark {
            name: name.clone(),
            native_duration: native_avg,
            loader_timing: avg_timing,
            exit_code: result.exit_code,
            passed: result.exit_code == 0,
            images_loaded: result.images_loaded,
            shims_installed: result.shims_installed,
        });
    }

    if json_output {
        println!("{}", matrix::export_json(&benchmarks));
    } else {
        matrix::print_matrix(&benchmarks);
    }
}

/// Run a corpus binary natively and return wall-clock duration.
fn run_native(binary: &std::path::Path) -> Duration {
    let start = Instant::now();
    let _ = Command::new(binary)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    start.elapsed()
}

/// Discover corpus tool binaries (corpus/<name>/app).
fn discover_corpus() -> Vec<(String, PathBuf)> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("corpus"))
        .unwrap_or_else(|| PathBuf::from("corpus"));

    let mut tools = Vec::new();

    let Ok(entries) = std::fs::read_dir(&base) else {
        return tools;
    };

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Some(fname) = dir.file_name() else { continue };
        let name = fname.to_string_lossy().to_string();
        let bin = dir.join("app");
        if bin.is_file() {
            tools.push((name, bin));
        }
    }

    tools.sort_by(|a, b| a.0.cmp(&b.0));
    tools
}

fn parse_flag(args: &[String], flag: &str) -> Option<usize> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
}

fn average_duration(durations: &[Duration]) -> Duration {
    if durations.is_empty() {
        return Duration::ZERO;
    }
    let total: Duration = durations.iter().sum();
    #[allow(clippy::cast_possible_truncation)]
    let n = durations.len() as u32;
    total / n
}

fn average_timing(timings: &[timing::PhaseTiming]) -> timing::PhaseTiming {
    #[allow(clippy::cast_possible_truncation)]
    let n = timings.len() as u32;
    if n == 0 {
        return timing::PhaseTiming {
            total: Duration::ZERO,
            parse: Duration::ZERO,
            security: Duration::ZERO,
            map_images: Duration::ZERO,
            fixups: Duration::ZERO,
            objc_registration: Duration::ZERO,
            protect: Duration::ZERO,
            initialize: Duration::ZERO,
            execution: Duration::ZERO,
            map_images_breakdown: timing::PhaseBreakdown::default(),
            objc_registration_breakdown: timing::PhaseBreakdown::default(),
        };
    }
    timing::PhaseTiming {
        total: timings.iter().map(|t| t.total).sum::<Duration>() / n,
        parse: timings.iter().map(|t| t.parse).sum::<Duration>() / n,
        security: timings.iter().map(|t| t.security).sum::<Duration>() / n,
        map_images: timings.iter().map(|t| t.map_images).sum::<Duration>() / n,
        fixups: timings.iter().map(|t| t.fixups).sum::<Duration>() / n,
        objc_registration: timings.iter().map(|t| t.objc_registration).sum::<Duration>() / n,
        protect: timings.iter().map(|t| t.protect).sum::<Duration>() / n,
        initialize: timings.iter().map(|t| t.initialize).sum::<Duration>() / n,
        execution: timings.iter().map(|t| t.execution).sum::<Duration>() / n,
        map_images_breakdown: average_breakdown(timings.iter().map(|t| &t.map_images_breakdown)),
        objc_registration_breakdown: average_breakdown(
            timings.iter().map(|t| &t.objc_registration_breakdown),
        ),
    }
}

fn average_breakdown<'a>(
    breakdowns: impl Iterator<Item = &'a timing::PhaseBreakdown>,
) -> timing::PhaseBreakdown {
    let mut timing_sums: std::collections::BTreeMap<String, u128> = std::collections::BTreeMap::new();
    let mut counter_sums: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut count = 0u128;

    for breakdown in breakdowns {
        if breakdown.is_empty() {
            continue;
        }
        count += 1;
        for metric in &breakdown.timings {
            *timing_sums.entry(metric.name.clone()).or_default() += metric.micros;
        }
        for metric in &breakdown.counters {
            *counter_sums.entry(metric.name.clone()).or_default() += metric.value;
        }
    }

    if count == 0 {
        return timing::PhaseBreakdown::default();
    }

    let mut timings: Vec<timing::SubphaseTiming> = timing_sums
        .into_iter()
        .map(|(name, total)| timing::SubphaseTiming {
            name,
            micros: total / count,
        })
        .collect();
    timings.sort_by(|a, b| b.micros.cmp(&a.micros));

    let counters: Vec<timing::CounterMetric> = counter_sums
        .into_iter()
        .map(|(name, total)| timing::CounterMetric {
            name,
            value: total / count as u64,
        })
        .collect();

    timing::PhaseBreakdown { timings, counters }
}

fn fmt_dur(d: Duration) -> String {
    let us = d.as_micros();
    if us < 1_000 {
        format!("{us}µs")
    } else if us < 1_000_000 {
        format!("{:.2}ms", d.as_secs_f64() * 1_000.0)
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}
