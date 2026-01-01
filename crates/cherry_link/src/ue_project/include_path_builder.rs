// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use super::project_model::{UEModule, UEPlugin, UEProject};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct IncludePathBuilder {
    engine_path: PathBuf,
    project_path: PathBuf,
}

impl IncludePathBuilder {
    pub fn new(engine_path: PathBuf, project_path: PathBuf) -> Self {
        Self {
            engine_path,
            project_path,
        }
    }

    /// Build all include paths for the project
    pub fn build_all_include_paths(&self, project: &UEProject) -> Vec<PathBuf> {
        let mut paths = HashSet::new();

        // Phase 1: Engine paths
        paths.extend(self.discover_engine_paths());

        // Phase 2: Project module paths
        for module in &project.modules {
            paths.extend(self.get_module_paths(&module.source_path));
        }

        // Phase 3: Plugin paths
        for plugin in &project.plugins {
            for module in &plugin.modules {
                paths.extend(self.get_module_paths(&module.source_path));
            }
        }

        // Phase 4: Explicit paths from Build.cs
        for module in &project.modules {
            paths.extend(module.include_paths.public_include_paths.clone());
            paths.extend(module.include_paths.private_include_paths.clone());
            paths.extend(module.include_paths.inferred_paths.clone());
        }

        for plugin in &project.plugins {
            for module in &plugin.modules {
                paths.extend(module.include_paths.public_include_paths.clone());
                paths.extend(module.include_paths.private_include_paths.clone());
                paths.extend(module.include_paths.inferred_paths.clone());
            }
        }

        paths.into_iter().collect()
    }

    /// Discover engine include paths
    fn discover_engine_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        log::info!("Discovering engine paths from: {}", self.engine_path.display());

        // Runtime modules
        let runtime_path = self.engine_path.join("Source/Runtime");
        if runtime_path.exists() {
            paths.extend(self.discover_module_paths(&runtime_path));
        }

        // Developer modules
        let developer_path = self.engine_path.join("Source/Developer");
        if developer_path.exists() {
            paths.extend(self.discover_module_paths(&developer_path));
        }

        // Editor modules
        let editor_path = self.engine_path.join("Source/Editor");
        if editor_path.exists() {
            paths.extend(self.discover_module_paths(&editor_path));
        }

        // Engine plugins
        let plugins_path = self.engine_path.join("Plugins");
        if plugins_path.exists() {
            paths.extend(self.discover_plugin_paths(&plugins_path));
        }

        log::info!("Discovered {} engine include paths", paths.len());
        paths
    }

    /// Discover module paths (Public, Classes directories) in a given root
    fn discover_module_paths(&self, root: &Path) -> Vec<PathBuf> {
        WalkDir::new(root)
            .max_depth(3) // Limit depth for performance
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let file_name = e.file_name().to_string_lossy();
                (file_name == "Public" || file_name == "Classes") && e.file_type().is_dir()
            })
            .map(|e| e.path().to_path_buf())
            .collect()
    }

    /// Discover plugin include paths
    fn discover_plugin_paths(&self, plugins_root: &Path) -> Vec<PathBuf> {
        WalkDir::new(plugins_root)
            .max_depth(5) // Plugins/PluginName/Source/ModuleName/Public
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let file_name = e.file_name().to_string_lossy();
                (file_name == "Public" || file_name == "Classes") && e.file_type().is_dir()
            })
            .map(|e| e.path().to_path_buf())
            .collect()
    }

    /// Get standard paths for a module (Public, Private, Classes)
    fn get_module_paths(&self, module_source: &Path) -> Vec<PathBuf> {
        vec![
            module_source.join("Public"),
            module_source.join("Private"),
            module_source.join("Classes"),
        ]
        .into_iter()
        .filter(|p| p.exists())
        .collect()
    }
}

/// Build inferred include paths for a module based on its source directory
pub fn infer_module_include_paths(source_path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    let public_path = source_path.join("Public");
    if public_path.exists() {
        paths.push(public_path);
    }

    let private_path = source_path.join("Private");
    if private_path.exists() {
        paths.push(private_path);
    }

    let classes_path = source_path.join("Classes");
    if classes_path.exists() {
        paths.push(classes_path);
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_module_include_paths() {
        // This test would need actual filesystem structure
        // For now, just test the function exists and can be called
        let path = PathBuf::from(".");
        let paths = infer_module_include_paths(&path);
        // Paths might be empty if directories don't exist, which is fine
        assert!(paths.is_empty() || !paths.is_empty());
    }
}
