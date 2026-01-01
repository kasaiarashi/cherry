// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use super::project_model::{UEModule, UEProject};

/// Build standard UE defines based on platform and configuration
pub fn build_standard_defines(platform: &str, configuration: &str) -> Vec<String> {
    let mut defines = Vec::new();

    // Platform defines
    let platform_upper = platform.to_uppercase();
    defines.push(format!("PLATFORM_{}=1", platform_upper));

    // Configuration defines
    let config_define = match configuration {
        "Debug" | "DebugGame" => "UE_BUILD_DEBUG=1",
        "Development" => "UE_BUILD_DEVELOPMENT=1",
        "Shipping" => "UE_BUILD_SHIPPING=1",
        "Test" => "UE_BUILD_TEST=1",
        _ => "UE_BUILD_DEVELOPMENT=1",
    };
    defines.push(config_define.to_string());

    // Always present for editor builds
    defines.push("WITH_EDITOR=1".to_string());
    defines.push("UE_EDITOR=1".to_string());

    // Unicode defines
    defines.push("UNICODE".to_string());
    defines.push("_UNICODE".to_string());

    // Common UE defines
    defines.push("UE_BUILD_MINIMAL=0".to_string());
    defines.push("WITH_ENGINE=1".to_string());
    defines.push("WITH_UNREAL_DEVELOPER_TOOLS=1".to_string());
    defines.push("WITH_APPLICATION_CORE=1".to_string());
    defines.push("WITH_COREUOBJECT=1".to_string());

    // Platform-specific defines
    match platform {
        "Windows" | "Win64" => {
            defines.push("PLATFORM_WINDOWS=1".to_string());
            defines.push("PLATFORM_MICROSOFT=1".to_string());
            defines.push("PLATFORM_DESKTOP=1".to_string());
            defines.push("OVERRIDE_PLATFORM_HEADER_NAME=Windows".to_string());
        }
        "Linux" => {
            defines.push("PLATFORM_LINUX=1".to_string());
            defines.push("PLATFORM_UNIX=1".to_string());
            defines.push("PLATFORM_DESKTOP=1".to_string());
        }
        "Mac" => {
            defines.push("PLATFORM_MAC=1".to_string());
            defines.push("PLATFORM_APPLE=1".to_string());
            defines.push("PLATFORM_DESKTOP=1".to_string());
        }
        _ => {}
    }

    defines
}

/// Build module API defines (e.g., MODULENAME_API for DLL export/import)
pub fn build_module_api_defines(modules: &[UEModule]) -> Vec<String> {
    modules
        .iter()
        .map(|m| {
            // Generate MODULENAME_API macro
            // In clangd config, we set it to empty since we don't need actual DLL export/import
            format!("{}_API=", m.name.to_uppercase())
        })
        .collect()
}

/// Build all defines for a project
pub fn build_all_defines(project: &UEProject, platform: &str, configuration: &str) -> Vec<String> {
    let mut defines = Vec::new();

    // Standard UE defines
    defines.extend(build_standard_defines(platform, configuration));

    // Module API defines
    let mut all_modules = project.modules.clone();
    for plugin in &project.plugins {
        all_modules.extend(plugin.modules.clone());
    }
    defines.extend(build_module_api_defines(&all_modules));

    // Custom defines from modules
    for module in &all_modules {
        defines.extend(module.defines.clone());
    }

    // Remove duplicates
    defines.sort();
    defines.dedup();

    defines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ue_project::project_model::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_standard_defines_windows() {
        let defines = build_standard_defines("Windows", "Development");

        assert!(defines.contains(&"PLATFORM_WINDOWS=1".to_string()));
        assert!(defines.contains(&"UE_BUILD_DEVELOPMENT=1".to_string()));
        assert!(defines.contains(&"WITH_EDITOR=1".to_string()));
        assert!(defines.contains(&"UNICODE".to_string()));
    }

    #[test]
    fn test_build_standard_defines_debug() {
        let defines = build_standard_defines("Windows", "Debug");
        assert!(defines.contains(&"UE_BUILD_DEBUG=1".to_string()));
    }

    #[test]
    fn test_build_module_api_defines() {
        let modules = vec![
            UEModule {
                name: "Core".to_string(),
                module_type: ModuleType::Runtime,
                loading_phase: "Default".to_string(),
                source_path: PathBuf::from("."),
                build_cs_path: PathBuf::from("."),
                dependencies: ModuleDependencies::default(),
                include_paths: IncludePaths::default(),
                defines: Vec::new(),
            },
            UEModule {
                name: "Engine".to_string(),
                module_type: ModuleType::Runtime,
                loading_phase: "Default".to_string(),
                source_path: PathBuf::from("."),
                build_cs_path: PathBuf::from("."),
                dependencies: ModuleDependencies::default(),
                include_paths: IncludePaths::default(),
                defines: Vec::new(),
            },
        ];

        let api_defines = build_module_api_defines(&modules);
        assert!(api_defines.contains(&"CORE_API=".to_string()));
        assert!(api_defines.contains(&"ENGINE_API=".to_string()));
    }

    #[test]
    fn test_build_all_defines() {
        let project = UEProject {
            uproject_path: PathBuf::from("."),
            project_name: "TestProject".to_string(),
            engine_association: "5.6".to_string(),
            engine_path: None,
            modules: vec![UEModule {
                name: "TestProject".to_string(),
                module_type: ModuleType::Runtime,
                loading_phase: "Default".to_string(),
                source_path: PathBuf::from("."),
                build_cs_path: PathBuf::from("."),
                dependencies: ModuleDependencies::default(),
                include_paths: IncludePaths::default(),
                defines: vec!["CUSTOM_DEFINE=1".to_string()],
            }],
            plugins: Vec::new(),
            target_platforms: Vec::new(),
        };

        let defines = build_all_defines(&project, "Windows", "Development");

        assert!(defines.contains(&"PLATFORM_WINDOWS=1".to_string()));
        assert!(defines.contains(&"UE_BUILD_DEVELOPMENT=1".to_string()));
        assert!(defines.contains(&"TESTPROJECT_API=".to_string()));
        assert!(defines.contains(&"CUSTOM_DEFINE=1".to_string()));
    }
}
