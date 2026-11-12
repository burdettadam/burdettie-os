//! Runtime event tracing for observability.
//!
//! Provides structured event capture across the runtime pipeline:
//! image loading, ObjC class/category registration, selector interning,
//! method dispatch, block lifecycle, retain/release, and Foundation
//! object creation.
//!
//! Events are collected in-memory and can be queried after execution
//! for debugging, alignment testing, and performance analysis.

use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;

/// Schema version for the tracer JSON export format.
pub const TRACER_SCHEMA_VERSION: u32 = 1;

/// A structured runtime event.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeEvent {
    /// Event category.
    pub kind: EventKind,
    /// Human-readable description.
    pub message: String,
}

/// Categories of runtime events for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum EventKind {
    /// Image loaded into registry.
    ImageLoad,
    /// Selector registered/interned.
    SelectorRegistration,
    /// ObjC class registered.
    ClassRegistration,
    /// ObjC category merged into class.
    CategoryMerge,
    /// Method lookup (hit, miss, super chain, host fallback).
    MethodLookup,
    /// Block copied or released.
    BlockLifecycle,
    /// Retain or release call.
    RetainRelease,
    /// Foundation object allocated.
    FoundationCreate,
    /// Pipeline stage (parse, fixup, protect, entry, etc.)
    Pipeline,
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImageLoad => write!(f, "image-load"),
            Self::SelectorRegistration => write!(f, "selector-reg"),
            Self::ClassRegistration => write!(f, "class-reg"),
            Self::CategoryMerge => write!(f, "category-merge"),
            Self::MethodLookup => write!(f, "method-lookup"),
            Self::BlockLifecycle => write!(f, "block-lifecycle"),
            Self::RetainRelease => write!(f, "retain-release"),
            Self::FoundationCreate => write!(f, "foundation-create"),
            Self::Pipeline => write!(f, "pipeline"),
        }
    }
}

/// Thread-safe event collector.
///
/// Clone-friendly (shared via `Arc<Mutex<...>>`), so the same tracer
/// can be passed to multiple runtime subsystems.
#[derive(Clone, Default)]
pub struct RuntimeTracer {
    events: Arc<Mutex<Vec<RuntimeEvent>>>,
    /// When true, events are collected. When false, `record` is a no-op.
    enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl RuntimeTracer {
    /// Create a new tracer that is initially enabled.
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    /// Create a disabled (no-op) tracer.
    pub fn disabled() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Record an event (no-op if disabled).
    pub fn record(&self, kind: EventKind, message: impl Into<String>) {
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let event = RuntimeEvent {
            kind,
            message: message.into(),
        };
        if let Ok(mut guard) = self.events.lock() {
            guard.push(event);
        }
    }

    /// Record a machine-parseable performance timing event.
    pub fn record_perf_timing(&self, scope: &str, name: &str, duration: Duration) {
        self.record(
            EventKind::Pipeline,
            format!("perf|{scope}|timing|{name}|{}", duration.as_micros()),
        );
    }

    /// Record a machine-parseable performance counter event.
    pub fn record_perf_counter(&self, scope: &str, name: &str, value: u64) {
        self.record(
            EventKind::Pipeline,
            format!("perf|{scope}|counter|{name}|{value}"),
        );
    }

    /// Return all collected events.
    pub fn events(&self) -> Vec<RuntimeEvent> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Return events matching a specific kind.
    pub fn events_of(&self, kind: EventKind) -> Vec<RuntimeEvent> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|e| e.kind == kind)
            .cloned()
            .collect()
    }

    /// Number of collected events.
    pub fn count(&self) -> usize {
        self.events.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Number of events matching a specific kind.
    pub fn count_of(&self, kind: EventKind) -> usize {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|e| e.kind == kind)
            .count()
    }

    /// Whether tracing is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Clear all collected events.
    pub fn clear(&self) {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Export all events as a versioned JSON string.
    ///
    /// The output conforms to `TRACER_SCHEMA_VERSION` (currently v1):
    ///
    /// ```json
    /// {
    ///   "schema_version": 1,
    ///   "event_count": 42,
    ///   "events": [
    ///     { "kind": "ImageLoad", "message": "loaded main binary" },
    ///     ...
    ///   ]
    /// }
    /// ```
    pub fn to_json(&self) -> String {
        let events = self.events();
        let wrapper = TracerExport {
            schema_version: TRACER_SCHEMA_VERSION,
            event_count: events.len(),
            events,
        };
        serde_json::to_string(&wrapper).unwrap_or_else(|_| "{}".to_string())
    }

    /// Export all events as a pretty-printed versioned JSON string.
    pub fn to_json_pretty(&self) -> String {
        let events = self.events();
        let wrapper = TracerExport {
            schema_version: TRACER_SCHEMA_VERSION,
            event_count: events.len(),
            events,
        };
        serde_json::to_string_pretty(&wrapper).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Versioned envelope for JSON export.
#[derive(Serialize)]
struct TracerExport {
    schema_version: u32,
    event_count: usize,
    events: Vec<RuntimeEvent>,
}

impl fmt::Debug for RuntimeTracer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count();
        f.debug_struct("RuntimeTracer")
            .field("enabled", &self.is_enabled())
            .field("events", &count)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracer_collects_events() {
        let tracer = RuntimeTracer::new();
        tracer.record(EventKind::ImageLoad, "loaded main binary");
        tracer.record(EventKind::ClassRegistration, "registered NSObject");
        tracer.record(EventKind::ImageLoad, "loaded libfoo.dylib");

        assert_eq!(tracer.count(), 3);
        assert_eq!(tracer.count_of(EventKind::ImageLoad), 2);
        assert_eq!(tracer.count_of(EventKind::ClassRegistration), 1);
    }

    #[test]
    fn perf_helpers_emit_pipeline_events() {
        let tracer = RuntimeTracer::new();
        tracer.record_perf_timing("map_images", "deps_total_us", Duration::from_micros(123));
        tracer.record_perf_counter("map_images", "deps_loaded", 7);

        let events = tracer.events_of(EventKind::Pipeline);
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0].message,
            "perf|map_images|timing|deps_total_us|123"
        );
        assert_eq!(events[1].message, "perf|map_images|counter|deps_loaded|7");
    }

    #[test]
    fn disabled_tracer_is_noop() {
        let tracer = RuntimeTracer::disabled();
        tracer.record(EventKind::Pipeline, "should be ignored");
        assert_eq!(tracer.count(), 0);
    }

    #[test]
    fn events_of_filters_correctly() {
        let tracer = RuntimeTracer::new();
        tracer.record(EventKind::SelectorRegistration, "sel: init");
        tracer.record(EventKind::Pipeline, "step 1");
        tracer.record(EventKind::SelectorRegistration, "sel: dealloc");

        let sels = tracer.events_of(EventKind::SelectorRegistration);
        assert_eq!(sels.len(), 2);
        assert!(sels[0].message.contains("init"));
        assert!(sels[1].message.contains("dealloc"));
    }

    #[test]
    fn tracer_clones_share_state() {
        let t1 = RuntimeTracer::new();
        let t2 = t1.clone();

        t1.record(EventKind::Pipeline, "from t1");
        t2.record(EventKind::Pipeline, "from t2");

        assert_eq!(t1.count(), 2);
        assert_eq!(t2.count(), 2);
    }

    #[test]
    fn clear_removes_all_events() {
        let tracer = RuntimeTracer::new();
        tracer.record(EventKind::Pipeline, "a");
        tracer.record(EventKind::Pipeline, "b");
        assert_eq!(tracer.count(), 2);
        tracer.clear();
        assert_eq!(tracer.count(), 0);
    }

    #[test]
    fn to_json_produces_versioned_output() {
        let tracer = RuntimeTracer::new();
        tracer.record(EventKind::ImageLoad, "loaded main");
        tracer.record(EventKind::Pipeline, "fixups applied");

        let json = tracer.to_json();
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(v["schema_version"], TRACER_SCHEMA_VERSION);
        assert_eq!(v["event_count"], 2);
        assert!(v["events"].is_array());
        assert_eq!(v["events"][0]["kind"], "ImageLoad");
        assert_eq!(v["events"][0]["message"], "loaded main");
        assert_eq!(v["events"][1]["kind"], "Pipeline");
    }

    #[test]
    fn event_kind_display() {
        assert_eq!(EventKind::ImageLoad.to_string(), "image-load");
        assert_eq!(EventKind::SelectorRegistration.to_string(), "selector-reg");
        assert_eq!(EventKind::ClassRegistration.to_string(), "class-reg");
        assert_eq!(EventKind::CategoryMerge.to_string(), "category-merge");
        assert_eq!(EventKind::MethodLookup.to_string(), "method-lookup");
        assert_eq!(EventKind::BlockLifecycle.to_string(), "block-lifecycle");
        assert_eq!(EventKind::RetainRelease.to_string(), "retain-release");
        assert_eq!(EventKind::FoundationCreate.to_string(), "foundation-create");
        assert_eq!(EventKind::Pipeline.to_string(), "pipeline");
    }

    #[test]
    fn event_kind_equality() {
        assert_eq!(EventKind::ImageLoad, EventKind::ImageLoad);
        assert_eq!(EventKind::Pipeline, EventKind::Pipeline);
        assert_ne!(EventKind::ImageLoad, EventKind::Pipeline);
        assert_ne!(EventKind::ClassRegistration, EventKind::CategoryMerge);
    }

    #[test]
    fn tracer_is_enabled() {
        let enabled = RuntimeTracer::new();
        assert!(enabled.is_enabled());

        let disabled = RuntimeTracer::disabled();
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn tracer_default() {
        let tracer = RuntimeTracer::default();
        // Default derive: AtomicBool defaults to false, so disabled
        assert!(!tracer.is_enabled());
        assert_eq!(tracer.count(), 0);
    }

    #[test]
    fn tracer_debug_format() {
        let tracer = RuntimeTracer::new();
        tracer.record(EventKind::Pipeline, "step");
        let dbg = format!("{:?}", tracer);
        assert!(dbg.contains("RuntimeTracer"));
        assert!(dbg.contains("1"));
    }

    #[test]
    fn disabled_tracer_discards() {
        let tracer = RuntimeTracer::disabled();
        tracer.record(EventKind::ImageLoad, "img1");
        tracer.record(EventKind::ClassRegistration, "cls1");
        tracer.record(EventKind::Pipeline, "step1");
        assert!(tracer.events().is_empty());
        assert!(tracer.events_of(EventKind::ImageLoad).is_empty());
    }

    #[test]
    fn events_of_filters_by_kind() {
        let tracer = RuntimeTracer::new();
        tracer.record(EventKind::ImageLoad, "img1");
        tracer.record(EventKind::ClassRegistration, "cls1");
        tracer.record(EventKind::ImageLoad, "img2");
        tracer.record(EventKind::MethodLookup, "lookup1");
        tracer.record(EventKind::ClassRegistration, "cls2");
        tracer.record(EventKind::ImageLoad, "img3");

        assert_eq!(tracer.events_of(EventKind::ImageLoad).len(), 3);
        assert_eq!(tracer.events_of(EventKind::ClassRegistration).len(), 2);
        assert_eq!(tracer.events_of(EventKind::MethodLookup).len(), 1);
        assert!(tracer.events_of(EventKind::RetainRelease).is_empty());
    }

    #[test]
    fn count_of_counts_correctly() {
        let tracer = RuntimeTracer::new();
        tracer.record(EventKind::FoundationCreate, "obj1");
        tracer.record(EventKind::FoundationCreate, "obj2");
        tracer.record(EventKind::FoundationCreate, "obj3");
        tracer.record(EventKind::Pipeline, "step");

        assert_eq!(tracer.count_of(EventKind::FoundationCreate), 3);
        assert_eq!(tracer.count_of(EventKind::Pipeline), 1);
        assert_eq!(tracer.count_of(EventKind::BlockLifecycle), 0);
        assert_eq!(tracer.count(), 4);
    }

    #[test]
    fn to_json_pretty_valid_json() {
        let tracer = RuntimeTracer::new();
        tracer.record(EventKind::ImageLoad, "loaded main");
        tracer.record(EventKind::SelectorRegistration, "sel: init");

        let pretty = tracer.to_json_pretty();
        let v: serde_json::Value = serde_json::from_str(&pretty).expect("valid JSON");

        assert_eq!(v["schema_version"], TRACER_SCHEMA_VERSION);
        assert_eq!(v["event_count"], 2);
        assert!(v["events"].is_array());
        // Pretty output should contain newlines
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn clear_empties_events() {
        let tracer = RuntimeTracer::new();
        tracer.record(EventKind::ImageLoad, "img");
        tracer.record(EventKind::Pipeline, "step");
        assert_eq!(tracer.count(), 2);

        tracer.clear();
        assert!(tracer.events().is_empty());
        assert!(tracer.events_of(EventKind::ImageLoad).is_empty());
        assert_eq!(tracer.count_of(EventKind::Pipeline), 0);
    }

    #[test]
    fn to_json_pretty_has_newlines() {
        let tracer = RuntimeTracer::new();
        tracer.record(EventKind::ImageLoad, "img");
        let pretty = tracer.to_json_pretty();
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn to_json_empty_tracer() {
        let tracer = RuntimeTracer::new();
        let json = tracer.to_json();
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["schema_version"], TRACER_SCHEMA_VERSION);
        assert_eq!(v["event_count"], 0);
        assert_eq!(v["events"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn runtime_event_clone() {
        let event = RuntimeEvent {
            kind: EventKind::MethodLookup,
            message: "lookup init".to_string(),
        };
        let cloned = event.clone();
        assert_eq!(cloned.kind, event.kind);
        assert_eq!(cloned.message, event.message);
    }
}
