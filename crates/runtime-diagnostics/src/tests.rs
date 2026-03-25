use crate::{DiagEvent, emit, drain, clear, has_events, event_count, snapshot};

#[test]
fn emit_and_drain() {
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
