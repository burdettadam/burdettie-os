//! Performance matrix formatting — tabular comparison output.

use std::time::Duration;

use crate::timing::{PhaseBreakdown, PhaseTiming};

/// Result of benchmarking a single corpus tool.
pub struct ToolBenchmark {
    pub name: String,
    pub native_duration: Duration,
    pub loader_timing: PhaseTiming,
    pub exit_code: i32,
    pub passed: bool,
    pub images_loaded: usize,
    pub shims_installed: usize,
}

/// Aggregate statistics across all benchmarks.
pub struct MatrixSummary {
    pub total_tools: usize,
    pub tools_passing: usize,
    pub avg_overhead: f64,
    pub max_overhead: (String, f64),
    pub min_overhead: (String, f64),
    pub global_bottleneck: String,
    pub phase_avg_pct: Vec<(&'static str, f64)>,
    pub avg_map_images_subphases: Vec<(String, f64)>,
    pub avg_objc_reg_subphases: Vec<(String, f64)>,
}

impl ToolBenchmark {
    pub fn overhead_ratio(&self) -> f64 {
        self.loader_timing.overhead_ratio(self.native_duration)
    }
}

/// Format a Duration for compact display (e.g., "1.23ms", "456µs", "2.10s").
fn fmt_duration(d: Duration) -> String {
    let us = d.as_micros();
    if us < 1_000 {
        format!("{us}µs")
    } else if us < 1_000_000 {
        format!("{:.2}ms", d.as_secs_f64() * 1_000.0)
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

/// Format a bar chart segment: █ for 10%, ▓ for 5%, spaces otherwise.
fn bar_chart(pct: f64, width: usize) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let filled = (((pct / 100.0) * width as f64).round() as usize).min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

/// Print the full performance matrix to stdout.
pub fn print_matrix(benchmarks: &[ToolBenchmark]) {
    if benchmarks.is_empty() {
        println!("No benchmarks to display.");
        return;
    }

    // ── Header ──────────────────────────────────────────────────────
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                    P E R F O R M A N C E   M A T R I X                                                        ║");
    println!("║                              darwin-loader vs Apple Native Stack                                                                ║");
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!();

    // ── Phase Breakdown Table ───────────────────────────────────────
    println!("┌──────────────────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬────────┐");
    println!("│ Tool                 │  Native  │  Parse   │ Security │ Map/Load │  Fixups  │ ObjC Reg │ Protect  │   Init   │   Exec   │  Total   │Overhead│");
    println!("├──────────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────┤");

    for b in benchmarks {
        let t = &b.loader_timing;
        let ratio = b.overhead_ratio();
        let ratio_str = if ratio.is_infinite() {
            "  ∞x".to_string()
        } else {
            format!("{ratio:>5.1}x")
        };

        println!(
            "│ {:<20} │ {:>8} │ {:>8} │ {:>8} │ {:>8} │ {:>8} │ {:>8} │ {:>8} │ {:>8} │ {:>8} │ {:>8} │{:>6} │",
            &b.name[..b.name.len().min(20)],
            fmt_duration(b.native_duration),
            fmt_duration(t.parse),
            fmt_duration(t.security),
            fmt_duration(t.map_images),
            fmt_duration(t.fixups),
            fmt_duration(t.objc_registration),
            fmt_duration(t.protect),
            fmt_duration(t.initialize),
            fmt_duration(t.execution),
            fmt_duration(t.total),
            ratio_str,
        );
    }

    println!("└──────────────────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴────────┘");

    // ── Overhead Ranking ────────────────────────────────────────────
    println!();
    println!("┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ OVERHEAD RANKING (worst → best)                                             │");
    println!("├──────────────────────┬─────────┬────────────────────────────────────────────┤");
    println!("│ Tool                 │Overhead │ Distribution                               │");
    println!("├──────────────────────┼─────────┼────────────────────────────────────────────┤");

    let mut sorted: Vec<&ToolBenchmark> = benchmarks.iter().collect();
    sorted.sort_by(|a, b| {
        b.overhead_ratio()
            .partial_cmp(&a.overhead_ratio())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let max_ratio: f64 = sorted
        .first()
        .map(|b| b.overhead_ratio())
        .unwrap_or(1.0)
        .min(1000.0); // cap for display

    for b in &sorted {
        let ratio = b.overhead_ratio();
        let ratio_str = if ratio.is_infinite() {
            "    ∞x".to_string()
        } else {
            format!("{ratio:>5.1}x")
        };
        let bar_pct = if max_ratio > 0.0 {
            (ratio.min(1000.0) / max_ratio) * 100.0
        } else {
            0.0
        };
        println!(
            "│ {:<20} │ {:>6}  │ {:>40} │",
            &b.name[..b.name.len().min(20)],
            ratio_str,
            bar_chart(bar_pct, 40),
        );
    }

    println!("└──────────────────────┴─────────┴────────────────────────────────────────────┘");

    // ── Bottleneck Analysis ─────────────────────────────────────────
    print_bottleneck_analysis(benchmarks);

    // ── Summary ─────────────────────────────────────────────────────
    let summary = compute_summary(benchmarks);
    println!();
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ SUMMARY                                                         │");
    println!("├──────────────────────────────────────────────────────────────────┤");
    println!(
        "│ Tools benchmarked:  {:>4}/{:<4}  passing                          │",
        summary.tools_passing, summary.total_tools
    );
    println!(
        "│ Average overhead:   {:>8.1}x                                     │",
        summary.avg_overhead
    );
    println!(
        "│ Worst overhead:     {:>8.1}x  ({:<20})              │",
        summary.max_overhead.1, summary.max_overhead.0
    );
    println!(
        "│ Best overhead:      {:>8.1}x  ({:<20})              │",
        summary.min_overhead.1, summary.min_overhead.0
    );
    println!("│ Global bottleneck:  {:<42} │", summary.global_bottleneck);
    println!("├──────────────────────────────────────────────────────────────────┤");
    println!("│ Average Time Distribution by Phase:                             │");
    for (name, pct) in &summary.phase_avg_pct {
        let bar = bar_chart(*pct, 35);
        println!("│   {:<16} {:>5.1}%  {bar:40}│", name, pct);
    }
    print_subphase_summary(&summary.avg_map_images_subphases, "map_images");
    print_subphase_summary(&summary.avg_objc_reg_subphases, "objc_reg");
    println!("└──────────────────────────────────────────────────────────────────┘");
    println!();
}

fn print_subphase_summary(subphases: &[(String, f64)], label: &str) {
    let visible: Vec<_> = subphases
        .iter()
        .filter(|(name, ms)| !name.starts_with("total_") && *ms > 0.0)
        .take(3)
        .collect();

    if visible.is_empty() {
        return;
    }
    println!("│ Top {label:<9} subphases (avg ms):                            │");
    for (name, ms) in visible {
        let pretty_name = name.strip_suffix("_us").unwrap_or(name);
        println!("│   {:<24} {:>7.3} ms{:>23}│", pretty_name, ms, "");
    }
}

/// Print bottleneck analysis: which phase dominates for each tool.
fn print_bottleneck_analysis(benchmarks: &[ToolBenchmark]) {
    println!();
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ BOTTLENECK ANALYSIS                                             │");
    println!("├──────────────────────┬───────────────┬──────────────────────────┤");
    println!("│ Tool                 │  Bottleneck   │ % of Total               │");
    println!("├──────────────────────┼───────────────┼──────────────────────────┤");

    for b in benchmarks {
        let (phase, dur) = b.loader_timing.bottleneck();
        let pct = if b.loader_timing.total.as_secs_f64() > 0.0 {
            (dur.as_secs_f64() / b.loader_timing.total.as_secs_f64()) * 100.0
        } else {
            0.0
        };
        println!(
            "│ {:<20} │ {:<13} │ {:>5.1}% {}│",
            &b.name[..b.name.len().min(20)],
            phase,
            pct,
            bar_chart(pct, 18),
        );
    }

    println!("└──────────────────────┴───────────────┴──────────────────────────┘");
}

fn compute_summary(benchmarks: &[ToolBenchmark]) -> MatrixSummary {
    let total_tools = benchmarks.len();
    let tools_passing = benchmarks.iter().filter(|b| b.passed).count();

    let ratios: Vec<f64> = benchmarks
        .iter()
        .map(ToolBenchmark::overhead_ratio)
        .filter(|r| r.is_finite())
        .collect();

    let avg_overhead = if ratios.is_empty() {
        f64::NAN
    } else {
        ratios.iter().sum::<f64>() / ratios.len() as f64
    };

    let max_overhead = benchmarks
        .iter()
        .filter(|b| b.overhead_ratio().is_finite())
        .max_by(|a, b| {
            a.overhead_ratio()
                .partial_cmp(&b.overhead_ratio())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|b| (b.name.clone(), b.overhead_ratio()))
        .unwrap_or_else(|| ("none".to_string(), 0.0));

    let min_overhead = benchmarks
        .iter()
        .filter(|b| b.overhead_ratio().is_finite())
        .min_by(|a, b| {
            a.overhead_ratio()
                .partial_cmp(&b.overhead_ratio())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|b| (b.name.clone(), b.overhead_ratio()))
        .unwrap_or_else(|| ("none".to_string(), 0.0));

    // Aggregate phase percentages across all tools
    let phase_names = [
        "parse",
        "security",
        "map_images",
        "fixups",
        "objc_reg",
        "protect",
        "initialize",
        "execution",
    ];

    let mut phase_totals: Vec<f64> = vec![0.0; phase_names.len()];
    let count = benchmarks.len() as f64;

    for b in benchmarks {
        let pcts = b.loader_timing.phase_percentages();
        for (name, pct) in &pcts {
            if let Some(idx) = phase_names.iter().position(|n| n == name) {
                phase_totals[idx] += pct;
            }
        }
    }

    let phase_avg_pct: Vec<(&'static str, f64)> = phase_names
        .iter()
        .zip(phase_totals.iter())
        .map(|(name, total)| (*name, total / count))
        .collect();

    // Find global bottleneck (highest average %)
    let global_bottleneck = phase_avg_pct
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(name, pct)| format!("{name} ({pct:.1}% avg)"))
        .unwrap_or_else(|| "unknown".to_string());

    let avg_map_images_subphases = average_breakdown_subphases(
        benchmarks
            .iter()
            .map(|b| &b.loader_timing.map_images_breakdown),
    );
    let avg_objc_reg_subphases = average_breakdown_subphases(
        benchmarks
            .iter()
            .map(|b| &b.loader_timing.objc_registration_breakdown),
    );

    MatrixSummary {
        total_tools,
        tools_passing,
        avg_overhead,
        max_overhead,
        min_overhead,
        global_bottleneck,
        phase_avg_pct,
        avg_map_images_subphases,
        avg_objc_reg_subphases,
    }
}

fn average_breakdown_subphases<'a>(
    breakdowns: impl Iterator<Item = &'a PhaseBreakdown>,
) -> Vec<(String, f64)> {
    let mut sums: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    let mut count = 0.0f64;

    for breakdown in breakdowns {
        if breakdown.is_empty() {
            continue;
        }
        count += 1.0;
        for timing in &breakdown.timings {
            *sums.entry(timing.name.clone()).or_default() +=
                timing.duration().as_secs_f64() * 1000.0;
        }
    }

    if count == 0.0 {
        return Vec::new();
    }

    let mut averaged: Vec<(String, f64)> = sums
        .into_iter()
        .map(|(name, total_ms)| (name, total_ms / count))
        .collect();
    averaged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    averaged
}

/// Export benchmarks as JSON for external tooling / CI integration.
pub fn export_json(benchmarks: &[ToolBenchmark]) -> String {
    #[derive(serde::Serialize)]
    struct JsonBenchmark {
        name: String,
        native_ms: f64,
        loader_total_ms: f64,
        overhead_ratio: f64,
        passed: bool,
        images_loaded: usize,
        shims_installed: usize,
        phases: serde_json::Value,
    }

    let entries: Vec<JsonBenchmark> = benchmarks
        .iter()
        .map(|b| {
            let t = &b.loader_timing;
            JsonBenchmark {
                name: b.name.clone(),
                native_ms: b.native_duration.as_secs_f64() * 1000.0,
                loader_total_ms: t.total.as_secs_f64() * 1000.0,
                overhead_ratio: b.overhead_ratio(),
                passed: b.passed,
                images_loaded: b.images_loaded,
                shims_installed: b.shims_installed,
                phases: serde_json::json!({
                    "parse_ms": t.parse.as_secs_f64() * 1000.0,
                    "security_ms": t.security.as_secs_f64() * 1000.0,
                    "map_images_ms": t.map_images.as_secs_f64() * 1000.0,
                    "fixups_ms": t.fixups.as_secs_f64() * 1000.0,
                    "objc_registration_ms": t.objc_registration.as_secs_f64() * 1000.0,
                    "protect_ms": t.protect.as_secs_f64() * 1000.0,
                    "initialize_ms": t.initialize.as_secs_f64() * 1000.0,
                    "execution_ms": t.execution.as_secs_f64() * 1000.0,
                    "map_images_breakdown": {
                        "timings_ms": t.map_images_breakdown.timings.iter().map(|metric| serde_json::json!({
                            "name": metric.name,
                            "ms": metric.duration().as_secs_f64() * 1000.0,
                        })).collect::<Vec<_>>(),
                        "counters": t.map_images_breakdown.counters.iter().map(|metric| serde_json::json!({
                            "name": metric.name,
                            "value": metric.value,
                        })).collect::<Vec<_>>(),
                    },
                    "objc_registration_breakdown": {
                        "timings_ms": t.objc_registration_breakdown.timings.iter().map(|metric| serde_json::json!({
                            "name": metric.name,
                            "ms": metric.duration().as_secs_f64() * 1000.0,
                        })).collect::<Vec<_>>(),
                        "counters": t.objc_registration_breakdown.counters.iter().map(|metric| serde_json::json!({
                            "name": metric.name,
                            "value": metric.value,
                        })).collect::<Vec<_>>(),
                    },
                }),
            }
        })
        .collect();

    serde_json::to_string_pretty(&entries).unwrap_or_else(|_| "[]".to_string())
}
