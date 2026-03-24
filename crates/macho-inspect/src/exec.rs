//! In-process Mach-O execution (macOS only).

#[cfg(target_os = "macos")]
use macho_exec::{call_constructors, call_entry, ImageRegistry, StubResolver};
#[cfg(target_os = "macos")]
use macho_parse::{parse, ParsedMachO};

#[cfg(target_os = "macos")]
pub fn print_exec(parsed: &ParsedMachO, data: &[u8], binary_path: &std::path::Path) {
    println!("=== In-Process Execution ===");

    let mapper = std::sync::Arc::new(platform_darwin_host::DarwinMemoryMapper);
    let mut registry = ImageRegistry::new(mapper);
    let binary_dir = binary_path.parent().unwrap_or(std::path::Path::new("."));

    // Recursively load dylib dependencies (breadth-first).
    // Start with the main binary's direct deps, then process each loaded dylib's deps.
    let mut queue: std::collections::VecDeque<(String, Vec<String>)> =
        std::collections::VecDeque::new();

    // Seed with main binary's deps and rpaths
    let app_rpaths = parsed.rpaths.clone();
    for dlib in &parsed.dylibs {
        if dlib.name.starts_with("/usr/lib/") || dlib.name.starts_with("/System/") {
            continue;
        }
        queue.push_back((dlib.name.clone(), app_rpaths.clone()));
    }

    // Load deps breadth-first
    while let Some((install_name, rpaths)) = queue.pop_front() {
        if registry.get(&install_name).is_some() {
            continue; // already loaded
        }
        let resolved = resolve_dylib_path(&install_name, &rpaths, binary_dir);
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
        // Parse to get sub-dependencies before loading
        let sub_parsed = match parse(&dylib_data) {
            Ok(p) => p,
            Err(e) => {
                println!("  [warn] parse {install_name} failed: {e}");
                continue;
            }
        };
        let sub_rpaths = sub_parsed.rpaths.clone();
        for sub_dlib in &sub_parsed.dylibs {
            if sub_dlib.name.starts_with("/usr/lib/") || sub_dlib.name.starts_with("/System/") {
                continue;
            }
            if registry.get(&sub_dlib.name).is_none() {
                queue.push_back((sub_dlib.name.clone(), sub_rpaths.clone()));
            }
        }
        match registry.load(&install_name, dylib_data) {
            Ok(_) => println!("  Loaded {install_name} ({})", path.display()),
            Err(e) => println!("  [warn] load {install_name} failed: {e}"),
        }
    }

    // Load the main binary
    let app_idx = match registry.load("__main__", data.to_vec()) {
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

    // Apply all fixups with cross-image resolution
    let host_syms: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
        std::sync::Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
    let stub = StubResolver::new(host_syms);
    if let Err(e) = registry.apply_all_fixups(&stub) {
        eprintln!("  fixup failed: {e}");
        return;
    }
    println!("  Fixups applied across all images");

    // Protect all images
    for loaded in registry.images() {
        if let Err(e) = loaded.image.protect() {
            eprintln!("  mprotect failed: {e}");
            return;
        }
    }

    // Call constructors for each image (in load order = bottom-up)
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

    // Call entry
    let app = &registry.images()[app_idx];
    let Some(entry) = &app.parsed.entry else {
        eprintln!("  No entry point found");
        return;
    };

    println!("  Calling entry...");
    println!("  ── output ──");
    match call_entry(&app.image, entry) {
        Ok(result) => {
            println!("  ── end ──");
            println!("  Exit code: {}", result.exit_code);
        }
        Err(e) => {
            eprintln!("  entry call failed: {e}");
        }
    }
}

/// Resolve a dylib install name to a local path using rpaths.
#[cfg(target_os = "macos")]
fn resolve_dylib_path(
    name: &str,
    rpaths: &[String],
    binary_dir: &std::path::Path,
) -> Option<std::path::PathBuf> {
    if let Some(rest) = name.strip_prefix("@rpath/") {
        for rpath in rpaths {
            let resolved_rpath = if rpath == "@executable_path" || rpath == "@loader_path" {
                binary_dir.to_path_buf()
            } else if let Some(r) = rpath.strip_prefix("@executable_path/") {
                binary_dir.join(r)
            } else if let Some(r) = rpath.strip_prefix("@loader_path/") {
                binary_dir.join(r)
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
        let candidate = binary_dir.join(rest);
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
