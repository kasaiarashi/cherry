// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Unreal Engine project parsing and CherrySight (IntelliSense) configuration module.
//!
//! This module provides functionality to:
//! - Parse .uproject, .uplugin, and .Build.cs files
//! - Build dependency graphs
//! - Discover include paths
//! - Generate preprocessor defines
//! - Generate clangd configuration for IntelliSense support

pub mod build_cs_parser;
pub mod cherrysight_manager;
pub mod clangd_config_generator;
pub mod defines_builder;
pub mod dependency_resolver;
pub mod include_path_builder;
pub mod project_model;
pub mod uplugin_parser;
pub mod uproject_parser;

// Re-export commonly used types
pub use project_model::{
    ClangdConfig, CompileFlags, DiagnosticsConfig, IncludePaths, IndexConfig, ModuleCache,
    ModuleDependencies, ModuleType, UEModule, UEPlugin, UEProject, UPluginFile, UProjectFile,
    UProjectModule, UProjectPluginReference,
};

pub use build_cs_parser::{parse_build_cs, parse_build_cs_from_str};
pub use cherrysight_manager::CherrySightManager;
pub use clangd_config_generator::{generate_clangd_config, write_clangd_config};
pub use defines_builder::{build_all_defines, build_module_api_defines, build_standard_defines};
pub use dependency_resolver::{resolve_dependencies, DependencyGraph};
pub use include_path_builder::{infer_module_include_paths, IncludePathBuilder};
pub use uplugin_parser::{parse_uplugin, parse_uplugin_from_str};
pub use uproject_parser::{parse_uproject, parse_uproject_from_str};

use std::path::PathBuf;

/// Find the engine path based on engine association string (e.g., "5.7", "5.6")
///
/// This function tries common installation locations for Unreal Engine on Windows:
/// 1. W:\Softwares\UE_<version>
/// 2. C:\Program Files\Epic Games\UE_<version>
/// 3. Environment variable UE_<version>_PATH
///
/// Returns None if the engine cannot be found.
pub fn find_engine_path(engine_association: &str) -> Option<PathBuf> {
    // Try common installation paths
    let common_paths = [
        format!("W:\\Softwares\\UE_{}", engine_association),
        format!("C:\\Program Files\\Epic Games\\UE_{}", engine_association),
    ];

    for path_str in &common_paths {
        let path = PathBuf::from(path_str);
        if path.exists() && path.join("Engine").exists() {
            log::info!("Found Unreal Engine {} at: {}", engine_association, path.display());
            return Some(path);
        }
    }

    // Try environment variable
    let env_var_name = format!("UE_{}_PATH", engine_association.replace('.', "_"));
    if let Ok(env_path) = std::env::var(&env_var_name) {
        let path = PathBuf::from(env_path);
        if path.exists() && path.join("Engine").exists() {
            log::info!("Found Unreal Engine {} from environment variable {}: {}",
                      engine_association, env_var_name, path.display());
            return Some(path);
        }
    }

    log::warn!("Could not find Unreal Engine {} installation. Tried common paths and environment variable {}",
              engine_association, env_var_name);
    None
}
