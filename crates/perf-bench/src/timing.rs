//! Phase-granular timing for the loader pipeline.
//!
//! Uses the typestate `Loader` API to instrument each phase transition
//! with `Instant` checkpoints, producing a `PhaseTiming` breakdown.

use std::path::Path;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use macho_runtime::typestate::Loader;
use macho_runtime::{RuntimeConfig, RuntimeResult, RuntimeTracer};

/// Named subphase timing metric, stored in microseconds for stable JSON export.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SubphaseTiming {
    pub name: String,
    pub micros: u128,
}

impl SubphaseTiming {
    pub fn duration(&self) -> Duration {
        #[allow(clippy::cast_possible_truncation)]
        {
            Duration::from_micros(self.micros as u64)
        }
    }
}

/// Named counter metric emitted by the runtime.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CounterMetric {
    pub name: String,
    pub value: u64,
}

/// Detailed subphase breakdown for a single top-level phase.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PhaseBreakdown {
    pub timings: Vec<SubphaseTiming>,
    pub counters: Vec<CounterMetric>,
}

impl PhaseBreakdown {
    pub fn is_empty(&self) -> bool {
        self.timings.is_empty() && self.counters.is_empty()
    }

    pub fn top_timing(&self) -> Option<&SubphaseTiming> {
        self.timings.iter().max_by_key(|metric| metric.micros)
    }
}

/// Timing breakdown for each loader pipeline phase.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PhaseTiming {
    /// Total wall-clock time from start to end.
    pub total: Duration,
    /// Parse the Mach-O binary (read + fat-slice + header parse).
    pub parse: Duration,
    /// Security policy evaluation.
    pub security: Duration,
    /// Map images into memory (dependency resolution + mmap).
    pub map_images: Duration,
    /// Apply fixups (rebases + binds + TLV patching).
    pub fixups: Duration,
    /// ObjC metadata registration pipeline.
    pub objc_registration: Duration,
    /// Segment protection (mprotect) + unwind registration.
    pub protect: Duration,
    /// +load dispatch + constructor execution.
    pub initialize: Duration,
    /// Time from entry point call to exit.
    pub execution: Duration,
    /// Detailed subphase timings + counters for the map_images phase.
    pub map_images_breakdown: PhaseBreakdown,
    /// Detailed subphase timings + counters for the objc registration phase.
    pub objc_registration_breakdown: PhaseBreakdown,
}

impl PhaseTiming {
    /// Returns the overhead ratio: total loader time / native baseline.
    pub fn overhead_ratio(&self, native_duration: Duration) -> f64 {
        if native_duration.is_zero() {
            return f64::INFINITY;
        }
        self.total.as_secs_f64() / native_duration.as_secs_f64()
    }

    /// Returns a sorted list of (phase_name, duration) from slowest to fastest.
    pub fn phases_sorted(&self) -> Vec<(&'static str, Duration)> {
        let mut phases = vec![
            ("parse", self.parse),
            ("security", self.security),
            ("map_images", self.map_images),
            ("fixups", self.fixups),
            ("objc_reg", self.objc_registration),
            ("protect", self.protect),
            ("initialize", self.initialize),
            ("execution", self.execution),
        ];
        phases.sort_by(|a, b| b.1.cmp(&a.1));
        phases
    }

    /// The fraction of total time spent in each phase (0.0–1.0).
    pub fn phase_percentages(&self) -> Vec<(&'static str, f64)> {
        let total_secs = self.total.as_secs_f64();
        if total_secs == 0.0 {
            return Vec::new();
        }
        self.phases_sorted()
            .into_iter()
            .map(|(name, dur)| (name, (dur.as_secs_f64() / total_secs) * 100.0))
            .collect()
    }

    /// Identifies the single biggest bottleneck phase.
    pub fn bottleneck(&self) -> (&'static str, Duration) {
        self.phases_sorted()
            .into_iter()
            .next()
            .unwrap_or(("unknown", Duration::ZERO))
    }
}

/// Serializes load_and_run calls (global ObjC state is process-wide).
static RUNTIME_LOCK: Mutex<()> = Mutex::new(());

struct RuntimeReuseGuard;

impl RuntimeReuseGuard {
    fn new() -> Self {
        reset_runtime_state();
        Self
    }
}

impl Drop for RuntimeReuseGuard {
    fn drop(&mut self) {
        reset_runtime_state();
    }
}

fn reset_runtime_state() {
    macho_runtime::reset_runtime_state_for_reuse();
    runtime_diagnostics::drain();
}

/// Run a binary through the typestate loader pipeline, timing each phase.
pub fn timed_load_and_run(binary_path: &Path) -> Result<(RuntimeResult, PhaseTiming), String> {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let _reuse_guard = RuntimeReuseGuard::new();

    let overall_start = Instant::now();
    let mut config = RuntimeConfig::permissive(binary_path);
    config.tracer = Some(RuntimeTracer::new());

    // Phase 1: Parse
    let t = Instant::now();
    let parsed = Loader::parse(binary_path, config).map_err(|e| format!("parse: {e}"))?;
    let parse = t.elapsed();

    // Phase 2: Security
    let t = Instant::now();
    let checked = parsed.check_security().map_err(|e| format!("security: {e}"))?;
    let security = t.elapsed();

    // Phase 3: Map images
    let t = Instant::now();
    let mapped = checked.map_images().map_err(|e| format!("map: {e}"))?;
    let map_images = t.elapsed();

    // Phase 4: Fixups
    let t = Instant::now();
    let fixed = mapped.apply_fixups().map_err(|e| format!("fixups: {e}"))?;
    let fixups = t.elapsed();

    // Phase 5: ObjC registration
    let t = Instant::now();
    let registered = fixed.register_objc().map_err(|e| format!("objc: {e}"))?;
    let objc_registration = t.elapsed();

    // Phase 6: Protect
    let t = Instant::now();
    let protected = registered.protect().map_err(|e| format!("protect: {e}"))?;
    let protect = t.elapsed();

    // Phase 7: Initialize (+load + constructors)
    let t = Instant::now();
    let initialized = protected.initialize().map_err(|e| format!("init: {e}"))?;
    let initialize = t.elapsed();

    // Phase 8: Run entry point
    let t = Instant::now();
    let result = initialized.run().map_err(|e| format!("run: {e}"))?;
    let execution = t.elapsed();

    let total = overall_start.elapsed();
    let (map_images_breakdown, objc_registration_breakdown) = result
        .tracer
        .as_ref()
        .map(parse_perf_breakdowns)
        .unwrap_or_else(|| (PhaseBreakdown::default(), PhaseBreakdown::default()));

    let timing = PhaseTiming {
        total,
        parse,
        security,
        map_images,
        fixups,
        objc_registration,
        protect,
        initialize,
        execution,
        map_images_breakdown,
        objc_registration_breakdown,
    };

    Ok((result, timing))
}

fn parse_perf_breakdowns(tracer: &RuntimeTracer) -> (PhaseBreakdown, PhaseBreakdown) {
    let mut scopes: BTreeMap<String, PhaseBreakdown> = BTreeMap::new();

    for event in tracer.events() {
        let parts: Vec<&str> = event.message.split('|').collect();
        if parts.len() != 5 || parts[0] != "perf" {
            continue;
        }

        let scope = parts[1].to_string();
        let metric_kind = parts[2];
        let name = parts[3].to_string();
        let value = parts[4];

        let breakdown = scopes.entry(scope).or_default();
        match metric_kind {
            "timing" => {
                if let Ok(micros) = value.parse::<u128>() {
                    breakdown.timings.push(SubphaseTiming { name, micros });
                }
            }
            "counter" => {
                if let Ok(count) = value.parse::<u64>() {
                    breakdown.counters.push(CounterMetric { name, value: count });
                }
            }
            _ => {}
        }
    }

    for breakdown in scopes.values_mut() {
        breakdown.timings.sort_by(|a, b| b.micros.cmp(&a.micros));
        breakdown.counters.sort_by(|a, b| a.name.cmp(&b.name));
    }

    (
        scopes.remove("map_images").unwrap_or_default(),
        scopes.remove("objc_reg").unwrap_or_default(),
    )
}
