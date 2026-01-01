// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a complete UE project with all metadata
#[derive(Debug, Clone)]
pub struct UEProject {
    pub uproject_path: PathBuf,
    pub project_name: String,
    pub engine_association: String, // "5.7", "5.6", etc.
    pub engine_path: Option<PathBuf>,
    pub modules: Vec<UEModule>,
    pub plugins: Vec<UEPlugin>,
    pub target_platforms: Vec<String>,
}

/// A module (project-level or plugin-level)
#[derive(Debug, Clone)]
pub struct UEModule {
    pub name: String,
    pub module_type: ModuleType,
    pub loading_phase: String,
    pub source_path: PathBuf,         // Path to Source/<ModuleName>/
    pub build_cs_path: PathBuf,       // Path to <ModuleName>.Build.cs
    pub dependencies: ModuleDependencies,
    pub include_paths: IncludePaths,
    pub defines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleType {
    Runtime,
    Editor,
    Developer,
    ThirdParty,
}

impl ModuleType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Runtime" => ModuleType::Runtime,
            "Editor" => ModuleType::Editor,
            "Developer" => ModuleType::Developer,
            "ThirdParty" => ModuleType::ThirdParty,
            _ => ModuleType::Runtime, // Default to Runtime
        }
    }
}

/// Dependencies extracted from .Build.cs
#[derive(Debug, Clone, Default)]
pub struct ModuleDependencies {
    pub public_dependencies: Vec<String>,
    pub private_dependencies: Vec<String>,
    pub public_delay_load_dlls: Vec<String>,
    pub private_delay_load_dlls: Vec<String>,
}

/// Include paths from .Build.cs or inferred
#[derive(Debug, Clone, Default)]
pub struct IncludePaths {
    pub public_include_paths: Vec<PathBuf>,
    pub private_include_paths: Vec<PathBuf>,
    /// Standard paths: Source/<Module>/Public, Source/<Module>/Private
    pub inferred_paths: Vec<PathBuf>,
}

/// A plugin (project plugin or engine plugin)
#[derive(Debug, Clone)]
pub struct UEPlugin {
    pub name: String,
    pub uplugin_path: PathBuf,
    pub plugin_root: PathBuf,
    pub version: i32,
    pub friendly_name: Option<String>,
    pub modules: Vec<UEModule>,
    pub plugin_dependencies: Vec<String>, // Other plugins this depends on
    pub is_engine_plugin: bool,
    pub can_contain_content: bool,
}

/// Parsed .uproject file
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UProjectFile {
    pub file_version: i32,
    #[serde(default)]
    pub engine_association: String,
    #[serde(default)]
    pub modules: Vec<UProjectModule>,
    #[serde(default)]
    pub plugins: Vec<UProjectPluginReference>,
    #[serde(default)]
    pub target_platforms: Vec<String>,

    // Optional fields for .uplugin files
    #[serde(default)]
    pub version: Option<i32>,
    #[serde(default)]
    pub version_name: Option<String>,
    #[serde(default)]
    pub friendly_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub created_by_url: Option<String>,
    #[serde(default, rename = "DocsURL")]
    pub docs_url: Option<String>,
    #[serde(default, rename = "MarketplaceURL")]
    pub marketplace_url: Option<String>,
    #[serde(default, rename = "SupportURL")]
    pub support_url: Option<String>,
    #[serde(default)]
    pub can_contain_content: Option<bool>,
    #[serde(default)]
    pub is_beta_version: Option<bool>,
    #[serde(default)]
    pub is_experimental_version: Option<bool>,
    #[serde(default)]
    pub installed: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UProjectModule {
    pub name: String,
    #[serde(rename = "Type")]
    pub module_type: String, // "Runtime", "Editor", etc.
    pub loading_phase: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UProjectPluginReference {
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub target_allow_list: Vec<String>, // ["Editor"], ["Client"], etc.
}

/// Parsed .uplugin file (same structure as .uproject)
pub type UPluginFile = UProjectFile;

/// Clangd configuration to be generated
#[derive(Debug, Clone)]
pub struct ClangdConfig {
    pub compile_flags: CompileFlags,
    pub diagnostics: Option<DiagnosticsConfig>,
    pub index: Option<IndexConfig>,
}

#[derive(Debug, Clone)]
pub struct CompileFlags {
    pub add: Vec<String>, // -I/path, -DDEFINE, etc.
    pub remove: Vec<String>, // Flags to remove if present
}

#[derive(Debug, Clone, Default)]
pub struct DiagnosticsConfig {
    pub unused_includes: Option<String>, // "Strict", "None"
    pub missing_includes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct IndexConfig {
    pub background: Option<String>, // "Build", "Skip"
}

/// Cache for parsed modules to avoid re-parsing unchanged files
#[derive(Debug, Clone)]
pub struct ModuleCache {
    pub cached_modules: HashMap<PathBuf, CachedModule>,
}

#[derive(Debug, Clone)]
pub struct CachedModule {
    pub module: UEModule,
    pub last_modified: std::time::SystemTime,
}

impl ModuleCache {
    pub fn new() -> Self {
        Self {
            cached_modules: HashMap::new(),
        }
    }

    pub fn get(&self, path: &PathBuf) -> Option<&CachedModule> {
        self.cached_modules.get(path)
    }

    pub fn insert(&mut self, path: PathBuf, module: UEModule, modified_time: std::time::SystemTime) {
        self.cached_modules.insert(
            path,
            CachedModule {
                module,
                last_modified: modified_time,
            },
        );
    }

    pub fn is_stale(&self, path: &PathBuf, current_modified: std::time::SystemTime) -> bool {
        if let Some(cached) = self.cached_modules.get(path) {
            cached.last_modified != current_modified
        } else {
            true
        }
    }
}
