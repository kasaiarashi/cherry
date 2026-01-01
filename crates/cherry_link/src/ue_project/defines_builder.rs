// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use super::project_model::{UEModule, UEProject};

/// Build standard UE defines based on platform and configuration
pub fn build_standard_defines(platform: &str, configuration: &str) -> Vec<String> {
    let mut defines = Vec::new();

    // ============================================================================
    // BUILD CONFIGURATION DEFINES (from Misc/Build.h)
    // UBT requires exactly one of these to be 1
    // ============================================================================
    match configuration {
        "Debug" | "DebugGame" => {
            defines.push("UE_BUILD_DEBUG=1".to_string());
            defines.push("UE_BUILD_DEVELOPMENT=0".to_string());
            defines.push("UE_BUILD_TEST=0".to_string());
            defines.push("UE_BUILD_SHIPPING=0".to_string());
        }
        "Development" => {
            defines.push("UE_BUILD_DEBUG=0".to_string());
            defines.push("UE_BUILD_DEVELOPMENT=1".to_string());
            defines.push("UE_BUILD_TEST=0".to_string());
            defines.push("UE_BUILD_SHIPPING=0".to_string());
        }
        "Shipping" => {
            defines.push("UE_BUILD_DEBUG=0".to_string());
            defines.push("UE_BUILD_DEVELOPMENT=0".to_string());
            defines.push("UE_BUILD_TEST=0".to_string());
            defines.push("UE_BUILD_SHIPPING=1".to_string());
        }
        "Test" => {
            defines.push("UE_BUILD_DEBUG=0".to_string());
            defines.push("UE_BUILD_DEVELOPMENT=0".to_string());
            defines.push("UE_BUILD_TEST=1".to_string());
            defines.push("UE_BUILD_SHIPPING=0".to_string());
        }
        _ => {
            // Default to Development
            defines.push("UE_BUILD_DEBUG=0".to_string());
            defines.push("UE_BUILD_DEVELOPMENT=1".to_string());
            defines.push("UE_BUILD_TEST=0".to_string());
            defines.push("UE_BUILD_SHIPPING=0".to_string());
        }
    }

    // ============================================================================
    // MANDATORY UBT BRIDGE DEFINES (from Misc/Build.h)
    // UBT must always define these to be 0 or 1
    // ============================================================================
    defines.push("WITH_EDITOR=1".to_string()); // Compiling with editor
    defines.push("WITH_ENGINE=1".to_string()); // Compiling with engine
    defines.push("WITH_UNREAL_DEVELOPER_TOOLS=1".to_string()); // Developer tools
    defines.push("WITH_PLUGIN_SUPPORT=1".to_string()); // Plugin support
    defines.push("IS_MONOLITHIC=0".to_string()); // Modular build (not monolithic)
    defines.push("IS_PROGRAM=0".to_string()); // Game/Editor, not standalone program

    // ============================================================================
    // TARGET TYPE DEFINES
    // ============================================================================
    defines.push("UE_GAME=0".to_string()); // Not a standalone game
    defines.push("UE_CLIENT=0".to_string()); // Not a client-only build
    defines.push("UE_EDITOR=1".to_string()); // Editor build
    defines.push("UE_SERVER=0".to_string()); // Not a dedicated server

    // ============================================================================
    // OPTIONAL BRIDGE DEFINES (from Misc/Build.h)
    // ============================================================================
    defines.push("WITH_EDITORONLY_DATA=1".to_string()); // Include editor-only data
    defines.push("WITH_UNREAL_TARGET_DEVELOPER_TOOLS=1".to_string());
    defines.push("WITH_ACCESSIBILITY=1".to_string());
    defines.push("WITH_PERFCOUNTERS=0".to_string());
    defines.push("WITH_AUTOMATION_WORKER=1".to_string());
    defines.push("WITH_HOT_RELOAD=1".to_string());
    defines.push("WITH_LIVE_CODING=0".to_string());
    defines.push("WITH_TEXT_ARCHIVE_SUPPORT=1".to_string());
    defines.push("WITH_SERVER_CODE=1".to_string()); // Include server-side code

    // ============================================================================
    // LOGGING AND DEBUGGING DEFINES (from Misc/Build.h)
    // ============================================================================
    defines.push("USE_LOGGING_IN_SHIPPING=0".to_string());
    defines.push("USE_CHECKS_IN_SHIPPING=0".to_string());
    defines.push("USE_ENSURES_IN_SHIPPING=0".to_string());
    defines.push("NO_LOGGING=0".to_string());
    defines.push("ALLOW_CONSOLE=1".to_string());
    defines.push("ALLOW_DEBUG_FILES=1".to_string());

    // Development configuration specific
    defines.push("DO_GUARD_SLOW=0".to_string());
    defines.push("DO_CHECK=1".to_string());
    defines.push("DO_ENSURE=1".to_string());

    // ============================================================================
    // STATS AND PROFILING (from Misc/Build.h)
    // ============================================================================
    defines.push("STATS=1".to_string());
    defines.push("USE_STATS_WITHOUT_ENGINE=0".to_string());
    defines.push("ENABLE_STATNAMEDEVENTS=0".to_string());
    defines.push("WITH_PROFILEGPU=1".to_string());
    defines.push("USE_NETWORK_PROFILER=1".to_string());

    // ============================================================================
    // ADDITIONAL ENGINE FEATURES
    // ============================================================================
    defines.push("WITH_APPLICATION_CORE=1".to_string());
    defines.push("WITH_COREUOBJECT=1".to_string());
    defines.push("WITH_LOGGING_TO_MEMORY=0".to_string());
    defines.push("UE_ENABLE_ICU=1".to_string()); // Internationalization
    defines.push("WITH_VERSE_VM=0".to_string());
    defines.push("WITH_DEV_AUTOMATION_TESTS=1".to_string());
    defines.push("WITH_PERF_AUTOMATION_TESTS=1".to_string());

    // ============================================================================
    // UNICODE SUPPORT
    // ============================================================================
    defines.push("UNICODE".to_string());
    defines.push("_UNICODE".to_string());

    // ============================================================================
    // PLATFORM-SPECIFIC DEFINES (from HAL/Platform.h)
    // ============================================================================
    match platform {
        "Windows" | "Win64" => {
            // Platform identification
            defines.push("PLATFORM_WINDOWS=1".to_string());
            defines.push("PLATFORM_MICROSOFT=1".to_string());
            defines.push("PLATFORM_DESKTOP=1".to_string()); // MANDATORY
            defines.push("PLATFORM_64BITS=1".to_string()); // MANDATORY
            defines.push("PLATFORM_CPU_X86_FAMILY=1".to_string());
            defines.push("PLATFORM_CPU_ARM_FAMILY=0".to_string());
            defines.push("OVERRIDE_PLATFORM_HEADER_NAME=Windows".to_string());

            // Windows SDK version targeting (Windows 10)
            defines.push("WINVER=0x0A00".to_string());
            defines.push("_WIN32_WINNT=0x0A00".to_string());

            // Platform capabilities
            defines.push("PLATFORM_LITTLE_ENDIAN=1".to_string());
            defines.push("PLATFORM_SUPPORTS_UNALIGNED_LOADS=1".to_string());
            defines.push("PLATFORM_EXCEPTIONS_DISABLED=0".to_string());
            defines.push("PLATFORM_SEH_EXCEPTIONS_DISABLED=0".to_string());
            defines.push("PLATFORM_SUPPORTS_PRAGMA_PACK=1".to_string());

            // Compiler identification
            defines.push("PLATFORM_COMPILER_CLANG=0".to_string());
            defines.push("PLATFORM_COMPILER_MSVC=1".to_string());

            // Set all other platforms to 0
            defines.push("PLATFORM_MAC=0".to_string());
            defines.push("PLATFORM_IOS=0".to_string());
            defines.push("PLATFORM_TVOS=0".to_string());
            defines.push("PLATFORM_ANDROID=0".to_string());
            defines.push("PLATFORM_APPLE=0".to_string());
            defines.push("PLATFORM_LINUX=0".to_string());
            defines.push("PLATFORM_UNIX=0".to_string());
            defines.push("PLATFORM_FREEBSD=0".to_string());
            defines.push("PLATFORM_SWITCH=0".to_string());
        }
        "Linux" => {
            defines.push("PLATFORM_LINUX=1".to_string());
            defines.push("PLATFORM_UNIX=1".to_string());
            defines.push("PLATFORM_DESKTOP=1".to_string()); // MANDATORY
            defines.push("PLATFORM_64BITS=1".to_string()); // MANDATORY
            defines.push("PLATFORM_CPU_X86_FAMILY=1".to_string());
            defines.push("PLATFORM_LITTLE_ENDIAN=1".to_string());

            // Set all other platforms to 0
            defines.push("PLATFORM_WINDOWS=0".to_string());
            defines.push("PLATFORM_MAC=0".to_string());
            defines.push("PLATFORM_IOS=0".to_string());
            defines.push("PLATFORM_ANDROID=0".to_string());
            defines.push("PLATFORM_APPLE=0".to_string());
            defines.push("PLATFORM_MICROSOFT=0".to_string());
        }
        "Mac" => {
            defines.push("PLATFORM_MAC=1".to_string());
            defines.push("PLATFORM_APPLE=1".to_string());
            defines.push("PLATFORM_DESKTOP=1".to_string()); // MANDATORY
            defines.push("PLATFORM_64BITS=1".to_string()); // MANDATORY
            defines.push("PLATFORM_LITTLE_ENDIAN=1".to_string());

            // Set all other platforms to 0
            defines.push("PLATFORM_WINDOWS=0".to_string());
            defines.push("PLATFORM_LINUX=0".to_string());
            defines.push("PLATFORM_IOS=0".to_string());
            defines.push("PLATFORM_ANDROID=0".to_string());
            defines.push("PLATFORM_UNIX=0".to_string());
            defines.push("PLATFORM_MICROSOFT=0".to_string());
        }
        _ => {
            // Default to Windows if platform is unknown
            defines.push("PLATFORM_WINDOWS=1".to_string());
            defines.push("PLATFORM_DESKTOP=1".to_string());
            defines.push("PLATFORM_64BITS=1".to_string());
        }
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

/// Build common engine module API defines
/// These are core UE modules that are always available but may not be explicitly parsed
pub fn build_engine_module_api_defines() -> Vec<String> {
    vec![
        // Core modules
        "CORE_API=",
        "COREUOBJECT_API=",
        "ENGINE_API=",
        "APPLICATIONCORE_API=",
        "TRACELOGS_API=",
        "TRACELOG_API=",

        // Common runtime modules
        "INPUTCORE_API=",
        "SLATE_API=",
        "SLATECORE_API=",
        "UNREALED_API=",
        "MAINFRAME_API=",

        // Rendering
        "RENDERER_API=",
        "RENDERCORE_API=",
        "RHI_API=",
        "RHICORE_API=",

        // Audio
        "AUDIOENGINE_API=",
        "AUDIOMIXER_API=",

        // Networking
        "SOCKETS_API=",
        "NETWORKING_API=",

        // Common editor modules
        "EDITORSTYLE_API=",
        "KISMET_API=",
        "BLUEPRINTGRAPH_API=",
        "PROPERTYEDITOR_API=",
        "EDITORFRAMEWORK_API=",
        "UNREALEDITORUTILITIES_API=",

        // Additional common modules
        "ASSETREGISTRY_API=",
        "PROJECTS_API=",
        "MESSAGING_API=",
        "DEVELOPERSETTINGS_API=",
        "TOOLMENUS_API=",
        "HTTP_API=",
        "JSON_API=",
        "JSONUTILITIES_API=",

        // Analytics and profiling
        "ANALYTICS_API=",
        "PROFILER_API=",
        "TRACEANALYSIS_API=",

        // Asset tools
        "ASSETTOOLS_API=",
        "CONTENTBROWSER_API=",

        // Misc common modules
        "SETTINGS_API=",
        "TARGETS_API=",
        "DESKTOPPLATFORM_API=",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Build all defines for a project
pub fn build_all_defines(project: &UEProject, platform: &str, configuration: &str) -> Vec<String> {
    let mut defines = Vec::new();

    // Standard UE defines
    defines.extend(build_standard_defines(platform, configuration));

    // Engine module API defines (Core, Engine, etc.)
    defines.extend(build_engine_module_api_defines());

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
