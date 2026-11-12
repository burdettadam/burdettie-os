//! Controlled entry: call the mapped binary's entry point.
//!
//! After segments are mapped and fixups applied, we compute the entry
//! address from LC_MAIN's offset relative to __TEXT and call it.
//!
//! # Safety
//!
//! This jumps into mapped code. Only use with known-safe fixture binaries.
//! The caller is responsible for ensuring:
//! - All segments are mapped with correct permissions
//! - All fixups have been applied
//! - The binary is a trivial test fixture (not arbitrary code)

use macho_types::EntryPoint;

use crate::ptrauth;
use crate::{ExecError, MappedImage};

#[cfg(target_arch = "x86_64")]
#[repr(C)]
struct DarwinProgramVars {
    mh: *const u8,
    nx_argc: *const i32,
    nx_argv: *const *const *const u8,
    environ: *const *const *const u8,
    progname: *const *const u8,
}

/// Result of calling a binary's entry point.
#[derive(Debug, Clone)]
pub struct EntryResult {
    /// The return value from main() (or equivalent).
    pub exit_code: i32,
    /// The computed entry address that was called.
    pub entry_address: u64,
}

/// Configuration for entry arguments passed to main().
#[derive(Debug, Clone, Default)]
pub struct EntryArgs {
    /// Program name (argv[0]).
    pub program_name: String,
    /// Additional arguments (argv[1..]).
    pub args: Vec<String>,
    /// Environment variables as "KEY=VALUE" pairs.
    pub env: Vec<String>,
}

/// Call the mapped binary's entry point.
///
/// Computes the entry address from the MappedImage's __TEXT base address
/// and the EntryPoint offset, then casts to a C function pointer and calls it.
///
/// For `LC_MAIN` binaries: entry_addr = text_base + entry_offset
/// For `LC_UNIXTHREAD` binaries: entry_addr = mapped segment base + (entry_vmaddr - segment_vmaddr).
///
/// # Safety
///
/// This function is inherently unsafe — it executes arbitrary mapped code.
/// Only call with known fixture binaries.
pub fn call_entry(image: &MappedImage, entry: &EntryPoint) -> Result<EntryResult, ExecError> {
    call_entry_with_args(image, entry, None)
}

fn resolve_entry_address(image: &MappedImage, entry: &EntryPoint) -> Result<u64, ExecError> {
    match entry {
        EntryPoint::Main { entry_offset, .. } => {
            let text_base = image.text_base()?;
            Ok(unsafe { text_base.add(*entry_offset as usize) } as u64)
        }
        EntryPoint::UnixThread { entry_address } => image
            .segments
            .iter()
            .find_map(|seg| {
                let seg_end = seg.vmaddr.saturating_add(seg.size as u64);
                if *entry_address < seg.vmaddr || *entry_address >= seg_end {
                    return None;
                }
                let offset = (*entry_address - seg.vmaddr) as usize;
                Some(unsafe { seg.base.add(offset) } as u64)
            })
            .ok_or(ExecError::NoEntryPoint),
    }
}

/// Call the entry point with custom arguments.
pub fn call_entry_with_args(
    image: &MappedImage,
    entry: &EntryPoint,
    args: Option<&EntryArgs>,
) -> Result<EntryResult, ExecError> {
    let entry_addr_u64 = resolve_entry_address(image, entry)?;

    match entry {
        EntryPoint::Main { entry_offset, .. } => {
            let text_base = image.text_base()?;
            log::info!(
                "Calling LC_MAIN entry at {entry_addr_u64:#x} (text_base={:#x} + offset={entry_offset:#x})",
                text_base as u64
            );
        }
        EntryPoint::UnixThread { entry_address } => {
            log::info!(
                "Calling LC_UNIXTHREAD entry at {entry_addr_u64:#x} (vmaddr={entry_address:#x}, slide={:#x})",
                image.slide
            );
        }
    }

    let stripped = ptrauth::ptrauth_strip(entry_addr_u64) as *const u8;

    let default_name = std::ffi::CString::new("app").unwrap();
    let (c_program, c_args, c_env): (
        std::ffi::CString,
        Vec<std::ffi::CString>,
        Vec<std::ffi::CString>,
    ) = if let Some(a) = args {
        let prog = std::ffi::CString::new(a.program_name.as_str())
            .unwrap_or_else(|_| default_name.clone());
        let extra: Vec<_> = a
            .args
            .iter()
            .filter_map(|s| std::ffi::CString::new(s.as_str()).ok())
            .collect();
        let env: Vec<_> = a
            .env
            .iter()
            .filter_map(|s| std::ffi::CString::new(s.as_str()).ok())
            .collect();
        (prog, extra, env)
    } else {
        (default_name, Vec::new(), Vec::new())
    };

    let mut argv_ptrs: Vec<*const u8> = Vec::with_capacity(2 + c_args.len());
    argv_ptrs.push(c_program.as_ptr().cast::<u8>());
    for a in &c_args {
        argv_ptrs.push(a.as_ptr().cast::<u8>());
    }
    argv_ptrs.push(std::ptr::null());

    let mut envp_ptrs: Vec<*const u8> = Vec::with_capacity(1 + c_env.len());
    for e in &c_env {
        envp_ptrs.push(e.as_ptr().cast::<u8>());
    }
    envp_ptrs.push(std::ptr::null());

    let argc = (argv_ptrs.len() - 1) as i32;

    log::debug!("calling entry at {entry_addr_u64:#x} (argc={argc})");

    #[cfg(target_arch = "x86_64")]
    let exit_code = {
        let argc_storage = argc;
        let argv_head = argv_ptrs.as_ptr();
        let envp_head = envp_ptrs.as_ptr();
        let progname = c_program.as_ptr().cast::<u8>();
        let progname_ptr = progname;
        let apple_ptrs = [std::ptr::null::<u8>()];
        let program_vars = DarwinProgramVars {
            mh: image.text_base()?.cast::<u8>(),
            nx_argc: &argc_storage,
            nx_argv: &argv_head,
            environ: &envp_head,
            progname: &progname_ptr,
        };
        let entry_fn: extern "C" fn(
            i32,
            *const *const u8,
            *const *const u8,
            *const *const u8,
            *const DarwinProgramVars,
        ) -> i32 = unsafe { std::mem::transmute(stripped) };
        entry_fn(
            argc_storage,
            argv_head,
            envp_head,
            apple_ptrs.as_ptr(),
            &program_vars,
        )
    };

    #[cfg(not(target_arch = "x86_64"))]
    let exit_code = {
        let entry_fn: extern "C" fn(i32, *const *const u8, *const *const u8) -> i32 =
            unsafe { std::mem::transmute(stripped) };
        entry_fn(argc, argv_ptrs.as_ptr(), envp_ptrs.as_ptr())
    };

    log::debug!("entry returned {exit_code}");

    Ok(EntryResult {
        exit_code,
        entry_address: entry_addr_u64,
    })
}

/// Call the binary's entry point via the ARM64 interpreter (Phase E3).
///
/// This executes the binary by interpreting ARM64 instructions instead of
/// running them natively. This enables executing ARM64 Mach-O binaries on
/// any host architecture.
///
/// # Safety
///
/// The mapped image segments must remain valid for the lifetime of the
/// interpreter call. Only use with known fixture binaries.
pub fn call_entry_interpreted(
    image: &MappedImage,
    entry: &EntryPoint,
    args: Option<&EntryArgs>,
    ffi_bridge: arm64_translate::FfiBridge,
    extra_segments: &[(u64, *mut u8, usize, bool)],
) -> Result<EntryResult, ExecError> {
    let entry_addr = resolve_entry_address(image, entry)?;

    match entry {
        EntryPoint::Main { entry_offset, .. } => {
            let text_base = image.text_base()?;
            log::info!(
                "Interpreter: entry at {entry_addr:#x} (text_base={:#x} + offset={entry_offset:#x})",
                text_base as u64
            );
        }
        EntryPoint::UnixThread { entry_address } => {
            log::info!(
                "Interpreter: LC_UNIXTHREAD entry at {entry_addr:#x} (vmaddr={entry_address:#x}, slide={:#x})",
                image.slide
            );
        }
    }

    let mut segments: Vec<(u64, *mut u8, usize, bool)> = image
        .segments
        .iter()
        .map(|s| {
            let writable = s.name != "__TEXT";
            (s.base as u64, s.base, s.size, writable)
        })
        .collect();
    segments.extend_from_slice(extra_segments);

    let default_name = std::ffi::CString::new("app").unwrap();
    let (c_program, c_args, c_env): (
        std::ffi::CString,
        Vec<std::ffi::CString>,
        Vec<std::ffi::CString>,
    ) = if let Some(a) = args {
        let prog = std::ffi::CString::new(a.program_name.as_str())
            .unwrap_or_else(|_| default_name.clone());
        let extra: Vec<_> = a
            .args
            .iter()
            .filter_map(|s| std::ffi::CString::new(s.as_str()).ok())
            .collect();
        let env: Vec<_> = a
            .env
            .iter()
            .filter_map(|s| std::ffi::CString::new(s.as_str()).ok())
            .collect();
        (prog, extra, env)
    } else {
        (default_name, Vec::new(), Vec::new())
    };

    let mut argv_ptrs: Vec<*const u8> = Vec::with_capacity(2 + c_args.len());
    argv_ptrs.push(c_program.as_ptr().cast::<u8>());
    for a in &c_args {
        argv_ptrs.push(a.as_ptr().cast::<u8>());
    }
    argv_ptrs.push(std::ptr::null());

    let mut envp_ptrs: Vec<*const u8> = Vec::with_capacity(1 + c_env.len());
    for e in &c_env {
        envp_ptrs.push(e.as_ptr().cast::<u8>());
    }
    envp_ptrs.push(std::ptr::null());

    let argc = (argv_ptrs.len() - 1) as i32;

    let mut mem = unsafe { arm64_translate::Memory::from_segments(&segments) };
    mem.host_passthrough = true;

    let mut interp = arm64_translate::interp::setup_for_main(
        mem,
        ffi_bridge,
        entry_addr,
        argc,
        argv_ptrs.as_ptr() as u64,
        envp_ptrs.as_ptr() as u64,
    );

    let exit_reason = interp.run();
    log::info!("Interpreter exit: {exit_reason:?}");

    let exit_code = match exit_reason {
        arm64_translate::ExitReason::Returned { code } => code as i32,
        arm64_translate::ExitReason::Exit { code } => code as i32,
        arm64_translate::ExitReason::ReturnedToHost { lr } => {
            log::debug!("Interpreter: RET to host LR {lr:#x}, using x0 as exit code");
            interp.cpu.x[0] as i32
        }
        arm64_translate::ExitReason::InstructionLimit { count } => {
            log::warn!("Interpreter: instruction limit reached ({count})");
            return Err(ExecError::NoEntryPoint);
        }
        arm64_translate::ExitReason::Unimplemented { pc, raw } => {
            eprintln!("Interpreter: unimplemented instruction {raw:#010x} at {pc:#x}");
            return Err(ExecError::NoEntryPoint);
        }
        arm64_translate::ExitReason::MemoryFault { pc, addr, is_write } => {
            eprintln!("Interpreter: memory fault at PC {pc:#x}, addr {addr:#x} (write={is_write})");
            return Err(ExecError::NoEntryPoint);
        }
        arm64_translate::ExitReason::BadBranch { pc, target } => {
            eprintln!("Interpreter: bad branch at {pc:#x} → {target:#x}");
            return Err(ExecError::NoEntryPoint);
        }
    };

    Ok(EntryResult {
        exit_code,
        entry_address: entry_addr,
    })
}
