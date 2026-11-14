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

#[cfg(all(unix, target_arch = "x86_64"))]
#[path = "interp_printf.rs"]
mod printf;

#[cfg(unix)]
extern "C" {
    fn dlsym(
        handle: *mut std::ffi::c_void,
        symbol: *const std::ffi::c_char,
    ) -> *mut std::ffi::c_void;
}

pub(crate) fn host_symbol_addr(symbol: &std::ffi::CStr) -> u64 {
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

    // No owner: exit is intercepted, but callback registration must fail.
    // The runtime pipelines replace these handlers with their launch registry.
    crate::exit_lifecycle::install_interpreter_lifecycle(&mut bridge, None, false);

    #[cfg(all(unix, target_arch = "x86_64"))]
    printf::install(&mut bridge);

    bridge
}

/// Call a resolved host IMP after the caller has excluded guest code.
///
/// # Safety
///
/// The receiver and arguments must be valid for the resolved implementation.
/// Unrecognized IMPs retain `call_host_capturing`'s existing ABI requirements
/// and limitations; this helper does not infer their signatures.
unsafe fn call_host_imp(imp: u64, receiver: u64, args: &FfiCallArgs) -> FfiResult {
    #[cfg(all(unix, target_arch = "x86_64"))]
    {
        let args = *args;
        FfiResult::with_interpreter(move |interpreter| {
            use cf_lite::cstring_observer::{
                with_cstring_observer, CStringObservation, CStringObserver,
            };
            let exports = interpreter.mem.cstring_exports();
            let observer: CStringObserver = std::sync::Arc::new(move |event| match event {
                CStringObservation::Created { object, value } => {
                    exports.observe_immutable(object, value);
                }
                CStringObservation::Invalidated { object } => exports.invalidate(object),
            });
            let (result, returned_string) = with_cstring_observer(observer, || {
                // SAFETY: the caller already excluded guest code and retains
                // the host shim's valid receiver/argument contract. This helper
                // matches an exact typed IMP; no pointer/length is reconstructed.
                if let Some(pointer) = unsafe {
                    cf_lite::FoundationShims::call_known_utf8string_imp(imp, receiver, args.x[1])
                } {
                    (FfiResult::Return(pointer), Some(pointer))
                } else {
                    // SAFETY: same resolved host-call contract, unchanged ABI.
                    (unsafe { call_host_imp_inner(imp, receiver, &args) }, None)
                }
            });
            if let Some(pointer) = returned_string {
                // Unobserved, mutable and other-interpreter strings do not
                // become exports. Preserve the getter's actual returned value.
                interpreter.mem.export_observed_cstring(receiver, pointer);
            }
            result
        })
    }
    #[cfg(not(all(unix, target_arch = "x86_64")))]
    // SAFETY: all native ARM64 and other existing host routes are unchanged.
    unsafe {
        call_host_imp_inner(imp, receiver, args)
    }
}

unsafe fn call_host_imp_inner(imp: u64, receiver: u64, args: &FfiCallArgs) -> FfiResult {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: the caller excluded guest code and supplies live arguments.
        // The adapter compares the actual IMP and calls only its known typed
        // implementation, never a selector-based or guessed function cast.
        if let Some((x0, x1)) = unsafe {
            cf_lite::FoundationShims::call_known_nsrange_imp(imp, receiver, args.x[1], args.x[2])
        } {
            return FfiResult::ReturnFull {
                x0,
                x1,
                d0: 0,
                d1: 0,
            };
        }
    }

    let mut host_args = args.x;
    host_args[0] = receiver;
    // SAFETY: this preserves the existing host-call contract for all other
    // IMPs. Super dispatch has replaced the objc_super pointer with receiver.
    let (x0, x1, d0, d1) =
        unsafe { call_host_capturing(imp, &host_args, args.host_x8, args.guest_sp, &args.d) };
    FfiResult::ReturnFull { x0, x1, d0, d1 }
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

            // SAFETY: lookup resolved the host implementation; guest code was
            // excluded above. The call retains its existing argument contract.
            unsafe { call_host_imp(imp, obj, args) }
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

            // SAFETY: the resolved IMP is outside guest code and receives the
            // receiver with the original selector and remaining arguments.
            unsafe { call_host_imp(imp, receiver, args) }
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

            // SAFETY: the resolved IMP is outside guest code and receives the
            // receiver with the original selector and remaining arguments.
            unsafe { call_host_imp(imp, receiver, args) }
        }),
    );
}

/// Check if an address is within the guest (mapped image) memory ranges.
fn is_in_guest(addr: u64, ranges: &[(u64, u64)]) -> bool {
    ranges.iter().any(|&(lo, hi)| addr >= lo && addr < hi)
}

#[cfg(all(test, unix, any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "interp_bridge_nsrange_tests.rs"]
mod nsrange_tests;

#[cfg(all(test, unix))]
mod exit_tests {
    use super::*;

    #[test]
    fn callback_registration_without_launch_never_reaches_host_atexit() {
        let bridge = build_interpreter_bridge(&[]);
        let mut table = libsystem_shims::ShimTable::new();
        libsystem_shims::LifecycleShims::install(&mut table);
        let args = FfiCallArgs {
            x: [0x1234, 0x5678, 0, 0, 0, 0, 0, 0],
            d: [0; 8],
            x8: 0,
            host_x8: 0,
            guest_sp: 0,
        };
        for (host, shim) in [(c"atexit", "_atexit"), (c"__cxa_atexit", "___cxa_atexit")] {
            let address = table.resolve(shim).unwrap();
            assert!(
                matches!(bridge.call_or_stub(address, &args), FfiResult::Return(result) if result as i32 == -1)
            );
            let address = host_symbol_addr(host);
            if address != 0 {
                assert!(
                    matches!(bridge.call_or_stub(address, &args), FfiResult::Return(result) if result as i32 == -1)
                );
            }
        }
    }

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
