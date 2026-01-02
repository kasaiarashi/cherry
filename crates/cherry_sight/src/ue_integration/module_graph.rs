// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Module dependency graph analysis

use crate::ue::{UEProject, UEModule};
use std::collections::{HashMap, HashSet};

/// Kind of module dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DependencyKind {
    /// Public dependency (PublicDependencyModuleNames)
    Public,
    /// Private dependency (PrivateDependencyModuleNames)
    Private,
    /// Dynamic load dependency
    Dynamic,
    /// Circular dependency (error)
    Circular,
}

/// Module dependency information
#[derive(Debug, Clone)]
pub struct ModuleDependency {
    pub from_module: String,
    pub to_module: String,
    pub kind: DependencyKind,
    pub depth: usize,
}

/// Module dependency graph analyzer
pub struct ModuleDependencyGraph {
    /// Module name -> dependencies
    dependencies: HashMap<String, Vec<ModuleDependency>>,
    /// Module name -> reverse dependencies
    reverse_deps: HashMap<String, Vec<String>>,
    /// Detected circular dependencies
    circular_deps: Vec<Vec<String>>,
}

impl ModuleDependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            reverse_deps: HashMap::new(),
            circular_deps: Vec::new(),
        }
    }

    /// Build dependency graph from UE project
    pub fn build_from_project(&mut self, project: &UEProject) {
        // Clear existing data
        self.dependencies.clear();
        self.reverse_deps.clear();
        self.circular_deps.clear();

        // Build forward dependencies
        for module in &project.modules {
            self.add_module_dependencies(module);
        }

        // Build reverse dependencies
        self.build_reverse_dependencies();

        // Detect circular dependencies
        self.detect_circular_dependencies();
    }

    /// Add dependencies for a single module
    fn add_module_dependencies(&mut self, module: &UEModule) {
        let module_name = module.name.clone();
        let mut deps = Vec::new();

        // Add all dependencies (UEModule has a single dependencies list)
        for dep_name in &module.dependencies {
            deps.push(ModuleDependency {
                from_module: module_name.clone(),
                to_module: dep_name.clone(),
                kind: DependencyKind::Public, // Default to public
                depth: 0,
            });
        }

        self.dependencies.insert(module_name, deps);
    }

    /// Build reverse dependency map
    fn build_reverse_dependencies(&mut self) {
        for (from_module, deps) in &self.dependencies {
            for dep in deps {
                self.reverse_deps
                    .entry(dep.to_module.clone())
                    .or_default()
                    .push(from_module.clone());
            }
        }
    }

    /// Detect circular dependencies using DFS
    fn detect_circular_dependencies(&mut self) {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        // Collect keys first to avoid borrowing issues
        let modules: Vec<String> = self.dependencies.keys().cloned().collect();
        for module in modules {
            if !visited.contains(&module) {
                self.dfs_detect_cycle(
                    &module,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                );
            }
        }
    }

    /// DFS cycle detection
    fn dfs_detect_cycle(
        &mut self,
        module: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) {
        visited.insert(module.to_string());
        rec_stack.insert(module.to_string());
        path.push(module.to_string());

        // Clone deps to avoid borrowing issues
        let deps = self.dependencies.get(module).cloned();
        if let Some(deps) = deps {
            for dep in deps {
                let dep_module = &dep.to_module;

                if !visited.contains(dep_module) {
                    self.dfs_detect_cycle(dep_module, visited, rec_stack, path);
                } else if rec_stack.contains(dep_module) {
                    // Found cycle
                    if let Some(cycle_start) = path.iter().position(|m| m == dep_module) {
                        let cycle = path[cycle_start..].to_vec();
                        self.circular_deps.push(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(module);
    }

    /// Get all dependencies of a module
    pub fn get_dependencies(&self, module: &str) -> Option<&Vec<ModuleDependency>> {
        self.dependencies.get(module)
    }

    /// Get all modules that depend on this module
    pub fn get_reverse_dependencies(&self, module: &str) -> Option<&Vec<String>> {
        self.reverse_deps.get(module)
    }

    /// Get detected circular dependencies
    pub fn get_circular_dependencies(&self) -> &Vec<Vec<String>> {
        &self.circular_deps
    }

    /// Get transitive dependencies (all dependencies recursively)
    pub fn get_transitive_dependencies(&self, module: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        self.collect_transitive_deps(module, &mut visited, &mut result);
        result
    }

    fn collect_transitive_deps(
        &self,
        module: &str,
        visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) {
        if visited.contains(module) {
            return;
        }
        visited.insert(module.to_string());

        if let Some(deps) = self.dependencies.get(module) {
            for dep in deps {
                let dep_module = &dep.to_module;
                if !visited.contains(dep_module) {
                    result.push(dep_module.clone());
                    self.collect_transitive_deps(dep_module, visited, result);
                }
            }
        }
    }

    /// Get module dependency depth (longest path from root)
    pub fn get_module_depth(&self, module: &str) -> usize {
        let mut max_depth = 0;
        let mut visited = HashSet::new();
        self.calculate_depth(module, 0, &mut max_depth, &mut visited);
        max_depth
    }

    fn calculate_depth(
        &self,
        module: &str,
        current_depth: usize,
        max_depth: &mut usize,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(module) {
            return;
        }
        visited.insert(module.to_string());

        *max_depth = (*max_depth).max(current_depth);

        if let Some(deps) = self.dependencies.get(module) {
            for dep in deps {
                self.calculate_depth(&dep.to_module, current_depth + 1, max_depth, visited);
            }
        }
    }
}

impl Default for ModuleDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ue::ModuleType;

    fn create_test_module(name: &str, deps: Vec<&str>) -> UEModule {
        UEModule {
            name: name.to_string(),
            path: std::path::PathBuf::from(format!("Source/{}", name)),
            module_type: ModuleType::Runtime,
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
            public_include_paths: Vec::new(),
            private_include_paths: Vec::new(),
            public_defines: Vec::new(),
        }
    }

    #[test]
    fn test_build_dependency_graph() {
        let mut graph = ModuleDependencyGraph::new();

        let modules = vec![
            create_test_module("GameModule", vec!["Core", "Engine", "Slate"]),
            create_test_module("Core", vec![]),
            create_test_module("Engine", vec!["Core"]),
        ];

        let project = UEProject {
            name: "TestProject".to_string(),
            path: std::path::PathBuf::from("Test"),
            engine_path: Some(std::path::PathBuf::from("Engine")),
            modules,
            plugins: Vec::new(),
            engine_association: "5.3".to_string(),
            category: "Game".to_string(),
            description: "Test".to_string(),
        };

        graph.build_from_project(&project);

        // Check dependencies
        let game_deps = graph.get_dependencies("GameModule").unwrap();
        assert_eq!(game_deps.len(), 3);

        // Check reverse dependencies
        let core_reverse = graph.get_reverse_dependencies("Core").unwrap();
        assert!(core_reverse.contains(&"GameModule".to_string()));
        assert!(core_reverse.contains(&"Engine".to_string()));
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut graph = ModuleDependencyGraph::new();

        let modules = vec![
            create_test_module("A", vec!["B"]),
            create_test_module("B", vec!["C"]),
            create_test_module("C", vec![]),
        ];

        let project = UEProject {
            name: "TestProject".to_string(),
            path: std::path::PathBuf::from("Test"),
            engine_path: Some(std::path::PathBuf::from("Engine")),
            modules,
            plugins: Vec::new(),
            engine_association: "5.3".to_string(),
            category: "Game".to_string(),
            description: "Test".to_string(),
        };

        graph.build_from_project(&project);

        let transitive = graph.get_transitive_dependencies("A");
        assert_eq!(transitive.len(), 2);
        assert!(transitive.contains(&"B".to_string()));
        assert!(transitive.contains(&"C".to_string()));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = ModuleDependencyGraph::new();

        let modules = vec![
            create_test_module("A", vec!["B"]),
            create_test_module("B", vec!["C"]),
            create_test_module("C", vec!["A"]), // Circular!
        ];

        let project = UEProject {
            name: "TestProject".to_string(),
            path: std::path::PathBuf::from("Test"),
            engine_path: Some(std::path::PathBuf::from("Engine")),
            modules,
            plugins: Vec::new(),
            engine_association: "5.3".to_string(),
            category: "Game".to_string(),
            description: "Test".to_string(),
        };

        graph.build_from_project(&project);

        let circular = graph.get_circular_dependencies();
        assert!(!circular.is_empty());
    }
}
