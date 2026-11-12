use std::fs;
use std::path::PathBuf;

use clap::Parser;
use macho_parse::{extract_exports, extract_fixups, parse, ExportedSymbol, Fixups, ParsedMachO};
use macho_types::header::{CpuType, FatBinary};
use macho_types::segment::TlvDescriptor;
use serde::Serialize;

mod exec;
mod print_info;
mod print_text;
mod why_failed;

#[derive(Parser)]
#[command(name = "macho-inspect", about = "Inspect Mach-O binary structure")]
struct Cli {
    /// Path to the Mach-O binary to inspect.
    path: PathBuf,

    /// Output as JSON instead of human-readable text.
    #[arg(long)]
    json: bool,

    /// Full JSON output including fixups, exports, ObjC metadata, TLV, and function starts.
    #[arg(long)]
    json_full: bool,

    /// Run security gate evaluation (Apple-default policy).
    #[arg(long)]
    security: bool,

    /// Show dependency resolution tree (transitive).
    #[arg(long)]
    resolve: bool,

    /// Show full execution plan (resolve + bind + init order).
    #[arg(long)]
    plan: bool,

    /// Map, fixup, and execute the binary in-process.
    #[arg(long)]
    exec: bool,

    /// Arguments to pass to the guest binary when using --exec.
    #[arg(long = "exec-args", num_args = 1.., allow_hyphen_values = true)]
    exec_args: Vec<String>,

    /// Show code signature details.
    #[arg(long)]
    codesig: bool,

    /// Show function start addresses.
    #[arg(long)]
    funcs: bool,

    /// Show full symbol table (name, address, type, section).
    #[arg(long)]
    symbols: bool,

    /// Show indirect symbol table (stubs, GOT, lazy symbol pointers).
    #[arg(long)]
    indirect: bool,

    /// Show ObjC selector strings from __objc_selrefs.
    #[arg(long)]
    selrefs: bool,

    /// Show Thread-Local Variable (TLV) descriptors.
    #[arg(long)]
    tlv: bool,

    /// Show relocation entries from section headers.
    #[arg(long)]
    relocs: bool,

    /// Show data-in-code entries (jump tables, embedded data in __TEXT).
    #[arg(long)]
    data_in_code: bool,

    /// Show C-strings from __cstring and related sections.
    #[arg(long)]
    strings: bool,

    /// Show verbose rebase entries (segment, offset, type).
    #[arg(long)]
    rebase: bool,

    /// Diagnose why a binary would fail at runtime.
    #[arg(long)]
    why_failed: bool,

    /// CPU architecture to select from fat/universal binaries (arm64, x86_64).
    #[arg(long, value_parser = parse_cpu_type)]
    cpu: Option<CpuType>,

    /// Enable GUI mode: open real windows for guest AppKit applications.
    #[cfg(feature = "gui")]
    #[arg(long)]
    gui: bool,
}

fn parse_cpu_type(s: &str) -> Result<CpuType, String> {
    match s.to_lowercase().as_str() {
        "arm64" | "aarch64" => Ok(CpuType::Arm64),
        "x86_64" | "x86-64" => Ok(CpuType::X86_64),
        "arm" => Ok(CpuType::Arm),
        "x86" | "i386" => Ok(CpuType::X86),
        _ => Err(format!("unknown CPU type '{s}' (try: arm64, x86_64)")),
    }
}

fn cpu_type_for_host_arch(arch: &str) -> CpuType {
    match arch {
        "x86_64" => CpuType::X86_64,
        "aarch64" => CpuType::Arm64,
        "arm" => CpuType::Arm,
        "x86" | "i386" | "i686" => CpuType::X86,
        _ => CpuType::Arm64,
    }
}

fn default_preferred_cpu() -> CpuType {
    cpu_type_for_host_arch(std::env::consts::ARCH)
}

fn fat_contains_cpu(fat: &FatBinary, cpu: CpuType) -> bool {
    fat.arches.iter().any(|arch| arch.cpu_type == cpu)
}

fn best_default_cpu(fat: &FatBinary) -> CpuType {
    let host_cpu = default_preferred_cpu();
    if fat_contains_cpu(fat, host_cpu) {
        host_cpu
    } else if fat_contains_cpu(fat, CpuType::Arm64) {
        CpuType::Arm64
    } else if fat_contains_cpu(fat, CpuType::X86_64) {
        CpuType::X86_64
    } else {
        fat.arches
            .first()
            .map(|arch| arch.cpu_type)
            .unwrap_or(host_cpu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macho_types::header::{FatArch, FatHeader};

    fn fat_with_arches(cpus: &[CpuType]) -> FatBinary {
        FatBinary {
            header: FatHeader {
                magic: 0,
                nfat_arch: cpus.len() as u32,
            },
            arches: cpus
                .iter()
                .enumerate()
                .map(|(index, cpu_type)| FatArch {
                    cpu_type: *cpu_type,
                    cpu_subtype: 0,
                    offset: index as u64,
                    size: 1,
                    align: 0,
                })
                .collect(),
        }
    }

    #[test]
    fn cpu_type_for_host_arch_maps_x86_64() {
        assert_eq!(cpu_type_for_host_arch("x86_64"), CpuType::X86_64);
    }

    #[test]
    fn cpu_type_for_host_arch_maps_aarch64() {
        assert_eq!(cpu_type_for_host_arch("aarch64"), CpuType::Arm64);
    }

    #[test]
    fn best_default_cpu_prefers_host_cpu_when_present() {
        let host_cpu = default_preferred_cpu();
        let other_cpu = if host_cpu == CpuType::Arm64 {
            CpuType::X86_64
        } else {
            CpuType::Arm64
        };
        let fat = fat_with_arches(&[other_cpu, host_cpu]);
        assert_eq!(best_default_cpu(&fat), host_cpu);
    }

    #[test]
    fn best_default_cpu_falls_back_to_arm64() {
        let fat = fat_with_arches(&[CpuType::Arm64, CpuType::X86]);
        assert_eq!(best_default_cpu(&fat), CpuType::Arm64);
    }

    #[test]
    fn best_default_cpu_falls_back_to_first_arch() {
        let fat = fat_with_arches(&[CpuType::Arm]);
        assert_eq!(best_default_cpu(&fat), CpuType::Arm);
    }
}

fn main() {
    // On Linux, ensure the Apple-ABI libc++ is globally visible so all C++
    // code in the process uses the Apple alternate-string-layout ABI.
    // Without this, internal calls within the system libc++ operate on
    // guest strings using the wrong ABI and crash.
    #[cfg(target_os = "linux")]
    {
        const APPLE_LIBCXX: &str = "/opt/apple-libcxx/libc++.so.1.0";
        if std::path::Path::new(APPLE_LIBCXX).exists() {
            let current = std::env::var("LD_PRELOAD").unwrap_or_default();
            if !current.contains(APPLE_LIBCXX) {
                let new_preload = if current.is_empty() {
                    APPLE_LIBCXX.to_string()
                } else {
                    format!("{current}:{APPLE_LIBCXX}")
                };
                std::env::set_var("LD_PRELOAD", &new_preload);
                let args: Vec<std::ffi::CString> = std::env::args()
                    .map(|a| std::ffi::CString::new(a).unwrap())
                    .collect();
                let c_args: Vec<*const std::ffi::c_char> = args
                    .iter()
                    .map(|a| a.as_ptr())
                    .chain(std::iter::once(std::ptr::null()))
                    .collect();
                let exe =
                    std::ffi::CString::new(std::env::current_exe().unwrap().to_str().unwrap())
                        .unwrap();
                unsafe { libc::execv(exe.as_ptr(), c_args.as_ptr()) };
                // execv only returns on error
                eprintln!("warning: failed to re-exec with LD_PRELOAD, continuing without Apple-ABI libc++");
            }
        }
    }

    env_logger::init();
    let cli = Cli::parse();

    let data = match fs::read(&cli.path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", cli.path.display());
            std::process::exit(1);
        }
    };

    let (parsed, effective_data) = if macho_parse::is_fat(&data) {
        let fat = macho_parse::parse_fat(&data).unwrap_or_else(|e| {
            eprintln!("error: failed to parse fat header: {e}");
            std::process::exit(1);
        });
        let preferred = cli.cpu.unwrap_or_else(|| best_default_cpu(&fat));
        println!(
            "Fat/universal binary detected — selecting {:?} slice",
            preferred
        );
        println!(
            "  Architectures: {}",
            fat.arches
                .iter()
                .map(|a| format!("{:?}", a.cpu_type))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let slice = macho_parse::fat_slice_for_cpu(&data, preferred).unwrap_or_else(|e| {
            eprintln!("error: failed to extract {:?} slice: {e}", preferred);
            std::process::exit(1);
        });
        let parsed = match parse(slice) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: failed to parse {:?} slice: {e}", preferred);
                std::process::exit(1);
            }
        };
        (parsed, std::borrow::Cow::Owned(slice.to_vec()))
    } else {
        let parsed = match parse(&data) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: failed to parse {}: {e}", cli.path.display());
                std::process::exit(1);
            }
        };
        (parsed, std::borrow::Cow::Borrowed(data.as_slice()))
    };

    let abs_path = fs::canonicalize(&cli.path).unwrap_or_else(|_| cli.path.clone());

    if cli.exec {
        #[cfg(feature = "gui")]
        if cli.gui {
            cf_lite::gui_bridge::set_gui_bridge(Box::new(
                window_backend_darwin::WinitGuiBridge::new(),
            ));
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        exec::print_exec(&parsed, &effective_data, &abs_path, &cli.exec_args);
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            eprintln!("error: --exec is only supported on macOS and Linux");
            std::process::exit(1);
        }
    } else if cli.why_failed {
        why_failed::print_why_failed(&parsed, &abs_path, &data);
    } else if cli.plan {
        print_info::print_plan(&parsed, &abs_path, &data);
    } else if cli.resolve {
        print_info::print_resolve(&parsed, &abs_path);
    } else if cli.json_full {
        print_json_full(&parsed, &data, &effective_data);
    } else if cli.json {
        print_json(&parsed);
    } else {
        let fixups = extract_fixups(&parsed, &data);
        let exports = extract_exports(&parsed, &data);
        print_text::print_text(&parsed, &data, &fixups, &exports);
    }

    if cli.symbols {
        print_text::print_symbols(&parsed);
    }

    if cli.indirect {
        print_text::print_indirect(&parsed, &data);
    }

    if cli.selrefs {
        let meta = macho_objc::parse_objc_metadata(&parsed, &data);
        if meta.selectors.is_empty() {
            println!("\n=== ObjC Selectors ===");
            println!("  (none)");
        } else {
            println!("\n=== ObjC Selectors ({}) ===", meta.selectors.len());
            for sel in &meta.selectors {
                println!("  {sel}");
            }
        }
    }

    if cli.tlv {
        let descriptors = macho_parse::parse_tlv_descriptors(&parsed, &effective_data);
        if descriptors.is_empty() {
            println!("\n=== TLV Descriptors ===");
            println!("  (none)");
        } else {
            println!("\n=== TLV Descriptors ({}) ===", descriptors.len());
            for (i, d) in descriptors.iter().enumerate() {
                println!(
                    "  [{i:2}] addr={:#x} thunk={:#x} key={:#x} offset={:#x}",
                    d.address, d.thunk, d.key, d.offset
                );
            }
        }
    }

    if cli.codesig {
        print_info::print_codesig(&parsed, &effective_data);
    }

    if cli.relocs {
        let relocs = macho_parse::parse_relocations(&parsed, &effective_data);
        if relocs.is_empty() {
            println!("\n=== Relocation Entries ===");
            println!("  (none)");
        } else {
            let total: usize = relocs.iter().map(|r| r.entries.len()).sum();
            println!("\n=== Relocation Entries ({total}) ===");
            for sr in &relocs {
                println!(
                    "  --- {},{} ({}) ---",
                    sr.segment,
                    sr.section,
                    sr.entries.len()
                );
                for (i, rel) in sr.entries.iter().enumerate() {
                    println!(
                        "    [{i:3}] addr={:#x} sym={} {:?} len={} {}{}",
                        rel.address,
                        rel.symbol_index,
                        rel.reloc_type,
                        rel.length,
                        if rel.extern_sym { "extern" } else { "local" },
                        if rel.pc_relative { " pcrel" } else { "" }
                    );
                }
            }
        }
    }

    if cli.funcs {
        let starts = macho_parse::parse_function_starts(&parsed, &effective_data);
        println!("\n=== Function Starts ({}) ===", starts.len());
        for (i, addr) in starts.iter().enumerate() {
            println!("  [{i:3}] {addr:#x}");
        }
    }

    if cli.data_in_code {
        let entries = macho_parse::parse_data_in_code(&parsed, &effective_data);
        if entries.is_empty() {
            println!("\n=== Data-in-Code ===");
            println!("  (none)");
        } else {
            println!("\n=== Data-in-Code ({}) ===", entries.len());
            for (i, e) in entries.iter().enumerate() {
                println!(
                    "  [{i:3}] offset={:#x} length={} kind={:?}",
                    e.offset, e.length, e.kind
                );
            }
        }
    }

    if cli.strings {
        let sections = macho_parse::extract_cstrings(&parsed, &effective_data);
        if sections.is_empty() {
            println!("\n=== C-Strings ===");
            println!("  (none)");
        } else {
            let total: usize = sections.iter().map(|s| s.strings.len()).sum();
            println!("\n=== C-Strings ({total}) ===");
            for ss in &sections {
                println!(
                    "  --- {},{} ({}) ---",
                    ss.segment,
                    ss.section,
                    ss.strings.len()
                );
                for s in &ss.strings {
                    println!("    {s}");
                }
            }
        }
    }

    if cli.rebase {
        let fixups = extract_fixups(&parsed, &effective_data);
        if fixups.rebases.is_empty() {
            println!("\n=== Rebase Entries ===");
            println!("  (none)");
        } else {
            println!("\n=== Rebase Entries ({}) ===", fixups.rebases.len());
            for (i, r) in fixups.rebases.iter().enumerate() {
                println!(
                    "  [{i:3}] seg[{}]+{:#x} type={}",
                    r.segment_index, r.segment_offset, r.rebase_type
                );
            }
        }
    }

    if cli.security {
        print_info::print_security(&parsed, &cli.path, &data);
    }
}

fn print_json(parsed: &ParsedMachO) {
    match serde_json::to_string_pretty(parsed) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("error: JSON serialisation failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Comprehensive JSON report including all parsed data.
#[derive(Serialize)]
struct JsonReport {
    parsed: ParsedMachO,
    fixups: Fixups,
    exports: Vec<ExportedSymbol>,
    objc: macho_objc::ObjcMetadata,
    function_starts: Vec<u64>,
    tlv_descriptors: Vec<TlvDescriptor>,
    indirect_symbols: Vec<macho_parse::IndirectSymbolEntry>,
    entitlement_keys: Vec<(String, bool)>,
    relocations: Vec<macho_parse::SectionRelocations>,
    data_in_code: Vec<macho_types::DataInCodeEntry>,
}

fn print_json_full(parsed: &ParsedMachO, data: &[u8], effective_data: &[u8]) {
    let fixups = extract_fixups(parsed, data);
    let exports = extract_exports(parsed, data);
    let objc = macho_objc::parse_objc_metadata(parsed, data);
    let function_starts = macho_parse::parse_function_starts(parsed, effective_data);
    let tlv_descriptors = macho_parse::parse_tlv_descriptors(parsed, effective_data);
    let indirect_symbols = macho_parse::resolve_indirect_symbols(parsed, data);
    let entitlement_keys = parsed
        .code_signature
        .as_ref()
        .map(|cs| cs.entitlement_keys())
        .unwrap_or_default();
    let relocations = macho_parse::parse_relocations(parsed, effective_data);
    let data_in_code = macho_parse::parse_data_in_code(parsed, effective_data);

    let report = JsonReport {
        parsed: parsed.clone(),
        fixups,
        exports,
        objc,
        function_starts,
        tlv_descriptors,
        indirect_symbols,
        entitlement_keys,
        relocations,
        data_in_code,
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("error: JSON serialisation failed: {e}");
            std::process::exit(1);
        }
    }
}
