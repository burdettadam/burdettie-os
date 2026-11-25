//! Real Objective-C dispatch creates, exports and retires owned string data.
use super::*;
use cf_lite::FoundationShims;
use objc_runtime::shims::dispatch::{
    objc_msg_send_addr, objc_msg_send_super2_addr, objc_msg_send_super_addr,
};

extern "C" fn other_getter(
    _receiver: u64,
    _selector: u64,
    value: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
    _a7: u64,
) -> u64 {
    value
}

fn require_return(reason: &ExitReason) {
    assert!(
        matches!(reason, ExitReason::ReturnedToHost { lr: 0 }),
        "{reason:?}"
    );
}

fn export_case(case: &str) {
    let mut runtime = objc_runtime::ObjcRuntime::new();
    FoundationShims::register_nsstring_class(&mut runtime);
    FoundationShims::register_nsstring_proper_class(&mut runtime);
    FoundationShims::register_nsmutablestring_class(&mut runtime);
    let constant_class = runtime
        .get_class("__NSCFConstantString")
        .unwrap()
        .mapped_addr;
    let string_class = runtime.get_class("NSString").unwrap().mapped_addr;
    let mutable_class = runtime.get_class("NSMutableString").unwrap().mapped_addr;
    *objc_runtime::OBJC_RUNTIME.write().unwrap() = Some(runtime);

    let text = c"  hello  ";
    let source = [
        constant_class,
        0x07c8,
        text.as_ptr() as u64,
        text.to_bytes().len() as u64,
    ];
    let charset = [cf_lite::nsobj::nscharacterset_class_addr(), 1_u64, 0, 0];
    let format = c"%s";
    let mut code = instruction_bytes();
    code.extend(
        [0xaa02_03e0_u32, 0xd65f_03c0]
            .into_iter()
            .flat_map(u32::to_le_bytes),
    );
    let callback_offset = code.len();
    code.extend(
        [
            0xaa1e_03f6_u32, // MOV X22, LR
            0xaa14_03e0,     // MOV X0, X20 (format)
            0xf900_03f5,     // STR X21, [SP] (variadic string pointer)
            0xd63f_0200,     // BLR X16 (printf)
            0xaa16_03fe,     // MOV LR, X22
            0xd65f_03c0,     // RET callback
        ]
        .into_iter()
        .flat_map(u32::to_le_bytes),
    );
    let base = code.as_ptr() as u64;
    let mut interpreter = interpreter(&mut code, format);
    let msg = objc_msg_send_addr();
    let receiver = if case == "unobserved" {
        source.as_ptr() as u64
    } else if case == "mutable" {
        require_return(&invoke(
            &mut interpreter,
            base,
            msg,
            &[mutable_class, c"string".as_ptr() as u64],
        ));
        interpreter.cpu.x[0]
    } else {
        require_return(&invoke(
            &mut interpreter,
            base,
            msg,
            &[
                source.as_ptr() as u64,
                c"stringByTrimmingCharactersInSet:".as_ptr() as u64,
                charset.as_ptr() as u64,
            ],
        ));
        interpreter.cpu.x[0]
    };
    assert_ne!(receiver, 0);
    if case == "earlier-interpreter" {
        // Replace the entire interpreter/collector, not just the stack cursor.
        interpreter = super::interpreter(&mut code, format);
    }

    let getter = c"UTF8String".as_ptr() as u64;
    let known_imp = {
        let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
        guard
            .as_ref()
            .unwrap()
            .get_class("__NSCFConstantString")
            .unwrap()
            .methods
            .iter()
            .find(|method| method.selector == "UTF8String")
            .unwrap()
            .imp
    };
    // SAFETY: receiver is the live source record or a real constructor result.
    // This typed call obtains its ordinary pointer, but bypasses interpreter
    // dispatch deliberately: a getter result alone must not authorize an export.
    let original =
        unsafe { FoundationShims::call_known_utf8string_imp(known_imp, receiver, getter) }.unwrap();
    assert_ne!(original, 0);
    assert!(interpreter.mem.copy_cstring_bounded(original, 32).is_err());

    if case == "other-imp" || case == "guest-imp" {
        let mut guard = objc_runtime::OBJC_RUNTIME.write().unwrap();
        let method = guard
            .as_mut()
            .unwrap()
            .get_class_mut("__NSCFConstantString")
            .unwrap()
            .methods
            .iter_mut()
            .find(|method| method.selector == "UTF8String")
            .unwrap();
        method.imp = if case == "guest-imp" {
            base + 16
        } else {
            other_getter as *const () as u64
        };
        // Keep the same selector and type encoding: neither is authority.
    }
    let super_record = [
        receiver,
        if case == "super2" {
            constant_class
        } else {
            string_class
        },
    ];
    let (target, first) = match case {
        "super" => (objc_msg_send_super_addr(), super_record.as_ptr() as u64),
        "super2" => (objc_msg_send_super2_addr(), super_record.as_ptr() as u64),
        _ => (msg, receiver),
    };
    require_return(&invoke(
        &mut interpreter,
        base,
        target,
        &[first, getter, original],
    ));
    let returned = interpreter.cpu.x[0];
    assert_eq!(
        returned, original,
        "export must preserve the actual getter pointer"
    );
    assert!(
        !interpreter.mem.is_valid(returned),
        "exported bytes are not executable mappings"
    );

    if matches!(
        case,
        "reinitialize" | "bytes-reinitialize" | "mutate" | "nested-reinitialize"
    ) {
        assert_eq!(
            interpreter.mem.copy_cstring_bounded(returned, 5).unwrap(),
            b"hello"
        );
        let replacement = c"after";
        let args = if case == "reinitialize" {
            vec![
                receiver,
                c"initWithUTF8String:".as_ptr() as u64,
                replacement.as_ptr() as u64,
            ]
        } else if case == "bytes-reinitialize" {
            vec![
                receiver,
                c"initWithBytes:length:encoding:".as_ptr() as u64,
                replacement.as_ptr() as u64,
                5,
                4,
            ]
        } else if case == "nested-reinitialize" {
            use cf_lite::cstring_observer::{
                with_cstring_observer, CStringObservation, CStringObserver,
            };
            let exports = interpreter.mem.cstring_exports();
            let outer: CStringObserver = std::sync::Arc::new(move |event| match event {
                CStringObservation::Created { object, value } => {
                    exports.observe_immutable(object, value);
                }
                CStringObservation::Invalidated { object } => exports.invalidate(object),
            });
            let mut nested_code = instruction_bytes();
            let nested_base = nested_code.as_ptr() as u64;
            let mut nested = super::interpreter(&mut nested_code, format);
            // Model a suspended outer host call entering a second interpreter.
            // The real nested IMP creates its own inner observer. Reinitializing
            // this object must retire the enclosing interpreter's export too.
            with_cstring_observer(outer, || {
                require_return(&invoke(
                    &mut nested,
                    nested_base,
                    msg,
                    &[
                        receiver,
                        c"initWithUTF8String:".as_ptr() as u64,
                        replacement.as_ptr() as u64,
                    ],
                ));
            });
            Vec::new()
        } else {
            // Exercise the common mutable replacement with a layout-valid
            // CString-backed receiver. It must retire the old snapshot before
            // freeing that backing, even if the object is repurposed this way.
            let mutable_super = [receiver, mutable_class];
            require_return(&invoke(
                &mut interpreter,
                base,
                objc_msg_send_super_addr(),
                &[
                    mutable_super.as_ptr() as u64,
                    c"setString:".as_ptr() as u64,
                    source.as_ptr() as u64,
                ],
            ));
            Vec::new()
        };
        if !args.is_empty() {
            require_return(&invoke(&mut interpreter, base, msg, &args));
        }
        assert!(interpreter.mem.copy_cstring_bounded(returned, 32).is_err());
    }

    markers(true);
    let positive = matches!(case, "normal" | "super" | "super2" | "cleanup");
    if case == "cleanup" {
        use libsystem_shims::exit_registry::{
            ExitCallbackAbi, ExitImageRange, ExitRoute, LaunchExitScope,
        };
        // SAFETY: the scope borrows all source-owned callback code. The
        // interpreter, snapshots and mappings stay live through its drain.
        let scope = unsafe {
            LaunchExitScope::enter(
                &code,
                ExitRoute::Arm64Interpreter,
                vec![ExitImageRange {
                    image: 0,
                    start: base,
                    end: base + code.len() as u64,
                    executable: true,
                }],
            )
        };
        let registry = scope.registry();
        assert_eq!(
            registry.register(base + callback_offset as u64, ExitCallbackAbi::Plain, 0, 0),
            0
        );
        crate::exit_lifecycle::install_interpreter_lifecycle(
            &mut interpreter.ffi,
            Some(registry),
            true,
        );
        interpreter.cpu.pc = base + 12; // main RET enters the actual exit drain.
        interpreter.cpu.x[30] = 0;
        interpreter.cpu.x[0] = 0;
        interpreter.cpu.x[16] = super::super::super::host_symbol_addr(c"printf");
        interpreter.cpu.x[20] = format.as_ptr() as u64;
        interpreter.cpu.x[21] = returned;
        require_return(&interpreter.run());
        assert!(
            interpreter.instruction_count > 1,
            "cleanup actually executed guest callback instructions"
        );
    } else {
        slots(&interpreter.mem, &[returned]);
        let reason = invoke(
            &mut interpreter,
            base,
            super::super::super::host_symbol_addr(c"printf"),
            &[format.as_ptr() as u64],
        );
        if positive {
            require_return(&reason);
            assert_eq!(interpreter.cpu.x[0], 5);
        } else {
            assert!(
                matches!(reason, ExitReason::FfiRejected { .. }),
                "{case}: {reason:?}"
            );
        }
    }
    markers(false);
}

#[test]
fn immutable_exports_require_creation_and_known_getter_and_survive_cleanup() {
    const CHILD: &str = "BURDETTIE_PRINTF_EXPORT_CASE";
    if let Ok(case) = std::env::var(CHILD) {
        export_case(&case);
        return;
    }
    for case in [
        "normal",
        "super",
        "super2",
        "cleanup",
        "unobserved",
        "mutable",
        "earlier-interpreter",
        "other-imp",
        "guest-imp",
        "reinitialize",
        "bytes-reinitialize",
        "mutate",
        "nested-reinitialize",
    ] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "interp_bridge::printf::tests::exports::immutable_exports_require_creation_and_known_getter_and_survive_cleanup", "--nocapture"])
            .env(CHILD, case).output().unwrap();
        assert!(output.status.success(), "{case}: {output:?}");
        let begin = b"BURDETTIE_PRINTF_BEGIN\n";
        let start = output
            .stdout
            .windows(begin.len())
            .position(|bytes| bytes == begin)
            .unwrap()
            + begin.len();
        let end = b"BURDETTIE_PRINTF_END\n";
        let length = output.stdout[start..]
            .windows(end.len())
            .position(|bytes| bytes == end)
            .unwrap();
        let expected: &[u8] = if matches!(case, "normal" | "super" | "super2" | "cleanup") {
            b"hello"
        } else {
            b""
        };
        assert_eq!(&output.stdout[start..start + length], expected, "{case}");
    }
}
