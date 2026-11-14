//! Bridge between the ARM64 interpreter and the runtime's shim infrastructure.
//!
//! When executing a Mach-O binary via the interpreter, branches to external
//! symbols (ObjC runtime, Foundation, libc) must be routed to the host
//! implementations. This module builds an [`FfiBridge`] that:
//!
//! - Registers special handlers for `objc_msgSend` variants that perform
//!   IMP lookup and either tail-call to guest code or call host shims directly.
//! - Enables `auto_call_host` so all other resolved symbols (shims + libc)
//!   are called transparently through their function pointers.
//!
//! This lets the interpreter run ObjC and Foundation fixtures end-to-end.

use arm64_translate::{call_host_capturing, FfiBridge, FfiCallArgs, FfiResult};
use objc_runtime::shims::dispatch::{
    objc_msg_send_addr, objc_msg_send_fp2ret_addr, objc_msg_send_stret_addr,
    objc_msg_send_super2_addr, objc_msg_send_super2_stret_addr, objc_msg_send_super_addr,
    objc_msg_send_super_stret_addr,
};

#[cfg(unix)]
extern "C" {
    fn dlsym(
        handle: *mut std::ffi::c_void,
        symbol: *const std::ffi::c_char,
    ) -> *mut std::ffi::c_void;
}

fn host_symbol_addr(symbol: &std::ffi::CStr) -> u64 {
    #[cfg(unix)]
    unsafe {
        // SAFETY: looking up a well-known process symbol via dlsym using the
        // platform's global search handle (which is not null on Darwin).
        dlsym(libc::RTLD_DEFAULT, symbol.as_ptr()) as u64
    }

    #[cfg(not(unix))]
    {
        let _ = symbol;
        0
    }
}

/// Build an [`FfiBridge`] for interpreter use with the full runtime.
///
/// `guest_ranges` should be `(base_addr, end_addr)` pairs from the mapped
/// image segments. These are used to distinguish guest IMPs (tail-call back
/// into the interpreter) from host IMPs (call directly).
pub fn build_interpreter_bridge(guest_ranges: &[(u64, u64)]) -> FfiBridge {
    let mut bridge = FfiBridge::new();
    bridge.auto_call_host = true;

    // Register objc_msgSend variants with IMP-dispatch logic.
    register_msg_send(
        &mut bridge,
        objc_msg_send_addr(),
        "_objc_msgSend",
        guest_ranges,
    );
    register_msg_send(
        &mut bridge,
        objc_msg_send_stret_addr(),
        "_objc_msgSend_stret",
        guest_ranges,
    );
    register_msg_send(
        &mut bridge,
        objc_msg_send_fp2ret_addr(),
        "_objc_msgSend_fp2ret",
        guest_ranges,
    );
    register_msg_send_super(
        &mut bridge,
        objc_msg_send_super_addr(),
        "_objc_msgSendSuper",
        guest_ranges,
    );
    register_msg_send_super(
        &mut bridge,
        objc_msg_send_super_stret_addr(),
        "_objc_msgSendSuper_stret",
        guest_ranges,
    );
    register_msg_send_super2(
        &mut bridge,
        objc_msg_send_super2_addr(),
        "_objc_msgSendSuper2",
        guest_ranges,
    );
    register_msg_send_super2(
        &mut bridge,
        objc_msg_send_super2_stret_addr(),
        "_objc_msgSendSuper2_stret",
        guest_ranges,
    );

    // Host fallback and the lifecycle shim table have different addresses.
    // Both must stop the interpreter; auto_call_host must never terminate its
    // embedding process. Normal guest cleanup needs a separate, launch-scoped
    // callback mechanism and is not implied by returning FfiResult::Exit.
    for sym in &["exit", "_exit", "_Exit", "__exit", "__Exit"] {
        let c_name = std::ffi::CString::new(*sym).unwrap();
        let addr = host_symbol_addr(&c_name);
        if addr != 0 {
            let name = sym.to_string();
            bridge.register(addr, &name, Box::new(|args| FfiResult::Exit(args.x[0])));
        }
    }

    let mut lifecycle_shims = libsystem_shims::ShimTable::new();
    libsystem_shims::LifecycleShims::install(&mut lifecycle_shims);
    for sym in ["_exit", "__exit", "_Exit", "__Exit"] {
        if let Some(addr) = lifecycle_shims.resolve(sym) {
            bridge.register(addr, sym, Box::new(|args| FfiResult::Exit(args.x[0])));
        }
    }

    bridge
}

/// Register an `objc_msgSend` handler: look up IMP, then tail-call or direct-call.
fn register_msg_send(bridge: &mut FfiBridge, addr: u64, name: &str, guest_ranges: &[(u64, u64)]) {
    let ranges: Vec<(u64, u64)> = guest_ranges.to_vec();
    let handler_name = name.to_string();
    bridge.register(
        addr,
        name,
        Box::new(move |args: &FfiCallArgs| {
            let (obj, sel) = (args.x[0], args.x[1]);

            // Nil message → return 0 (matches Apple ABI).
            if obj == 0 {
                return FfiResult::Return(0);
            }

            let imp = objc_runtime::shims::dispatch::lookup::objc_msg_lookup_imp(obj, sel);
            log::debug!(
                "{}: obj={obj:#x} sel={sel:#x} → imp={imp:#x} (guest={})",
                handler_name,
                if imp != 0 {
                    is_in_guest(imp, &ranges)
                } else {
                    false
                },
            );
            if imp == 0 {
                log::warn!("objc_msgSend: lookup returned 0 for obj={obj:#x} sel={sel:#x}");
                return FfiResult::Return(0);
            }

            // If IMP is within guest memory, tail-call back into the interpreter.
            if is_in_guest(imp, &ranges) {
                return FfiResult::TailCall(imp);
            }

            // Host IMP — call directly with full ABI capture (integer + FP args).
            unsafe {
                let (x0, x1, d0, d1) =
                    call_host_capturing(imp, &args.x, args.host_x8, args.guest_sp, &args.d);
                FfiResult::ReturnFull { x0, x1, d0, d1 }
            }
        }),
    );
}

/// Register an `objc_msgSendSuper` handler.
/// x0 = &objc_super, x1 = SEL. The objc_super struct is { id receiver; Class super_class; }.
fn register_msg_send_super(
    bridge: &mut FfiBridge,
    addr: u64,
    name: &str,
    guest_ranges: &[(u64, u64)],
) {
    let ranges: Vec<(u64, u64)> = guest_ranges.to_vec();
    bridge.register(
        addr,
        name,
        Box::new(move |args: &FfiCallArgs| {
            let super_ptr = args.x[0];
            let sel = args.x[1];

            if super_ptr == 0 {
                return FfiResult::Return(0);
            }

            // Read the objc_super struct: { id receiver; Class super_class; }
            let receiver = unsafe { *(super_ptr as *const u64) };
            let super_class = unsafe { *((super_ptr + 8) as *const u64) };

            // Look up method on the superclass.
            let imp = objc_runtime::shims::dispatch::lookup::objc_msg_super_lookup_imp(
                receiver,
                super_class,
                sel,
            );
            if imp == 0 {
                return FfiResult::Return(0);
            }

            if is_in_guest(imp, &ranges) {
                return FfiResult::TailCallWithX0(imp, receiver);
            }

            unsafe {
                let super_args = [
                    receiver, sel, args.x[2], args.x[3], args.x[4], args.x[5], args.x[6], args.x[7],
                ];
                let (x0, x1, d0, d1) =
                    call_host_capturing(imp, &super_args, args.host_x8, args.guest_sp, &args.d);
                FfiResult::ReturnFull { x0, x1, d0, d1 }
            }
        }),
    );
}

/// Register an `objc_msgSendSuper2` handler (super_class is the class itself, not the superclass).
fn register_msg_send_super2(
    bridge: &mut FfiBridge,
    addr: u64,
    name: &str,
    guest_ranges: &[(u64, u64)],
) {
    let ranges: Vec<(u64, u64)> = guest_ranges.to_vec();
    bridge.register(
        addr,
        name,
        Box::new(move |args: &FfiCallArgs| {
            let super_ptr = args.x[0];
            let sel = args.x[1];

            if super_ptr == 0 {
                return FfiResult::Return(0);
            }

            let receiver = unsafe { *(super_ptr as *const u64) };
            let class_ptr = unsafe { *((super_ptr + 8) as *const u64) };

            // For msgSendSuper2, the second field is the CLASS (not superclass).
            // We need to get the superclass from the runtime.
            let imp = objc_runtime::shims::dispatch::lookup::objc_msg_super2_lookup_imp(
                receiver, class_ptr, sel,
            );
            if imp == 0 {
                return FfiResult::Return(0);
            }

            if is_in_guest(imp, &ranges) {
                return FfiResult::TailCallWithX0(imp, receiver);
            }

            unsafe {
                let super_args = [
                    receiver, sel, args.x[2], args.x[3], args.x[4], args.x[5], args.x[6], args.x[7],
                ];
                let (x0, x1, d0, d1) =
                    call_host_capturing(imp, &super_args, args.host_x8, args.guest_sp, &args.d);
                FfiResult::ReturnFull { x0, x1, d0, d1 }
            }
        }),
    );
}

/// Check if an address is within the guest (mapped image) memory ranges.
fn is_in_guest(addr: u64, ranges: &[(u64, u64)]) -> bool {
    ranges.iter().any(|&(lo, hi)| addr >= lo && addr < hi)
}

#[cfg(all(test, unix))]
mod exit_tests {
    use super::*;

    #[test]
    fn exit_symbols_stop_interpreter_without_terminating_host() {
        const CHILD_TARGET: &str = "BURDETTIE_INTERPRETER_EXIT_TARGET";
        if let Ok(target) = std::env::var(CHILD_TARGET) {
            let (provider, symbol) = target.split_once(':').unwrap();
            let address = if provider == "shim" {
                let mut table = libsystem_shims::ShimTable::new();
                libsystem_shims::LifecycleShims::install(&mut table);
                table.resolve(symbol).unwrap()
            } else {
                host_symbol_addr(&std::ffi::CString::new(symbol).unwrap())
            };
            assert_ne!(address, 0, "exit target {target} is missing");

            // Source-owned AArch64 BLR X16, with status in X0 and the actual
            // resolved shim/libc address in X16. Run the real decode/FFI path.
            let mut code = 0xd63f_0200_u32.to_le_bytes();
            let base = code.as_mut_ptr() as u64;
            // SAFETY: code stays alive and is never mutated while Memory and
            // the interpreter refer to its read-only instruction bytes.
            let memory = unsafe {
                arm64_translate::Memory::from_segments(&[(
                    base,
                    code.as_mut_ptr(),
                    code.len(),
                    false,
                )])
            };
            let mut cpu = arm64_translate::CpuState::new();
            cpu.pc = base;
            cpu.sp = memory.initial_sp();
            cpu.x[0] = 37;
            cpu.x[16] = address;
            let bridge = build_interpreter_bridge(&[(base, base + code.len() as u64)]);
            let mut interpreter = arm64_translate::Interpreter::new(cpu, memory, bridge);
            assert!(matches!(
                interpreter.run(),
                arm64_translate::ExitReason::Exit { code: 37 }
            ));
            println!("BURDETTIE_INTERPRETER_HOST_SURVIVED");
            return;
        }

        // A regression may call libc::_exit through auto_call_host. Isolate it
        // so the parent reports a test failure instead of silently exiting the
        // entire test harness with the guest's exit code.
        for target in [
            "shim:_exit",
            "shim:__exit",
            "shim:_Exit",
            "shim:__Exit",
            "host:exit",
            "host:_exit",
            "host:_Exit",
        ] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "interp_bridge::exit_tests::exit_symbols_stop_interpreter_without_terminating_host", "--nocapture"])
                .env(CHILD_TARGET, target)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{target} terminated its host: {output:?}"
            );
            assert!(
                String::from_utf8_lossy(&output.stdout)
                    .contains("BURDETTIE_INTERPRETER_HOST_SURVIVED"),
                "{target} did not complete the interpreter path: {output:?}"
            );
        }
    }
}
