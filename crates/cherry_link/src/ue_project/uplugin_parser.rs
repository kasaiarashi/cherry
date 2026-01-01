// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use super::project_model::UPluginFile;
use anyhow::{Context, Result};
use std::path::Path;

/// Parse a .uplugin file from disk
pub fn parse_uplugin(path: &Path) -> Result<UPluginFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read .uplugin file: {}", path.display()))?;

    parse_uplugin_from_str(&content)
        .with_context(|| format!("Failed to parse .uplugin file: {}", path.display()))
}

/// Parse a .uplugin file from a string (for testing)
pub fn parse_uplugin_from_str(content: &str) -> Result<UPluginFile> {
    serde_json::from_str(content).context("Failed to parse .uplugin JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_uplugin() {
        let json = r#"{
            "FileVersion": 3,
            "Version": 1,
            "VersionName": "1.0",
            "FriendlyName": "OWF AI System",
            "Description": "Advanced AI system for NPCs",
            "Category": "Open World Framework",
            "CreatedBy": "Krishna Teja Mekala",
            "CanContainContent": true,
            "Modules": [
                {
                    "Name": "OWF_AISystem",
                    "Type": "Runtime",
                    "LoadingPhase": "Default"
                }
            ],
            "Plugins": [
                {
                    "Name": "OWF_Globals",
                    "Enabled": true
                }
            ]
        }"#;

        let result = parse_uplugin_from_str(json).unwrap();
        assert_eq!(result.file_version, 3);
        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.modules[0].name, "OWF_AISystem");
        assert_eq!(result.plugins.len(), 1);
        assert_eq!(result.plugins[0].name, "OWF_Globals");
    }

    #[test]
    fn test_parse_editor_uplugin() {
        let json = r#"{
            "FileVersion": 3,
            "Version": 1,
            "FriendlyName": "OWF Globals Editor",
            "CanContainContent": false,
            "Modules": [
                {
                    "Name": "OWF_GlobalsEditor",
                    "Type": "Editor",
                    "LoadingPhase": "Default"
                }
            ],
            "Plugins": [
                {
                    "Name": "OWF_Globals",
                    "Enabled": true
                }
            ]
        }"#;

        let result = parse_uplugin_from_str(json).unwrap();
        assert_eq!(result.modules[0].module_type, "Editor");
    }
}
