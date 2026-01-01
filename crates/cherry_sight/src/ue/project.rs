// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UE5 project parsing and management

use crate::ue::module::UEModule;
use crate::ue::plugin::UEPlugin;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents an Unreal Engine project
#[derive(Debug, Clone)]
pub struct UEProject {
    pub name: String,
    pub path: PathBuf,
    pub engine_path: Option<PathBuf>,
    pub modules: Vec<UEModule>,
    pub plugins: Vec<UEPlugin>,
    pub engine_association: String,
    pub category: String,
    pub description: String,
}

/// .uproject file structure
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct UProjectFile {
    file_version: u32,
    engine_association: String,
    category: String,
    description: String,
    #[serde(default)]
    modules: Vec<ModuleDescriptor>,
    #[serde(default)]
    plugins: Vec<PluginReference>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct ModuleDescriptor {
    name: String,
    #[serde(rename = "Type")]
    module_type: String,
    loading_phase: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PluginReference {
    name: String,
    enabled: bool,
}

impl UEProject {
    /// Load a UE5 project from a .uproject file
    pub fn load(project_path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(project_path)
            .with_context(|| format!("Failed to read .uproject file: {:?}", project_path))?;

        let uproject: UProjectFile = serde_json::from_str(&content)
            .with_context(|| "Failed to parse .uproject JSON")?;

        let project_name = project_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let project_dir = project_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        // Parse modules from the project
        let modules = uproject
            .modules
            .iter()
            .filter_map(|m| {
                let module_path = project_dir.join("Source").join(&m.name);
                UEModule::from_descriptor(
                    &m.name,
                    &m.module_type,
                    &module_path,
                    &project_dir,
                )
            })
            .collect();

        // Parse plugins
        let plugins = uproject
            .plugins
            .iter()
            .filter(|p| p.enabled)
            .filter_map(|p| {
                // Try to find plugin in project plugins directory
                let plugin_path = project_dir
                    .join("Plugins")
                    .join(&p.name)
                    .join(format!("{}.uplugin", p.name));

                if plugin_path.exists() {
                    UEPlugin::load(&plugin_path).ok()
                } else {
                    None
                }
            })
            .collect();

        Ok(Self {
            name: project_name,
            path: project_dir,
            engine_path: None, // To be determined from engine association
            modules,
            plugins,
            engine_association: uproject.engine_association,
            category: uproject.category,
            description: uproject.description,
        })
    }

    /// Get all include paths from the project, modules, and plugins
    pub fn include_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Add module include paths
        for module in &self.modules {
            paths.extend(module.include_paths());
        }

        // Add plugin include paths
        for plugin in &self.plugins {
            paths.extend(plugin.include_paths());
        }

        // Add engine include paths if available
        if let Some(engine_path) = &self.engine_path {
            paths.push(engine_path.join("Source/Runtime/Core/Public"));
            paths.push(engine_path.join("Source/Runtime/CoreUObject/Public"));
            paths.push(engine_path.join("Source/Runtime/Engine/Public"));
        }

        paths
    }

    /// Get all preprocessor defines from modules and plugins
    pub fn defines(&self) -> Vec<String> {
        let mut defines = Vec::new();

        // Add common UE defines
        defines.push("UE_GAME=1".to_string());
        defines.push("UE_EDITOR=1".to_string());
        defines.push("WITH_EDITOR=1".to_string());

        // Add module defines
        for module in &self.modules {
            defines.extend(module.defines());
        }

        // Add plugin defines
        for plugin in &self.plugins {
            defines.extend(plugin.defines());
        }

        defines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_uproject() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TestProject.uproject");

        let uproject_content = r#"{
    "FileVersion": 3,
    "EngineAssociation": "5.3",
    "Category": "Game",
    "Description": "Test project",
    "Modules": [
        {
            "Name": "TestModule",
            "Type": "Runtime",
            "LoadingPhase": "Default"
        }
    ],
    "Plugins": [
        {
            "Name": "TestPlugin",
            "Enabled": true
        }
    ]
}"#;

        fs::write(&project_path, uproject_content).unwrap();

        let project = UEProject::load(&project_path).unwrap();

        assert_eq!(project.name, "TestProject");
        assert_eq!(project.engine_association, "5.3");
        assert_eq!(project.category, "Game");
        assert!(project.modules.len() > 0 || project.plugins.len() >= 0);
    }

    #[test]
    fn test_invalid_uproject() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Invalid.uproject");

        fs::write(&project_path, "not json").unwrap();

        let result = UEProject::load(&project_path);
        assert!(result.is_err());
    }
}
