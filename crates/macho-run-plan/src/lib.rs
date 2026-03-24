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
