//! Cleanup ownership and execution for the two direct runtime pipelines.

use std::sync::Arc;

use arm64_translate::{ExitReason, FfiBridge, FfiResult, Interpreter};
use libsystem_shims::exit_registry::{
    ExitCallbackAbi, ExitImageRange, ExitKind, ExitRoute, LaunchExitRegistry, LaunchExitScope,
};
use libsystem_shims::LifecycleShims;
use macho_exec::ImageRegistry;
use macho_types::{segment::VmProt, CpuType};

use crate::RuntimeError;

#[cfg(all(test, unix))]
#[path = "exit_lifecycle_tests.rs"]
mod tests;

/// Borrow the complete fixed image set, before any guest initializer runs.
/// Dynamic image unloading and guest-thread ownership are not supported here.
pub(crate) fn enter_launch(
    images: &ImageRegistry,
    route: ExitRoute,
) -> Result<LaunchExitScope<'_>, RuntimeError> {
    let expected = match route {
        ExitRoute::Arm64Interpreter => Some(CpuType::Arm64),
        ExitRoute::Native if cfg!(target_arch = "aarch64") => Some(CpuType::Arm64),
        ExitRoute::Native if cfg!(target_arch = "x86_64") => Some(CpuType::X86_64),
        ExitRoute::Native => None,
    };
    for image in images.images() {
        if Some(image.parsed.header.cpu_type) != expected {
            return Err(RuntimeError::LifecycleUnsupported {
                reason: format!(
                    "image {} has {:?} code, incompatible with {route:?} on this host",
                    image.install_name, image.parsed.header.cpu_type,
                ),
            });
        }
    }
    let ranges = images
        .images()
        .iter()
        .enumerate()
        .flat_map(|(image, loaded)| {
            loaded
                .image
                .segments
                .iter()
                .map(move |segment| ExitImageRange {
                    image,
                    start: segment.base as u64,
                    end: (segment.base as u64).saturating_add(segment.size as u64),
                    executable: segment.protection.contains(VmProt::EXECUTE),
                })
        })
        .collect();
    // SAFETY: this guard borrows the fixed ImageRegistry until after cleanup.
    // Both pipelines retain all images and protect executable segments before
    // invoking code. Every image's architecture was checked for this route.
    Ok(unsafe { LaunchExitScope::enter(images, route, ranges) })
}

pub(crate) fn complete_native(registry: &LaunchExitRegistry, status: i32) {
    registry.request_exit(ExitKind::Normal, status);
    registry.finalize_native(None);
    registry.discard();
    LifecycleShims::flush_stdio();
}

/// Install handlers at both shim and host fallback addresses. With no owner,
/// registration fails instead of leaking a guest pointer into host atexit.
pub(crate) fn install_interpreter_lifecycle(
    bridge: &mut FfiBridge,
    registry: Option<Arc<LaunchExitRegistry>>,
    finish_on_return: bool,
) {
    let mut table = libsystem_shims::ShimTable::new();
    LifecycleShims::install(&mut table);

    for (host, shim, kind) in [
        ("exit", "_exit", ExitKind::Normal),
        ("_exit", "__exit", ExitKind::Immediate),
        ("_Exit", "__Exit", ExitKind::Immediate),
        ("__exit", "_Exit", ExitKind::Immediate),
        ("__Exit", "__Exit", ExitKind::Immediate),
    ] {
        for address in addresses(host, shim, &table) {
            let registry = registry.clone();
            bridge.register(
                address,
                shim,
                Box::new(move |args| {
                    let status = args.x[0] as i32;
                    if let Some(registry) = &registry {
                        registry.request_exit(kind, status);
                    }
                    FfiResult::Exit(status as u64)
                }),
            );
        }
    }

    for (host, shim, abi) in [
        ("atexit", "_atexit", ExitCallbackAbi::Plain),
        ("__cxa_atexit", "___cxa_atexit", ExitCallbackAbi::Cxa),
    ] {
        for address in addresses(host, shim, &table) {
            let registry = registry.clone();
            bridge.register(
                address,
                shim,
                Box::new(move |args| {
                    let (argument, dso) = match abi {
                        ExitCallbackAbi::Plain => (0, 0),
                        ExitCallbackAbi::Cxa => (args.x[1], args.x[2]),
                    };
                    let result = registry.as_ref().map_or(-1, |registry| {
                        registry.register(args.x[0], abi, argument, dso)
                    });
                    FfiResult::Return(result as u64)
                }),
            );
        }
    }

    for address in addresses("__cxa_finalize", "___cxa_finalize", &table) {
        let registry = registry.clone();
        bridge.register(
            address,
            "___cxa_finalize",
            Box::new(move |args| {
                let Some(registry) = registry.clone() else {
                    return FfiResult::Return(0);
                };
                let dso = (args.x[0] != 0).then_some(args.x[0]);
                FfiResult::with_interpreter(move |interpreter| {
                    match finalize_interpreted(interpreter, &registry, dso) {
                        Ok(()) => FfiResult::Return(0),
                        Err(reason) => FfiResult::Stop(reason),
                    }
                })
            }),
        );
    }

    for address in addresses("abort", "_abort", &table) {
        let registry = registry.clone();
        bridge.register(
            address,
            "_abort",
            Box::new(move |_| {
                if let Some(registry) = &registry {
                    registry.request_exit(ExitKind::Immediate, 1);
                }
                FfiResult::Exit(1)
            }),
        );
    }

    if let Some(registry) = registry {
        bridge.on_completion(move |interpreter, reason| {
            complete_interpreted(interpreter, &registry, reason, finish_on_return)
        });
    }
}

fn addresses(host: &str, shim: &str, table: &libsystem_shims::ShimTable) -> Vec<u64> {
    let name = std::ffi::CString::new(host).expect("static lifecycle symbol");
    let host_address = crate::interp_bridge::host_symbol_addr(&name);
    [Some(host_address), table.resolve(shim)]
        .into_iter()
        .flatten()
        .filter(|address| *address != 0)
        .collect()
}

fn finalize_interpreted(
    interpreter: &mut Interpreter,
    registry: &LaunchExitRegistry,
    dso: Option<u64>,
) -> Result<(), ExitReason> {
    while let Some(callback) = registry.take_next(ExitRoute::Arm64Interpreter, dso) {
        // The callback was removed before invocation. This is the suspended
        // interpreter itself: its stack, argument memory and mapped images all
        // remain alive, and no native function-pointer cast occurs.
        match interpreter.call_guest_callback(callback.address, callback.argument) {
            ExitReason::Returned { .. } => {}
            reason @ ExitReason::Exit { .. } => return Err(reason),
            reason => {
                registry.discard();
                return Err(reason);
            }
        }
    }
    Ok(())
}

fn complete_interpreted(
    interpreter: &mut Interpreter,
    registry: &LaunchExitRegistry,
    reason: ExitReason,
    finish_on_return: bool,
) -> ExitReason {
    match reason {
        ExitReason::Returned { .. } | ExitReason::ReturnedToHost { .. } if !finish_on_return => {
            return reason;
        }
        ExitReason::Returned { .. }
        | ExitReason::ReturnedToHost { .. }
        | ExitReason::Exit { .. } => {}
        _ => {
            registry.discard();
            return reason;
        }
    }

    loop {
        if let Some((ExitKind::Immediate, code)) = registry.requested_exit() {
            return ExitReason::Exit {
                code: i64::from(code),
            };
        }
        match finalize_interpreted(interpreter, registry, None) {
            Ok(()) => break,
            // A callback's recursive normal exit replaces the status and
            // abandons that callback's continuation. Drain remaining entries.
            Err(ExitReason::Exit { .. }) if registry.requested_exit().is_some() => {}
            Err(reason) => {
                registry.discard();
                return reason;
            }
        }
    }
    let result = match registry.requested_exit() {
        Some((_, code)) => ExitReason::Exit {
            code: i64::from(code),
        },
        None => reason,
    };
    registry.discard();
    LifecycleShims::flush_stdio();
    result
}
