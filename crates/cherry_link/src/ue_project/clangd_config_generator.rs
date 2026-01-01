// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Enhanced clangd configuration file with PathMatch support (like unreal-clangd)
#[derive(Debug, Serialize, Deserialize)]
pub struct ClangdConfigFile {
    /// Conditional block for PathMatch (applies config only to matching files)
    #[serde(rename = "If", skip_serializing_if = "Option::is_none")]
    pub condition: Option<IfCondition>,

    #[serde(rename = "CompileFlags")]
    pub compile_flags: CompileFlagsSection,

    #[serde(rename = "InlayHints", skip_serializing_if = "Option::is_none")]
    pub inlay_hints: Option<InlayHintsSection>,

    #[serde(rename = "Diagnostics", skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticsSection>,

    #[serde(rename = "Index", skip_serializing_if = "Option::is_none")]
    pub index: Option<IndexSection>,
}

/// Conditional block for applying config to specific file patterns
#[derive(Debug, Serialize, Deserialize)]
pub struct IfCondition {
    #[serde(rename = "PathMatch")]
    pub path_match: Vec<String>,

    #[serde(rename = "PathExclude", skip_serializing_if = "Option::is_none")]
    pub path_exclude: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileFlagsSection {
    /// Path to compilation database (e.g., ".cherry")
    #[serde(rename = "CompilationDatabase", skip_serializing_if = "Option::is_none")]
    pub compilation_database: Option<String>,

    /// Explicit compiler path (e.g., "clang-cl.exe")
    #[serde(rename = "Compiler", skip_serializing_if = "Option::is_none")]
    pub compiler: Option<String>,

    #[serde(rename = "Add")]
    pub add: Vec<String>,

    #[serde(rename = "Remove", skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
}

/// InlayHints configuration for better IDE experience
#[derive(Debug, Serialize, Deserialize)]
pub struct InlayHintsSection {
    #[serde(rename = "Enabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<String>,

    #[serde(rename = "DeducedTypes", skip_serializing_if = "Option::is_none")]
    pub deduced_types: Option<String>,

    #[serde(rename = "ParameterNames", skip_serializing_if = "Option::is_none")]
    pub parameter_names: Option<String>,

    #[serde(rename = "Designators", skip_serializing_if = "Option::is_none")]
    pub designators: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticsSection {
    #[serde(rename = "UnusedIncludes", skip_serializing_if = "Option::is_none")]
    pub unused_includes: Option<String>,

    #[serde(rename = "MissingIncludes", skip_serializing_if = "Option::is_none")]
    pub missing_includes: Option<String>,

    #[serde(rename = "ClangTidy", skip_serializing_if = "Option::is_none")]
    pub clang_tidy: Option<ClangTidySection>,

    #[serde(rename = "Suppress", skip_serializing_if = "Option::is_none")]
    pub suppress: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClangTidySection {
    #[serde(rename = "Remove", skip_serializing_if = "Option::is_none")]
    pub remove: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexSection {
    #[serde(rename = "Background", skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,

    #[serde(rename = "StandardLibrary", skip_serializing_if = "Option::is_none")]
    pub standard_library: Option<String>,
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
        condition: None,
        compile_flags: CompileFlagsSection {
            compilation_database: None,
            compiler: None,
            add: add_flags,
            remove: Some(vec![
                "-W*".to_string(), // Remove default warnings, we'll add back what we need
                "--target=*".to_string(), // Remove any conflicting target specs
                "-x*".to_string(), // Remove any conflicting language specs
            ]),
        },
        inlay_hints: None,
        diagnostics: Some(DiagnosticsSection {
            unused_includes: Some("None".to_string()), // UE uses lots of forward decls
            missing_includes: Some("Strict".to_string()),
            clang_tidy: None,
            suppress: None,
        }),
        index: Some(IndexSection {
            background: Some("Build".to_string()),
            standard_library: None,
        }),
    };

    serde_yaml::to_string(&config).context("Failed to serialize clangd config to YAML")
}

/// Warning flags from unreal-clangd that work well with UE projects
const UE_WARNING_FLAGS: &[&str] = &[
    "-Wno-enum-enum-conversion",
    "-Wno-enum-float-conversion",
    "-Wno-ambiguous-reversed-operator",
    "-Wno-deprecated-anon-enum-enum-conversion",
    "-Wno-deprecated-volatile",
    "-Wno-unused-but-set-variable",
    "-Wno-unused-but-set-parameter",
    "-Wno-ordered-compare-function-pointers",
    "-Wno-bitwise-instead-of-logical",
    "-Wno-gnu-string-literal-operator-template",
    "-Wno-inconsistent-missing-override",
    "-Wno-invalid-offsetof",
    "-Wno-switch",
    "-Wno-tautological-compare",
    "-Wno-unknown-pragmas",
    "-Wno-unused-function",
    "-Wno-unused-lambda-capture",
    "-Wno-unused-local-typedef",
    "-Wno-unused-private-field",
    "-Wno-unused-variable",
    "-Wno-undefined-var-template",
    "-Wno-float-conversion",
    "-Wno-implicit-float-conversion",
    "-Wno-implicit-int-conversion",
    "-Wno-c++11-narrowing",
    "-Wno-microsoft",
    "-Wno-msvc-include",
    "-Wno-pragma-pack",
    "-Wno-inline-new-delete",
    "-Wno-implicit-exception-spec-mismatch",
    "-Wno-undefined-bool-conversion",
    "-Wno-deprecated-writable-strings",
    "-Wno-deprecated-register",
    "-Wno-switch-enum",
    "-Wno-logical-op-parentheses",
    "-Wno-null-arithmetic",
    "-Wno-deprecated-declarations",
    "-Wno-return-type-c-linkage",
    "-Wno-ignored-attributes",
    "-Wno-uninitialized",
    "-Wno-return-type",
    "-Wno-unused-parameter",
    "-Wno-ignored-qualifiers",
    "-Wno-expansion-to-defined",
    "-Wno-sign-compare",
    "-Wno-missing-field-initializers",
    "-Wno-nonportable-include-path",
    "-Wno-invalid-token-paste",
    "-Wno-null-pointer-arithmetic",
    "-Wno-constant-logical-operand",
    "-Wno-unused-value",
    "-Wno-bitfield-enum-conversion",
];

/// MSVC flags to remove from compile_commands.json
const MSVC_FLAGS_TO_REMOVE: &[&str] = &[
    "/FI*",
    "/Yu*",
    "/Yc*",
    "/Fp*",
    "/EH*",
    "/GR*",
    "/W*",
    "/wd*",
    "/we*",
    "/Zc:*",
    "/permissive*",
    "/JMC",
    "/ZI",
    "/Zi",
    "/FS",
    "/MP*",
    "-m32",
    "-m64",
];

/// Generate enhanced clangd configuration for Unreal Engine projects
/// Based on unreal-clangd VSCode extension configuration
pub fn generate_ue_clangd_config(
    project_root: &Path,
    cherry_dir: &Path,
    macro_helper_path: Option<&Path>,
    compiler_path: Option<&str>,
    include_paths: &[PathBuf],
    defines: &[String],
) -> Result<String> {
    let mut add_flags = Vec::new();

    // C++ standard and mode
    add_flags.push("/std:c++20".to_string());
    add_flags.push("/TP".to_string()); // Treat as C++
    add_flags.push("-ferror-limit=0".to_string()); // Don't stop on too many errors

    // Force include macro helper if provided
    if let Some(helper_path) = macro_helper_path {
        let helper_str = helper_path.display().to_string().replace("\\", "/");
        add_flags.push(format!("/FI{}", helper_str));
    }

    // Add include paths
    for path in include_paths {
        add_flags.push(format!("-I{}", path.display().to_string().replace("\\", "/")));
    }

    // Add defines
    for define in defines {
        add_flags.push(format!("-D{}", define));
    }

    // Add all warning suppressions from unreal-clangd
    for flag in UE_WARNING_FLAGS {
        add_flags.push(flag.to_string());
    }

    // Build remove flags list
    let remove_flags: Vec<String> = MSVC_FLAGS_TO_REMOVE.iter().map(|s| s.to_string()).collect();

    // Build PathMatch patterns for project files
    let path_match = vec![
        ".cherry/completionHelper.cpp".to_string(),
        "Plugins/.*\\.(cpp|h|hpp|inl)".to_string(),
        "Source/.*\\.(cpp|h|hpp|inl)".to_string(),
    ];

    let config = ClangdConfigFile {
        condition: Some(IfCondition {
            path_match,
            path_exclude: None,
        }),
        compile_flags: CompileFlagsSection {
            compilation_database: Some(".cherry".to_string()),
            compiler: compiler_path.map(|s| s.to_string()),
            add: add_flags,
            remove: Some(remove_flags),
        },
        inlay_hints: Some(InlayHintsSection {
            enabled: Some("Yes".to_string()),
            deduced_types: Some("Yes".to_string()),
            parameter_names: Some("Yes".to_string()),
            designators: Some("Yes".to_string()),
        }),
        diagnostics: Some(DiagnosticsSection {
            unused_includes: Some("None".to_string()),
            missing_includes: None,
            clang_tidy: Some(ClangTidySection {
                remove: Some("*".to_string()),
            }),
            suppress: Some(vec![
                "builtin_definition".to_string(),
                "pp_file_not_found".to_string(),
                "incomplete_nested_name_spec".to_string(),
                "use_of_tag_name_without_tag".to_string(),
                "*".to_string(), // Suppress all diagnostics for UE
            ]),
        }),
        index: Some(IndexSection {
            background: Some("Skip".to_string()), // Don't index in background - saves memory/CPU
            standard_library: Some("No".to_string()),
        })
    };

    // Generate YAML with document separator
    let yaml = serde_yaml::to_string(&config).context("Failed to serialize clangd config to YAML")?;
    Ok(format!("---\n{}", yaml))
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
            condition: None,
            compile_flags: CompileFlagsSection {
                compilation_database: None,
                compiler: None,
                add: vec!["-Itest".to_string(), "-DTEST=1".to_string()],
                remove: Some(vec!["-W*".to_string()]),
            },
            inlay_hints: None,
            diagnostics: Some(DiagnosticsSection {
                unused_includes: Some("None".to_string()),
                missing_includes: Some("Strict".to_string()),
                clang_tidy: None,
                suppress: None,
            }),
            index: Some(IndexSection {
                background: Some("Build".to_string()),
                standard_library: None,
            }),
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("CompileFlags:"));
        assert!(yaml.contains("Add:"));
        assert!(yaml.contains("-Itest"));
    }

    #[test]
    fn test_ue_clangd_config_with_pathmatch() {
        let config = ClangdConfigFile {
            condition: Some(IfCondition {
                path_match: vec!["Source/.*\\.cpp".to_string()],
                path_exclude: None,
            }),
            compile_flags: CompileFlagsSection {
                compilation_database: Some(".cherry".to_string()),
                compiler: Some("clang-cl.exe".to_string()),
                add: vec!["/std:c++20".to_string()],
                remove: None,
            },
            inlay_hints: Some(InlayHintsSection {
                enabled: Some("Yes".to_string()),
                deduced_types: Some("Yes".to_string()),
                parameter_names: Some("Yes".to_string()),
                designators: Some("Yes".to_string()),
            }),
            diagnostics: None,
            index: None,
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("If:"));
        assert!(yaml.contains("PathMatch:"));
        assert!(yaml.contains("CompilationDatabase:"));
        assert!(yaml.contains("Compiler:"));
        assert!(yaml.contains("InlayHints:"));
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
