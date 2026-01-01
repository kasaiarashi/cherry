// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClangdConfigFile {
    #[serde(rename = "CompileFlags")]
    pub compile_flags: CompileFlagsSection,

    #[serde(rename = "Diagnostics", skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticsSection>,

    #[serde(rename = "Index", skip_serializing_if = "Option::is_none")]
    pub index: Option<IndexSection>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileFlagsSection {
    #[serde(rename = "Add")]
    pub add: Vec<String>,

    #[serde(rename = "Remove", skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticsSection {
    #[serde(rename = "UnusedIncludes", skip_serializing_if = "Option::is_none")]
    pub unused_includes: Option<String>,

    #[serde(rename = "MissingIncludes", skip_serializing_if = "Option::is_none")]
    pub missing_includes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexSection {
    #[serde(rename = "Background", skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

/// Generate clangd configuration from include paths and defines
pub fn generate_clangd_config(
    include_paths: &[PathBuf],
    defines: &[String],
) -> Result<String> {
    let mut add_flags = Vec::new();

    // ============================================================================
    // CRITICAL: Platform/Language specification (fixes NSString errors on Windows)
    // ============================================================================
    // Explicitly tell clang this is C++ (not Objective-C++) to prevent parsing
    // Apple platform-specific headers like NSString
    add_flags.push("-xc++".to_string());

    // Specify Windows as the target platform to exclude non-Windows headers
    add_flags.push("--target=x86_64-pc-windows-msvc".to_string());

    // Add include paths
    for path in include_paths {
        add_flags.push(format!("-I{}", path.display()));
    }

    // Add defines
    for define in defines {
        add_flags.push(format!("-D{}", define));
    }

    // Add standard C++ version for UE
    add_flags.push("-std=c++20".to_string());

    // Suppress common UE-specific warnings
    add_flags.push("-Wno-unused-parameter".to_string());
    add_flags.push("-Wno-microsoft-enum-value".to_string());
    add_flags.push("-Wno-unused-variable".to_string());
    add_flags.push("-Wno-unused-private-field".to_string());

    // Add clang-specific flags for better UE support
    add_flags.push("-fms-extensions".to_string()); // Microsoft extensions (for __declspec, etc.)
    add_flags.push("-fms-compatibility".to_string()); // Better MSVC compatibility
    add_flags.push("-fms-compatibility-version=19.38".to_string()); // VS 2022 17.8
    add_flags.push("-fdelayed-template-parsing".to_string()); // MSVC-style template parsing
    add_flags.push("-ferror-limit=0".to_string()); // Show all errors (not just first few)

    let config = ClangdConfigFile {
        compile_flags: CompileFlagsSection {
            add: add_flags,
            remove: Some(vec![
                "-W*".to_string(), // Remove default warnings, we'll add back what we need
                "--target=*".to_string(), // Remove any conflicting target specs
                "-x*".to_string(), // Remove any conflicting language specs
            ]),
        },
        diagnostics: Some(DiagnosticsSection {
            unused_includes: Some("None".to_string()), // UE uses lots of forward decls
            missing_includes: Some("Strict".to_string()),
        }),
        index: Some(IndexSection {
            background: Some("Build".to_string()),
        }),
    };

    serde_yaml::to_string(&config).context("Failed to serialize clangd config to YAML")
}

/// Write clangd configuration to a .clangd file in the project root
pub fn write_clangd_config(config: &str, project_root: &Path) -> Result<()> {
    let config_path = project_root.join(".clangd");
    std::fs::write(&config_path, config)
        .with_context(|| format!("Failed to write .clangd config to {}", config_path.display()))?;

    log::info!("Generated .clangd configuration at {}", config_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_clangd_config() {
        let include_paths = vec![
            PathBuf::from("W:/Engine/Source/Runtime/Core/Public"),
            PathBuf::from("W:/Project/Source/MyGame/Public"),
        ];

        let defines = vec![
            "PLATFORM_WINDOWS=1".to_string(),
            "UE_BUILD_DEVELOPMENT=1".to_string(),
            "WITH_EDITOR=1".to_string(),
        ];

        let config = generate_clangd_config(&include_paths, &defines).unwrap();

        // Verify YAML structure
        assert!(config.contains("CompileFlags:"));
        assert!(config.contains("Add:"));
        assert!(config.contains("-I"));
        assert!(config.contains("-DPLATFORM_WINDOWS=1"));
        assert!(config.contains("-std=c++20"));
        assert!(config.contains("Diagnostics:"));
        assert!(config.contains("UnusedIncludes: None"));
    }

    #[test]
    fn test_clangd_config_serialization() {
        let config = ClangdConfigFile {
            compile_flags: CompileFlagsSection {
                add: vec!["-Itest".to_string(), "-DTEST=1".to_string()],
                remove: Some(vec!["-W*".to_string()]),
            },
            diagnostics: Some(DiagnosticsSection {
                unused_includes: Some("None".to_string()),
                missing_includes: Some("Strict".to_string()),
            }),
            index: Some(IndexSection {
                background: Some("Build".to_string()),
            }),
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("CompileFlags:"));
        assert!(yaml.contains("Add:"));
        assert!(yaml.contains("-Itest"));
    }

    #[test]
    fn test_clangd_config_deserialization() {
        let yaml = r#"
CompileFlags:
  Add:
    - -Itest
    - -DTEST=1
  Remove:
    - -W*
Diagnostics:
  UnusedIncludes: None
Index:
  Background: Build
"#;

        let config: ClangdConfigFile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.compile_flags.add.len(), 2);
        assert_eq!(config.compile_flags.add[0], "-Itest");
        assert!(config.diagnostics.is_some());
    }
}
