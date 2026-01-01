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
