use super::*;
use arm64_translate::{CpuState, Interpreter};
use std::ffi::{CStr, CString};

#[path = "interp_printf_export_tests.rs"]
mod exports;

fn mapped(format: &CStr, strings: &[&CStr]) -> Memory {
    let mut segments = vec![(
        format.as_ptr() as u64,
        format.as_ptr().cast_mut().cast(),
        format.to_bytes_with_nul().len(),
        false,
    )];
    segments.extend(strings.iter().map(|value| {
        (
            value.as_ptr() as u64,
            value.as_ptr().cast_mut().cast(),
            value.to_bytes_with_nul().len(),
            false,
        )
    }));
    // SAFETY: callers keep these read-only strings live for the Memory's use.
    unsafe { Memory::from_segments(&segments) }
}

fn slots(memory: &Memory, arguments: &[u64]) {
    for (index, value) in arguments.iter().enumerate() {
        assert!(memory.write_u64(memory.initial_sp() + index as u64 * 8, *value));
    }
}

#[test]
fn explicit_stack_types_preserve_padding_unsigned_and_byte_strings() {
    let format = c"literal %% %d %d %lu %s";
    let string = c"\xffbytes";
    let memory = mapped(format, &[string]);
    slots(
        &memory,
        &[
            0x1122_3344_ffff_ffff,
            0xdead_beef_8000_0000,
            u64::MAX,
            string.as_ptr() as u64,
        ],
    );
    assert_eq!(
        format_printf(&memory, format.as_ptr() as u64, memory.initial_sp()),
        Ok(b"literal % -1 -2147483648 18446744073709551615 \xffbytes".to_vec())
    );
}

#[test]
fn rejects_unmapped_misaligned_unterminated_and_unsupported_inputs() {
    let memory = mapped(c"%d", &[]);
    slots(&memory, &[9]);
    assert_eq!(
        format_printf(&memory, 1, memory.initial_sp()),
        Err(PrintfError::FormatUnreadable)
    );
    assert_eq!(
        format_printf(&memory, c"%d".as_ptr() as u64, 1),
        Err(PrintfError::MisalignedStack)
    );
    assert_eq!(
        format_printf(&memory, c"%d".as_ptr() as u64, 0x1000),
        Err(PrintfError::StackUnreadable)
    );
    for format in [c"%n", c"%f", c"%lld", c"%u", c"%p", c"%*s", c"%1$d", c"%"] {
        let memory = mapped(format, &[]);
        assert_eq!(
            format_printf(&memory, format.as_ptr() as u64, memory.initial_sp()),
            Err(PrintfError::UnsupportedConversion),
            "{format:?} must not fall through to host formatting"
        );
    }
    let mut memory = mapped(c"%s", &[]);
    memory.host_passthrough = true;
    slots(&memory, &[c"live but unregistered".as_ptr() as u64]);
    assert_eq!(
        format_printf(&memory, c"%s".as_ptr() as u64, memory.initial_sp()),
        Err(PrintfError::StringUnreadable)
    );
    let unterminated = [b'x'; 8];
    // SAFETY: this exact bounded array remains live for the read.
    let memory =
        unsafe { Memory::from_segments(&[(0x4000, unterminated.as_ptr().cast_mut(), 8, false)]) };
    assert_eq!(
        format_printf(&memory, 0x4000, memory.initial_sp()),
        Err(PrintfError::FormatUnreadable)
    );
}

#[test]
fn format_argument_string_and_output_limits_are_enforced() {
    let huge_format = CString::new(vec![b'a'; MAX_FORMAT_BYTES + 1]).unwrap();
    let memory = mapped(&huge_format, &[]);
    assert_eq!(
        format_printf(&memory, huge_format.as_ptr() as u64, memory.initial_sp()),
        Err(PrintfError::FormatUnreadable)
    );

    let too_many = CString::new("%d".repeat(MAX_ARGUMENTS + 1)).unwrap();
    let memory = mapped(&too_many, &[]);
    // A caller reserving more than 32 slots must lower its SP; initial_sp()
    // intentionally leaves only 256 bytes above it in the interpreter stack.
    let stack = memory.initial_sp() - 512;
    for index in 0..MAX_ARGUMENTS {
        assert!(memory.write_u64(stack + index as u64 * 8, 1));
    }
    assert_eq!(
        format_printf(&memory, too_many.as_ptr() as u64, stack),
        Err(PrintfError::TooManyArguments)
    );

    let huge_string = CString::new(vec![
        b'x';
        arm64_translate::mem::MAX_EXPORTED_CSTRING_BYTES + 1
    ])
    .unwrap();
    let memory = mapped(c"%s", &[&huge_string]);
    slots(&memory, &[huge_string.as_ptr() as u64]);
    assert_eq!(
        format_printf(&memory, c"%s".as_ptr() as u64, memory.initial_sp()),
        Err(PrintfError::StringUnreadable)
    );

    let string =
        CString::new(vec![b'x'; arm64_translate::mem::MAX_EXPORTED_CSTRING_BYTES]).unwrap();
    let repeated = CString::new("%s".repeat(17)).unwrap();
    let memory = mapped(&repeated, &[&string]);
    slots(&memory, &[string.as_ptr() as u64; 17]);
    assert_eq!(
        format_printf(&memory, repeated.as_ptr() as u64, memory.initial_sp()),
        Err(PrintfError::OutputLimit)
    );
    let exact = CString::new("%s".repeat(16)).unwrap();
    let memory = mapped(&exact, &[&string]);
    slots(&memory, &[string.as_ptr() as u64; 16]);
    let output = format_printf(&memory, exact.as_ptr() as u64, memory.initial_sp()).unwrap();
    assert_eq!(output.len(), MAX_OUTPUT_BYTES);
    assert!(output.iter().all(|byte| *byte == b'x'));
}

extern "C" fn unrelated(
    first: u64,
    _a1: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
    _a7: u64,
) -> u64 {
    first + 1
}

fn instruction_bytes() -> Vec<u8> {
    [
        0xaa1e_03f3_u32, // MOV X19, LR
        0xd63f_0200,     // BLR X16
        0xaa13_03fe,     // MOV LR, X19
        0xd65f_03c0,     // RET
    ]
    .into_iter()
    .flat_map(u32::to_le_bytes)
    .collect()
}

fn interpreter(code: &mut [u8], format: &CStr) -> Interpreter {
    let base = code.as_mut_ptr() as u64;
    // SAFETY: caller keeps code and format live until the interpreter drops.
    let memory = unsafe {
        Memory::from_segments(&[
            (base, code.as_mut_ptr(), code.len(), false),
            (
                format.as_ptr() as u64,
                format.as_ptr().cast_mut().cast(),
                format.to_bytes_with_nul().len(),
                false,
            ),
        ])
    };
    let mut cpu = CpuState::new();
    cpu.pc = base;
    cpu.sp = memory.initial_sp();
    cpu.x[30] = 0;
    let bridge =
        crate::interp_bridge::build_interpreter_bridge(&[(base, base + code.len() as u64)]);
    let mut interpreter = Interpreter::new(cpu, memory, bridge);
    interpreter.set_instruction_limit(32);
    interpreter
}

fn invoke(interpreter: &mut Interpreter, code: u64, target: u64, args: &[u64]) -> ExitReason {
    interpreter.cpu.pc = code;
    interpreter.cpu.x[30] = 0;
    interpreter.cpu.x[16] = target;
    interpreter.cpu.x[..args.len()].copy_from_slice(args);
    interpreter.run()
}

fn markers(begin: bool) {
    // SAFETY: puts consumes the static NUL-terminated marker, and fflush(NULL)
    // flushes valid C streams. Each case runs in its own test subprocess.
    unsafe {
        libc::puts(
            if begin {
                c"BURDETTIE_PRINTF_BEGIN"
            } else {
                c"BURDETTIE_PRINTF_END"
            }
            .as_ptr(),
        );
        libc::fflush(std::ptr::null_mut());
    }
}

fn dispatch_case(case: &str) -> Vec<u8> {
    let mut code = instruction_bytes();
    let base = code.as_ptr() as u64;
    let format = if case == "reject" {
        c"before%n"
    } else {
        c"%d/%lu/%%"
    };
    let mut interpreter = interpreter(&mut code, format);
    let printf = super::super::host_symbol_addr(c"printf");
    assert_ne!(printf, 0);
    assert!(interpreter.ffi.is_ffi_target(printf));
    slots(&interpreter.mem, &[0xaabb_ccdd_ffff_ffff, u64::MAX]);
    let (target, first) = if case == "other-target" {
        let target = unrelated as *const () as u64;
        assert!(!interpreter.ffi.is_ffi_target(target));
        (target, 41)
    } else {
        (printf, format.as_ptr() as u64)
    };
    if case == "misaligned" {
        interpreter.cpu.sp += 8;
    }
    markers(true);
    let result = invoke(
        &mut interpreter,
        base,
        target,
        &[first, 8, 7, 6, 5, 4, 3, 2],
    );
    let expected = if case == "reject" || case == "misaligned" {
        assert!(
            matches!(result, ExitReason::FfiRejected { .. }),
            "{result:?}"
        );
        Vec::new()
    } else {
        assert!(
            matches!(result, ExitReason::ReturnedToHost { lr: 0 }),
            "{result:?}"
        );
        if case == "other-target" {
            assert_eq!(interpreter.cpu.x[0], 42);
            Vec::new()
        } else {
            let expected = b"-1/18446744073709551615/%".to_vec();
            assert_eq!(interpreter.cpu.x[0], expected.len() as u64);
            expected
        }
    };
    markers(false);
    expected
}

#[test]
fn real_printf_dispatch_uses_guest_stack_and_preserves_other_targets() {
    const CHILD: &str = "BURDETTIE_PRINTF_DISPATCH_CASE";
    if let Ok(case) = std::env::var(CHILD) {
        dispatch_case(&case);
        return;
    }
    for case in ["stack", "reject", "misaligned", "other-target"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "interp_bridge::printf::tests::real_printf_dispatch_uses_guest_stack_and_preserves_other_targets", "--nocapture"])
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
        let expected: &[u8] = if case == "stack" {
            b"-1/18446744073709551615/%"
        } else {
            b""
        };
        assert_eq!(&output.stdout[start..start + length], expected, "{case}");
    }
}
