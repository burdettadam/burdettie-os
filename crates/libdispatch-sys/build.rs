fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" | "ios" | "tvos" | "watchos" => {
            // libdispatch is part of libSystem on Apple platforms
            println!("cargo:rustc-link-lib=System");
        }
        "linux" => {
            // Try pkg-config first (libdispatch-dev package)
            if pkg_config::probe_library("libdispatch").is_ok() {
                return;
            }
            // Fall back to Swift toolchain path
            let swift_lib = "/usr/lib/swift/linux";
            if std::path::Path::new(&format!("{swift_lib}/libdispatch.so")).exists() {
                println!("cargo:rustc-link-search=native={swift_lib}");
                println!("cargo:rustc-link-lib=dispatch");
                return;
            }
            // Last resort: just try linking and let the linker resolve it
            println!("cargo:rustc-link-lib=dispatch");
            println!("cargo:warning=libdispatch not found via pkg-config or Swift toolchain; linker may fail");
        }
        other => {
            println!("cargo:warning=libdispatch-sys: unsupported target OS '{other}', linking may fail");
            println!("cargo:rustc-link-lib=dispatch");
        }
    }
}
