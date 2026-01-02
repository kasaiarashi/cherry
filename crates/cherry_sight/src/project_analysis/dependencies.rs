// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Project-wide dependency analysis

use crate::util::FileId;
use std::collections::{HashMap, HashSet, VecDeque};

/// Dependency graph for files
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// File dependencies (file -> files it depends on)
    dependencies: HashMap<FileId, Vec<FileId>>,
    /// Reverse dependencies (file -> files that depend on it)
    reverse_dependencies: HashMap<FileId, Vec<FileId>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            reverse_dependencies: HashMap::new(),
        }
    }

    /// Add a dependency edge
    pub fn add_dependency(&mut self, from: FileId, to: FileId) {
        self.dependencies
            .entry(from)
            .or_default()
            .push(to);

        self.reverse_dependencies
            .entry(to)
            .or_default()
            .push(from);
    }

    /// Get direct dependencies for a file
    pub fn get_dependencies(&self, file_id: FileId) -> Vec<FileId> {
        self.dependencies
            .get(&file_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get files that depend on this file
    pub fn get_dependents(&self, file_id: FileId) -> Vec<FileId> {
        self.reverse_dependencies
            .get(&file_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Find circular dependencies
    pub fn find_cycles(&self) -> Vec<Vec<FileId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        // Collect all files
        let files: Vec<FileId> = self.dependencies.keys().copied().collect();

        for file in files {
            if !visited.contains(&file) {
                self.dfs_find_cycle(
                    file,
                    &mut visited,
                    &mut rec_stack,
                    &mut Vec::new(),
                    &mut cycles,
                );
            }
        }

        cycles
    }

    /// DFS to detect cycles
    fn dfs_find_cycle(
        &self,
        file: FileId,
        visited: &mut HashSet<FileId>,
        rec_stack: &mut HashSet<FileId>,
        path: &mut Vec<FileId>,
        cycles: &mut Vec<Vec<FileId>>,
    ) {
        visited.insert(file);
        rec_stack.insert(file);
        path.push(file);

        if let Some(deps) = self.dependencies.get(&file) {
            for &dep in deps {
                if !visited.contains(&dep) {
                    self.dfs_find_cycle(dep, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(&dep) {
                    // Found cycle
                    if let Some(cycle_start) = path.iter().position(|&f| f == dep) {
                        cycles.push(path[cycle_start..].to_vec());
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(&file);
    }

    /// Get transitive dependencies (all dependencies recursively)
    pub fn get_transitive_dependencies(&self, file_id: FileId) -> HashSet<FileId> {
        let mut result = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(file_id);

        while let Some(current) = queue.pop_front() {
            if let Some(deps) = self.dependencies.get(&current) {
                for &dep in deps {
                    if result.insert(dep) {
                        queue.push_back(dep);
                    }
                }
            }
        }

        result
    }

    /// Calculate coupling metrics
    pub fn calculate_coupling(&self, file_id: FileId) -> CouplingMetrics {
        let afferent = self.get_dependents(file_id).len();
        let efferent = self.get_dependencies(file_id).len();

        CouplingMetrics {
            afferent_coupling: afferent,
            efferent_coupling: efferent,
        }
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Coupling metrics
#[derive(Debug, Clone, Copy)]
pub struct CouplingMetrics {
    /// Afferent coupling (Ca) - number of files that depend on this file
    pub afferent_coupling: usize,
    /// Efferent coupling (Ce) - number of files this file depends on
    pub efferent_coupling: usize,
}

impl CouplingMetrics {
    /// Calculate instability (I = Ce / (Ca + Ce))
    /// Range: 0 (stable) to 1 (unstable)
    pub fn instability(&self) -> f64 {
        let total = self.afferent_coupling + self.efferent_coupling;
        if total == 0 {
            0.0
        } else {
            self.efferent_coupling as f64 / total as f64
        }
    }

    /// Check if file is highly unstable
    pub fn is_highly_unstable(&self) -> bool {
        self.instability() > 0.8
    }
}

/// Dependency analyzer
pub struct DependencyAnalyzer {
    graph: DependencyGraph,
}

impl DependencyAnalyzer {
    pub fn new() -> Self {
        Self {
            graph: DependencyGraph::new(),
        }
    }

    /// Build dependency graph from include analysis
    pub fn build_graph(&mut self, _files: &[FileId]) {
        // In full implementation, would:
        // 1. Parse #include directives
        // 2. Resolve include paths
        // 3. Build dependency graph
    }

    /// Get dependency graph
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Find files with high fan-in (many dependents)
    pub fn find_high_fan_in(&self, threshold: usize) -> Vec<FileId> {
        self.graph
            .dependencies
            .keys()
            .filter(|&&file| self.graph.get_dependents(file).len() > threshold)
            .copied()
            .collect()
    }

    /// Find files with high fan-out (many dependencies)
    pub fn find_high_fan_out(&self, threshold: usize) -> Vec<FileId> {
        self.graph
            .dependencies
            .iter()
            .filter(|(_, deps)| deps.len() > threshold)
            .map(|(&file, _)| file)
            .collect()
    }
}

impl Default for DependencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency violation
#[derive(Debug, Clone)]
pub struct DependencyViolation {
    pub kind: ViolationKind,
    pub files: Vec<FileId>,
    pub message: String,
}

/// Violation kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    CircularDependency,
    HighCoupling,
    LayerViolation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_creation() {
        let graph = DependencyGraph::new();
        assert!(graph.dependencies.is_empty());
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = DependencyGraph::new();
        let file1 = FileId::new(1);
        let file2 = FileId::new(2);

        graph.add_dependency(file1, file2);

        assert_eq!(graph.get_dependencies(file1), vec![file2]);
        assert_eq!(graph.get_dependents(file2), vec![file1]);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();
        let file1 = FileId::new(1);
        let file2 = FileId::new(2);
        let file3 = FileId::new(3);

        // Create cycle: 1 -> 2 -> 3 -> 1
        graph.add_dependency(file1, file2);
        graph.add_dependency(file2, file3);
        graph.add_dependency(file3, file1);

        let cycles = graph.find_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut graph = DependencyGraph::new();
        let file1 = FileId::new(1);
        let file2 = FileId::new(2);
        let file3 = FileId::new(3);

        graph.add_dependency(file1, file2);
        graph.add_dependency(file2, file3);

        let transitive = graph.get_transitive_dependencies(file1);
        assert!(transitive.contains(&file2));
        assert!(transitive.contains(&file3));
    }

    #[test]
    fn test_coupling_metrics() {
        let mut graph = DependencyGraph::new();
        let file1 = FileId::new(1);
        let file2 = FileId::new(2);
        let file3 = FileId::new(3);

        graph.add_dependency(file1, file2);
        graph.add_dependency(file3, file2);

        let metrics = graph.calculate_coupling(file2);
        assert_eq!(metrics.afferent_coupling, 2); // file1 and file3 depend on file2
        assert_eq!(metrics.efferent_coupling, 0); // file2 doesn't depend on others
    }

    #[test]
    fn test_instability_metric() {
        let metrics = CouplingMetrics {
            afferent_coupling: 5,
            efferent_coupling: 2,
        };

        let instability = metrics.instability();
        assert!(instability > 0.2 && instability < 0.3);
    }

    #[test]
    fn test_is_highly_unstable() {
        let stable = CouplingMetrics {
            afferent_coupling: 9,
            efferent_coupling: 1,
        };
        assert!(!stable.is_highly_unstable());

        let unstable = CouplingMetrics {
            afferent_coupling: 1,
            efferent_coupling: 9,
        };
        assert!(unstable.is_highly_unstable());
    }

    #[test]
    fn test_dependency_analyzer() {
        let analyzer = DependencyAnalyzer::new();
        assert!(analyzer.graph().dependencies.is_empty());
    }
}
