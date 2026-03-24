//! Specialized info printers: resolve, plan, security, codesig.

use std::path::Path;

use macho_parse::{verify_page_hashes, ParsedMachO};
use macho_resolve::ResolutionPlan;
use macho_run_plan::ExecutionPlan;
use trust_policy::TrustPolicy;

pub fn print_resolve(parsed: &ParsedMachO, binary_path: &Path) {
    let plan = ResolutionPlan::from_parsed_transitive(parsed, binary_path.to_path_buf());

    println!("=== Dependency Resolution Tree ===");
    println!("root: {}", binary_path.display());

    // Group edges by parent to print a tree
    for node in &plan.graph.nodes {
        if node.index == 0 {
            continue;
        }
        let indent = "  ".repeat(node.depth);
        let edge = plan.graph.edges.iter().find(|e| e.to == node.index);
        let raw = edge.map(|e| e.raw_name.as_str()).unwrap_or("?");
        println!("{indent}{raw}");
        println!("{indent}  → {}", node.path.display());
    }

    if !plan.unresolved.is_empty() {
        println!("\n=== Unresolved ===");
        for name in &plan.unresolved {
            println!("  [UNRESOLVED] {name}");
        }
    }

    println!("\n=== Initialization Order ===");
    let init_order = plan.init_order();
    for (i, &idx) in init_order.iter().enumerate() {
        let node = &plan.graph.nodes[idx];
        let label = node
            .install_name
            .as_deref()
            .unwrap_or_else(|| node.path.to_str().unwrap_or("?"));
        println!("  {}: {label}", i + 1);
    }
}

pub fn print_plan(parsed: &ParsedMachO, binary_path: &Path, data: &[u8]) {
    let plan = ExecutionPlan::from_parsed_full(parsed, binary_path.to_path_buf(), data);
    print!("{}", plan.display());
}

pub fn print_security(parsed: &ParsedMachO, binary_path: &Path, data: &[u8]) {
    println!("\n=== Security Evaluation (Apple-default policy) ===");

    let policy = TrustPolicy::apple_default();
    let plan = ExecutionPlan::from_parsed_secured(parsed, binary_path.to_path_buf(), data, &policy);

    if let Some(ref sec) = plan.security {
        for decision in &sec.decisions {
            let status = match &decision.verdict {
                trust_policy::SecurityVerdict::Allow => "ALLOW",
                trust_policy::SecurityVerdict::Deny { .. } => "DENY",
                trust_policy::SecurityVerdict::AllowWithWarning { .. } => "WARN",
            };
            let detail = match &decision.verdict {
                trust_policy::SecurityVerdict::Deny { reason } => format!(" — {reason}"),
                trust_policy::SecurityVerdict::AllowWithWarning { warning } => {
                    format!(" — {warning}")
                }
                _ => String::new(),
            };
            println!("  [{status:5}] {}{detail}", decision.gate);
        }

        println!();
        if sec.allowed {
            println!("  Result: ALLOWED");
        } else {
            println!("  Result: DENIED");
            for reason in &sec.denied_reasons {
                println!("    → {reason}");
            }
        }

        if sec.strip_dyld_env {
            println!("  DYLD_* env vars: STRIPPED");
        }
        if sec.hardened_runtime {
            println!("  Hardened runtime: ACTIVE");
        }
    }

    // Also show dylib validation for each dependency
    if !plan.dependencies.graph.edges.is_empty() {
        println!("\n=== Dylib Validation ===");
        for edge in &plan.dependencies.graph.edges {
            let node = &plan.dependencies.graph.nodes[edge.to];
            let verdict = policy.validate_dylib(binary_path, &node.path);
            let status = match &verdict {
                trust_policy::SecurityVerdict::Allow => "ALLOW",
                trust_policy::SecurityVerdict::Deny { .. } => "DENY",
                trust_policy::SecurityVerdict::AllowWithWarning { .. } => "WARN",
            };
            println!("  [{status:5}] {}", node.path.display());
        }
    }
}

pub fn print_codesig(parsed: &ParsedMachO, data: &[u8]) {
    println!("\n=== Code Signature ===");
    match &parsed.code_signature {
        None => println!("  Not present"),
        Some(cs) => {
            println!(
                "  SuperBlob: magic={:#x} length={} count={}",
                cs.super_blob.magic, cs.super_blob.length, cs.super_blob.count
            );
            if let Some(cd) = &cs.code_directory {
                println!("  CodeDirectory:");
                println!("    version:       {:#x}", cd.version);
                println!("    flags:         {:?}", cd.flags);
                println!("    identity:      {}", cd.identity);
                println!(
                    "    team_id:       {}",
                    if cd.team_id.is_empty() {
                        "(none)"
                    } else {
                        &cd.team_id
                    }
                );
                println!("    hash_type:     {:?}", cd.hash_type);
                println!("    page_size:     {} bytes", 1u64 << cd.page_size_log2);
                println!("    code_limit:    {:#x}", cd.code_limit);
                println!("    code_slots:    {}", cd.n_code_slots);
                println!("    special_slots: {}", cd.n_special_slots);
                let hash_hex: String = cd.cd_hash.iter().map(|b| format!("{b:02x}")).collect();
                println!("    CDHash:        {hash_hex}");

                let result = verify_page_hashes(data, cd);
                if result.mismatches.is_empty() {
                    println!(
                        "    Page hashes:   {}/{} verified OK",
                        result.verified, result.verified
                    );
                } else {
                    println!(
                        "    Page hashes:   MISMATCH — {}/{} clean, {} bad",
                        result.verified - result.mismatches.len(),
                        result.verified,
                        result.mismatches.len()
                    );
                    for m in &result.mismatches {
                        println!(
                            "      slot {} ({:#x}..{:#x})",
                            m.slot, m.file_range.start, m.file_range.end
                        );
                    }
                }
            }
            println!(
                "  CMS signature:   {}",
                if cs.has_cms_signature {
                    "present"
                } else {
                    "none"
                }
            );
            if let Some(ent) = &cs.entitlements_plist {
                let preview: String = ent.chars().take(200).collect();
                println!(
                    "  Entitlements:    {preview}{}",
                    if ent.len() > 200 { "..." } else { "" }
                );
                let keys = cs.entitlement_keys();
                if !keys.is_empty() {
                    println!("  Entitlement keys:");
                    for (key, val) in &keys {
                        println!("    {key} = {val}");
                    }
                }
            }
        }
    }
}
