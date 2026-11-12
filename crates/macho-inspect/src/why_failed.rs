//! `--why-failed` analysis: diagnose why a binary would fail at runtime.
//!
//! Performs static analysis: dependency resolution, symbol binding, and
//! ObjC metadata inspection — then reports all compatibility issues.

use std::path::Path;

use macho_parse::ParsedMachO;
use macho_run_plan::ExecutionPlan;

pub fn print_why_failed(parsed: &ParsedMachO, binary_path: &Path, data: &[u8]) {
    let plan = ExecutionPlan::from_parsed_full(parsed, binary_path.to_path_buf(), data);

    println!(
        "=== Compatibility Diagnosis: {} ===\n",
        binary_path.display()
    );

    let mut issue_count = 0;

    // ── 1. Unresolved dylibs ───────────────────────────────────────

    if !plan.dependencies.unresolved.is_empty() {
        println!(
            "UNRESOLVED DYLIBS ({}):",
            plan.dependencies.unresolved.len()
        );
        for name in &plan.dependencies.unresolved {
            println!("  ✗ {name}");
            issue_count += 1;
        }
        println!();
    }

    // ── 2. Failed symbol bindings ──────────────────────────────────

    if let Some(ref bindings) = plan.symbol_bindings {
        if !bindings.failed.is_empty() {
            println!("FAILED SYMBOL BINDINGS ({}):", bindings.failed.len());
            for sym in &bindings.failed {
                // Find the binding entry for more detail
                if let Some(b) = bindings.bindings.iter().find(|b| b.name == *sym) {
                    println!("  ✗ {sym}  (dylib ordinal: {})", b.dylib_ordinal);
                } else {
                    println!("  ✗ {sym}");
                }
                issue_count += 1;
            }
            println!();
        }

        // Summary line for bindings
        let total = bindings.bindings.len();
        let ok = bindings.bound_count;
        let weak = bindings.weak_unbound_count;
        let failed = bindings.failed.len();
        println!("Symbol bindings: {ok}/{total} bound, {weak} weak-unbound, {failed} failed");
    }

    // ── 3. ObjC metadata summary ──────────────────────────────────

    if let Some(ref meta) = plan.objc_metadata {
        let n_classes = meta.class_refs.len();
        let n_cats = meta.category_count;
        let n_sels = meta.selectors.len();
        let n_protos = meta.protocol_count;
        println!(
            "ObjC metadata: {n_classes} class refs, {n_cats} categories, \
             {n_sels} selectors, {n_protos} protocols"
        );
    }

    // ── 4. Initializer info ────────────────────────────────────────

    if let Some(ref inits) = plan.initializers {
        let n_ctors = inits.constructors.len();
        let n_steps = inits.init_steps.len();
        if n_ctors > 0 || n_steps > 0 {
            println!("Initializers: {n_ctors} constructors, {n_steps} init steps");
        }
    }

    // ── 5. Dependency tree depth ───────────────────────────────────

    let max_depth = plan
        .dependencies
        .graph
        .nodes
        .iter()
        .map(|n| n.depth)
        .max()
        .unwrap_or(0);
    let n_deps = plan.dependencies.graph.nodes.len().saturating_sub(1);
    println!("Dependencies: {n_deps} dylibs, max depth {max_depth}");

    // ── Verdict ────────────────────────────────────────────────────

    println!();
    if issue_count == 0 {
        println!("VERDICT: All static checks passed — binary should load successfully.");
    } else {
        println!(
            "VERDICT: {issue_count} issue{} found — binary may fail at runtime.",
            if issue_count == 1 { "" } else { "s" }
        );
    }
}
