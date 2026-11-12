use std::path::PathBuf;

use macho_map::MappingPlan;
use macho_objc::{InitializerInfo, ObjcMetadata, ObjcPresence};
use macho_parse::ParsedMachO;
use macho_resolve::{BindingReport, ResolutionPlan};
use macho_types::EntryPoint;
use serde::{Deserialize, Serialize};
use trust_policy::{CodeSignatureInfo, SecurityReport, TrustPolicy};

/// Complete execution plan for a Mach-O binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub binary_path: PathBuf,
    pub mapping: MappingPlan,
    pub dependencies: ResolutionPlan,
    pub objc: ObjcPresence,
    pub objc_metadata: Option<ObjcMetadata>,
    pub initializers: Option<InitializerInfo>,
    pub symbol_bindings: Option<BindingReport>,
    pub security: Option<SecurityReport>,
    pub init_order: Vec<String>,
    pub entry: Option<EntryPoint>,
}

impl ExecutionPlan {
    /// Build an execution plan from a parsed Mach-O.
    pub fn from_parsed(parsed: &ParsedMachO, binary_path: PathBuf) -> Self {
        Self::build(parsed, binary_path, false, None, None)
    }

    /// Build an execution plan with transitive dependency resolution.
    pub fn from_parsed_transitive(parsed: &ParsedMachO, binary_path: PathBuf) -> Self {
        Self::build(parsed, binary_path, true, None, None)
    }

    /// Build a full execution plan with symbol binding.
    pub fn from_parsed_full(parsed: &ParsedMachO, binary_path: PathBuf, raw_data: &[u8]) -> Self {
        Self::build(parsed, binary_path, true, Some(raw_data), None)
    }

    /// Build a full execution plan with symbol binding and security evaluation.
    pub fn from_parsed_secured(
        parsed: &ParsedMachO,
        binary_path: PathBuf,
        raw_data: &[u8],
        policy: &TrustPolicy,
    ) -> Self {
        Self::build(parsed, binary_path, true, Some(raw_data), Some(policy))
    }

    fn build(
        parsed: &ParsedMachO,
        binary_path: PathBuf,
        transitive: bool,
        raw_data: Option<&[u8]>,
        policy: Option<&TrustPolicy>,
    ) -> Self {
        let mapping = MappingPlan::from_parsed(parsed);
        let dependencies = if transitive {
            ResolutionPlan::from_parsed_transitive(parsed, binary_path.clone())
        } else {
            ResolutionPlan::from_parsed(parsed, binary_path.clone())
        };
        let objc = macho_objc::detect_objc(parsed);

        // Run security evaluation if a policy is provided
        let security = policy.map(|p| {
            let cs_info = parsed
                .code_signature
                .as_ref()
                .map(CodeSignatureInfo::from_parsed)
                .unwrap_or_default();
            let entitlement_keys: Vec<(String, bool)> = parsed
                .code_signature
                .as_ref()
                .map(|cs| cs.entitlement_keys())
                .unwrap_or_default();
            p.evaluate_binary_parsed_with_keys(
                &binary_path,
                parsed.header.flags,
                &cs_info,
                &entitlement_keys,
            )
        });

        // Parse ObjC metadata and initializers if raw data available
        let (objc_metadata, initializers, symbol_bindings) = if let Some(data) = raw_data {
            let meta = macho_objc::parse_objc_metadata(parsed, data);
            let inits = macho_objc::extract_initializers(parsed, data);
            let bindings = macho_resolve::bind_symbols(data, &dependencies);
            (Some(meta), Some(inits), Some(bindings))
        } else {
            (None, None, None)
        };

        // Use proper bottom-up init order from the dependency graph
        let order_indices = dependencies.init_order();
        let init_order: Vec<String> = order_indices
            .iter()
            .map(|&idx| {
                let node = &dependencies.graph.nodes[idx];
                node.install_name
                    .clone()
                    .unwrap_or_else(|| node.path.to_string_lossy().into_owned())
            })
            .collect();

        Self {
            binary_path,
            mapping,
            dependencies,
            objc,
            objc_metadata,
            initializers,
            symbol_bindings,
            security,
            init_order,
            entry: parsed.entry.clone(),
        }
    }

    /// Pretty-print a numbered execution plan.
    pub fn display(&self) -> String {
        let mut out = String::new();
        let mut step = 1;

        out.push_str(&format!(
            "{step}. Load main binary: {}\n",
            self.binary_path.display()
        ));
        step += 1;

        for region in &self.mapping.regions {
            out.push_str(&format!(
                "{step}. Map {} vmaddr={:#010x} vmsize={:#x} prot={} maxprot={}\n",
                region.segment_name,
                region.vmaddr,
                region.vmsize,
                region.protection,
                region.max_protection
            ));
            step += 1;
        }

        for edge in &self.dependencies.graph.edges {
            let node = &self.dependencies.graph.nodes[edge.to];
            out.push_str(&format!(
                "{step}. Resolve {} → {}\n",
                edge.raw_name,
                node.path.display()
            ));
            step += 1;
        }

        if !self.dependencies.unresolved.is_empty() {
            for name in &self.dependencies.unresolved {
                out.push_str(&format!("{step}. [UNRESOLVED] {name}\n"));
                step += 1;
            }
        }

        // Symbol binding summary
        if let Some(ref bindings) = self.symbol_bindings {
            out.push_str(&format!(
                "{step}. Bind symbols: {} bound, {} failed, {} weak-unbound\n",
                bindings.bound_count,
                bindings.failed.len(),
                bindings.weak_unbound_count
            ));
            step += 1;
            for name in &bindings.failed {
                out.push_str(&format!("    [BIND FAILED] {name}\n"));
            }
        } else {
            out.push_str(&format!("{step}. Build symbol binding order\n"));
            step += 1;
        }

        // Initializer detail
        if let Some(ref inits) = self.initializers {
            out.push_str(&format!("{step}. Run initializers in dependency order\n"));
            step += 1;
            for init_step in &inits.init_steps {
                out.push_str(&format!("    {init_step}\n"));
            }
        } else {
            out.push_str(&format!("{step}. Run initializers in dependency order\n"));
            step += 1;
        }

        if let Some(entry) = &self.entry {
            match entry {
                EntryPoint::Main { entry_offset, .. } => {
                    out.push_str(&format!(
                        "{step}. Transfer to LC_MAIN entry (offset {entry_offset:#x})\n"
                    ));
                }
                EntryPoint::UnixThread { entry_address } => {
                    out.push_str(&format!(
                        "{step}. Transfer to LC_UNIXTHREAD entry ({entry_address:#x})\n"
                    ));
                }
            }
        }

        // ObjC detail
        if let Some(ref meta) = self.objc_metadata {
            if meta.presence.has_objc {
                out.push_str(&format!(
                    "\nObjC: {} classes, {} categories, {} protocols\n",
                    meta.class_refs.len(),
                    meta.category_count,
                    meta.protocol_count
                ));
            }
        } else if self.objc.has_objc {
            out.push_str(&format!(
                "\nNote: binary contains ObjC metadata sections: {:?}\n",
                self.objc.sections_found
            ));
        }

        // Security report
        if let Some(ref sec) = self.security {
            out.push('\n');
            if sec.allowed {
                out.push_str("Security: all gates passed\n");
            } else {
                out.push_str("Security: DENIED\n");
                for reason in &sec.denied_reasons {
                    out.push_str(&format!("  - {reason}\n"));
                }
            }
            if sec.strip_dyld_env {
                out.push_str("  DYLD_* environment variables stripped\n");
            }
            if sec.hardened_runtime {
                out.push_str("  Hardened runtime active\n");
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macho_map::MapRegion;
    use macho_resolve::{DependencyEdge, DependencyGraph, ImageNode};
    use macho_types::segment::VmProt;
    use trust_policy::{GateDecision, SecurityVerdict};

    /// Helper to build a minimal ExecutionPlan with no deps and no security.
    fn minimal_plan(path: &str) -> ExecutionPlan {
        ExecutionPlan {
            binary_path: PathBuf::from(path),
            mapping: MappingPlan { regions: vec![] },
            dependencies: ResolutionPlan {
                graph: DependencyGraph {
                    nodes: vec![],
                    edges: vec![],
                },
                rpaths: vec![],
                unresolved: vec![],
                transitive: false,
            },
            objc: ObjcPresence {
                has_objc: false,
                sections_found: vec![],
            },
            objc_metadata: None,
            initializers: None,
            symbol_bindings: None,
            security: None,
            init_order: vec![],
            entry: None,
        }
    }

    #[test]
    fn display_contains_binary_path() {
        let plan = minimal_plan("/usr/local/bin/hello");
        let out = plan.display();
        assert!(out.contains("/usr/local/bin/hello"));
        assert!(out.contains("Load main binary"));
    }

    #[test]
    fn empty_plan_display() {
        let plan = minimal_plan("/tmp/empty");
        let out = plan.display();
        // Should still produce load step and init/binding steps
        assert!(out.contains("Load main binary"));
        assert!(out.contains("Build symbol binding order"));
        assert!(out.contains("Run initializers"));
    }

    #[test]
    fn plan_init_order_empty() {
        let plan = minimal_plan("/tmp/nodeps");
        assert!(plan.init_order.is_empty());
        // Display should not mention dependency edges
        let out = plan.display();
        assert!(!out.contains("Resolve"));
    }

    #[test]
    fn display_with_mapping_regions() {
        let mut plan = minimal_plan("/tmp/mapped");
        plan.mapping.regions.push(MapRegion {
            segment_name: "__TEXT".to_string(),
            vmaddr: 0x1_0000_0000,
            vmsize: 0x4000,
            fileoff: 0,
            filesize: 0x4000,
            protection: VmProt::READ | VmProt::EXECUTE,
            max_protection: VmProt::READ | VmProt::EXECUTE,
            zerofill_size: 0,
        });
        let out = plan.display();
        assert!(out.contains("__TEXT"));
        assert!(out.contains("Map"));
    }

    #[test]
    fn display_with_entry_main() {
        let mut plan = minimal_plan("/tmp/entry");
        plan.entry = Some(EntryPoint::Main {
            entry_offset: 0x1234,
            stack_size: 0,
        });
        let out = plan.display();
        assert!(out.contains("LC_MAIN"));
        assert!(out.contains("0x1234"));
    }

    #[test]
    fn display_with_entry_unixthread() {
        let mut plan = minimal_plan("/tmp/old");
        plan.entry = Some(EntryPoint::UnixThread {
            entry_address: 0xdead_beef,
        });
        let out = plan.display();
        assert!(out.contains("LC_UNIXTHREAD"));
        assert!(out.contains("0xdeadbeef"));
    }

    #[test]
    fn security_report_all_passed() {
        let mut plan = minimal_plan("/tmp/secure");
        plan.security = Some(SecurityReport {
            decisions: vec![GateDecision {
                gate: "codesign".to_string(),
                verdict: SecurityVerdict::Allow,
            }],
            allowed: true,
            denied_reasons: vec![],
            strip_dyld_env: false,
            hardened_runtime: false,
            cs_info: None,
        });
        let out = plan.display();
        assert!(out.contains("Security: all gates passed"));
        assert!(!out.contains("DENIED"));
    }

    #[test]
    fn security_report_denied() {
        let mut plan = minimal_plan("/tmp/denied");
        plan.security = Some(SecurityReport {
            decisions: vec![GateDecision {
                gate: "codesign".to_string(),
                verdict: SecurityVerdict::Deny {
                    reason: "unsigned".to_string(),
                },
            }],
            allowed: false,
            denied_reasons: vec!["unsigned binary".to_string()],
            strip_dyld_env: true,
            hardened_runtime: true,
            cs_info: None,
        });
        let out = plan.display();
        assert!(out.contains("Security: DENIED"));
        assert!(out.contains("unsigned binary"));
        assert!(out.contains("DYLD_*"));
        assert!(out.contains("Hardened runtime"));
    }

    #[test]
    fn display_with_dependency_edges() {
        let mut plan = minimal_plan("/tmp/deps");
        plan.dependencies.graph.nodes.push(ImageNode {
            index: 0,
            path: PathBuf::from("/tmp/deps"),
            install_name: None,
            depth: 0,
        });
        plan.dependencies.graph.nodes.push(ImageNode {
            index: 1,
            path: PathBuf::from("/usr/lib/libSystem.B.dylib"),
            install_name: Some("/usr/lib/libSystem.B.dylib".to_string()),
            depth: 1,
        });
        plan.dependencies.graph.edges.push(DependencyEdge {
            from: 0,
            to: 1,
            raw_name: "/usr/lib/libSystem.B.dylib".to_string(),
        });
        let out = plan.display();
        assert!(out.contains("Resolve"));
        assert!(out.contains("libSystem.B.dylib"));
    }

    #[test]
    fn display_with_unresolved() {
        let mut plan = minimal_plan("/tmp/missing");
        plan.dependencies
            .unresolved
            .push("/usr/lib/libmissing.dylib".to_string());
        let out = plan.display();
        assert!(out.contains("[UNRESOLVED]"));
        assert!(out.contains("libmissing.dylib"));
    }

    #[test]
    fn display_with_objc_metadata_shows_counts() {
        let mut plan = minimal_plan("/tmp/objc");
        plan.objc.has_objc = true;
        plan.objc_metadata = Some(macho_objc::ObjcMetadata {
            presence: macho_objc::ObjcPresence {
                has_objc: true,
                sections_found: vec!["__objc_classlist".to_string()],
            },
            class_refs: vec![],
            categories: vec![],
            protocols: vec![],
            category_count: 3,
            protocol_count: 2,
            selectors: vec![],
            selref_addrs: vec![],
            image_info: None,
        });
        let out = plan.display();
        assert!(out.contains("ObjC:"), "output was: {out}");
        assert!(out.contains("3 categories"), "output was: {out}");
        assert!(out.contains("2 protocols"), "output was: {out}");
    }

    #[test]
    fn display_bind_failed_shows_failed_symbols() {
        let mut plan = minimal_plan("/tmp/bindfail");
        plan.symbol_bindings = Some(BindingReport {
            bindings: vec![],
            failed: vec!["_missing_sym".to_string(), "_another_sym".to_string()],
            bound_count: 5,
            weak_unbound_count: 0,
        });
        let out = plan.display();
        assert!(out.contains("[BIND FAILED]"), "output was: {out}");
        assert!(out.contains("_missing_sym"), "output was: {out}");
        assert!(out.contains("_another_sym"), "output was: {out}");
        assert!(out.contains("2 failed"), "output was: {out}");
    }

    #[test]
    fn display_security_with_cs_info() {
        let mut plan = minimal_plan("/tmp/cs");
        plan.security = Some(SecurityReport {
            decisions: vec![],
            allowed: true,
            denied_reasons: vec![],
            strip_dyld_env: false,
            hardened_runtime: true,
            cs_info: Some(trust_policy::CodeSignatureInfo {
                present: true,
                adhoc: true,
                runtime: true,
                restrict: false,
                library_validation: false,
                get_task_allow: true,
                identity: "com.example.test".to_string(),
                team_id: String::new(),
                cd_hash: "abc123".to_string(),
                has_cms_signature: false,
                has_entitlements: true,
            }),
        });
        let out = plan.display();
        assert!(
            out.contains("Security: all gates passed"),
            "output was: {out}"
        );
        assert!(out.contains("Hardened runtime"), "output was: {out}");
    }
}
