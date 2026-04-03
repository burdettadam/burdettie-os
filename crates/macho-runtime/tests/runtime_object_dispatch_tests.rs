//! NSObject methods, msgSendSuper, and dynamic NSString integration tests.

mod common;

// ── NSObject methods + metaclass dispatch tests (Phase 28, T13) ───

#[test]
fn runtime_nsobject_methods_exit_zero() {
    let Some(result) = common::run_fixture("nsobject_methods/app") else { return };

    assert_eq!(result.exit_code, 0, "nsobject_methods should exit 0");
}

#[test]
fn runtime_nsobject_methods_metaclass_dispatch() {
    let Some(_result) = common::run_fixture("nsobject_methods/app") else { return };

    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
    if let Some(rt) = guard.as_ref() {
        // Widget and BaseObj should be registered
        let widget = rt.get_class("Widget").expect("Widget should be registered");
        let base = rt
            .get_class("BaseObj")
            .expect("BaseObj should be registered");

        // Both should have superclass_addr pointing up the chain
        // Widget → BaseObj → NSObject (superclass chain, repaired post-fixup)
        assert!(
            widget.superclass_addr.is_some(),
            "Widget should have superclass"
        );
        assert!(
            base.superclass_addr.is_some(),
            "BaseObj should have superclass (NSObject)"
        );

        // NSObject should be registered as synthetic root
        let nsobject = rt
            .get_class("NSObject")
            .expect("NSObject should be registered");
        assert!(nsobject.superclass_addr.is_none(), "NSObject is root class");

        // Widget should find 'widgetId' (own method)
        let wid_imp = rt.msg_lookup(widget.mapped_addr, "widgetId");
        assert!(wid_imp.is_some(), "Widget should find 'widgetId'");

        // Widget should find 'baseValue' via superclass chain
        let bv_imp = rt.msg_lookup(widget.mapped_addr, "baseValue");
        assert!(
            bv_imp.is_some(),
            "Widget should find 'baseValue' via superclass"
        );

        // Widget should find 'init' via NSObject (two levels up)
        let init_imp = rt.msg_lookup(widget.mapped_addr, "init");
        assert!(
            init_imp.is_some(),
            "Widget should find 'init' from NSObject"
        );

        // Widget should find 'respondsToSelector:' via NSObject
        let rts_imp = rt.msg_lookup(widget.mapped_addr, "respondsToSelector:");
        assert!(
            rts_imp.is_some(),
            "Widget should find 'respondsToSelector:' from NSObject"
        );
    }
}

// ── objc_msgSendSuper / Super2 tests (Phase 29, T14) ──────────────

#[test]
fn runtime_objc_super_exit_zero() {
    let Some(result) = common::run_fixture("objc_super/app") else { return };

    assert_eq!(result.exit_code, 0, "objc_super should exit 0");
}

#[test]
fn runtime_objc_super_class_hierarchy() {
    let Some(_result) = common::run_fixture("objc_super/app") else { return };

    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
    if let Some(rt) = guard.as_ref() {
        // Three-level hierarchy: Puppy → Dog → Animal → NSObject
        let puppy = rt.get_class("Puppy").expect("Puppy should be registered");
        let dog = rt.get_class("Dog").expect("Dog should be registered");
        let animal = rt.get_class("Animal").expect("Animal should be registered");
        let nsobject = rt
            .get_class("NSObject")
            .expect("NSObject should be registered");

        // Superclass chain integrity
        assert!(
            puppy.superclass_addr.is_some(),
            "Puppy should have superclass"
        );
        assert!(dog.superclass_addr.is_some(), "Dog should have superclass");
        assert!(
            animal.superclass_addr.is_some(),
            "Animal should have superclass (NSObject)"
        );
        assert!(nsobject.superclass_addr.is_none(), "NSObject is root class");

        // Puppy should find 'legs' (own), 'sound' via Dog, 'init' via NSObject
        assert!(
            rt.msg_lookup(puppy.mapped_addr, "legs").is_some(),
            "Puppy.legs"
        );
        assert!(
            rt.msg_lookup(puppy.mapped_addr, "sound").is_some(),
            "Puppy.sound via Dog"
        );
        assert!(
            rt.msg_lookup(puppy.mapped_addr, "init").is_some(),
            "Puppy.init via NSObject"
        );

        // Dog should find 'legs' via Animal (not overridden)
        assert!(
            rt.msg_lookup(dog.mapped_addr, "legs").is_some(),
            "Dog.legs via Animal"
        );
    }
}

// ── T15: Dynamic NSString ──────────────────────────────────────────

#[test]
fn runtime_nsstring_dynamic_exit_zero() {
    let Some(result) = common::run_fixture("nsstring_dynamic/app") else { return };

    assert_eq!(result.exit_code, 0, "nsstring_dynamic should exit 0");
}

#[test]
fn runtime_nsstring_class_hierarchy() {
    let Some(_result) = common::run_fixture("nsstring_dynamic/app") else { return };

    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
    if let Some(rt) = guard.as_ref() {
        // NSObject → NSString → __NSCFConstantString
        let nsobject = rt.get_class("NSObject").expect("NSObject registered");
        let nsstring = rt.get_class("NSString").expect("NSString registered");
        let cfconst = rt
            .get_class("__NSCFConstantString")
            .expect("__NSCFConstantString registered");

        // NSString superclass = NSObject
        assert_eq!(
            nsstring.superclass_addr,
            Some(nsobject.mapped_addr),
            "NSString super should be NSObject"
        );

        // __NSCFConstantString superclass = NSString
        assert_eq!(
            cfconst.superclass_addr,
            Some(nsstring.mapped_addr),
            "__NSCFConstantString super should be NSString"
        );

        // NSString should have instance methods
        assert!(
            rt.msg_lookup(nsstring.mapped_addr, "length").is_some(),
            "NSString.length"
        );
        assert!(
            rt.msg_lookup(nsstring.mapped_addr, "UTF8String").is_some(),
            "NSString.UTF8String"
        );
        assert!(
            rt.msg_lookup(nsstring.mapped_addr, "initWithUTF8String:")
                .is_some(),
            "NSString.initWithUTF8String:"
        );
        assert!(
            rt.msg_lookup(nsstring.mapped_addr, "isEqualToString:")
                .is_some(),
            "NSString.isEqualToString:"
        );

        // __NSCFConstantString inherits NSString methods
        assert!(
            rt.msg_lookup(cfconst.mapped_addr, "length").is_some(),
            "CFConst inherits length"
        );
        assert!(
            rt.msg_lookup(cfconst.mapped_addr, "isEqualToString:")
                .is_some(),
            "CFConst inherits isEqualToString:"
        );
    }
}
