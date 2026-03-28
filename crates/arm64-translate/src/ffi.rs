//! FFI bridge: route ARM64 function calls to host functions.
//!
//! When the interpreter encounters a BL/BLR to an address outside the
//! mapped code segments (e.g., a resolved stub like `_puts` or `_exit`),
//! it uses this bridge to call the host function with the correct calling
//! convention translation.

use std::collections::HashMap;

/// Result of an FFI call from interpreted ARM64 code.
#[derive(Debug)]
pub enum FfiResult {
    /// Normal return: store value in x0, resume at LR.
    Return(u64),
    /// Full return capturing all ABI return registers (x0, x1, d0, d1).
    /// Used by auto_call_host to faithfully relay struct/FP returns.
    ReturnFull { x0: u64, x1: u64, d0: u64, d1: u64 },
    /// Exit the interpreter with this code.
    Exit(u64),
    /// Tail-call: redirect PC to this address without modifying registers.
    /// Used by `objc_msgSend` to jump to the resolved IMP in guest code.
    TailCall(u64),
    /// Tail-call with x0 override: redirect PC and set x0 to the given value.
    /// Used by `objc_msgSendSuper` to replace the `&objc_super` pointer in x0
    /// with the receiver before jumping to the resolved IMP in guest code.
    TailCallWithX0(u64, u64),
}

/// Arguments passed to a host function from interpreted ARM64 code.
pub struct FfiCallArgs {
    /// Integer registers x0-x7.
    pub x: [u64; 8],
    /// FP registers d0-d7 (as raw u64 bit patterns).
    pub d: [u64; 8],
    /// The guest's x8 register (indirect struct return pointer).
    pub x8: u64,
    /// Pre-translated host pointer for x8 (0 if x8==0 or untranslatable).
    pub host_x8: u64,
    /// Pre-translated host pointer to variadic stack arguments (0 if none).
    pub guest_sp: u64,
}

/// A host function that can be called from interpreted ARM64 code.
///
/// Receives the current register file (x0–x7 and d0-d7 contain arguments per
/// AAPCS64) and returns an [`FfiResult`].
pub type HostFn = Box<dyn Fn(&FfiCallArgs) -> FfiResult + Send>;

/// FFI bridge: maps guest code addresses to host function implementations.
pub struct FfiBridge {
    /// Map from guest target address → host function handler.
    handlers: HashMap<u64, HostFn>,
    /// Map from guest target address → name (for logging).
    names: HashMap<u64, String>,
    /// If true, call unregistered non-zero targets as host function pointers.
    /// This enables transparent forwarding to shim/libc functions resolved
    /// during fixups without requiring explicit registration of each one.
    pub auto_call_host: bool,
}

impl FfiBridge {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            names: HashMap::new(),
            auto_call_host: false,
        }
    }

    /// Register a host function at a guest address.
    pub fn register(&mut self, guest_addr: u64, name: impl Into<String>, handler: HostFn) {
        let name = name.into();
        log::debug!("FFI bridge: registered {} at {:#x}", name, guest_addr);
        self.names.insert(guest_addr, name);
        self.handlers.insert(guest_addr, handler);
    }

    /// Check if an address is a known FFI target.
    pub fn is_ffi_target(&self, addr: u64) -> bool {
        self.handlers.contains_key(&addr)
    }

    /// Call a host function with the given argument registers.
    /// Returns `Some(FfiResult)` if the address is registered,
    /// `None` if the address is not an FFI target.
    pub fn call(&self, addr: u64, args: &FfiCallArgs) -> Option<FfiResult> {
        let handler = self.handlers.get(&addr)?;
        let name = self.names.get(&addr).map(|s| s.as_str()).unwrap_or("???");
        log::trace!("FFI call: {} ({:#x}) args=[{:#x}, {:#x}, {:#x}, ...]",
            name, addr, args.x[0], args.x[1], args.x[2]);
        Some(handler(args))
    }

    /// Get the name of an FFI target (for diagnostics).
    pub fn name_of(&self, addr: u64) -> Option<&str> {
        self.names.get(&addr).map(|s| s.as_str())
    }

    /// Call a registered handler, or fall back.
    ///
    /// If `auto_call_host` is true, unregistered non-zero targets are called
    /// as host function pointers (safe when targets come from fixup resolution).
    /// Otherwise, returns `FfiResult::Return(0)` as a silent stub.
    ///
    /// `guest_x8`: the guest's x8 register (indirect struct return pointer).
    /// `guest_sp`: the guest's stack pointer (for variadic args on the stack).
    /// `fp_args`: the guest's d0-d7 FP register values.
    pub fn call_or_stub(&self, addr: u64, args: &FfiCallArgs) -> FfiResult {
        if let Some(result) = self.call(addr, args) {
            return result;
        }
        if self.auto_call_host && addr != 0 {
            log::debug!("FFI auto-call: host fn at {addr:#x}");
            // SAFETY: addr came from fixup resolution (CompositeResolver/StubResolver)
            // and is a valid host function pointer.
            unsafe {
                let (x0, x1, d0, d1) = call_host_capturing(addr, &args.x, args.host_x8, args.guest_sp, &args.d);
                return FfiResult::ReturnFull { x0, x1, d0, d1 };
            }
        }
        log::warn!("FFI stub: unknown target {addr:#x}, returning 0");
        FfiResult::Return(0)
    }
}

/// Call a host function and capture all ABI return registers (x0, x1, d0, d1).
///
/// On ARM64, integer results use x0/x1 and FP results use d0/d1.
/// Small structs may be spread across x0+x1 or d0+d1.
///
/// `guest_x8`: the guest's x8 register value. On ARM64, x8 is the indirect
/// result location register — functions returning large structs (>16 bytes)
/// write through the pointer in x8.
///
/// `guest_sp`: if non-zero, a host pointer to guest stack data. Copies 128
/// bytes from this address onto the host stack before the call. Required for
/// variadic C functions (printf, etc.) on ARM64 macOS, which read variadic
/// arguments from the stack rather than registers.
///
/// `fp_args`: the guest's d0-d7 FP register values. These are loaded into
/// the host d0-d7 before the call. Required for ObjC methods and C functions
/// that receive floating-point arguments (e.g., `[NSNumber numberWithDouble:]`).
#[cfg(target_arch = "aarch64")]
pub unsafe fn call_host_capturing(
    addr: u64,
    args: &[u64; 8],
    guest_x8: u64,
    guest_sp: u64,
    fp_args: &[u64; 8],
) -> (u64, u64, u64, u64) {
    let x0_ret: u64;
    let x1_ret: u64;
    let d0_ret: u64;
    let d1_ret: u64;
    unsafe {
        core::arch::asm!(
            // Allocate 128 bytes on host stack for variadic arguments.
            // On ARM64 macOS, variadic args are always passed on the stack.
            "sub sp, sp, #128",
            // If guest_sp is nonzero, copy guest stack data to the host stack.
            // This makes variadic functions (printf, snprintf, etc.) work
            // because they read va_args from the stack, not registers.
            "cbz x13, 2f",
            "ldp x11, x12, [x13]",
            "stp x11, x12, [sp]",
            "ldp x11, x12, [x13, #16]",
            "stp x11, x12, [sp, #16]",
            "ldp x11, x12, [x13, #32]",
            "stp x11, x12, [sp, #32]",
            "ldp x11, x12, [x13, #48]",
            "stp x11, x12, [sp, #48]",
            "ldp x11, x12, [x13, #64]",
            "stp x11, x12, [sp, #64]",
            "ldp x11, x12, [x13, #80]",
            "stp x11, x12, [sp, #80]",
            "ldp x11, x12, [x13, #96]",
            "stp x11, x12, [sp, #96]",
            "ldp x11, x12, [x13, #112]",
            "stp x11, x12, [sp, #112]",
            "2:",
            // Load FP arguments d0-d7 from the fp_args array.
            // x14 holds the base pointer to fp_args[].
            "ldr d0, [x14]",
            "ldr d1, [x14, #8]",
            "ldr d2, [x14, #16]",
            "ldr d3, [x14, #24]",
            "ldr d4, [x14, #32]",
            "ldr d5, [x14, #40]",
            "ldr d6, [x14, #48]",
            "ldr d7, [x14, #56]",
            "blr x16",
            // Restore host stack (always, regardless of guest_sp).
            "add sp, sp, #128",
            "fmov x9, d0",
            "fmov x10, d1",
            in("x13") guest_sp,
            in("x14") fp_args.as_ptr(),
            in("x16") addr,
            in("x8") guest_x8,
            in("x0") args[0],
            in("x1") args[1],
            in("x2") args[2],
            in("x3") args[3],
            in("x4") args[4],
            in("x5") args[5],
            in("x6") args[6],
            in("x7") args[7],
            lateout("x0") x0_ret,
            lateout("x1") x1_ret,
            lateout("x9") d0_ret,
            lateout("x10") d1_ret,
            lateout("x11") _,
            lateout("x12") _,
            lateout("x14") _,
            clobber_abi("C"),
        );
    }
    (x0_ret, x1_ret, d0_ret, d1_ret)
}

/// Fallback for non-aarch64 hosts: only captures x0 (integer return).
#[cfg(not(target_arch = "aarch64"))]
pub unsafe fn call_host_capturing(
    addr: u64,
    args: &[u64; 8],
    _guest_x8: u64,
    _guest_sp: u64,
    _fp_args: &[u64; 8],
) -> (u64, u64, u64, u64) {
    let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> u64 =
        std::mem::transmute(addr);
    let result = f(args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7]);
    (result, 0, 0, 0)
}

impl Default for FfiBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Register standard libc FFI handlers.
///
/// This sets up handlers for common C library functions that fixture binaries
/// call: `puts`, `printf`, `exit`, `strlen`, `memset`, etc.
///
/// The `resolved` iterator yields (guest_addr, symbol_name) pairs from the
/// stub resolver / shim table.
pub fn register_libc_handlers(
    bridge: &mut FfiBridge,
    resolved: impl IntoIterator<Item = (u64, String)>,
) {
    for (addr, name) in resolved {
        match name.as_str() {
            "_puts" | "puts" => {
                bridge.register(addr, &name, Box::new(|_args| {
                    FfiResult::Return(0)
                }));
            }
            "_exit" | "exit" => {
                bridge.register(addr, &name, Box::new(|args: &FfiCallArgs| {
                    FfiResult::Exit(args.x[0])
                }));
            }
            "_printf" | "printf" | "_fprintf" | "fprintf" => {
                bridge.register(addr, &name, Box::new(|_args| {
                    FfiResult::Return(0)
                }));
            }
            "_strlen" | "strlen" => {
                bridge.register(addr, &name, Box::new(|_args| {
                    FfiResult::Return(0)
                }));
            }
            "_malloc" | "malloc" | "_calloc" | "calloc" | "_free" | "free" => {
                bridge.register(addr, &name, Box::new(|_args| {
                    FfiResult::Return(0)
                }));
            }
            _ => {
                let sym = name.clone();
                bridge.register(addr, &name, Box::new(move |_args| {
                    log::debug!("FFI stub call: {} (returning 0)", sym);
                    FfiResult::Return(0)
                }));
            }
        }
    }
}
