fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_arch == "aarch64" && target_os == "macos" {
        cc::Build::new()
            .file("src/nslog_trampoline.s")
            .compile("nslog_trampoline");

        cc::Build::new()
            .file("src/format_trampoline.s")
            .compile("format_trampoline");

        println!("cargo:rustc-link-lib=static=nslog_trampoline");
        println!("cargo:rustc-link-lib=static=format_trampoline");
        println!("cargo:rerun-if-changed=src/nslog_trampoline.s");
        println!("cargo:rerun-if-changed=src/format_trampoline.s");
    }
}
