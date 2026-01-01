// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Build.cs file parser
//!
//! Parses .Build.cs files to extract:
//! - Module dependencies (PublicDependencyModuleNames, PrivateDependencyModuleNames)
//! - Include paths (PublicIncludePaths, PrivateIncludePaths)
//! - Preprocessor defines (PublicDefinitions, PrivateDefinitions)

use anyhow::Result;
use regex::Regex;
use std::path::Path;

/// Parsed Build.cs information
#[derive(Debug, Clone, Default)]
pub struct BuildCsInfo {
    pub public_dependencies: Vec<String>,
    pub private_dependencies: Vec<String>,
    pub public_include_paths: Vec<String>,
    pub private_include_paths: Vec<String>,
    pub public_definitions: Vec<String>,
    pub private_definitions: Vec<String>,
}

impl BuildCsInfo {
    /// Parse a Build.cs file
    pub fn parse(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;

        let mut info = BuildCsInfo::default();

        // Parse public dependency modules
        if let Some(deps) = Self::extract_string_array(&content, "PublicDependencyModuleNames") {
            info.public_dependencies = deps;
        }

        // Parse private dependency modules
        if let Some(deps) = Self::extract_string_array(&content, "PrivateDependencyModuleNames")
        {
            info.private_dependencies = deps;
        }

        // Parse public include paths
        if let Some(paths) = Self::extract_string_array(&content, "PublicIncludePaths") {
            info.public_include_paths = paths;
        }

        // Parse private include paths
        if let Some(paths) = Self::extract_string_array(&content, "PrivateIncludePaths") {
            info.private_include_paths = paths;
        }

        // Parse public definitions
        if let Some(defs) = Self::extract_string_array(&content, "PublicDefinitions") {
            info.public_definitions = defs;
        }

        // Parse private definitions
        if let Some(defs) = Self::extract_string_array(&content, "PrivateDefinitions") {
            info.private_definitions = defs;
        }

        Ok(info)
    }

    /// Extract string array from Build.cs content
    /// Example: PublicDependencyModuleNames.AddRange(new string[] { "Core", "Engine" });
    fn extract_string_array(content: &str, property: &str) -> Option<Vec<String>> {
        // Pattern to match: PropertyName.AddRange(new string[] { "item1", "item2" })
        // or: PropertyName.Add("item")
        let pattern_addrange = format!(
            r#"{}\s*\.\s*AddRange\s*\(\s*new\s+string\[\]\s*\{{\s*([^}}]*)\s*\}}\s*\)"#,
            regex::escape(property)
        );

        let pattern_add = format!(
            r#"{}\s*\.\s*Add\s*\(\s*"([^"]*)"\s*\)"#,
            regex::escape(property)
        );

        let mut items = Vec::new();

        // Compile regex outside loop
        let string_re = Regex::new(r#""([^"]*)""#).ok();

        // Try AddRange pattern
        if let Ok(re) = Regex::new(&pattern_addrange) {
            for cap in re.captures_iter(content) {
                if let Some(list) = cap.get(1) {
                    let list_str = list.as_str();
                    // Extract quoted strings
                    if let Some(ref string_regex) = string_re {
                        for string_cap in string_regex.captures_iter(list_str) {
                            if let Some(item) = string_cap.get(1) {
                                items.push(item.as_str().to_string());
                            }
                        }
                    }
                }
            }
        }

        // Try Add pattern
        if let Ok(re) = Regex::new(&pattern_add) {
            for cap in re.captures_iter(content) {
                if let Some(item) = cap.get(1) {
                    items.push(item.as_str().to_string());
                }
            }
        }

        if items.is_empty() {
            None
        } else {
            Some(items)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_build_cs() {
        let temp_dir = TempDir::new().unwrap();
        let build_cs_path = temp_dir.path().join("TestModule.Build.cs");

        let build_cs_content = r#"
using UnrealBuildTool;

public class TestModule : ModuleRules
{
    public TestModule(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(new string[] {
            "Core",
            "CoreUObject",
            "Engine"
        });

        PrivateDependencyModuleNames.AddRange(new string[] {
            "Slate",
            "SlateCore"
        });

        PublicIncludePaths.Add("$(ModuleDir)/Public");
        PrivateIncludePaths.Add("$(ModuleDir)/Private");

        PublicDefinitions.Add("WITH_TESTMODULE=1");
    }
}
"#;

        fs::write(&build_cs_path, build_cs_content).unwrap();

        let info = BuildCsInfo::parse(&build_cs_path).unwrap();

        assert!(info.public_dependencies.contains(&"Core".to_string()));
        assert!(info.public_dependencies.contains(&"Engine".to_string()));
        assert!(info.private_dependencies.contains(&"Slate".to_string()));
        assert!(info.public_definitions.contains(&"WITH_TESTMODULE=1".to_string()));
    }

    #[test]
    fn test_extract_string_array() {
        let content = r#"
        PublicDependencyModuleNames.AddRange(new string[] { "Core", "Engine" });
        PrivateDependencyModuleNames.Add("Slate");
        "#;

        let public_deps =
            BuildCsInfo::extract_string_array(content, "PublicDependencyModuleNames").unwrap();
        assert_eq!(public_deps, vec!["Core", "Engine"]);

        let private_deps =
            BuildCsInfo::extract_string_array(content, "PrivateDependencyModuleNames").unwrap();
        assert_eq!(private_deps, vec!["Slate"]);
    }
}
