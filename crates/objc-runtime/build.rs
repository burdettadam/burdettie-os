fn main() {
    // Build the assembly trampoline on aarch64 targets (macOS and Linux)
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_arch == "aarch64" && (target_os == "macos" || target_os == "linux") {
        cc::Build::new()
            .file("src/trampoline.S")
            .compile("objc_trampoline");

        // Tell Cargo to link the static library in tests and dependents
        println!("cargo:rustc-link-lib=static=objc_trampoline");
        println!("cargo:rerun-if-changed=src/trampoline.S");
    }

    if target_arch == "x86_64" && (target_os == "macos" || target_os == "linux") {
        cc::Build::new()
            .file("src/trampoline_x86_64.s")
            .flag("-x")
            .flag("assembler-with-cpp")
            .compile("objc_trampoline");

        println!("cargo:rustc-link-lib=static=objc_trampoline");
        println!("cargo:rerun-if-changed=src/trampoline_x86_64.s");
    }

    // Link libc++abi for __cxa_throw / __cxa_begin_catch / etc.
    if target_os == "macos" {
        println!("cargo:rustc-link-lib=c++abi");
    } else if target_os == "linux" {
        // On Linux, libc++abi may be available; fall back to stdc++ for
        // the C++ unwind ABI (__cxa_throw etc.)
        println!("cargo:rustc-link-lib=stdc++");
    }

    // Link GNUstep's libobjc2 when the feature is enabled.
    if std::env::var("CARGO_FEATURE_LIBOBJC2").is_ok() {
        println!("cargo:rustc-link-lib=objc");
    }
}
