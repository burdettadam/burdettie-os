//! Human-readable text output for Mach-O inspection.

use macho_parse::{ExportedSymbol, Fixups, ParsedMachO};

pub fn print_text(parsed: &ParsedMachO, data: &[u8], fixups: &Fixups, exports: &[ExportedSymbol]) {
    // ── Header ──
    println!("=== Mach-O Header ===");
    println!("  magic:    {:#010x}", parsed.header.magic);
    println!("  cpu_type: {:?}", parsed.header.cpu_type);
    println!("  file_type: {:?}", parsed.header.file_type);
    println!("  commands: {}", parsed.header.num_commands);
    println!("  flags:    {:?}", parsed.header.flags);

    if let Some(uuid) = &parsed.uuid {
        let s: Vec<String> = uuid.iter().map(|b| format!("{b:02X}")).collect();
        println!("  uuid:     {}", s.join(""));
    }

    // ── Entry point ──
    if let Some(entry) = &parsed.entry {
        println!("\n=== Entry Point ===");
        match entry {
            macho_types::EntryPoint::Main {
                entry_offset,
                stack_size,
            } => {
                println!("  LC_MAIN offset: {entry_offset:#x}");
                println!("  stack_size:     {stack_size:#x}");
            }
            macho_types::EntryPoint::UnixThread { entry_address } => {
                println!("  LC_UNIXTHREAD address: {entry_address:#x}");
            }
        }
    }

    // ── Segments ──
    if !parsed.segments.is_empty() {
        println!("\n=== Segments ===");
        for seg in &parsed.segments {
            println!(
                "  {} vmaddr={:#010x} vmsize={:#x} fileoff={:#x} filesize={:#x} prot={} maxprot={}",
                seg.name,
                seg.vmaddr,
                seg.vmsize,
                seg.fileoff,
                seg.filesize,
                seg.initprot,
                seg.maxprot
            );
            for sect in &seg.sections {
                println!(
                    "    {} addr={:#010x} size={:#x} offset={:#x}",
                    sect.sectname, sect.addr, sect.size, sect.offset
                );
            }
        }
    }

    // ── Dylibs ──
    if !parsed.dylibs.is_empty() {
        println!("\n=== Dylib Dependencies ===");
        for dlib in &parsed.dylibs {
            println!(
                "  {} (compat: {}, current: {})",
                dlib.name,
                dlib.compat_version_string(),
                dlib.current_version_string()
            );
        }
    }

    // ── Rpaths ──
    if !parsed.rpaths.is_empty() {
        println!("\n=== RPaths ===");
        for rpath in &parsed.rpaths {
            println!("  {rpath}");
        }
    }

    // ── Load commands summary ──
    println!("\n=== Load Commands ({}) ===", parsed.commands.len());
    for cmd in &parsed.commands {
        println!("  {:?}  size={}", cmd.cmd, cmd.cmd_size);
    }

    // ── Symbols summary ──
    if let Some(syms) = &parsed.symbols {
        let total = syms.symbols.len();
        let undefined = syms
            .symbols
            .iter()
            .filter(|s| matches!(s.sym_type, macho_types::SymbolType::Undefined))
            .count();
        let defined = syms
            .symbols
            .iter()
            .filter(|s| matches!(s.sym_type, macho_types::SymbolType::Defined))
            .count();
        println!("\n=== Symbols ===");
        println!("  total: {total}  defined: {defined}  undefined: {undefined}");
    }

    // ── Fixups ──
    println!("\n=== Fixups ===");
    println!("  model: {:?}", fixups.model);
    if !fixups.rebases.is_empty() {
        println!("  rebases: {}", fixups.rebases.len());
    }
    if !fixups.binds.is_empty() {
        println!("  binds: {}", fixups.binds.len());
        for bind in fixups.binds.iter().take(10) {
            println!(
                "    seg[{}]+{:#x} → dylib[{}] {}{}",
                bind.segment_index,
                bind.segment_offset,
                bind.dylib_ordinal,
                bind.symbol_name,
                if bind.is_weak { " [weak]" } else { "" }
            );
        }
        if fixups.binds.len() > 10 {
            println!("    ... and {} more", fixups.binds.len() - 10);
        }
    }
    if !fixups.weak_binds.is_empty() {
        println!("  weak_binds: {}", fixups.weak_binds.len());
        for bind in fixups.weak_binds.iter().take(10) {
            println!(
                "    seg[{}]+{:#x} → {}",
                bind.segment_index, bind.segment_offset, bind.symbol_name
            );
        }
        if fixups.weak_binds.len() > 10 {
            println!("    ... and {} more", fixups.weak_binds.len() - 10);
        }
    }
    if !fixups.lazy_binds.is_empty() {
        println!("  lazy_binds: {}", fixups.lazy_binds.len());
        for bind in fixups.lazy_binds.iter().take(10) {
            println!(
                "    seg[{}]+{:#x} → dylib[{}] {}",
                bind.segment_index, bind.segment_offset, bind.dylib_ordinal, bind.symbol_name
            );
        }
        if fixups.lazy_binds.len() > 10 {
            println!("    ... and {} more", fixups.lazy_binds.len() - 10);
        }
    }
    if !fixups.chained.is_empty() {
        println!("  chained fixups: {}", fixups.chained.len());
    }

    // ── Exports ──
    if !exports.is_empty() {
        println!("\n=== Exports ({}) ===", exports.len());
        for exp in exports.iter().take(20) {
            println!(
                "  {} addr={:#x} kind={} flags={:#x}{}",
                exp.name,
                exp.address,
                exp.kind(),
                exp.flags,
                if exp.is_weak() { " [weak]" } else { "" }
            );
        }
        if exports.len() > 20 {
            println!("  ... and {} more", exports.len() - 20);
        }
    }

    // ── ObjC Metadata ──
    let objc = macho_objc::parse_objc_metadata(parsed, data);
    if objc.presence.has_objc {
        println!("\n=== ObjC Metadata ===");
        println!("  sections: {}", objc.presence.sections_found.join(", "));
        if !objc.class_refs.is_empty() {
            println!("  classes: {}", objc.class_refs.len());
            for cls in &objc.class_refs {
                if let Some(name) = &cls.name {
                    println!(
                        "    {name} ptr={:#x} data={:#x}",
                        cls.pointer_addr, cls.class_data_addr
                    );
                } else {
                    println!(
                        "    <unknown> ptr={:#x} data={:#x}",
                        cls.pointer_addr, cls.class_data_addr
                    );
                }
            }
        }
        if objc.category_count > 0 {
            println!("  categories: {}", objc.category_count);
        }
        if objc.protocol_count > 0 {
            println!("  protocols: {}", objc.protocol_count);
        }
        if !objc.selectors.is_empty() {
            println!("  selectors: {}", objc.selectors.len());
        }
        if let Some(info) = &objc.image_info {
            println!(
                "  image_info: version={} flags={:#x}",
                info.version, info.flags
            );
        }
    }

    // ── Build/Source Version ──
    for cmd in &parsed.commands {
        match &cmd.payload {
            macho_types::load_command::LoadCommandPayload::BuildVersion {
                platform,
                minos,
                sdk,
                ..
            } => {
                println!("\n=== Build Version ===");
                println!(
                    "  platform: {} ({})",
                    macho_types::load_command::platform_name(*platform),
                    platform
                );
                println!(
                    "  min_os:   {}",
                    macho_types::load_command::decode_version(*minos)
                );
                println!(
                    "  sdk:      {}",
                    macho_types::load_command::decode_version(*sdk)
                );
            }
            macho_types::load_command::LoadCommandPayload::SourceVersion { version } => {
                println!("\n=== Source Version ===");
                println!(
                    "  {}",
                    macho_types::load_command::decode_source_version(*version)
                );
            }
            _ => {}
        }
    }
}

pub fn print_symbols(parsed: &ParsedMachO) {
    if let Some(syms) = &parsed.symbols {
        println!("\n=== Symbol Table ({}) ===", syms.symbols.len());
        for sym in &syms.symbols {
            let mut flags = Vec::new();
            if sym.is_weak_ref() {
                flags.push("weak_ref");
            }
            if sym.is_weak_def() {
                flags.push("weak_def");
            }
            if sym.is_no_dead_strip() {
                flags.push("no_dead_strip");
            }
            if sym.is_referenced_dynamically() {
                flags.push("ref_dyn");
            }
            let flag_str = if flags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", flags.join(","))
            };
            println!(
                "  {:#010x}  {:?}  sect={}  {}{}",
                sym.value, sym.sym_type, sym.section_index, sym.name, flag_str
            );
        }
    } else {
        println!("\n=== Symbol Table ===");
        println!("  (none)");
    }
}

pub fn print_indirect(parsed: &ParsedMachO, data: &[u8]) {
    let entries = macho_parse::resolve_indirect_symbols(parsed, data);
    if entries.is_empty() {
        println!("\n=== Indirect Symbol Table ===");
        println!("  (none)");
    } else {
        println!("\n=== Indirect Symbol Table ({}) ===", entries.len());
        let mut current_section = String::new();
        for entry in &entries {
            if entry.section != current_section {
                current_section = entry.section.clone();
                println!("  --- {} ---", current_section);
            }
            println!(
                "    [{:2}] {:#010x}  {}",
                entry.slot_index, entry.slot_addr, entry.symbol_name
            );
        }
    }
}
