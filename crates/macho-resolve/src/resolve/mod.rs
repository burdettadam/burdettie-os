//! Dylib dependency resolution and transitive dependency graph construction.

mod path_resolver;

pub use path_resolver::PathResolver;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use macho_parse::{parse, ParsedMachO};
use serde::{Deserialize, Serialize};

/// A node in the dependency graph — one loaded image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageNode {
    pub index: usize,
    pub path: PathBuf,
    pub install_name: Option<String>,
    pub depth: usize,
}

/// An edge in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: usize,
    pub to: usize,
    pub raw_name: String,
}

/// Dependency graph for a Mach-O binary and its transitive dylibs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<ImageNode>,
    pub edges: Vec<DependencyEdge>,
}

/// Complete resolution plan: dependency graph + path resolution results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionPlan {
    pub graph: DependencyGraph,
    pub rpaths: Vec<String>,
    pub unresolved: Vec<String>,
    pub transitive: bool,
}

/// DYLD environment variable search paths.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchEnv {
    pub library_path: Vec<PathBuf>,
    pub framework_path: Vec<PathBuf>,
    pub fallback_library_path: Vec<PathBuf>,
    pub fallback_framework_path: Vec<PathBuf>,
}

impl SearchEnv {
    pub fn from_env() -> Self {
        fn read_path_list(var: &str) -> Vec<PathBuf> {
            std::env::var(var)
                .unwrap_or_default()
                .split(':')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .collect()
        }

        Self {
            library_path: read_path_list("DYLD_LIBRARY_PATH"),
            framework_path: read_path_list("DYLD_FRAMEWORK_PATH"),
            fallback_library_path: read_path_list("DYLD_FALLBACK_LIBRARY_PATH"),
            fallback_framework_path: read_path_list("DYLD_FALLBACK_FRAMEWORK_PATH"),
        }
    }

    pub fn merge_dyld_env_entries(&mut self, entries: &[String]) {
        for entry in entries {
            if let Some((key, val)) = entry.split_once('=') {
                let paths: Vec<PathBuf> = val
                    .split(':')
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
                    .collect();
                match key {
                    "DYLD_LIBRARY_PATH" => self.library_path.extend(paths),
                    "DYLD_FRAMEWORK_PATH" => self.framework_path.extend(paths),
                    "DYLD_FALLBACK_LIBRARY_PATH" => self.fallback_library_path.extend(paths),
                    "DYLD_FALLBACK_FRAMEWORK_PATH" => self.fallback_framework_path.extend(paths),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── SearchEnv tests ─────────────────────────────────────────────

    #[test]
    fn search_env_default() {
        let env = SearchEnv::default();
        assert!(env.library_path.is_empty());
        assert!(env.framework_path.is_empty());
        assert!(env.fallback_library_path.is_empty());
        assert!(env.fallback_framework_path.is_empty());
    }

    #[test]
    fn merge_empty_entries() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&[]);
        assert!(env.library_path.is_empty());
        assert!(env.framework_path.is_empty());
    }

    #[test]
    fn merge_single_path() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&["DYLD_LIBRARY_PATH=/usr/lib".to_string()]);
        assert_eq!(env.library_path, vec![PathBuf::from("/usr/lib")]);
    }

    #[test]
    fn merge_colon_separated() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&["DYLD_LIBRARY_PATH=/a:/b".to_string()]);
        assert_eq!(
            env.library_path,
            vec![PathBuf::from("/a"), PathBuf::from("/b")]
        );
    }

    #[test]
    fn merge_multiple_entries() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&[
            "DYLD_LIBRARY_PATH=/lib1".to_string(),
            "DYLD_FRAMEWORK_PATH=/fw1:/fw2".to_string(),
            "DYLD_FALLBACK_LIBRARY_PATH=/fallback".to_string(),
        ]);
        assert_eq!(env.library_path, vec![PathBuf::from("/lib1")]);
        assert_eq!(
            env.framework_path,
            vec![PathBuf::from("/fw1"), PathBuf::from("/fw2")]
        );
        assert_eq!(env.fallback_library_path, vec![PathBuf::from("/fallback")]);
        assert!(env.fallback_framework_path.is_empty());
    }

    #[test]
    fn merge_ignores_unknown_keys() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&["SOME_OTHER_VAR=/x".to_string()]);
        assert!(env.library_path.is_empty());
        assert!(env.framework_path.is_empty());
    }

    #[test]
    fn merge_skips_empty_segments() {
        let mut env = SearchEnv::default();
        env.merge_dyld_env_entries(&["DYLD_LIBRARY_PATH=/a::/b:".to_string()]);
        assert_eq!(
            env.library_path,
            vec![PathBuf::from("/a"), PathBuf::from("/b")]
        );
    }

    // ── DependencyGraph / init_order tests ──────────────────────────

    fn make_plan(nodes: Vec<ImageNode>, edges: Vec<DependencyEdge>) -> ResolutionPlan {
        ResolutionPlan {
            graph: DependencyGraph { nodes, edges },
            rpaths: vec![],
            unresolved: vec![],
            transitive: false,
        }
    }

    #[test]
    fn init_order_empty() {
        let plan = make_plan(vec![], vec![]);
        assert!(plan.init_order().is_empty());
    }

    #[test]
    fn init_order_single_node() {
        let plan = make_plan(
            vec![ImageNode {
                index: 0,
                path: PathBuf::from("/bin/a"),
                install_name: None,
                depth: 0,
            }],
            vec![],
        );
        assert_eq!(plan.init_order(), vec![0]);
    }

    #[test]
    fn init_order_linear_chain() {
        // 0 → 1 → 2   ⟹  leaves first: [2, 1, 0]
        let plan = make_plan(
            vec![
                ImageNode {
                    index: 0,
                    path: PathBuf::from("/a"),
                    install_name: None,
                    depth: 0,
                },
                ImageNode {
                    index: 1,
                    path: PathBuf::from("/b"),
                    install_name: None,
                    depth: 1,
                },
                ImageNode {
                    index: 2,
                    path: PathBuf::from("/c"),
                    install_name: None,
                    depth: 2,
                },
            ],
            vec![
                DependencyEdge {
                    from: 0,
                    to: 1,
                    raw_name: "b".into(),
                },
                DependencyEdge {
                    from: 1,
                    to: 2,
                    raw_name: "c".into(),
                },
            ],
        );
        assert_eq!(plan.init_order(), vec![2, 1, 0]);
    }
}

const STANDARD_DYLIB_PATHS: &[&str] = &["/usr/lib", "/usr/local/lib"];
const STANDARD_FRAMEWORK_PATHS: &[&str] = &["/Library/Frameworks", "/System/Library/Frameworks"];

impl ResolutionPlan {
    #[allow(clippy::needless_pass_by_value)] // public API — taking PathBuf for caller convenience
    pub fn from_parsed(parsed: &ParsedMachO, executable_path: PathBuf) -> Self {
        Self::resolve_inner(parsed, &executable_path, false)
    }

    #[allow(clippy::needless_pass_by_value)] // public API — taking PathBuf for caller convenience
    pub fn from_parsed_transitive(parsed: &ParsedMachO, executable_path: PathBuf) -> Self {
        Self::resolve_inner(parsed, &executable_path, true)
    }

    fn resolve_inner(parsed: &ParsedMachO, executable_path: &Path, transitive: bool) -> Self {
        let mut all_rpaths = parsed.rpaths.clone();

        let mut search_env = SearchEnv::default();
        let dyld_env_entries: Vec<String> = parsed
            .commands
            .iter()
            .filter_map(|cmd| {
                if let macho_types::load_command::LoadCommandPayload::DyldEnvironment { name } =
                    &cmd.payload
                {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();
        search_env.merge_dyld_env_entries(&dyld_env_entries);

        let mut nodes = vec![ImageNode {
            index: 0,
            path: executable_path.to_path_buf(),
            install_name: None,
            depth: 0,
        }];
        let mut edges = Vec::new();
        let mut unresolved = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();
        seen.insert(executable_path.to_path_buf());

        struct WorkItem {
            parent_idx: usize,
            raw_name: String,
            depth: usize,
            loader_path: PathBuf,
            rpaths_for_resolver: Vec<String>,
        }

        let mut queue: Vec<WorkItem> = parsed
            .dylibs
            .iter()
            .map(|d| WorkItem {
                parent_idx: 0,
                raw_name: d.name.clone(),
                depth: 1,
                loader_path: executable_path.to_path_buf(),
                rpaths_for_resolver: parsed.rpaths.clone(),
            })
            .collect();

        while let Some(item) = queue.pop() {
            let resolver = PathResolver::new_full(
                executable_path.to_path_buf(),
                item.loader_path.clone(),
                &item.rpaths_for_resolver,
                search_env.clone(),
            );

            if let Some(resolved) = resolver.resolve(&item.raw_name) {
                let canonical = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());

                if seen.contains(&canonical) {
                    if let Some(existing) = nodes.iter().find(|n| {
                        n.path.canonicalize().unwrap_or_else(|_| n.path.clone()) == canonical
                    }) {
                        edges.push(DependencyEdge {
                            from: item.parent_idx,
                            to: existing.index,
                            raw_name: item.raw_name,
                        });
                    }
                    continue;
                }

                let idx = nodes.len();
                seen.insert(canonical);

                let child_parsed = match std::fs::read(&resolved) {
                    Ok(data) => match parse(&data) {
                        Ok(p) => Some(p),
                        Err(e) => {
                            log::warn!("resolve: failed to parse {}: {e}", resolved.display());
                            None
                        }
                    },
                    Err(e) => {
                        log::warn!("resolve: failed to read {}: {e}", resolved.display());
                        None
                    }
                };

                let install_name = child_parsed.as_ref().and_then(|p| {
                    p.commands.iter().find_map(|cmd| {
                        if let macho_types::load_command::LoadCommandPayload::IdDylib(ref d) =
                            cmd.payload
                        {
                            Some(d.name.clone())
                        } else {
                            None
                        }
                    })
                });

                nodes.push(ImageNode {
                    index: idx,
                    path: resolved.clone(),
                    install_name,
                    depth: item.depth,
                });
                edges.push(DependencyEdge {
                    from: item.parent_idx,
                    to: idx,
                    raw_name: item.raw_name,
                });

                if transitive {
                    if let Some(ref child) = child_parsed {
                        for rp in &child.rpaths {
                            if !all_rpaths.contains(rp) {
                                all_rpaths.push(rp.clone());
                            }
                        }

                        let mut merged_rpaths = item.rpaths_for_resolver.clone();
                        for rp in &child.rpaths {
                            if !merged_rpaths.contains(rp) {
                                merged_rpaths.push(rp.clone());
                            }
                        }

                        for dlib in &child.dylibs {
                            queue.push(WorkItem {
                                parent_idx: idx,
                                raw_name: dlib.name.clone(),
                                depth: item.depth + 1,
                                loader_path: resolved.clone(),
                                rpaths_for_resolver: merged_rpaths.clone(),
                            });
                        }
                    }
                }
            } else if !unresolved.contains(&item.raw_name) {
                unresolved.push(item.raw_name);
            }
        }

        Self {
            graph: DependencyGraph { nodes, edges },
            rpaths: all_rpaths,
            unresolved,
            transitive,
        }
    }

    /// Compute bottom-up initialization order (matching dyld semantics).
    pub fn init_order(&self) -> Vec<usize> {
        let n = self.graph.nodes.len();
        if n == 0 {
            return vec![];
        }

        let mut children: Vec<Vec<usize>> = vec![vec![]; n];
        for edge in &self.graph.edges {
            children[edge.from].push(edge.to);
        }

        let mut visited = vec![false; n];
        let mut order = Vec::with_capacity(n);

        fn dfs(node: usize, children: &[Vec<usize>], visited: &mut [bool], order: &mut Vec<usize>) {
            if visited[node] {
                return;
            }
            visited[node] = true;
            for &child in &children[node] {
                dfs(child, children, visited, order);
            }
            order.push(node);
        }

        dfs(0, &children, &mut visited, &mut order);
        order
    }
}
