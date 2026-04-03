use crate::{DiagEvent, emit, drain, clear, has_events, event_count, snapshot};
use crate::{drain_stamped, snapshot_stamped, total_emitted};
use crate::ring::{DiagRing, DiagCategory};
use std::sync::Mutex;

/// Serialize tests that use the process-global diagnostic ring.
static COLLECTOR_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn emit_and_drain() {
    let _guard = COLLECTOR_TEST_LOCK.lock().unwrap();
    clear();
    emit(DiagEvent::ClassMissing { name: "Foo".into() });
    emit(DiagEvent::SymbolMissing { name: "_bar".into() });
    assert_eq!(event_count(), 2);
    assert!(has_events());

    let events = drain();
    assert_eq!(events.len(), 2);
    assert_eq!(event_count(), 0);
    assert!(!has_events());
}

#[test]
fn snapshot_does_not_clear() {
    let _guard = COLLECTOR_TEST_LOCK.lock().unwrap();
    clear();
    emit(DiagEvent::ClassMissing { name: "A".into() });
    let snap = snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(event_count(), 1);
    clear();
}

#[test]
fn display_selector_missing() {
    let ev = DiagEvent::SelectorMissing {
        selector: "doStuff:".into(),
        class_name: "NSFoo".into(),
        receiver: 0x1234,
    };
    let s = format!("{ev}");
    assert!(s.contains("NSFoo"));
    assert!(s.contains("doStuff:"));
}

#[test]
fn display_symbol_blocked() {
    let ev = DiagEvent::SymbolBlocked { name: "_dlopen".into() };
    assert_eq!(format!("{ev}"), "symbol blocked by policy: _dlopen");
}

#[test]
fn display_bind_failed() {
    let ev = DiagEvent::BindFailed { ordinal: 3, symbol: "_missing".into() };
    assert_eq!(format!("{ev}"), "bind failed: ordinal 3 '_missing'");
}

#[test]
fn display_host_fallback() {
    let ev = DiagEvent::HostFallback {
        reason: "tagged pointer".into(),
        selector: "init".into(),
        receiver: 0xff,
    };
    let s = format!("{ev}");
    assert!(s.contains("tagged pointer"));
    assert!(s.contains("init"));
}

#[test]
fn display_method_dispatch() {
    let ev = DiagEvent::MethodDispatch {
        selector: "alloc".into(),
        isa: 0x100,
        imp: 0x200,
    };
    let s = format!("{ev}");
    assert!(s.contains("alloc"));
    assert!(s.contains("0x100"));
    assert!(s.contains("0x200"));
}

#[test]
fn display_unknown_isa() {
    let ev = DiagEvent::UnknownIsa {
        isa: 0xdead,
        selector: "foo".into(),
        receiver: 0xbeef,
    };
    let s = format!("{ev}");
    assert!(s.contains("0xdead"));
    assert!(s.contains("foo"));
}

#[test]
fn display_category_orphan() {
    let ev = DiagEvent::CategoryOrphan { class_addr: 0x42 };
    assert_eq!(format!("{ev}"), "category targets unknown class at 0x42");
}

#[test]
fn display_super_selector_missing() {
    let ev = DiagEvent::SuperSelectorMissing {
        selector: "dealloc".into(),
        receiver: 0x10,
        super_class: 0x20,
    };
    let s = format!("{ev}");
    assert!(s.contains("dealloc"));
    assert!(s.contains("0x20"));
}

#[test]
fn display_forwarding_entered() {
    let ev = DiagEvent::ForwardingEntered {
        selector: "forward:".into(),
        receiver: 0xa,
        isa: 0xb,
    };
    let s = format!("{ev}");
    assert!(s.contains("forward:"));
}

#[test]
fn events_are_clone_and_eq() {
    let a = DiagEvent::ClassMissing { name: "X".into() };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn class_addr_unknown_display() {
    let ev = DiagEvent::ClassAddrUnknown { addr: 0xc0de };
    assert!(format!("{ev}").contains("0xc0de"));
}

#[test]
fn super2_unknown_class_display() {
    let ev = DiagEvent::Super2UnknownClass { current_class: 0xface };
    assert!(format!("{ev}").contains("0xface"));
}

#[test]
fn symbol_missing_display() {
    let ev = DiagEvent::SymbolMissing { name: "_CFRelease".into() };
    assert!(format!("{ev}").contains("_CFRelease"));
}

// ── Resource-graph event display tests ──────────────────────────────

#[test]
fn display_resource_request() {
    let ev = DiagEvent::ResourceRequest {
        kind: "dylib".into(),
        name: "/usr/lib/libSystem.B.dylib".into(),
    };
    let s = format!("{ev}");
    assert!(s.contains("dylib"));
    assert!(s.contains("libSystem"));
}

#[test]
fn display_resource_resolved() {
    let ev = DiagEvent::ResourceResolved {
        kind: "symbol".into(),
        name: "_objc_msgSend".into(),
        addr: 0x1000,
    };
    let s = format!("{ev}");
    assert!(s.contains("resolved"));
    assert!(s.contains("_objc_msgSend"));
    assert!(s.contains("0x1000"));
}

#[test]
fn display_resource_failed() {
    let ev = DiagEvent::ResourceFailed {
        kind: "dylib".into(),
        name: "libfoo.dylib".into(),
        reason: "not found".into(),
    };
    let s = format!("{ev}");
    assert!(s.contains("failed"));
    assert!(s.contains("libfoo"));
    assert!(s.contains("not found"));
}

// ── DiagRing unit tests ─────────────────────────────────────────────

#[test]
fn ring_push_and_snapshot() {
    let mut ring = DiagRing::with_capacity(4);
    ring.push(DiagEvent::ClassMissing { name: "A".into() });
    ring.push(DiagEvent::ClassMissing { name: "B".into() });
    assert_eq!(ring.len(), 2);
    assert_eq!(ring.total_emitted(), 2);
    assert_eq!(ring.evicted(), 0);

    let snap = ring.snapshot();
    assert_eq!(snap.len(), 2);
    assert_eq!(snap[0].seq, 0);
    assert_eq!(snap[1].seq, 1);
}

#[test]
fn ring_eviction() {
    let mut ring = DiagRing::with_capacity(3);
    for i in 0..5u64 {
        ring.push(DiagEvent::SymbolMissing { name: format!("sym{i}") });
    }
    // Should keep only the last 3
    assert_eq!(ring.len(), 3);
    assert_eq!(ring.total_emitted(), 5);
    assert_eq!(ring.evicted(), 2);

    let snap = ring.snapshot();
    assert_eq!(snap[0].seq, 2);
    assert_eq!(snap[1].seq, 3);
    assert_eq!(snap[2].seq, 4);
}

#[test]
fn ring_counters_survive_eviction() {
    let mut ring = DiagRing::with_capacity(2);
    ring.push(DiagEvent::ClassMissing { name: "A".into() });
    ring.push(DiagEvent::ClassMissing { name: "B".into() });
    ring.push(DiagEvent::ClassMissing { name: "C".into() }); // evicts A
    assert_eq!(ring.counters.class_lookup, 3);
    assert_eq!(ring.counters.total(), 3);
    assert_eq!(ring.len(), 2);
}

#[test]
fn ring_drain_clears_but_preserves_counters() {
    let mut ring = DiagRing::with_capacity(8);
    ring.push(DiagEvent::HostFallback {
        reason: "test".into(),
        selector: "init".into(),
        receiver: 0,
    });
    let drained = ring.drain();
    assert_eq!(drained.len(), 1);
    assert!(ring.is_empty());
    assert_eq!(ring.counters.host_fallback, 1);
}

#[test]
fn ring_by_category() {
    let mut ring = DiagRing::with_capacity(16);
    ring.push(DiagEvent::ClassMissing { name: "X".into() });
    ring.push(DiagEvent::ResourceRequest { kind: "dylib".into(), name: "foo".into() });
    ring.push(DiagEvent::ResourceResolved { kind: "dylib".into(), name: "foo".into(), addr: 0x100 });
    ring.push(DiagEvent::SymbolMissing { name: "_bar".into() });

    let rg = ring.by_category(DiagCategory::ResourceGraph);
    assert_eq!(rg.len(), 2);
    assert_eq!(ring.counters.resource_graph, 2);
}

#[test]
fn ring_tail() {
    let mut ring = DiagRing::with_capacity(16);
    for i in 0..10u64 {
        ring.push(DiagEvent::SymbolMissing { name: format!("s{i}") });
    }
    let last3 = ring.tail(3);
    assert_eq!(last3.len(), 3);
    assert_eq!(last3[0].seq, 7);
    assert_eq!(last3[2].seq, 9);
}

#[test]
fn ring_category_of_all_variants() {
    // Ensure every variant classifies without panic
    let events = vec![
        DiagEvent::SelectorMissing { selector: "a".into(), class_name: "b".into(), receiver: 0 },
        DiagEvent::SuperSelectorMissing { selector: "a".into(), receiver: 0, super_class: 0 },
        DiagEvent::ForwardingEntered { selector: "a".into(), receiver: 0, isa: 0 },
        DiagEvent::ClassMissing { name: "a".into() },
        DiagEvent::ClassAddrUnknown { addr: 0 },
        DiagEvent::Super2UnknownClass { current_class: 0 },
        DiagEvent::CategoryOrphan { class_addr: 0 },
        DiagEvent::SymbolBlocked { name: "a".into() },
        DiagEvent::SymbolMissing { name: "a".into() },
        DiagEvent::BindFailed { ordinal: 0, symbol: "a".into() },
        DiagEvent::HostFallback { reason: "a".into(), selector: "b".into(), receiver: 0 },
        DiagEvent::UnknownIsa { isa: 0, selector: "a".into(), receiver: 0 },
        DiagEvent::MethodDispatch { selector: "a".into(), isa: 0, imp: 0 },
        DiagEvent::ResourceRequest { kind: "a".into(), name: "b".into() },
        DiagEvent::ResourceResolved { kind: "a".into(), name: "b".into(), addr: 0 },
        DiagEvent::ResourceFailed { kind: "a".into(), name: "b".into(), reason: "c".into() },
    ];
    for e in &events {
        let _ = DiagCategory::of(e);
    }
}

// ── Collector API tests ─────────────────────────────────────────────

#[test]
fn drain_stamped_has_seq_numbers() {
    let _guard = COLLECTOR_TEST_LOCK.lock().unwrap();
    clear();
    emit(DiagEvent::ClassMissing { name: "A".into() });
    emit(DiagEvent::SymbolMissing { name: "_b".into() });
    emit(DiagEvent::ClassMissing { name: "C".into() });

    let stamped = drain_stamped();
    assert_eq!(stamped.len(), 3);
    // Sequence numbers should be monotonically increasing
    assert!(stamped[0].seq < stamped[1].seq);
    assert!(stamped[1].seq < stamped[2].seq);
    // Verify event content is preserved
    assert_eq!(stamped[0].event, DiagEvent::ClassMissing { name: "A".into() });
    assert_eq!(stamped[2].event, DiagEvent::ClassMissing { name: "C".into() });
    // Buffer should be empty after drain
    assert_eq!(event_count(), 0);
}

#[test]
fn total_emitted_counts_all() {
    let _guard = COLLECTOR_TEST_LOCK.lock().unwrap();
    clear();
    for i in 0..5 {
        emit(DiagEvent::SymbolMissing { name: format!("sym{i}") });
    }
    let before = total_emitted();
    // total_emitted counts all events ever pushed into the global ring
    // Since the global ring is shared, just verify we got at least 5
    assert!(before >= 5, "total_emitted={before}");
    clear();
    // total_emitted does not reset on clear
    let after = total_emitted();
    assert!(after >= before, "after={after} before={before}");
}

#[test]
fn snapshot_stamped_returns_copies_with_seq() {
    let _guard = COLLECTOR_TEST_LOCK.lock().unwrap();
    clear();
    emit(DiagEvent::HostFallback {
        reason: "test".into(),
        selector: "init".into(),
        receiver: 0x10,
    });
    emit(DiagEvent::ClassMissing { name: "Foo".into() });

    let stamped = snapshot_stamped();
    assert_eq!(stamped.len(), 2);
    assert!(stamped[0].seq < stamped[1].seq);
    // Snapshot should NOT clear the buffer
    assert_eq!(event_count(), 2);
    clear();
}

#[test]
fn category_counters_all_fields() {
    // Use a fresh ring so we can check exact counts
    let mut ring = DiagRing::with_capacity(64);
    ring.push(DiagEvent::SelectorMissing {
        selector: "foo".into(), class_name: "Bar".into(), receiver: 0,
    });
    ring.push(DiagEvent::ForwardingEntered {
        selector: "x".into(), receiver: 0, isa: 0,
    });
    ring.push(DiagEvent::ClassMissing { name: "A".into() });
    ring.push(DiagEvent::SymbolBlocked { name: "s".into() });
    ring.push(DiagEvent::SymbolMissing { name: "t".into() });
    ring.push(DiagEvent::HostFallback {
        reason: "r".into(), selector: "s".into(), receiver: 0,
    });
    ring.push(DiagEvent::MethodDispatch {
        selector: "m".into(), isa: 0, imp: 0,
    });
    ring.push(DiagEvent::ResourceRequest {
        kind: "dylib".into(), name: "lib".into(),
    });

    assert_eq!(ring.counters.selector_dispatch, 2); // SelectorMissing + ForwardingEntered
    assert_eq!(ring.counters.class_lookup, 1);
    assert_eq!(ring.counters.symbol_resolution, 2); // SymbolBlocked + SymbolMissing
    assert_eq!(ring.counters.host_fallback, 1);
    assert_eq!(ring.counters.method_trace, 1);
    assert_eq!(ring.counters.resource_graph, 1);
    assert_eq!(ring.counters.total(), 8);
}

#[test]
fn diag_event_display_class_missing() {
    let ev = DiagEvent::ClassMissing { name: "NSWidgetController".into() };
    let s = format!("{ev}");
    assert!(s.contains("class missing"));
    assert!(s.contains("NSWidgetController"));
}
