// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use super::project_model::{IncludePaths, ModuleDependencies};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Parse a .Build.cs file to extract dependencies and other metadata
pub fn parse_build_cs(path: &Path) -> Result<(ModuleDependencies, IncludePaths, Vec<String>)> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read .Build.cs file: {}", path.display()))?;

    parse_build_cs_from_str(&content)
}

/// Parse a .Build.cs file from a string (for testing)
pub fn parse_build_cs_from_str(content: &str) -> Result<(ModuleDependencies, IncludePaths, Vec<String>)> {
    let mut deps = ModuleDependencies::default();
    let mut includes = IncludePaths::default();
    let mut defines = Vec::new();

    // Extract dependencies using AddRange pattern
    static ADDRANGE_REGEX: OnceLock<Regex> = OnceLock::new();
    let addrange_regex = ADDRANGE_REGEX.get_or_init(|| {
        Regex::new(r"(?m)(Public|Private)DependencyModuleNames\.AddRange\s*\(\s*new\s+string\s*\[\]\s*\{([^}]+)\}")
            .unwrap()
    });

    for cap in addrange_regex.captures_iter(content) {
        let visibility = &cap[1]; // "Public" or "Private"
        let array_content = &cap[2];

        let modules = extract_string_literals(array_content);

        if visibility == "Public" {
            deps.public_dependencies.extend(modules);
        } else {
            deps.private_dependencies.extend(modules);
        }
    }

    // Extract dependencies using Add pattern
    static ADD_REGEX: OnceLock<Regex> = OnceLock::new();
    let add_regex = ADD_REGEX.get_or_init(|| {
        Regex::new(r#"(?m)(Public|Private)DependencyModuleNames\.Add\s*\(\s*"([^"]+)"\s*\)"#)
            .unwrap()
    });

    for cap in add_regex.captures_iter(content) {
        let visibility = &cap[1];
        let module = cap[2].to_string();

        if visibility == "Public" {
            deps.public_dependencies.push(module);
        } else {
            deps.private_dependencies.push(module);
        }
    }

    // Extract include paths using AddRange pattern
    static INCLUDE_ADDRANGE_REGEX: OnceLock<Regex> = OnceLock::new();
    let include_addrange_regex = INCLUDE_ADDRANGE_REGEX.get_or_init(|| {
        Regex::new(r"(?m)(Public|Private)IncludePaths\.AddRange\s*\(\s*new\s+string\s*\[\]\s*\{([^}]+)\}")
            .unwrap()
    });

    for cap in include_addrange_regex.captures_iter(content) {
        let visibility = &cap[1];
        let array_content = &cap[2];

        let paths: Vec<PathBuf> = extract_string_literals(array_content)
            .into_iter()
            .map(PathBuf::from)
            .collect();

        if visibility == "Public" {
            includes.public_include_paths.extend(paths);
        } else {
            includes.private_include_paths.extend(paths);
        }
    }

    // Extract include paths using Add pattern
    static INCLUDE_ADD_REGEX: OnceLock<Regex> = OnceLock::new();
    let include_add_regex = INCLUDE_ADD_REGEX.get_or_init(|| {
        Regex::new(r#"(?m)(Public|Private)IncludePaths\.Add\s*\(\s*"([^"]+)"\s*\)"#)
            .unwrap()
    });

    for cap in include_add_regex.captures_iter(content) {
        let visibility = &cap[1];
        let path = PathBuf::from(&cap[2]);

        if visibility == "Public" {
            includes.public_include_paths.push(path);
        } else {
            includes.private_include_paths.push(path);
        }
    }

    // Extract definitions
    static DEFINITIONS_REGEX: OnceLock<Regex> = OnceLock::new();
    let definitions_regex = DEFINITIONS_REGEX.get_or_init(|| {
        Regex::new(r#"(?m)PublicDefinitions\.Add\s*\(\s*"([^"]+)"\s*\)"#)
            .unwrap()
    });

    for cap in definitions_regex.captures_iter(content) {
        defines.push(cap[1].to_string());
    }

    Ok((deps, includes, defines))
}

/// Extract string literals from a comma-separated list
fn extract_string_literals(text: &str) -> Vec<String> {
    static STRING_LITERAL_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = STRING_LITERAL_REGEX.get_or_init(|| {
        Regex::new(r#""([^"]+)""#).unwrap()
    });

    regex
        .captures_iter(text)
        .map(|cap| cap[1].to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_build_cs() {
        let content = r#"
        using UnrealBuildTool;

        public class OWF_AudioSystem : ModuleRules
        {
            public OWF_AudioSystem(ReadOnlyTargetRules Target) : base(Target)
            {
                PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;

                PublicDependencyModuleNames.AddRange(
                    new string[]
                    {
                        "Core", "CoreUObject", "Engine", "OWF_Globals"
                    }
                );

                PrivateDependencyModuleNames.AddRange(
                    new string[]
                    {
                        "Slate",
                        "SlateCore"
                    }
                );
            }
        }
        "#;

        let (deps, _, _) = parse_build_cs_from_str(content).unwrap();
        assert_eq!(deps.public_dependencies, vec!["Core", "CoreUObject", "Engine", "OWF_Globals"]);
        assert_eq!(deps.private_dependencies, vec!["Slate", "SlateCore"]);
    }

    #[test]
    fn test_parse_build_cs_with_add() {
        let content = r#"
        public class MyModule : ModuleRules
        {
            public MyModule(ReadOnlyTargetRules Target) : base(Target)
            {
                PublicDependencyModuleNames.Add("Core");
                PublicDependencyModuleNames.Add("Engine");
                PrivateDependencyModuleNames.Add("Slate");
            }
        }
        "#;

        let (deps, _, _) = parse_build_cs_from_str(content).unwrap();
        assert_eq!(deps.public_dependencies, vec!["Core", "Engine"]);
        assert_eq!(deps.private_dependencies, vec!["Slate"]);
    }

    #[test]
    fn test_parse_build_cs_with_includes() {
        let content = r#"
        public class MyModule : ModuleRules
        {
            public MyModule(ReadOnlyTargetRules Target) : base(Target)
            {
                PublicIncludePaths.AddRange(
                    new string[] {
                        "Runtime/Engine/Public",
                        "Runtime/Core/Public"
                    }
                );

                PrivateIncludePaths.Add("MyModule/Private/Internal");
            }
        }
        "#;

        let (_, includes, _) = parse_build_cs_from_str(content).unwrap();
        assert_eq!(includes.public_include_paths.len(), 2);
        assert_eq!(includes.private_include_paths.len(), 1);
    }

    #[test]
    fn test_parse_build_cs_with_defines() {
        let content = r#"
        public class MyModule : ModuleRules
        {
            public MyModule(ReadOnlyTargetRules Target) : base(Target)
            {
                PublicDefinitions.Add("WITH_GAMEPLAY_DEBUGGER=1");
                PublicDefinitions.Add("UE_ENABLE_DEBUG_DRAWING=1");
            }
        }
        "#;

        let (_, _, defines) = parse_build_cs_from_str(content).unwrap();
        assert_eq!(defines, vec!["WITH_GAMEPLAY_DEBUGGER=1", "UE_ENABLE_DEBUG_DRAWING=1"]);
    }

    #[test]
    fn test_parse_mixed_build_cs() {
        let content = r#"
        public class ComplexModule : ModuleRules
        {
            public ComplexModule(ReadOnlyTargetRules Target) : base(Target)
            {
                PublicDependencyModuleNames.AddRange(new string[] { "Core", "Engine" });
                PublicDependencyModuleNames.Add("GameplayTags");
                PrivateDependencyModuleNames.Add("Slate");
                PrivateDependencyModuleNames.AddRange(new string[] { "SlateCore", "InputCore" });
            }
        }
        "#;

        let (deps, _, _) = parse_build_cs_from_str(content).unwrap();
        assert_eq!(deps.public_dependencies, vec!["Core", "Engine", "GameplayTags"]);
        assert_eq!(deps.private_dependencies, vec!["Slate", "SlateCore", "InputCore"]);
    }
}
