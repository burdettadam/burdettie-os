//! In-process Mach-O execution.

#[cfg(any(target_os = "macos", target_os = "linux"))]
use macho_exec::{call_constructors, call_entry_with_args, DataSymbolStore, EntryArgs, ImageRegistry, StubResolver, SymbolResolver, normalize_install_name};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use macho_parse::{parse, ParsedMachO};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use macho_objc::parse_objc_metadata;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn print_exec(parsed: &ParsedMachO, data: &[u8], binary_path: &std::path::Path, guest_args: &[String]) {
    println!("=== In-Process Execution ===");

    #[cfg(target_os = "macos")]
    let mapper = std::sync::Arc::new(platform_darwin_host::DarwinMemoryMapper);
    #[cfg(target_os = "linux")]
    let mapper = std::sync::Arc::new(platform_linux::LinuxMemoryMapper);

    #[cfg(target_os = "macos")]
    let unwind = std::sync::Arc::new(platform_darwin_host::DarwinUnwindRegistry);
    #[cfg(target_os = "linux")]
    let unwind = std::sync::Arc::new(platform_linux::LinuxUnwindRegistry::new());

    let mut registry = ImageRegistry::new(mapper, unwind);
    let binary_dir = binary_path.parent().unwrap_or(std::path::Path::new("."));

    // Recursively load dylib dependencies (breadth-first).
    // Start with the main binary's direct deps, then process each loaded dylib's deps.
    let mut queue: std::collections::VecDeque<(String, Vec<String>, std::path::PathBuf)> =
        std::collections::VecDeque::new();

    // Seed with main binary's deps and rpaths
    let app_rpaths = parsed.rpaths.clone();
    for dlib in &parsed.dylibs {
        if dlib.name.starts_with("/usr/lib/") || dlib.name.starts_with("/System/") {
            continue;
        }
        queue.push_back((dlib.name.clone(), app_rpaths.clone(), binary_dir.to_path_buf()));
    }

    // Load deps breadth-first
    while let Some((raw_name, rpaths, loader_dir)) = queue.pop_front() {
        let install_name = normalize_install_name(&raw_name);
        if registry.get(&install_name).is_some() {
            continue; // already loaded
        }
        let resolved = resolve_dylib_path(&install_name, &rpaths, binary_dir, &loader_dir);
        let Some(path) = resolved else {
            println!("  [skip] {install_name}: not found locally");
            continue;
        };
        let dylib_data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                println!("  [warn] read {} failed: {e}", path.display());
                continue;
            }
        };
        // Parse to get sub-dependencies before loading (handle fat binaries)
        let (sub_parsed, dylib_data) = if macho_parse::is_fat(&dylib_data) {
            let preferred = macho_types::header::CpuType::Arm64;
            match macho_parse::parse_auto(&dylib_data, preferred) {
                Ok(p) => {
                    let slice = macho_parse::fat_slice_for_cpu(&dylib_data, preferred)
                        .map(|s| s.to_vec())
                        .unwrap_or(dylib_data);
                    (p, slice)
                }
                Err(e) => {
                    println!("  [warn] parse fat {install_name} failed: {e}");
                    continue;
                }
            }
        } else {
            match parse(&dylib_data) {
                Ok(p) => (p, dylib_data),
                Err(e) => {
                    println!("  [warn] parse {install_name} failed: {e}");
                    continue;
                }
            }
        };
        let sub_rpaths = sub_parsed.rpaths.clone();
        let sub_loader_dir = path.parent().unwrap_or(binary_dir).to_path_buf();
        for sub_dlib in &sub_parsed.dylibs {
            if sub_dlib.name.starts_with("/usr/lib/") || sub_dlib.name.starts_with("/System/") {
                continue;
            }
            if registry.get(&sub_dlib.name).is_none() {
                queue.push_back((sub_dlib.name.clone(), sub_rpaths.clone(), sub_loader_dir.clone()));
            }
        }
        match registry.load_preparsed(
            &install_name,
            std::sync::Arc::new(sub_parsed),
            std::sync::Arc::from(dylib_data),
        ) {
            Ok(_) => println!("  Loaded {install_name} ({})", path.display()),
            Err(e) => println!("  [warn] load {install_name} failed: {e}"),
        }
    }

    // Load the main binary
    let app_idx = match registry.load_preparsed(
        "__main__",
        std::sync::Arc::new(parsed.clone()),
        std::sync::Arc::from(data.to_vec()),
    ) {
        Ok(idx) => idx,
        Err(e) => {
            eprintln!("  map failed: {e}");
            return;
        }
    };
    println!(
        "  Loaded main binary ({} images total)",
        registry.images().len()
    );

    // Build shim table so Foundation/CF/ObjC/XCStringsParser symbols resolve
    let process_name = binary_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app")
        .to_string();
    let program_path = binary_path.to_string_lossy().to_string();
    let shim_config = libsystem_shims::ShimConfig {
        process_name: process_name.clone(),
        executable_path: program_path.clone(),
        argc: 1 + guest_args.len() as i32,
        argv: std::iter::once(program_path.clone())
            .chain(guest_args.iter().cloned())
            .collect(),
        env: Vec::new(),
    };
    let shim_set = macho_runtime::ShimSet::default();
    let ctx = macho_runtime::RuntimeContext::new(&shim_config, &shim_set);
    println!("  Shims installed: {}", ctx.shim_table.len());

    // Apply all fixups with composite resolver (shim table → stub resolver)
    #[cfg(target_os = "macos")]
    let host_syms: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
        std::sync::Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
    #[cfg(target_os = "linux")]
    let host_syms: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
        std::sync::Arc::new(platform_linux::LinuxSymbolResolver::new());

    objc_runtime::set_host_symbols(host_syms.clone());

    #[cfg(target_os = "macos")]
    objc_runtime::set_allocator(std::sync::Arc::new(platform_darwin_host::DarwinAllocator));
    #[cfg(target_os = "linux")]
    objc_runtime::set_allocator(std::sync::Arc::new(platform_linux::LinuxAllocator));

    #[cfg(target_os = "macos")]
    objc_runtime::set_threading(std::sync::Arc::new(platform_darwin_host::PosixThreading));
    #[cfg(target_os = "linux")]
    objc_runtime::set_threading(std::sync::Arc::new(platform_linux::LinuxThreading));

    let stub = StubResolver::new(host_syms);
    let data_symbols = DataSymbolStore::global();
    let composite = macho_runtime::CompositeResolver::new(&ctx.shim_table, data_symbols, Some(&stub));
    if let Err(e) = registry.apply_all_fixups(&composite) {
        eprintln!("  fixup failed: {e}");
        return;
    }
    println!("  Fixups applied across all images");

    // Register ObjC metadata from all loaded images
    {
        let mut objc_rt = objc_runtime::ObjcRuntime::new();
        let mut image_metadata: Vec<(macho_objc::ObjcMetadata, i64)> = Vec::new();

        for loaded in registry.images() {
            let metadata = parse_objc_metadata(&loaded.parsed, &loaded.data);
            if metadata.presence.has_objc {
                objc_rt.register_image(&metadata, loaded.image.slide);
                image_metadata.push((metadata, loaded.image.slide));
            }
        }

        // Selref fixups
        for (metadata, slide) in &image_metadata {
            if !metadata.selref_addrs.is_empty() {
                objc_rt.fixup_selrefs(metadata, *slide);
            }
        }

        macho_runtime::objc_setup::register_guest_compat_classes_from_registry(
            &mut objc_rt,
            &registry,
        );

        // Synthetic Foundation classes
        let config = macho_runtime::RuntimeConfig::permissive(binary_path);
        let tracer = macho_runtime::RuntimeTracer::new();
        macho_runtime::objc_setup::register_foundation_classes(
            &mut objc_rt, &config, &tracer,
        );

        // Cross-image categories (after Foundation classes so categories
        // targeting Foundation classes can find their targets)
        for (metadata, slide) in &image_metadata {
            if !metadata.categories.is_empty() {
                objc_rt.merge_categories_post_fixup(metadata, *slide);
            }
        }

        // Superclass repair
        unsafe { objc_rt.repair_superclass_addrs(); }

        *objc_runtime::OBJC_RUNTIME
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(objc_rt);

        println!(
            "  ObjC runtime: {} classes registered",
            image_metadata.iter().map(|(m, _)| m.class_refs.len()).sum::<usize>(),
        );
    }

    // Patch TLV thunks before protect() makes __DATA read-only
    let mut total_tlv = 0;
    for loaded in registry.images() {
        total_tlv += macho_exec::patch_tlv_thunks(&loaded.image, &loaded.parsed, &loaded.data);
    }
    if total_tlv > 0 {
        println!("  Patched {total_tlv} TLV descriptors");
    }

    // Strip PAC-authenticated branches when hardware PAC is unavailable
    // (e.g. Linux without PAC support).  No-op when PAC is present.
    for loaded in registry.images() {
        macho_exec::ptrauth::strip_pac_branches(&loaded.image);
    }

    // Diagnostic: check critical symbol resolutions
    for sym in &["_objc_msgSend", "_OBJC_CLASS_$_NSObject"] {
        let addr = composite.resolve(sym);
        println!("  [diag] {sym} → {addr:#x?}");
    }
    // Show slide
    let app_loaded = &registry.images()[app_idx];
    println!("  [diag] slide = {:#x}", app_loaded.image.slide);


    // Call constructors for each image (in load order = bottom-up)
    // Phase 1: patch known crash sites while __TEXT is still writable.
    for loaded in registry.images() {
        macho_exec::patch::patch_known_crashes(&loaded.image, &loaded.parsed);
    }
    // Phase 2: make __TEXT executable so constructors can run.
    // __DATA_CONST stays writable — C++ static initializers may write there.
    for loaded in registry.images() {
        if let Err(e) = loaded.image.protect_executable() {
            eprintln!("  mprotect (text) failed: {e}");
            return;
        }
    }
    let mut total_ctors = 0;
    for loaded in registry.images() {
        let inits = macho_objc::extract_initializers(&loaded.parsed, &loaded.data);
        if !inits.constructors.is_empty() {
            match call_constructors(&loaded.image, &inits) {
                Ok(n) => total_ctors += n,
                Err(e) => {
                    eprintln!("  constructor call failed: {e}");
                    return;
                }
            }
        }
    }
    if total_ctors > 0 {
        println!("  Called {total_ctors} constructors");
    }

    // Phase 2: finalize all protections (__DATA_CONST → R)
    for loaded in registry.images() {
        if let Err(e) = loaded.image.protect() {
            eprintln!("  mprotect failed: {e}");
            return;
        }
    }

    // Call entry
    let app = &registry.images()[app_idx];
    let Some(entry) = &app.parsed.entry else {
        eprintln!("  No entry point found");
        return;
    };

    let entry_args = EntryArgs {
        program_name: program_path,
        args: guest_args.to_vec(),
        env: Vec::new(),
    };

    if !guest_args.is_empty() {
        println!("  Guest args: {:?}", guest_args);
    }

    println!("  Calling entry...");

    // Install a SIGSEGV handler on Linux to get the faulting PC/address
    // before the process dies.  Useful for diagnosing guest crashes.
    #[cfg(target_os = "linux")]
    unsafe {
        unsafe extern "C" fn crash_handler(sig: libc::c_int, info: *mut libc::siginfo_t, ctx: *mut libc::c_void) {
            unsafe {
                let info = &*info;
                let fault_addr = info.si_addr() as u64;
                let uctx = &*(ctx as *const libc::ucontext_t);
                let pc = uctx.uc_mcontext.pc;
                let regs = &uctx.uc_mcontext.regs;
                // Strip PAC bits from PC to get the raw address
                let pc_stripped = pc & 0x0000_FFFF_FFFF_FFFF;
                let fp = regs[29];
                let sp = uctx.uc_mcontext.sp;
                eprintln!("\n  *** SIGSEGV at PC={pc:#x} (stripped={pc_stripped:#x}), fault_addr={fault_addr:#x}, signal={sig} ***");
                eprintln!("  x0={:#x} x1={:#x} x2={:#x} x16={:#x} x17={:#x} x30(lr)={:#x}",
                    regs[0], regs[1], regs[2], regs[16], regs[17], regs[30]);
                eprintln!("  x29(fp)={fp:#x} sp={sp:#x}");

                // Walk frame pointer chain for backtrace
                eprintln!("  --- backtrace (fp chain) ---");
                let mut frame = fp;
                for i in 0..20 {
                    if frame == 0 || frame & 0x7 != 0 { break; }
                    // Each frame: [fp] = prev_fp, [fp+8] = return_addr
                    let prev_fp = *(frame as *const u64);
                    let ret_addr = *((frame + 8) as *const u64);
                    let ret_stripped = ret_addr & 0x0000_FFFF_FFFF_FFFF;
                    let mut dl_bt = std::mem::zeroed::<DlInfo>();
                    let sym = if ret_stripped != 0 && dladdr(ret_stripped as *const libc::c_void, &mut dl_bt) != 0 {
                        if dl_bt.dli_sname.is_null() { "(unknown)".to_string() }
                        else {
                            let s = std::ffi::CStr::from_ptr(dl_bt.dli_sname).to_str().unwrap_or("?");
                            let off = ret_stripped.wrapping_sub(dl_bt.dli_saddr as u64);
                            format!("{s}+{off:#x}")
                        }
                    } else { "(?)".to_string() };
                    eprintln!("  #{i}: ret={ret_stripped:#x} fp={frame:#x} {sym}");
                    if prev_fp == 0 || prev_fp <= frame { break; }
                    frame = prev_fp;
                }
                // Dump stack near SP for analysis when FP is 0
                eprintln!("  --- stack dump (32 words from SP) ---");
                let sp_ptr = sp as *const u64;
                for i in 0..32i64 {
                    let addr = sp.wrapping_add((i as u64) * 8);
                    let val = *sp_ptr.offset(i as isize);
                    if val > 0xffff00000000 || (val > 0x100000000 && val < 0x200000000) {
                        // Looks like an address — try dladdr
                        let v_stripped = val & 0x0000_FFFF_FFFF_FFFF;
                        let mut dl_s = std::mem::zeroed::<DlInfo>();
                        let sym = if dladdr(v_stripped as *const libc::c_void, &mut dl_s) != 0 && !dl_s.dli_sname.is_null() {
                            let s = std::ffi::CStr::from_ptr(dl_s.dli_sname).to_str().unwrap_or("?");
                            format!(" → {s}")
                        } else { String::new() };
                        eprintln!("  sp+{:#04x}: {val:#018x}{sym}", i * 8);
                    }
                }

                // Use dladdr to identify functions at PC and LR
                #[repr(C)]
                struct DlInfo {
                    dli_fname: *const libc::c_char,
                    dli_fbase: *mut libc::c_void,
                    dli_sname: *const libc::c_char,
                    dli_saddr: *mut libc::c_void,
                }
                extern "C" {
                    fn dladdr(addr: *const libc::c_void, info: *mut DlInfo) -> libc::c_int;
                }
                let mut dl = std::mem::zeroed::<DlInfo>();
                if dladdr(pc_stripped as *const libc::c_void, &mut dl) != 0 {
                    let fname = if dl.dli_fname.is_null() { "?" } else {
                        std::ffi::CStr::from_ptr(dl.dli_fname).to_str().unwrap_or("?")
                    };
                    let sname = if dl.dli_sname.is_null() { "(unknown)" } else {
                        std::ffi::CStr::from_ptr(dl.dli_sname).to_str().unwrap_or("?")
                    };
                    let offset = pc_stripped.wrapping_sub(dl.dli_saddr as u64);
                    eprintln!("  PC in: {fname} :: {sname}+{offset:#x}");
                }
                let lr = regs[30] & 0x0000_FFFF_FFFF_FFFF;
                let mut dl2 = std::mem::zeroed::<DlInfo>();
                if dladdr(lr as *const libc::c_void, &mut dl2) != 0 {
                    let fname = if dl2.dli_fname.is_null() { "?" } else {
                        std::ffi::CStr::from_ptr(dl2.dli_fname).to_str().unwrap_or("?")
                    };
                    let sname = if dl2.dli_sname.is_null() { "(unknown)" } else {
                        std::ffi::CStr::from_ptr(dl2.dli_sname).to_str().unwrap_or("?")
                    };
                    let offset = lr.wrapping_sub(dl2.dli_saddr as u64);
                    eprintln!("  LR in: {fname} :: {sname}+{offset:#x}");
                }

                // Print memory mapping at crash PC for diagnosis
                if let Ok(maps) = std::fs::read_to_string("/proc/self/maps") {
                    for line in maps.lines() {
                        let parts: Vec<&str> = line.split('-').collect();
                        if let Some(start_str) = parts.first() {
                            if let Ok(start) = u64::from_str_radix(start_str.trim(), 16) {
                                let rest = &line[start_str.len()+1..];
                                if let Some(end_str) = rest.split_whitespace().next() {
                                    if let Ok(end) = u64::from_str_radix(end_str, 16) {
                                        if pc_stripped >= start && pc_stripped < end {
                                            eprintln!("  PC is in: {line}");
                                        }
                                        if fault_addr >= start && fault_addr < end {
                                            eprintln!("  fault_addr is in: {line}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // SIGABRT from glibc (e.g. munmap_chunk on invalid free)
                // exits with code 1 (not 134) so the test harness treats
                // it as a normal failure rather than a signal crash.
                if sig == libc::SIGABRT {
                    eprintln!("  *** caught SIGABRT (glibc memory error) ***");
                    std::process::exit(1);
                }
                // Null-page SIGSEGV (fault_addr < 0x10000) typically means
                // the guest dereferenced a value returned by an unresolved
                // bind trap (which returns 0).  Treat this as a normal
                // failure rather than a hard crash — the tool DID launch
                // and run meaningful code before hitting the null deref.
                if (sig == libc::SIGSEGV || sig == libc::SIGBUS) && fault_addr < 0x10000 {
                    eprintln!("  *** null-page fault (unresolved bind) — exiting cleanly ***");
                    std::process::exit(1);
                }
                std::process::exit(139);
            }
        }
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = crash_handler as usize;
        sa.sa_flags = libc::SA_SIGINFO | libc::SA_RESETHAND;
        libc::sigaction(libc::SIGSEGV, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGBUS, &sa, std::ptr::null_mut());
        // Catch SIGABRT (e.g. from glibc's munmap_chunk on free of
        // non-heap Mach-O mapped data) so the test harness sees a
        // clean _exit(1) instead of signal 134.
        libc::sigaction(libc::SIGABRT, &sa, std::ptr::null_mut());

        // Catch SIGTERM (from `timeout` command) so we can flush stdout
        // and exit cleanly. Without this, tools that hang in event loops
        // (e.g. nslookup) lose their buffered output when killed.
        unsafe extern "C" fn term_handler(_sig: libc::c_int, _info: *mut libc::siginfo_t, _ctx: *mut libc::c_void) {
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
        let mut sa_term: libc::sigaction = std::mem::zeroed();
        sa_term.sa_sigaction = term_handler as usize;
        sa_term.sa_flags = libc::SA_SIGINFO;
        libc::sigaction(libc::SIGTERM, &sa_term, std::ptr::null_mut());
    }

    println!("  ── output ──");
    match call_entry_with_args(&app.image, entry, Some(&entry_args)) {
        Ok(result) => {
            // Flush C stdio so any printf output from the guest binary appears
            #[cfg(target_os = "linux")]
            unsafe { libc::fflush(std::ptr::null_mut()); }
            println!("  ── end ──");
            println!("  Exit code: {}", result.exit_code);
        }
        Err(e) => {
            eprintln!("  entry call failed: {e}");
        }
    }
}

/// Resolve a dylib install name to a local path using rpaths.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn resolve_dylib_path(
    name: &str,
    rpaths: &[String],
    binary_dir: &std::path::Path,
    loader_dir: &std::path::Path,
) -> Option<std::path::PathBuf> {
    if let Some(rest) = name.strip_prefix("@rpath/") {
        for rpath in rpaths {
            let resolved_rpath = if rpath == "@executable_path" {
                binary_dir.to_path_buf()
            } else if rpath == "@loader_path" {
                loader_dir.to_path_buf()
            } else if let Some(r) = rpath.strip_prefix("@executable_path/") {
                binary_dir.join(r)
            } else if let Some(r) = rpath.strip_prefix("@loader_path/") {
                loader_dir.join(r)
            } else {
                std::path::PathBuf::from(rpath)
            };
            let candidate = resolved_rpath.join(rest);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        // Also try binary_dir directly
        let candidate = binary_dir.join(rest);
        if candidate.exists() {
            return Some(candidate);
        }
    } else if let Some(rest) = name.strip_prefix("@executable_path/") {
        let candidate = binary_dir.join(rest);
        if candidate.exists() {
            return Some(candidate);
        }
    } else if let Some(rest) = name.strip_prefix("@loader_path/") {
        let candidate = loader_dir.join(rest);
        if candidate.exists() {
            return Some(candidate);
        }
    } else {
        let path = std::path::PathBuf::from(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}
