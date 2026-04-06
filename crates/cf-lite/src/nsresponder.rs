//! NSResponder shim: base class for the responder chain.
//!
//! Object layout:
//!
//!   offset  0: isa              (u64)
//!   offset  8: next_responder   (u64 — pointer to next responder, or 0)
//!
//! Total: 16 bytes.

fn receiver_is_kind_of(obj: u64, class_addr: u64) -> bool {
    if obj == 0 || class_addr == 0 {
        return false;
    }

    let isa = unsafe { *(obj as *const u64) };
    if isa == class_addr {
        return true;
    }

    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return false;
    };

    let mut current = Some(isa);
    let mut depth = 0;
    while let Some(addr) = current {
        if depth > 64 {
            break;
        }
        if addr == class_addr {
            return true;
        }
        current = rt.get_class_by_addr(addr).and_then(|class| class.superclass_addr);
        depth += 1;
    }

    false
}

fn resolved_event_window(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    if receiver_is_kind_of(obj, crate::nsobj::nswindow_class_addr()) {
        return obj;
    }

    if receiver_is_kind_of(obj, crate::nsobj::nsview_class_addr()) {
        return crate::nsview::shim_nsview_window(obj, 0);
    }

    let mut current = obj;
    let mut depth = 0;
    while current != 0 && depth < 64 {
        let next = shim_nsresponder_next_responder(current, 0);
        if next == 0 || next == current {
            break;
        }

        if receiver_is_kind_of(next, crate::nsobj::nswindow_class_addr()) {
            return next;
        }

        if receiver_is_kind_of(next, crate::nsobj::nsview_class_addr()) {
            let window = crate::nsview::shim_nsview_window(next, 0);
            if window != 0 {
                return window;
            }
        }

        current = next;
        depth += 1;
    }

    let key_window = crate::nsapplication::current_key_window();
    if key_window != 0 {
        return key_window;
    }

    crate::nsapplication::current_main_window()
}

fn ensure_event_window(obj: u64, event: u64) -> u64 {
    if event != 0 {
        let window = crate::nsevent::event_window(event);
        if window != 0 {
            return window;
        }
    }

    let derived = resolved_event_window(obj);
    if event != 0 && derived != 0 {
        crate::nsevent::set_event_window(event, derived);
    }
    derived
}

fn next_event_target(current: u64, event_window: u64) -> u64 {
    if current == 0 {
        return 0;
    }

    if receiver_is_kind_of(current, crate::nsobj::nsview_class_addr()) {
        let superview = crate::nsview::shim_nsview_superview(current, 0);
        if superview != 0 {
            return superview;
        }

        let window = crate::nsview::shim_nsview_window(current, 0);
        if window != 0 && window != current {
            return window;
        }

        return 0;
    }

    if receiver_is_kind_of(current, crate::nsobj::nswindow_class_addr()) {
        return crate::nsapplication::shim_nsapplication_shared_application(0, 0);
    }

    if receiver_is_kind_of(current, crate::nsobj::nsapplication_class_addr()) {
        return 0;
    }

    let next = shim_nsresponder_next_responder(current, 0);
    if next != 0 && next != current {
        return next;
    }

    if event_window != 0 && current != event_window {
        return event_window;
    }

    let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
    if app != 0 && current != app {
        return app;
    }

    0
}

fn next_try_to_perform_target(current: u64) -> u64 {
    if current == 0 {
        return 0;
    }

    if receiver_is_kind_of(current, crate::nsobj::nsview_class_addr()) {
        let superview = crate::nsview::shim_nsview_superview(current, 0);
        if superview != 0 {
            return superview;
        }

        let window = crate::nsview::shim_nsview_window(current, 0);
        if window != 0 && window != current {
            return window;
        }
    }

    if receiver_is_kind_of(current, crate::nsobj::nswindow_class_addr()) {
        let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
        if app != current {
            return app;
        }
        return 0;
    }

    let next = shim_nsresponder_next_responder(current, 0);
    if next != 0 && next != current {
        return next;
    }

    0
}

fn dispatch_selector_named_to_target(target: u64, selector: &str, arg: u64) -> bool {
    if target == 0 || selector.is_empty() {
        return false;
    }

    let isa = unsafe { *(target as *const u64) };
    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return false;
    };

    let Some(imp) = rt.msg_lookup(isa, selector) else {
        return false;
    };

    let sel_ptr = rt.selector_ptr(selector).unwrap_or(0) as u64;
    drop(guard);

    let func: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
    func(target, sel_ptr, arg);
    true
}

fn forward_event_to_next_responder(obj: u64, selector: &str, event: u64) {
    if obj == 0 {
        return;
    }

    let previous_event = crate::nsapplication::push_current_event(event);
    let event_window = ensure_event_window(obj, event);

    let next = next_event_target(obj, event_window);
    if next != 0 && next != obj && dispatch_selector_named_to_target(next, selector, event) {
        crate::nsapplication::restore_previous_current_event(previous_event, event);
        return;
    }

    crate::nsapplication::restore_previous_current_event(previous_event, event);

    log::debug!("NSResponder {} (stub)", selector);
}

/// `-[NSResponder nextResponder]`
pub extern "C" fn shim_nsresponder_next_responder(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + 8) as *const u64) }
}

/// `-[NSResponder setNextResponder:]`
pub extern "C" fn shim_nsresponder_set_next_responder(obj: u64, _cmd: u64, next: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + 8) as *mut u64) = next; }
}

/// `-[NSResponder keyDown:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_key_down(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "keyDown:", event);
}

/// `-[NSResponder keyUp:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_key_up(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "keyUp:", event);
}

/// `-[NSResponder mouseDown:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_mouse_down(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "mouseDown:", event);
}

/// `-[NSResponder mouseUp:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_mouse_up(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "mouseUp:", event);
}

/// `-[NSResponder mouseMoved:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_mouse_moved(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "mouseMoved:", event);
}

/// `-[NSResponder mouseDragged:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_mouse_dragged(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "mouseDragged:", event);
}

/// `-[NSResponder rightMouseDown:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_right_mouse_down(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "rightMouseDown:", event);
}

/// `-[NSResponder rightMouseUp:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_right_mouse_up(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "rightMouseUp:", event);
}

/// `-[NSResponder rightMouseDragged:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_right_mouse_dragged(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "rightMouseDragged:", event);
}

/// `-[NSResponder flagsChanged:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_flags_changed(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "flagsChanged:", event);
}

/// `-[NSResponder scrollWheel:]` — forwards to next responder when available.
pub extern "C" fn shim_nsresponder_scroll_wheel(obj: u64, _cmd: u64, event: u64) {
    forward_event_to_next_responder(obj, "scrollWheel:", event);
}

/// `-[NSResponder tryToPerform:with:]`
pub extern "C" fn shim_nsresponder_try_to_perform_with(obj: u64, _cmd: u64, action: u64, arg: u64) -> u8 {
    if obj == 0 || action == 0 {
        return 0;
    }

    let mut current = obj;
    let mut depth = 0;
    while current != 0 && depth < 64 {
        if crate::nsapplication::target_responds_to_selector_ptr(action, current) {
            return u8::from(crate::nsapplication::dispatch_selector_ptr_to_target(action, current, arg));
        }

        let next = next_try_to_perform_target(current);
        if next == 0 || next == current {
            break;
        }

        current = next;
        depth += 1;
    }

    0
}

/// `-[NSResponder acceptsFirstResponder]` — returns NO by default.
pub extern "C" fn shim_nsresponder_accepts_first_responder(_obj: u64, _cmd: u64) -> u8 {
    0
}

/// `-[NSResponder becomeFirstResponder]` — returns YES by default.
pub extern "C" fn shim_nsresponder_become_first_responder(_obj: u64, _cmd: u64) -> u8 {
    1
}

/// `-[NSResponder resignFirstResponder]` — returns YES by default.
pub extern "C" fn shim_nsresponder_resign_first_responder(_obj: u64, _cmd: u64) -> u8 {
    1
}

/// `-[NSResponder performKeyEquivalent:]` — returns NO by default.
pub extern "C" fn shim_nsresponder_perform_key_equivalent(_obj: u64, _cmd: u64, _event: u64) -> u8 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    const SIZE: usize = 16;

    static RESPONDER_TEST_LOCK: Mutex<()> = Mutex::new(());
    static EVENT_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_EVENT_TARGET: AtomicU64 = AtomicU64::new(0);
    static LAST_EVENT_ARG: AtomicU64 = AtomicU64::new(0);
    static CURRENT_EVENT_DURING_CALL: AtomicU64 = AtomicU64::new(0);
    static EVENT_WINDOW_DURING_CALL: AtomicU64 = AtomicU64::new(0);
    static ACTION_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
    static LAST_ACTION_TARGET: AtomicU64 = AtomicU64::new(0);
    static LAST_ACTION_ARG: AtomicU64 = AtomicU64::new(0);

    const EVENT_PROBE_CLASS_ADDR: u64 = 0x0000_0001_0050_2000;

    extern "C" fn event_probe_key_down(obj: u64, _cmd: u64, event: u64) {
        EVENT_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        LAST_EVENT_TARGET.store(obj, Ordering::SeqCst);
        LAST_EVENT_ARG.store(event, Ordering::SeqCst);
        let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
        CURRENT_EVENT_DURING_CALL.store(
            crate::nsapplication::shim_nsapplication_current_event(app, 0),
            Ordering::SeqCst,
        );
        EVENT_WINDOW_DURING_CALL.store(
            crate::nsevent::shim_nsevent_window(event, 0),
            Ordering::SeqCst,
        );
    }

    extern "C" fn action_probe_imp(obj: u64, _cmd: u64, arg: u64) {
        ACTION_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        LAST_ACTION_TARGET.store(obj, Ordering::SeqCst);
        LAST_ACTION_ARG.store(arg, Ordering::SeqCst);
    }

    fn reset_event_probe() {
        EVENT_CALL_COUNT.store(0, Ordering::SeqCst);
        LAST_EVENT_TARGET.store(0, Ordering::SeqCst);
        LAST_EVENT_ARG.store(0, Ordering::SeqCst);
        CURRENT_EVENT_DURING_CALL.store(0, Ordering::SeqCst);
        EVENT_WINDOW_DURING_CALL.store(0, Ordering::SeqCst);
        ACTION_CALL_COUNT.store(0, Ordering::SeqCst);
        LAST_ACTION_TARGET.store(0, Ordering::SeqCst);
        LAST_ACTION_ARG.store(0, Ordering::SeqCst);
    }

    fn install_event_probe_class(rt: &mut objc_runtime::ObjcRuntime) {
        let mut info = objc_runtime::ClassInfo {
            name: "ResponderEventProbe".to_string(),
            mapped_addr: EVENT_PROBE_CLASS_ADDR,
            methods: vec![objc_runtime::MethodEntry {
                selector: "keyDown:".into(),
                imp: event_probe_key_down as *const () as u64,
                type_encoding: "v@:@".into(),
            }, objc_runtime::MethodEntry {
                selector: "performAction:".into(),
                imp: action_probe_imp as *const () as u64,
                type_encoding: "v@:@".into(),
            }],
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 16,
            is_root: false,
            is_meta: false,
            superclass_addr: Some(crate::nsobj::nsresponder_class_addr()),
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        };
        info.rebuild_indices();
        rt.register_synthetic_class(info);
    }

    fn with_responder_runtime<R>(f: impl FnOnce() -> R) -> R {
        let saved_runtime = {
            let mut guard = objc_runtime::OBJC_RUNTIME.write().unwrap_or_else(|e| e.into_inner());
            let saved = std::mem::take(&mut *guard);

            let mut rt = objc_runtime::ObjcRuntime::new();
            crate::FoundationShims::register_nsobject_class(&mut rt);
            crate::FoundationShims::register_nsresponder_class(&mut rt);
            crate::FoundationShims::register_nsapplication_class(&mut rt);
            crate::FoundationShims::register_nswindow_class(&mut rt);
            crate::FoundationShims::register_nsview_class(&mut rt);
            crate::FoundationShims::register_nsevent_class(&mut rt);
            install_event_probe_class(&mut rt);

            *guard = Some(rt);
            saved
        };

        let result = f();

        let mut guard = objc_runtime::OBJC_RUNTIME.write().unwrap_or_else(|e| e.into_inner());
        *guard = saved_runtime;
        result
    }

    fn intern_selector(name: &str) -> u64 {
        let mut guard = objc_runtime::OBJC_RUNTIME.write().unwrap_or_else(|e| e.into_inner());
        guard
            .as_mut()
            .expect("test ObjC runtime should be installed")
            .intern_selector(name) as u64
    }

    fn alloc_responder() -> u64 {
        alloc_obj(SIZE, crate::nsobj::nsresponder_class_addr())
    }

    // ── next_responder ──────────────────────────────────────────

    #[test]
    fn next_responder_default_is_zero() {
        let obj = alloc_responder();
        assert_eq!(shim_nsresponder_next_responder(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn next_responder_nil_returns_zero() {
        assert_eq!(shim_nsresponder_next_responder(0, 0), 0);
    }

    #[test]
    fn next_responder_returns_set_value() {
        let obj = alloc_responder();
        let next = alloc_responder();
        write_field(obj, 8, next);
        assert_eq!(shim_nsresponder_next_responder(obj, 0), next);
        free_obj(next);
        free_obj(obj);
    }

    // ── set_next_responder ──────────────────────────────────────

    #[test]
    fn set_next_responder_stores_value() {
        let obj = alloc_responder();
        let next = alloc_responder();
        shim_nsresponder_set_next_responder(obj, 0, next);
        assert_eq!(read_field(obj, 8), next);
        free_obj(next);
        free_obj(obj);
    }

    #[test]
    fn set_next_responder_nil_is_noop() {
        // Must not crash when obj is nil.
        shim_nsresponder_set_next_responder(0, 0, 42);
    }

    #[test]
    fn set_next_responder_clears_to_zero() {
        let obj = alloc_responder();
        let next = alloc_responder();
        shim_nsresponder_set_next_responder(obj, 0, next);
        shim_nsresponder_set_next_responder(obj, 0, 0);
        assert_eq!(shim_nsresponder_next_responder(obj, 0), 0);
        free_obj(next);
        free_obj(obj);
    }

    // ── responder chain walk ────────────────────────────────────

    #[test]
    fn responder_chain_walk() {
        let a = alloc_responder();
        let b = alloc_responder();
        let c = alloc_responder();
        shim_nsresponder_set_next_responder(a, 0, b);
        shim_nsresponder_set_next_responder(b, 0, c);
        // Walk: a -> b -> c -> 0
        assert_eq!(shim_nsresponder_next_responder(a, 0), b);
        assert_eq!(shim_nsresponder_next_responder(b, 0), c);
        assert_eq!(shim_nsresponder_next_responder(c, 0), 0);
        free_obj(c);
        free_obj(b);
        free_obj(a);
    }

    // ── key_down (stub) ─────────────────────────────────────────

    #[test]
    fn key_down_does_not_crash() {
        let obj = alloc_responder();
        shim_nsresponder_key_down(obj, 0, 0);
        free_obj(obj);
    }

    #[test]
    fn key_down_forwards_to_next_responder_and_sets_current_event() {
        let _guard = RESPONDER_TEST_LOCK.lock().unwrap();
        crate::nsapplication::reset_runtime_state_for_reuse();
        reset_event_probe();

        with_responder_runtime(|| {
            let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
            let obj = alloc_responder();
            let next = alloc_obj(SIZE, EVENT_PROBE_CLASS_ADDR);
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            shim_nsresponder_set_next_responder(obj, 0, next);
            shim_nsresponder_key_down(obj, 0, event);

            assert_eq!(EVENT_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_EVENT_TARGET.load(Ordering::SeqCst), next);
            assert_eq!(LAST_EVENT_ARG.load(Ordering::SeqCst), event);
            assert_eq!(CURRENT_EVENT_DURING_CALL.load(Ordering::SeqCst), event);
            assert_eq!(EVENT_WINDOW_DURING_CALL.load(Ordering::SeqCst), 0);
            assert_eq!(crate::nsapplication::shim_nsapplication_current_event(app, 0), event);

            free_obj(event);
            free_obj(next);
            free_obj(obj);
        });

        crate::nsapplication::reset_runtime_state_for_reuse();
    }

    #[test]
    fn key_down_nil_does_not_crash() {
        shim_nsresponder_key_down(0, 0, 0);
    }

    #[test]
    fn key_down_on_view_derives_event_window_from_receiver() {
        let _guard = RESPONDER_TEST_LOCK.lock().unwrap();
        crate::nsapplication::reset_runtime_state_for_reuse();

        with_responder_runtime(|| {
            let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
            let window = crate::nswindow::shim_nswindow_init_with_content_rect(
                0, 0, 10.0, 20.0, 300.0, 200.0, 1, 2, 0,
            );
            let content = crate::nswindow::shim_nswindow_content_view(window, 0);
            let view = crate::nsview::shim_nsview_init_with_frame(0, 0, 1.0, 2.0, 50.0, 40.0);
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            crate::nsview::shim_nsview_add_subview(content, 0, view);
            shim_nsresponder_key_down(view, 0, event);

            assert_eq!(crate::nsevent::shim_nsevent_window(event, 0), window);
            assert_eq!(crate::nsapplication::shim_nsapplication_current_event(app, 0), event);

            free_obj(event);
            free_obj(view);
            free_obj(window);
        });

        crate::nsapplication::reset_runtime_state_for_reuse();
    }

    #[test]
    fn key_down_without_next_uses_key_window_for_event_context() {
        let _guard = RESPONDER_TEST_LOCK.lock().unwrap();
        crate::nsapplication::reset_runtime_state_for_reuse();

        with_responder_runtime(|| {
            let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
            let window = crate::nswindow::shim_nswindow_init_with_content_rect(
                0, 0, 10.0, 20.0, 300.0, 200.0, 1, 2, 0,
            );
            let responder = alloc_responder();
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            crate::nswindow::shim_nswindow_make_key_and_order_front(window, 0, 0);
            shim_nsresponder_key_down(responder, 0, event);

            assert_eq!(crate::nsevent::shim_nsevent_window(event, 0), window);
            assert_eq!(crate::nsapplication::shim_nsapplication_current_event(app, 0), event);

            free_obj(event);
            free_obj(responder);
            free_obj(window);
        });

        crate::nsapplication::reset_runtime_state_for_reuse();
    }

    #[test]
    fn nested_event_dispatch_restores_previous_current_event() {
        let _guard = RESPONDER_TEST_LOCK.lock().unwrap();
        crate::nsapplication::reset_runtime_state_for_reuse();
        reset_event_probe();

        with_responder_runtime(|| {
            let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
            let obj = alloc_responder();
            let next = alloc_obj(SIZE, EVENT_PROBE_CLASS_ADDR);
            let previous_event = alloc_obj(64, crate::nsobj::nsevent_class_addr());
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            crate::nsapplication::set_current_event(previous_event);
            shim_nsresponder_set_next_responder(obj, 0, next);
            shim_nsresponder_key_down(obj, 0, event);

            assert_eq!(CURRENT_EVENT_DURING_CALL.load(Ordering::SeqCst), event);
            assert_eq!(crate::nsapplication::shim_nsapplication_current_event(app, 0), previous_event);

            free_obj(event);
            free_obj(previous_event);
            free_obj(next);
            free_obj(obj);
        });

        crate::nsapplication::reset_runtime_state_for_reuse();
    }

    // ── key_up (stub) ───────────────────────────────────────────

    #[test]
    fn key_up_does_not_crash() {
        let obj = alloc_responder();
        shim_nsresponder_key_up(obj, 0, 0);
        free_obj(obj);
    }

    #[test]
    fn key_up_nil_does_not_crash() {
        shim_nsresponder_key_up(0, 0, 0);
    }

    #[test]
    fn flags_changed_does_not_crash() {
        let obj = alloc_responder();
        shim_nsresponder_flags_changed(obj, 0, 0);
        free_obj(obj);
    }

    #[test]
    fn flags_changed_nil_does_not_crash() {
        shim_nsresponder_flags_changed(0, 0, 0);
    }

    // ── mouse_down (stub) ───────────────────────────────────────

    #[test]
    fn mouse_down_does_not_crash() {
        let obj = alloc_responder();
        shim_nsresponder_mouse_down(obj, 0, 0);
        free_obj(obj);
    }

    #[test]
    fn mouse_down_without_next_updates_current_event() {
        let _guard = RESPONDER_TEST_LOCK.lock().unwrap();
        crate::nsapplication::reset_runtime_state_for_reuse();

        with_responder_runtime(|| {
            let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
            let obj = alloc_responder();
            let event = alloc_obj(64, crate::nsobj::nsevent_class_addr());

            shim_nsresponder_mouse_down(obj, 0, event);

            assert_eq!(crate::nsapplication::shim_nsapplication_current_event(app, 0), event);

            free_obj(event);
            free_obj(obj);
        });

        crate::nsapplication::reset_runtime_state_for_reuse();
    }

    #[test]
    fn mouse_down_nil_does_not_crash() {
        shim_nsresponder_mouse_down(0, 0, 0);
    }

    #[test]
    fn mouse_moved_does_not_crash() {
        let obj = alloc_responder();
        shim_nsresponder_mouse_moved(obj, 0, 0);
        free_obj(obj);
    }

    #[test]
    fn mouse_moved_nil_does_not_crash() {
        shim_nsresponder_mouse_moved(0, 0, 0);
    }

    // ── mouse_up (stub) ─────────────────────────────────────────

    #[test]
    fn mouse_up_does_not_crash() {
        let obj = alloc_responder();
        shim_nsresponder_mouse_up(obj, 0, 0);
        free_obj(obj);
    }

    #[test]
    fn mouse_up_nil_does_not_crash() {
        shim_nsresponder_mouse_up(0, 0, 0);
    }

    #[test]
    fn scroll_wheel_does_not_crash() {
        let obj = alloc_responder();
        shim_nsresponder_scroll_wheel(obj, 0, 0);
        free_obj(obj);
    }

    #[test]
    fn scroll_wheel_nil_does_not_crash() {
        shim_nsresponder_scroll_wheel(0, 0, 0);
    }

    // ── becomeFirstResponder ────────────────────────────────────

    #[test]
    fn become_first_responder_returns_yes() {
        let obj = alloc_responder();
        assert_eq!(shim_nsresponder_become_first_responder(obj, 0), 1);
        free_obj(obj);
    }

    #[test]
    fn become_first_responder_nil_returns_yes() {
        assert_eq!(shim_nsresponder_become_first_responder(0, 0), 1);
    }

    // ── resignFirstResponder ────────────────────────────────────

    #[test]
    fn resign_first_responder_returns_yes() {
        let obj = alloc_responder();
        assert_eq!(shim_nsresponder_resign_first_responder(obj, 0), 1);
        free_obj(obj);
    }

    #[test]
    fn resign_first_responder_nil_returns_yes() {
        assert_eq!(shim_nsresponder_resign_first_responder(0, 0), 1);
    }

    // ── performKeyEquivalent ────────────────────────────────────

    #[test]
    fn perform_key_equivalent_returns_no() {
        let obj = alloc_responder();
        assert_eq!(shim_nsresponder_perform_key_equivalent(obj, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn perform_key_equivalent_nil_returns_no() {
        assert_eq!(shim_nsresponder_perform_key_equivalent(0, 0, 0), 0);
    }

    #[test]
    fn try_to_perform_dispatches_on_self() {
        let _guard = RESPONDER_TEST_LOCK.lock().unwrap();
        crate::nsapplication::reset_runtime_state_for_reuse();
        reset_event_probe();

        with_responder_runtime(|| {
            let obj = alloc_obj(SIZE, EVENT_PROBE_CLASS_ADDR);
            let action = intern_selector("performAction:");

            assert_eq!(shim_nsresponder_try_to_perform_with(obj, 0, action, 0xCAFE), 1);
            assert_eq!(ACTION_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_ACTION_TARGET.load(Ordering::SeqCst), obj);
            assert_eq!(LAST_ACTION_ARG.load(Ordering::SeqCst), 0xCAFE);

            free_obj(obj);
        });

        crate::nsapplication::reset_runtime_state_for_reuse();
    }

    #[test]
    fn try_to_perform_walks_next_responder_chain() {
        let _guard = RESPONDER_TEST_LOCK.lock().unwrap();
        crate::nsapplication::reset_runtime_state_for_reuse();
        reset_event_probe();

        with_responder_runtime(|| {
            let obj = alloc_responder();
            let next = alloc_obj(SIZE, EVENT_PROBE_CLASS_ADDR);
            let action = intern_selector("performAction:");

            shim_nsresponder_set_next_responder(obj, 0, next);
            assert_eq!(shim_nsresponder_try_to_perform_with(obj, 0, action, 0xBEEF), 1);
            assert_eq!(ACTION_CALL_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(LAST_ACTION_TARGET.load(Ordering::SeqCst), next);
            assert_eq!(LAST_ACTION_ARG.load(Ordering::SeqCst), 0xBEEF);

            free_obj(next);
            free_obj(obj);
        });

        crate::nsapplication::reset_runtime_state_for_reuse();
    }

    #[test]
    fn try_to_perform_returns_no_when_unhandled() {
        let _guard = RESPONDER_TEST_LOCK.lock().unwrap();
        crate::nsapplication::reset_runtime_state_for_reuse();

        with_responder_runtime(|| {
            let obj = alloc_responder();
            let action = intern_selector("performAction:");

            assert_eq!(shim_nsresponder_try_to_perform_with(obj, 0, action, 0xDEAD), 0);

            free_obj(obj);
        });

        crate::nsapplication::reset_runtime_state_for_reuse();
    }

    // ── accepts_first_responder ─────────────────────────────────

    #[test]
    fn accepts_first_responder_returns_no() {
        let obj = alloc_responder();
        assert_eq!(shim_nsresponder_accepts_first_responder(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn accepts_first_responder_nil_returns_no() {
        assert_eq!(shim_nsresponder_accepts_first_responder(0, 0), 0);
    }
}
