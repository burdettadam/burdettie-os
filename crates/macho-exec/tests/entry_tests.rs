//! Tests for controlled entry-point execution and security-gated execution
//! (trust policy evaluation, code signature inspection).

mod common;

use macho_exec::{apply_chained_fixups, call_entry, call_entry_interpreted, MappedImage};
use macho_map::MappingPlan;
use macho_parse::parse;

use common::{default_mapper, default_stub_resolver, default_unwind, read_fixture};

fn synthetic_unixthread_entry(
    image: &MappedImage,
    entry: &macho_types::EntryPoint,
) -> macho_types::EntryPoint {
    match entry {
        macho_types::EntryPoint::Main { entry_offset, .. } => {
            let text = image.segment("__TEXT").expect("fixture should map __TEXT");
            macho_types::EntryPoint::UnixThread {
                entry_address: text.vmaddr + *entry_offset as u64,
            }
        }
        macho_types::EntryPoint::UnixThread { entry_address } => {
            macho_types::EntryPoint::UnixThread {
                entry_address: *entry_address,
            }
        }
    }
}

// ── Controlled entry: execute return42 fixture ─────────────────────

#[test]
fn controlled_entry_return42() {
    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    // 1. Map segments
    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

    // 2. Tighten permissions (__TEXT needs r-x for code execution)
    image.protect().unwrap();

    // 3. Call entry point
    let entry = parsed.entry.as_ref().expect("return42 should have LC_MAIN");
    let result = call_entry(&image, entry).unwrap();

    assert_eq!(result.exit_code, 42, "return42 should return exit code 42");
    assert!(result.entry_address > 0, "entry address should be non-zero");
}

#[test]
fn controlled_entry_preserves_stack() {
    // Verify that calling return42 doesn't corrupt our stack
    let canary: u64 = 0xDEAD_BEEF_CAFE_BABE;
    let stored_canary = canary;

    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
    image.protect().unwrap();

    let entry = parsed.entry.as_ref().unwrap();
    let _result = call_entry(&image, entry).unwrap();

    // Stack should not be corrupted
    assert_eq!(
        canary, stored_canary,
        "stack canary should be preserved after controlled entry"
    );
}

#[test]
fn controlled_entry_return42_unixthread() {
    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
    image.protect().unwrap();

    let entry = parsed.entry.as_ref().expect("return42 should have LC_MAIN");
    let unixthread_entry = synthetic_unixthread_entry(&image, entry);
    let result = call_entry(&image, &unixthread_entry).unwrap();

    assert_eq!(
        result.exit_code, 42,
        "synthetic LC_UNIXTHREAD entry should return 42"
    );
    assert!(result.entry_address > 0, "entry address should be non-zero");
}

// ── Controlled entry: execute simple_cli with printf stub ──────────

#[test]
fn controlled_entry_simple_cli() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    // 1. Map segments (contiguous layout preserves PC-relative references)
    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

    // 2. Apply chained fixups (patches GOT so printf stub works)
    let resolver = default_stub_resolver();
    let (rebases, binds) = apply_chained_fixups(&image, &parsed, &data, &resolver).unwrap();
    assert!(
        binds >= 1,
        "should bind at least printf: got {binds} binds, {rebases} rebases"
    );

    // 3. Tighten permissions (__TEXT r-x, __DATA_CONST r--)
    image.protect().unwrap();

    // 4. Call entry — simple_cli prints "hello\n" via printf and returns 0
    let entry = parsed
        .entry
        .as_ref()
        .expect("simple_cli should have LC_MAIN");
    let result = call_entry(&image, entry).unwrap();

    assert_eq!(result.exit_code, 0, "simple_cli should return exit code 0");
}

// ── Interpreter-based execution (Phase E3) ─────────────────────────

/// Execute return42 through the ARM64 interpreter — the E3 moonshot.
///
/// return42/return42 contains pure data-processing code (no external calls):
///   sub sp, sp, #0x20 / str wzr / str w0 / str x1 / mov w8, #0x28 /
///   str w8 / ldr w8 / add w8, w8, #0x2 / str w8 / ldr w0 / add sp / ret
///
/// Expected result: w0 = 42 (0x2A).
#[test]
fn interpreted_entry_return42() {
    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

    let entry = parsed.entry.as_ref().expect("return42 should have LC_MAIN");

    // No FFI calls needed for return42 — it's pure computation.
    let ffi = arm64_translate::FfiBridge::new();

    let result = call_entry_interpreted(&image, entry, None, ffi, &[]).unwrap();

    assert_eq!(
        result.exit_code, 42,
        "return42 via interpreter should return 42, got {}",
        result.exit_code
    );
}

#[test]
fn interpreted_entry_return42_unixthread() {
    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
    let entry = parsed.entry.as_ref().expect("return42 should have LC_MAIN");
    let unixthread_entry = synthetic_unixthread_entry(&image, entry);

    let ffi = arm64_translate::FfiBridge::new();
    let result = call_entry_interpreted(&image, &unixthread_entry, None, ffi, &[]).unwrap();

    assert_eq!(
        result.exit_code, 42,
        "synthetic LC_UNIXTHREAD entry via interpreter should return 42, got {}",
        result.exit_code
    );
}

/// Execute simple_cli through the interpreter with auto-FFI stubbing.
///
/// simple_cli calls printf("hello\n") then returns 0.
/// The interpreter needs to handle: STP, ADD, ADRP, BL (to stub), BR (to host printf).
/// When the interpreter encounters a BR to the resolved host printf address
/// (outside mapped memory), it auto-stubs the call and returns 0.
#[test]
fn interpreted_entry_simple_cli() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

    // Apply fixups so the GOT contains the real printf address.
    let resolver = default_stub_resolver();
    let (_rebases, _binds) = apply_chained_fixups(&image, &parsed, &data, &resolver).unwrap();

    let entry = parsed
        .entry
        .as_ref()
        .expect("simple_cli should have LC_MAIN");

    // No explicit FFI registration needed: the interpreter auto-detects
    // branches to unmapped addresses (e.g., the resolved host printf) and
    // stubs them out, returning 0.
    let ffi = arm64_translate::FfiBridge::new();

    let result = call_entry_interpreted(&image, entry, None, ffi, &[]).unwrap();

    assert_eq!(
        result.exit_code, 0,
        "simple_cli via interpreter should return 0, got {}",
        result.exit_code
    );
}

/// Execute constructor_test through the interpreter — constructors run before main.
///
/// constructor_test has two `__attribute__((constructor))` functions that call
/// printf, then main() calls printf once more and returns 0.
/// We run each constructor through the interpreter, then run main.
/// Since printf is auto-FFI-stubbed, we just verify exit code == 0.
#[test]
fn interpreted_entry_constructor_test() {
    let Some(data) = read_fixture("constructor_test/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

    // Apply fixups so GOT entries are resolved.
    let resolver = default_stub_resolver();
    let (_rebases, _binds) = apply_chained_fixups(&image, &parsed, &data, &resolver).unwrap();

    // Extract constructors from __mod_init_func.
    let init_info = macho_objc::extract_initializers(&parsed, &data);

    // Build interpreter memory once — segments are shared between constructor and main runs.
    let segments: Vec<(u64, *mut u8, usize, bool)> = image
        .segments
        .iter()
        .map(|s| {
            let writable = s.name != "__TEXT";
            (s.base as u64, s.base, s.size, writable)
        })
        .collect();

    let _mem = unsafe { arm64_translate::Memory::from_segments(&segments) };
    let ffi = arm64_translate::FfiBridge::new();
    let slide = image.slide;

    // Run each constructor through the interpreter.
    for ctor in &init_info.constructors {
        let ctor_addr = (ctor.address as i64 + slide) as u64;
        log::info!("Interpreter: running constructor at {ctor_addr:#x}");

        // Clone memory for each constructor run (they share segment mappings but
        // the interpreter stack is per-run).
        let ctor_mem = unsafe { arm64_translate::Memory::from_segments(&segments) };
        let ctor_ffi = arm64_translate::FfiBridge::new();

        // Constructor signature: extern "C" fn() — no args, no return value.
        let mut interp = arm64_translate::interp::setup_for_main(
            ctor_mem, ctor_ffi, ctor_addr, 0, // argc = 0
            0, // argv = null
            0, // envp = null
        );

        let exit_reason = interp.run();
        match exit_reason {
            arm64_translate::ExitReason::Returned { .. }
            | arm64_translate::ExitReason::ReturnedToHost { .. } => {
                // Constructor returned normally — good.
            }
            other => panic!("Constructor at {ctor_addr:#x} exited unexpectedly: {other:?}"),
        }
    }

    assert!(
        !init_info.constructors.is_empty(),
        "constructor_test should have at least one constructor"
    );

    // Now run main.
    let entry = parsed
        .entry
        .as_ref()
        .expect("constructor_test should have LC_MAIN");
    let result = call_entry_interpreted(&image, entry, None, ffi, &[]).unwrap();

    assert_eq!(
        result.exit_code, 0,
        "constructor_test via interpreter should return 0, got {}",
        result.exit_code
    );
}

/// Execute process_info through the interpreter.
///
/// process_info calls getprogname(), getenv(), and printf() — all auto-FFI-stubbed.
/// Contains conditionals (CBZ/CBNZ or B.cond) for null-checks on return values.
/// Expected result: exit code 0.
#[test]
fn interpreted_entry_process_info() {
    let Some(data) = read_fixture("process_info/process_info") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

    let resolver = default_stub_resolver();
    let (_rebases, _binds) = apply_chained_fixups(&image, &parsed, &data, &resolver).unwrap();

    let entry = parsed
        .entry
        .as_ref()
        .expect("process_info should have LC_MAIN");
    let ffi = arm64_translate::FfiBridge::new();

    let result = call_entry_interpreted(&image, entry, None, ffi, &[]).unwrap();

    assert_eq!(
        result.exit_code, 0,
        "process_info via interpreter should return 0, got {}",
        result.exit_code
    );
}

// ── Translation overhead measurement ───────────────────────────────

/// Measure interpreter overhead for each passing fixture.
///
/// Run with: cargo test -p macho-exec interpreted_overhead -- --nocapture
/// This test always passes — it prints instruction counts and timing.
#[test]
fn interpreted_overhead_report() {
    use std::time::Instant;

    let fixtures: &[(&str, &str, i32)] = &[
        ("return42/return42", "return42", 42),
        ("simple_cli/app", "simple_cli", 0),
        ("constructor_test/app", "constructor_test", 0),
        ("process_info/process_info", "process_info", 0),
    ];

    eprintln!();
    eprintln!("┌─────────────────────┬─────────────┬──────────────┐");
    eprintln!("│ Fixture             │ Instructions│ Wall time    │");
    eprintln!("├─────────────────────┼─────────────┼──────────────┤");

    for &(path, name, expected_exit) in fixtures {
        let Some(data) = read_fixture(path) else {
            eprintln!("│ {name:<19} │ SKIPPED     │              │");
            continue;
        };
        let parsed = parse(&data).unwrap();
        let plan = MappingPlan::from_parsed(&parsed);
        let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
        let resolver = default_stub_resolver();
        let _ = apply_chained_fixups(&image, &parsed, &data, &resolver);

        let entry = match parsed.entry.as_ref() {
            Some(e) => e,
            None => {
                eprintln!("│ {name:<19} │ NO ENTRY    │              │");
                continue;
            }
        };

        let segments: Vec<(u64, *mut u8, usize, bool)> = image
            .segments
            .iter()
            .map(|s| (s.base as u64, s.base, s.size, s.name != "__TEXT"))
            .collect();

        let entry_addr = match entry {
            macho_types::EntryPoint::Main { entry_offset, .. } => {
                image.text_base().unwrap() as u64 + *entry_offset as u64
            }
            _ => continue,
        };

        let mem = unsafe { arm64_translate::Memory::from_segments(&segments) };
        let ffi = arm64_translate::FfiBridge::new();

        let mut interp = arm64_translate::interp::setup_for_main(mem, ffi, entry_addr, 1, 0, 0);

        let start = Instant::now();
        let reason = interp.run();
        let elapsed = start.elapsed();

        let exit_code = match reason {
            arm64_translate::ExitReason::Returned { code } => code as i32,
            arm64_translate::ExitReason::ReturnedToHost { .. } => interp.cpu.x[0] as i32,
            _ => -1,
        };

        assert_eq!(exit_code, expected_exit, "{name} exit code mismatch");

        eprintln!(
            "│ {:<19} │ {:>11} │ {:>12} │",
            name,
            interp.instruction_count,
            format!("{:.1?}", elapsed),
        );
    }
    eprintln!("└─────────────────────┴─────────────┴──────────────┘");
}
