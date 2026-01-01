// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UE5 plugin parsing

use crate::ue::module::UEModule;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a UE5 plugin
#[derive(Debug, Clone)]
pub struct UEPlugin {
    pub name: String,
    pub path: PathBuf,
    pub version: u32,
    pub version_name: String,
    pub friendly_name: String,
    pub description: String,
    pub category: String,
    pub modules: Vec<UEModule>,
}

/// .uplugin file structure
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct UPluginFile {
    file_version: u32,
    version: u32,
    version_name: String,
    friendly_name: String,
    description: String,
    category: String,
    #[serde(default)]
    modules: Vec<PluginModuleDescriptor>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PluginModuleDescriptor {
    name: String,
    #[serde(rename = "Type")]
    module_type: String,
    loading_phase: String,
}

impl UEPlugin {
    /// Load a plugin from a .uplugin file
    pub fn load(plugin_path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(plugin_path)
            .with_context(|| format!("Failed to read .uplugin file: {:?}", plugin_path))?;

        let uplugin: UPluginFile = serde_json::from_str(&content)
            .with_context(|| "Failed to parse .uplugin JSON")?;

        let plugin_name = plugin_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let plugin_dir = plugin_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        // Parse modules from the plugin
        let modules = uplugin
            .modules
            .iter()
            .filter_map(|m| {
                let module_path = plugin_dir.join("Source").join(&m.name);
                UEModule::from_descriptor(
                    &m.name,
                    &m.module_type,
                    &module_path,
                    &plugin_dir,
                )
            })
            .collect();

        Ok(Self {
            name: plugin_name,
            path: plugin_dir,
            version: uplugin.version,
            version_name: uplugin.version_name,
            friendly_name: uplugin.friendly_name,
            description: uplugin.description,
            category: uplugin.category,
            modules,
        })
    }

    /// Get all include paths from plugin modules
    pub fn include_paths(&self) -> Vec<PathBuf> {
        self.modules
            .iter()
            .flat_map(|m| m.include_paths())
            .collect()
    }

    /// Get all defines from plugin modules
    pub fn defines(&self) -> Vec<String> {
        self.modules.iter().flat_map(|m| m.defines()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_uplugin() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("TestPlugin.uplugin");

        let uplugin_content = r#"{
    "FileVersion": 3,
    "Version": 1,
    "VersionName": "1.0",
    "FriendlyName": "Test Plugin",
    "Description": "A test plugin",
    "Category": "Testing",
    "Modules": [
        {
            "Name": "TestPluginModule",
            "Type": "Runtime",
            "LoadingPhase": "Default"
        }
    ]
}"#;

        fs::write(&plugin_path, uplugin_content).unwrap();

        let plugin = UEPlugin::load(&plugin_path).unwrap();

        assert_eq!(plugin.name, "TestPlugin");
        assert_eq!(plugin.version, 1);
        assert_eq!(plugin.friendly_name, "Test Plugin");
    }

    #[test]
    fn test_invalid_uplugin() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("Invalid.uplugin");

        fs::write(&plugin_path, "not json").unwrap();

        let result = UEPlugin::load(&plugin_path);
        assert!(result.is_err());
    }
}
