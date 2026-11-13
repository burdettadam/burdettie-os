//! In-process Mach-O execution.

#[cfg(any(target_os = "macos", target_os = "linux"))]
use macho_exec::{
    call_constructors, call_entry_with_args, normalize_install_name, DataSymbolStore, EntryArgs,
    ImageRegistry, StubResolver, SymbolResolver,
};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use macho_objc::parse_objc_metadata;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use macho_parse::{parse, ParsedMachO};

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn inherited_environment() -> Vec<(String, String)> {
    std::env::vars().collect()
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn print_exec(
    parsed: &ParsedMachO,
    data: &[u8],
    binary_path: &std::path::Path,
    guest_args: &[String],
) {
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
        queue.push_back((
            dlib.name.clone(),
            app_rpaths.clone(),
            binary_dir.to_path_buf(),
        ));
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
            let preferred = parsed.header.cpu_type;
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
                queue.push_back((
                    sub_dlib.name.clone(),
                    sub_rpaths.clone(),
                    sub_loader_dir.clone(),
                ));
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
    let inherited_env = inherited_environment();
    let shim_config = libsystem_shims::ShimConfig {
        process_name: process_name.clone(),
        executable_path: program_path.clone(),
        argc: 1 + guest_args.len() as i32,
        argv: std::iter::once(program_path.clone())
            .chain(guest_args.iter().cloned())
            .collect(),
        env: inherited_env.clone(),
    };
    let shim_set = macho_runtime::ShimSet::default();
    let ctx = macho_runtime::RuntimeContext::new(&shim_config, &shim_set);
    println!("  Shims installed: {}", ctx.shim_table.len());

    // Start BurdettieOS cloud services (domain rewrite + local server)
    burdettie_services::set_guest_binary(&process_name);
    let _burdettie_handle = {
        let domain_config = burdettie_domain_map::RewriteConfig::from_env();
        if domain_config.enabled {
            let svc_config = burdettie_services::ServiceConfig::from_env();

            // Initialize the structured os_log pipeline
            let log_format = match svc_config.log_format {
                burdettie_services::config::LogFormat::Json => {
                    burdettie_log_pipeline::OutputFormat::Json
                }
                burdettie_services::config::LogFormat::Logfmt => {
                    burdettie_log_pipeline::OutputFormat::Logfmt
                }
                burdettie_services::config::LogFormat::Text => {
                    burdettie_log_pipeline::OutputFormat::Text
                }
            };
            let log_sink = match &svc_config.log_sink {
                burdettie_services::config::LogSink::Stdout => {
                    burdettie_log_pipeline::SinkConfig::Stdout
                }
                burdettie_services::config::LogSink::File(path) => {
                    burdettie_log_pipeline::SinkConfig::File(path.clone())
                }
                burdettie_services::config::LogSink::Http(url) => {
                    burdettie_log_pipeline::SinkConfig::Http(url.clone())
                }
            };
            burdettie_log_pipeline::init_with_sink(log_format, log_sink);

            match burdettie_services::ServiceServer::start(&svc_config) {
                Ok(handle) => {
                    let mut final_config = domain_config;
                    final_config.target_port = handle.port;
                    burdettie_domain_map::init(final_config);
                    Some(handle)
                }
                Err(e) => {
                    log::warn!("burdettie-services: failed to start: {e}");
                    burdettie_domain_map::init(burdettie_domain_map::RewriteConfig::disabled());
                    None
                }
            }
        } else {
            burdettie_domain_map::init(domain_config);
            None
        }
    };

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
    let composite =
        macho_runtime::CompositeResolver::new(&ctx.shim_table, data_symbols, Some(&stub));
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
        macho_runtime::objc_setup::register_foundation_classes(&mut objc_rt, &config, &tracer);

        // Cross-image categories (after Foundation classes so categories
        // targeting Foundation classes can find their targets)
        for (metadata, slide) in &image_metadata {
            if !metadata.categories.is_empty() {
                objc_rt.merge_categories_post_fixup(metadata, *slide);
            }
        }

        // Superclass repair
        unsafe {
            objc_rt.repair_superclass_addrs();
        }

        *objc_runtime::OBJC_RUNTIME
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(objc_rt);

        println!(
            "  ObjC runtime: {} classes registered",
            image_metadata
                .iter()
                .map(|(m, _)| m.class_refs.len())
                .sum::<usize>(),
        );
    }

    // Patch TLV thunks before protect() makes __DATA read-only
    let mut total_tlv = 0;
    for loaded in registry.images() {
        total_tlv += macho_exec::patch_tlv_thunks(&loaded.image, &loaded.parsed);
    }
    if total_tlv > 0 {
        println!("  Patched {total_tlv} TLV descriptors");
    }

    // Strip PAC-authenticated branches when hardware PAC is unavailable
    // (e.g. Linux without PAC support).  No-op when PAC is present.
    for loaded in registry.images() {
        macho_exec::ptrauth::strip_pac_branches(&loaded.image, loaded.parsed.header.cpu_type);
    }

    // Diagnostic: check critical symbol resolutions
    for sym in &["_objc_msgSend", "_OBJC_CLASS_$_NSObject"] {
        let addr = composite.resolve(sym);
        println!("  [diag] {sym} → {addr:#x?}");
    }
    // Show slide
    let app_loaded = &registry.images()[app_idx];
    println!("  [diag] slide = {:#x}", app_loaded.image.slide);

    #[cfg(target_os = "linux")]
    unsafe {
        install_guest_crash_handler();
    }

    println!("  [stage] patching known crashes");
    for loaded in registry.images() {
        macho_exec::patch::patch_known_crashes(&loaded.image, &loaded.parsed);
    }

    println!("  [stage] protecting executable segments");
    for loaded in registry.images() {
        if let Err(e) = loaded.image.protect_executable() {
            eprintln!("  mprotect (text) failed: {e}");
            return;
        }
    }

    let mut total_ctors = 0;
    println!("  [stage] calling constructors");
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

    println!("  [stage] finalizing protections");
    for loaded in registry.images() {
        if let Err(e) = loaded.image.protect() {
            eprintln!("  mprotect failed: {e}");
            return;
        }
    }

    let app = &registry.images()[app_idx];
    let Some(entry) = &app.parsed.entry else {
        eprintln!("  No entry point found");
        return;
    };

    let entry_args = EntryArgs {
        program_name: program_path,
        args: guest_args.to_vec(),
        env: inherited_env
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect(),
    };

    if !guest_args.is_empty() {
        println!("  Guest args: {:?}", guest_args);
    }

    println!("  [stage] calling entry");
    match call_entry_with_args(&app.image, entry, Some(&entry_args)) {
        Ok(result) => {
            println!("  Entry returned with exit code {}", result.exit_code);
        }
        Err(e) => {
            eprintln!("  entry call failed: {e}");
        }
    }
}

#[cfg(target_os = "linux")]
unsafe fn install_guest_crash_handler() {
    unsafe extern "C" fn crash_handler(
        sig: libc::c_int,
        info: *mut libc::siginfo_t,
        ctx: *mut libc::c_void,
    ) {
        unsafe {
            let info = &*info;
            let fault_addr = info.si_addr() as u64;
            let uctx = &*(ctx as *const libc::ucontext_t);
            #[cfg(target_arch = "aarch64")]
            let (_pc, pc_stripped, fp, sp) = {
                let pc = uctx.uc_mcontext.pc;
                let regs = &uctx.uc_mcontext.regs;
                let pc_stripped = pc & 0x0000_FFFF_FFFF_FFFF;
                let fp = regs[29];
                let sp = uctx.uc_mcontext.sp;
                eprintln!("\n  *** SIGSEGV at PC={pc:#x} (stripped={pc_stripped:#x}), fault_addr={fault_addr:#x}, signal={sig} ***");
                eprintln!(
                    "  x0={:#x} x1={:#x} x2={:#x} x16={:#x} x17={:#x} x30(lr)={:#x}",
                    regs[0], regs[1], regs[2], regs[16], regs[17], regs[30]
                );
                eprintln!("  x29(fp)={fp:#x} sp={sp:#x}");
                (pc, pc_stripped, fp, sp)
            };
            #[cfg(target_arch = "x86_64")]
            let (_pc, pc_stripped, fp, sp) = {
                let gregs = &uctx.uc_mcontext.gregs;
                let pc = gregs[libc::REG_RIP as usize] as u64;
                let fp = gregs[libc::REG_RBP as usize] as u64;
                let sp = gregs[libc::REG_RSP as usize] as u64;
                let rdi = gregs[libc::REG_RDI as usize] as u64;
                let rsi = gregs[libc::REG_RSI as usize] as u64;
                let rdx = gregs[libc::REG_RDX as usize] as u64;
                let rcx = gregs[libc::REG_RCX as usize] as u64;
                let r8 = gregs[libc::REG_R8 as usize] as u64;
                let r9 = gregs[libc::REG_R9 as usize] as u64;
                let rax = gregs[libc::REG_RAX as usize] as u64;
                eprintln!(
                    "\n  *** SIGSEGV at PC={pc:#x}, fault_addr={fault_addr:#x}, signal={sig} ***"
                );
                eprintln!("  host RIP={pc:#x} RBP={fp:#x} RSP={sp:#x}");
                eprintln!("  args: RDI={rdi:#x} RSI={rsi:#x} RDX={rdx:#x} RCX={rcx:#x} R8={r8:#x} R9={r9:#x}");
                eprintln!("  RAX={rax:#x}");
                (pc, pc, fp, sp)
            };
            #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
            let (_pc, pc_stripped, fp, sp) = {
                eprintln!(
                    "\n  *** SIGSEGV at fault_addr={fault_addr:#x}, signal={sig} (register dump unavailable on {}) ***",
                    std::env::consts::ARCH
                );
                (0, 0, 0, 0)
            };

            let mapped_regions = std::fs::read_to_string("/proc/self/maps")
                .ok()
                .map(|maps| {
                    maps.lines()
                        .filter_map(|line| {
                            let (range, _) = line.split_once(' ')?;
                            let (start, end) = range.split_once('-')?;
                            let start = u64::from_str_radix(start, 16).ok()?;
                            let end = u64::from_str_radix(end, 16).ok()?;
                            Some((start, end, line.to_string()))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let mapping_for = |addr: u64| {
                mapped_regions
                    .iter()
                    .find(|(start, end, _)| addr >= *start && addr < *end)
            };

            let mut dl = std::mem::zeroed::<DlInfo>();
            if dladdr(pc_stripped as *const libc::c_void, &mut dl) != 0 {
                let fname = if dl.dli_fname.is_null() {
                    "?"
                } else {
                    std::ffi::CStr::from_ptr(dl.dli_fname)
                        .to_str()
                        .unwrap_or("?")
                };
                let sname = if dl.dli_sname.is_null() {
                    "(unknown)"
                } else {
                    std::ffi::CStr::from_ptr(dl.dli_sname)
                        .to_str()
                        .unwrap_or("?")
                };
                let offset = pc_stripped.wrapping_sub(dl.dli_saddr as u64);
                eprintln!("  PC in: {fname} :: {sname}+{offset:#x}");
            }
            if let Some((_, _, line)) = mapping_for(pc_stripped) {
                eprintln!("  PC is in: {line}");
            }
            if let Some((_, _, line)) = mapping_for(fault_addr) {
                eprintln!("  fault_addr is in: {line}");
            } else {
                eprintln!("  fault_addr={fault_addr:#x} NOT in any mapped region");
            }
            if let Some((_, _, line)) = mapping_for(fp) {
                eprintln!("  RBP/fp is in: {line}");
            }
            if let Some((_, _, line)) = mapping_for(sp) {
                eprintln!("  RSP/sp is in: {line}");
            }

            eprintln!("  --- backtrace (fp chain) ---");
            let frame_region = mapping_for(fp)
                .map(|(_, _, line)| line.as_str())
                .unwrap_or("");
            let stack_region = mapping_for(sp)
                .map(|(_, _, line)| line.as_str())
                .unwrap_or("");
            if fp == 0
                || fp & 0x7 != 0
                || !frame_region.contains("[stack]")
                || !stack_region.contains("[stack]")
            {
                eprintln!(
                    "  skipping fp-chain backtrace: fp={fp:#x} sp={sp:#x} not in a mapped stack region"
                );
            } else {
                let mut frame = fp;
                for i in 0..20 {
                    if frame == 0 || frame & 0x7 != 0 {
                        break;
                    }
                    if mapping_for(frame).is_none() || mapping_for(frame + 8).is_none() {
                        eprintln!("  stopping fp-chain backtrace at unmapped frame {frame:#x}");
                        break;
                    }
                    let prev_fp = *(frame as *const u64);
                    let ret_addr = *((frame + 8) as *const u64);
                    let ret_stripped = ret_addr & 0x0000_FFFF_FFFF_FFFF;
                    let mut dl_bt = std::mem::zeroed::<DlInfo>();
                    let sym = if ret_stripped != 0
                        && dladdr(ret_stripped as *const libc::c_void, &mut dl_bt) != 0
                    {
                        if dl_bt.dli_sname.is_null() {
                            "(unknown)".to_string()
                        } else {
                            let s = std::ffi::CStr::from_ptr(dl_bt.dli_sname)
                                .to_str()
                                .unwrap_or("?");
                            let off = ret_stripped.wrapping_sub(dl_bt.dli_saddr as u64);
                            format!("{s}+{off:#x}")
                        }
                    } else {
                        "(?)".to_string()
                    };
                    eprintln!("  #{i}: ret={ret_stripped:#x} fp={frame:#x} {sym}");
                    if prev_fp == 0 || prev_fp <= frame {
                        break;
                    }
                    frame = prev_fp;
                }
            }

            eprintln!("  --- stack dump (32 words from SP) ---");
            if sp == 0 || sp & 0x7 != 0 || !stack_region.contains("[stack]") {
                eprintln!("  skipping stack dump: sp={sp:#x} not in a mapped stack region");
            } else {
                let sp_ptr = sp as *const u64;
                for i in 0..32i64 {
                    let slot_addr = sp + (i as u64 * 8);
                    if mapping_for(slot_addr).is_none() {
                        eprintln!("  stopping stack dump at unmapped slot {slot_addr:#x}");
                        break;
                    }
                    let val = *sp_ptr.offset(i as isize);
                    if val > 0xffff00000000 || (val > 0x100000000 && val < 0x200000000) {
                        let v_stripped = val & 0x0000_FFFF_FFFF_FFFF;
                        let mut dl_s = std::mem::zeroed::<DlInfo>();
                        let sym = if dladdr(v_stripped as *const libc::c_void, &mut dl_s) != 0
                            && !dl_s.dli_sname.is_null()
                        {
                            let s = std::ffi::CStr::from_ptr(dl_s.dli_sname)
                                .to_str()
                                .unwrap_or("?");
                            format!(" → {s}")
                        } else {
                            String::new()
                        };
                        eprintln!("  sp+{:#04x}: {val:#018x}{sym}", i * 8);
                    }
                }
            }
            #[cfg(target_arch = "aarch64")]
            {
                let lr = uctx.uc_mcontext.regs[30] & 0x0000_FFFF_FFFF_FFFF;
                let mut dl2 = std::mem::zeroed::<DlInfo>();
                if dladdr(lr as *const libc::c_void, &mut dl2) != 0 {
                    let fname = if dl2.dli_fname.is_null() {
                        "?"
                    } else {
                        std::ffi::CStr::from_ptr(dl2.dli_fname)
                            .to_str()
                            .unwrap_or("?")
                    };
                    let sname = if dl2.dli_sname.is_null() {
                        "(unknown)"
                    } else {
                        std::ffi::CStr::from_ptr(dl2.dli_sname)
                            .to_str()
                            .unwrap_or("?")
                    };
                    let offset = lr.wrapping_sub(dl2.dli_saddr as u64);
                    eprintln!("  LR in: {fname} :: {sname}+{offset:#x}");
                }
            }

            if sig == libc::SIGABRT {
                eprintln!("  *** caught SIGABRT (glibc memory error) ***");
                std::process::exit(1);
            }
            if (sig == libc::SIGSEGV || sig == libc::SIGBUS) && fault_addr < 0x10000 {
                eprintln!("  *** null-page fault (unresolved bind) — exiting cleanly ***");
                std::process::exit(1);
            }
            std::process::exit(139);
        }
    }

    #[repr(C)]
    struct DlInfo {
        dli_fname: *const libc::c_char,
        dli_fbase: *mut libc::c_void,
        dli_sname: *const libc::c_char,
        dli_saddr: *mut libc::c_void,
    }

    unsafe extern "C" {
        fn dladdr(addr: *const libc::c_void, info: *mut DlInfo) -> libc::c_int;
    }

    let mut sa: libc::sigaction = std::mem::zeroed();
    sa.sa_sigaction = crash_handler as *const () as usize;
    sa.sa_flags = libc::SA_SIGINFO | libc::SA_RESETHAND;
    libc::sigaction(libc::SIGSEGV, &sa, std::ptr::null_mut());
    libc::sigaction(libc::SIGBUS, &sa, std::ptr::null_mut());
    libc::sigaction(libc::SIGABRT, &sa, std::ptr::null_mut());
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
