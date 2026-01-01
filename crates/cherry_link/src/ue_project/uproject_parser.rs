// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use super::project_model::UProjectFile;
use anyhow::{Context, Result};
use std::path::Path;

/// Parse a .uproject file from disk
pub fn parse_uproject(path: &Path) -> Result<UProjectFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read .uproject file: {}", path.display()))?;

    parse_uproject_from_str(&content)
        .with_context(|| format!("Failed to parse .uproject file: {}", path.display()))
}

/// Parse a .uproject file from a string (for testing)
pub fn parse_uproject_from_str(content: &str) -> Result<UProjectFile> {
    serde_json::from_str(content).context("Failed to parse .uproject JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_uproject() {
        let json = r#"{
            "FileVersion": 3,
            "EngineAssociation": "5.6",
            "Modules": [
                {
                    "Name": "MyGame",
                    "Type": "Runtime",
                    "LoadingPhase": "Default"
                }
            ],
            "Plugins": [],
            "TargetPlatforms": ["Windows"]
        }"#;

        let result = parse_uproject_from_str(json).unwrap();
        assert_eq!(result.file_version, 3);
        assert_eq!(result.engine_association, "5.6");
        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.modules[0].name, "MyGame");
        assert_eq!(result.modules[0].module_type, "Runtime");
    }

    #[test]
    fn test_parse_uproject_with_plugins() {
        let json = r#"{
            "FileVersion": 3,
            "EngineAssociation": "5.7",
            "Modules": [
                {
                    "Name": "OpenWorldFramework",
                    "Type": "Runtime",
                    "LoadingPhase": "Default"
                }
            ],
            "Plugins": [
                {
                    "Name": "ModelingToolsEditorMode",
                    "Enabled": true,
                    "TargetAllowList": ["Editor"]
                },
                {
                    "Name": "OWF_Globals",
                    "Enabled": true
                }
            ],
            "TargetPlatforms": ["Windows"]
        }"#;

        let result = parse_uproject_from_str(json).unwrap();
        assert_eq!(result.engine_association, "5.7");
        assert_eq!(result.plugins.len(), 2);
        assert_eq!(result.plugins[0].name, "ModelingToolsEditorMode");
        assert_eq!(result.plugins[0].enabled, true);
        assert_eq!(result.plugins[0].target_allow_list, vec!["Editor"]);
        assert_eq!(result.plugins[1].name, "OWF_Globals");
        assert_eq!(result.plugins[1].target_allow_list.len(), 0);
    }

    #[test]
    fn test_parse_minimal_uproject() {
        let json = r#"{
            "FileVersion": 3,
            "EngineAssociation": "5.5"
        }"#;

        let result = parse_uproject_from_str(json).unwrap();
        assert_eq!(result.engine_association, "5.5");
        assert_eq!(result.modules.len(), 0);
        assert_eq!(result.plugins.len(), 0);
        assert_eq!(result.target_platforms.len(), 0);
    }
}
