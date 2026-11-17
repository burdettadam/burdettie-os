//! The real decoder, Objective-C lookup and return-register writeback must
//! preserve the known range result. Children isolate the global ObjC table
//! and turn accidental native execution of guest instructions into a failure.

use super::*;
use arm64_translate::{CpuState, ExitReason, Interpreter, Memory};
use cf_lite::FoundationShims;

const SELECTOR: &std::ffi::CStr = c"rangeOfCharacterFromSet:";
const NOT_FOUND: u64 = i64::MAX as u64;

extern "C" fn unrelated_integer_imp(
    _receiver: u64,
    _selector: u64,
    _argument: u64,
    _x3: u64,
    _x4: u64,
    _x5: u64,
    _x6: u64,
    _x7: u64,
) -> u64 {
    77
}

fn run_case(route: &str, case: &str) {
    let mut runtime = objc_runtime::ObjcRuntime::new();
    FoundationShims::register_nsstring_class(&mut runtime);
    FoundationShims::register_nsstring_proper_class(&mut runtime);
    let mut table = libsystem_shims::ShimTable::new();
    FoundationShims::install(&mut table);
    let constant_class = table.resolve("___CFConstantStringClassReference").unwrap();
    let string_class = runtime.get_class("NSString").unwrap().mapped_addr;

    let text = match case {
        "empty" => c"",
        "missing" => c"abcdef",
        _ => c"abc123def",
    };
    // Compiler-emitted CFString layout and the shim's predefined decimal set.
    // Both records and their backing text stay live through interpreter.run().
    let string = [
        constant_class,
        0x07c8,
        text.as_ptr() as u64,
        text.to_bytes().len() as u64,
    ];
    let charset = [cf_lite::nsobj::nscharacterset_class_addr(), 3_u64, 0, 0];
    let receiver = string.as_ptr() as u64;
    let selector = SELECTOR.as_ptr() as u64;

    let words = [
        0xaa1e_03f3_u32, // MOV X19, LR: preserve the caller's return address.
        0xd63f_0200,     // BLR X16: actual objc_msgSend/Super/Super2 handler.
        0xaa13_03fe,     // MOV LR, X19.
        0xd65f_03c0,     // RET entry.
        0xaa00_03e1,     // Guest override: MOV X1, X0 (prove receiver rewriting).
        0xd280_03a0,     // MOV X0, #29.
        0xd65f_03c0,     // RET guest method.
    ];
    let mut code: Vec<u8> = words.into_iter().flat_map(u32::to_le_bytes).collect();
    let base = code.as_mut_ptr() as u64;
    if case == "other-imp" || case == "guest-imp" {
        let method = runtime
            .get_class_mut("NSString")
            .unwrap()
            .methods
            .iter_mut()
            .find(|method| method.selector == "rangeOfCharacterFromSet:")
            .unwrap();
        method.imp = if case == "guest-imp" {
            base + 16
        } else {
            unrelated_integer_imp as *const () as u64
        };
        if case == "other-imp" {
            method.type_encoding = "Q@:@".into();
        }
    }
    *objc_runtime::OBJC_RUNTIME.write().unwrap() = Some(runtime);

    let superclass = if route == "super2" {
        constant_class
    } else {
        string_class
    };
    let super_record = [receiver, superclass];
    let (target, first_argument) = match route {
        "normal" => (objc_msg_send_addr(), receiver),
        "super" => (objc_msg_send_super_addr(), super_record.as_ptr() as u64),
        "super2" => (objc_msg_send_super2_addr(), super_record.as_ptr() as u64),
        _ => panic!("unknown dispatch route {route}"),
    };
    // SAFETY: code remains allocated and unchanged until the interpreter and
    // Memory are dropped. It contains only the source-owned instructions above.
    let memory = unsafe { Memory::from_segments(&[(base, code.as_mut_ptr(), code.len(), false)]) };
    let mut cpu = CpuState::new();
    cpu.pc = base;
    cpu.sp = memory.initial_sp();
    cpu.x[0] = first_argument;
    cpu.x[1] = selector;
    cpu.x[2] = charset.as_ptr() as u64;
    cpu.x[16] = target;
    let bridge = build_interpreter_bridge(&[(base, base + code.len() as u64)]);
    let mut interpreter = Interpreter::new(cpu, memory, bridge);
    interpreter.set_instruction_limit(32);
    assert!(matches!(
        interpreter.run(),
        ExitReason::ReturnedToHost { lr: 0 }
    ));

    let actual = (interpreter.cpu.x[0], interpreter.cpu.x[1]);
    match case {
        "found" => assert_eq!(actual, (3, 1), "{route} must retain both NSRange fields"),
        "missing" | "empty" => assert_eq!(actual, (NOT_FOUND, 0)),
        // A selector is not a signature: a different resolved IMP must retain
        // its existing integer-return route, with no guessed NSRange cast.
        "other-imp" => assert_eq!(actual.0, 77),
        // This must run as guest code, not as a native function pointer.
        "guest-imp" => assert_eq!(actual, (29, receiver)),
        _ => panic!("unknown range case {case}"),
    }
    println!("BURDETTIE_INTERPRETED_NSRANGE_PASSED:{route}:{case}");
}

#[test]
fn known_nsrange_imp_preserves_both_words_through_real_dispatch() {
    const CHILD: &str = "BURDETTIE_INTERPRETED_NSRANGE_CASE";
    if let Ok(case) = std::env::var(CHILD) {
        let (route, case) = case.split_once(':').unwrap();
        run_case(route, case);
        return;
    }

    let mut failures = Vec::new();
    for route in ["normal", "super", "super2"] {
        for case in ["found", "missing", "empty", "other-imp", "guest-imp"] {
            let case = format!("{route}:{case}");
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "interp_bridge::nsrange_tests::known_nsrange_imp_preserves_both_words_through_real_dispatch",
                    "--nocapture",
                ])
                .env(CHILD, &case)
                .output()
                .unwrap();
            if !output.status.success()
                || !String::from_utf8_lossy(&output.stdout)
                    .contains(&format!("BURDETTIE_INTERPRETED_NSRANGE_PASSED:{case}"))
            {
                failures.push(format!("{case}: {output:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "dispatch failures: {failures:#?}");
}
