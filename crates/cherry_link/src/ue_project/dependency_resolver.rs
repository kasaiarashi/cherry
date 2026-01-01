// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use super::project_model::{UEModule, UEPlugin, UEProject};
use anyhow::{Context, Result};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Topo;
use petgraph::Direction;
use std::collections::HashMap;

/// Dependency graph for UE modules
pub struct DependencyGraph {
    graph: DiGraph<String, ()>,
    node_map: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    /// Create a new dependency graph from a UE project
    pub fn from_project(project: &UEProject) -> Self {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Add all modules as nodes
        for module in &project.modules {
            let idx = graph.add_node(module.name.clone());
            node_map.insert(module.name.clone(), idx);
        }

        for plugin in &project.plugins {
            for module in &plugin.modules {
                let idx = graph.add_node(module.name.clone());
                node_map.insert(module.name.clone(), idx);
            }
        }

        // Add edges for dependencies
        for module in &project.modules {
            Self::add_module_dependencies(&mut graph, &node_map, module);
        }

        for plugin in &project.plugins {
            for module in &plugin.modules {
                Self::add_module_dependencies(&mut graph, &node_map, module);
            }
        }

        Self { graph, node_map }
    }

    /// Add dependencies for a module to the graph
    fn add_module_dependencies(
        graph: &mut DiGraph<String, ()>,
        node_map: &HashMap<String, NodeIndex>,
        module: &UEModule,
    ) {
        if let Some(&from_idx) = node_map.get(&module.name) {
            // Add public dependencies
            for dep in &module.dependencies.public_dependencies {
                if let Some(&to_idx) = node_map.get(dep) {
                    graph.add_edge(from_idx, to_idx, ());
                }
            }

            // Add private dependencies
            for dep in &module.dependencies.private_dependencies {
                if let Some(&to_idx) = node_map.get(dep) {
                    graph.add_edge(from_idx, to_idx, ());
                }
            }
        }
    }

    /// Get topological sort of modules (build order)
    pub fn topological_sort(&self) -> Vec<String> {
        let mut topo = Topo::new(&self.graph);
        let mut result = Vec::new();

        while let Some(node_idx) = topo.next(&self.graph) {
            if let Some(module_name) = self.graph.node_weight(node_idx) {
                result.push(module_name.clone());
            }
        }

        result
    }

    /// Detect circular dependencies
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        use petgraph::algo::kosaraju_scc;

        let sccs = kosaraju_scc(&self.graph);
        let mut cycles = Vec::new();

        for scc in sccs {
            if scc.len() > 1 {
                let cycle: Vec<String> = scc
                    .iter()
                    .filter_map(|&idx| self.graph.node_weight(idx).cloned())
                    .collect();
                cycles.push(cycle);
            }
        }

        cycles
    }

    /// Get all transitive dependencies for a module
    pub fn get_transitive_dependencies(&self, module_name: &str) -> Vec<String> {
        if let Some(&start_idx) = self.node_map.get(module_name) {
            let mut visited = Vec::new();
            self.dfs_dependencies(start_idx, &mut visited);
            visited
                .into_iter()
                .filter(|name| name != module_name)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Depth-first search to collect all dependencies
    fn dfs_dependencies(&self, node_idx: NodeIndex, visited: &mut Vec<String>) {
        if let Some(module_name) = self.graph.node_weight(node_idx) {
            if !visited.contains(module_name) {
                visited.push(module_name.clone());

                // Visit all neighbors (dependencies)
                for neighbor in self.graph.neighbors_directed(node_idx, Direction::Outgoing) {
                    self.dfs_dependencies(neighbor, visited);
                }
            }
        }
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

/// Resolve all module dependencies and return them in build order
pub fn resolve_dependencies(project: &UEProject) -> Result<Vec<String>> {
    let graph = DependencyGraph::from_project(project);

    // Check for cycles
    let cycles = graph.detect_cycles();
    if !cycles.is_empty() {
        log::warn!("Circular dependencies detected:");
        for cycle in &cycles {
            log::warn!("  Cycle: {}", cycle.join(" -> "));
        }
    }

    // Return topological sort
    Ok(graph.topological_sort())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ue_project::project_model::*;
    use std::path::PathBuf;

    fn create_test_module(name: &str, public_deps: Vec<&str>, private_deps: Vec<&str>) -> UEModule {
        UEModule {
            name: name.to_string(),
            module_type: ModuleType::Runtime,
            loading_phase: "Default".to_string(),
            source_path: PathBuf::from("."),
            build_cs_path: PathBuf::from("."),
            dependencies: ModuleDependencies {
                public_dependencies: public_deps.iter().map(|s| s.to_string()).collect(),
                private_dependencies: private_deps.iter().map(|s| s.to_string()).collect(),
                public_delay_load_dlls: Vec::new(),
                private_delay_load_dlls: Vec::new(),
            },
            include_paths: IncludePaths::default(),
            defines: Vec::new(),
        }
    }

    #[test]
    fn test_simple_dependency_graph() {
        let modules = vec![
            create_test_module("Core", vec![], vec![]),
            create_test_module("Engine", vec!["Core"], vec![]),
            create_test_module("MyGame", vec!["Engine"], vec![]),
        ];

        let project = UEProject {
            uproject_path: PathBuf::from("."),
            project_name: "Test".to_string(),
            engine_association: "5.6".to_string(),
            engine_path: None,
            modules,
            plugins: Vec::new(),
            target_platforms: Vec::new(),
        };

        let graph = DependencyGraph::from_project(&project);
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);

        let topo = graph.topological_sort();
        // Core should come before Engine, Engine before MyGame
        let core_pos = topo.iter().position(|n| n == "Core").unwrap();
        let engine_pos = topo.iter().position(|n| n == "Engine").unwrap();
        let mygame_pos = topo.iter().position(|n| n == "MyGame").unwrap();
        assert!(core_pos < engine_pos);
        assert!(engine_pos < mygame_pos);
    }

    #[test]
    fn test_transitive_dependencies() {
        let modules = vec![
            create_test_module("Core", vec![], vec![]),
            create_test_module("Engine", vec!["Core"], vec![]),
            create_test_module("Slate", vec!["Core"], vec![]),
            create_test_module("MyGame", vec!["Engine"], vec!["Slate"]),
        ];

        let project = UEProject {
            uproject_path: PathBuf::from("."),
            project_name: "Test".to_string(),
            engine_association: "5.6".to_string(),
            engine_path: None,
            modules,
            plugins: Vec::new(),
            target_platforms: Vec::new(),
        };

        let graph = DependencyGraph::from_project(&project);
        let deps = graph.get_transitive_dependencies("MyGame");

        // MyGame depends on Engine, Slate, and transitively on Core
        assert!(deps.contains(&"Engine".to_string()));
        assert!(deps.contains(&"Slate".to_string()));
        assert!(deps.contains(&"Core".to_string()));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let modules = vec![
            create_test_module("ModuleA", vec!["ModuleB"], vec![]),
            create_test_module("ModuleB", vec!["ModuleA"], vec![]),
        ];

        let project = UEProject {
            uproject_path: PathBuf::from("."),
            project_name: "Test".to_string(),
            engine_association: "5.6".to_string(),
            engine_path: None,
            modules,
            plugins: Vec::new(),
            target_platforms: Vec::new(),
        };

        let graph = DependencyGraph::from_project(&project);
        let cycles = graph.detect_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
    }
}
