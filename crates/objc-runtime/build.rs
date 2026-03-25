fn main() {
    // Only build the assembly trampoline on aarch64-apple targets
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_arch == "aarch64" && target_os == "macos" {
        cc::Build::new()
            .file("src/trampoline.s")
            .compile("objc_trampoline");

        // Tell Cargo to link the static library in tests and dependents
        println!("cargo:rustc-link-lib=static=objc_trampoline");
        println!("cargo:rerun-if-changed=src/trampoline.s");
    }

    // Link libc++abi for __cxa_throw / __cxa_begin_catch / etc.
    if target_os == "macos" {
        println!("cargo:rustc-link-lib=c++abi");
    }
}
