// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

use super::build_cs_parser::parse_build_cs;
use super::clangd_config_generator::{generate_clangd_config, write_clangd_config};
use super::defines_builder::build_all_defines;
use super::include_path_builder::{infer_module_include_paths, IncludePathBuilder};
use super::project_model::{
    ModuleType, UEModule, UEPlugin, UEProject, UPluginFile, UProjectFile,
};
use super::uplugin_parser::parse_uplugin;
use super::uproject_parser::parse_uproject;
use crate::solution::{find_solution_file, parse_solution};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct CherrySightManager {
    project_root: PathBuf,
    uproject_path: PathBuf,
}

impl CherrySightManager {
    pub fn new(project_root: PathBuf, uproject_path: PathBuf) -> Self {
        Self {
            project_root,
            uproject_path,
        }
    }

    /// Main entry point: auto-configure CherrySight on workspace open
    pub fn auto_configure(&self) -> Result<()> {
        log::info!("Starting UE CherrySight auto-configuration...");

        // Step 1: Parse .uproject
        log::info!("Parsing .uproject file: {}", self.uproject_path.display());
        let uproject_file = parse_uproject(&self.uproject_path)?;
        log::info!(
            "Found {} modules and {} plugin references",
            uproject_file.modules.len(),
            uproject_file.plugins.len()
        );

        // Step 2: Detect engine path
        log::info!("Detecting engine path...");
        let engine_path = self.detect_engine_path(&uproject_file.engine_association)?;
        log::info!("Engine path: {}", engine_path.display());

        // Step 3: Discover and parse plugins
        log::info!("Discovering plugins...");
        let plugins = self.discover_plugins(&uproject_file, &engine_path)?;
        log::info!("Discovered {} plugins", plugins.len());

        // Step 4: Parse project modules
        log::info!("Parsing project modules...");
        let project_modules = self.parse_project_modules(&uproject_file)?;
        log::info!("Parsed {} project modules", project_modules.len());

        // Step 5: Build project model
        let project = UEProject {
            uproject_path: self.uproject_path.clone(),
            project_name: self
                .uproject_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            engine_association: uproject_file.engine_association.clone(),
            engine_path: Some(engine_path.clone()),
            modules: project_modules,
            plugins,
            target_platforms: uproject_file.target_platforms.clone(),
        };

        // Step 6: Build include paths
        log::info!("Building include paths...");
        let builder = IncludePathBuilder::new(engine_path.clone(), self.project_root.clone());
        let include_paths = builder.build_all_include_paths(&project);
        log::info!("Generated {} include paths", include_paths.len());

        // Step 7: Build defines
        log::info!("Building preprocessor defines...");
        let defines = build_all_defines(&project, "Windows", "Development");
        log::info!("Generated {} defines", defines.len());

        // Step 8: Generate .clangd config
        log::info!("Generating .clangd configuration...");
        let config = generate_clangd_config(&include_paths, &defines)?;

        // Step 9: Write .clangd
        write_clangd_config(&config, &self.project_root)?;

        log::info!("UE CherrySight auto-configuration complete!");
        Ok(())
    }

    /// Detect engine path from engine association or solution file
    fn detect_engine_path(&self, engine_association: &str) -> Result<PathBuf> {
        // Try to find engine path from solution file
        if let Some(sln_path) = find_solution_file(&self.project_root) {
            log::info!("Found solution file: {}", sln_path.display());
            if let Ok(solution) = parse_solution(&sln_path) {
                if let Some(engine_path) = solution.detect_engine_path() {
                    log::info!("Detected engine path from solution: {}", engine_path.display());
                    return Ok(engine_path);
                }
            }
        }

        // Fallback: Try common paths
        let common_paths = vec![
            PathBuf::from(format!("W:\\Softwares\\UE_{}", engine_association)),
            PathBuf::from(format!("C:\\Program Files\\Epic Games\\UE_{}", engine_association)),
            PathBuf::from(format!("W:\\Softwares\\UnrealEngine-{}", engine_association)),
        ];

        for path in common_paths {
            if path.exists() {
                log::info!("Found engine at common path: {}", path.display());
                return Ok(path);
            }
        }

        anyhow::bail!(
            "Could not detect engine path for engine association: {}",
            engine_association
        )
    }

    /// Discover and parse all plugins (project plugins and enabled engine plugins)
    fn discover_plugins(
        &self,
        uproject_file: &UProjectFile,
        engine_path: &Path,
    ) -> Result<Vec<UEPlugin>> {
        let mut plugins = Vec::new();

        // Discover project plugins
        let project_plugins_dir = self.project_root.join("Plugins");
        if project_plugins_dir.exists() {
            plugins.extend(self.scan_plugins_directory(&project_plugins_dir, false)?);
        }

        // Discover enabled engine plugins
        for plugin_ref in &uproject_file.plugins {
            if !plugin_ref.enabled {
                continue;
            }

            // Try to find the plugin in engine
            let engine_plugins_dir = engine_path.join("Plugins");
            if let Some(plugin) = self.find_plugin(&plugin_ref.name, &engine_plugins_dir, true)? {
                plugins.push(plugin);
            }
        }

        Ok(plugins)
    }

    /// Scan a plugins directory for .uplugin files
    fn scan_plugins_directory(
        &self,
        plugins_dir: &Path,
        is_engine_plugin: bool,
    ) -> Result<Vec<UEPlugin>> {
        let mut plugins = Vec::new();

        for entry in WalkDir::new(plugins_dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("uplugin") {
                if let Ok(plugin) = self.parse_plugin_file(entry.path(), is_engine_plugin) {
                    plugins.push(plugin);
                }
            }
        }

        Ok(plugins)
    }

    /// Find a specific plugin by name
    fn find_plugin(
        &self,
        plugin_name: &str,
        plugins_dir: &Path,
        is_engine_plugin: bool,
    ) -> Result<Option<UEPlugin>> {
        for entry in WalkDir::new(plugins_dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("uplugin") {
                if let Some(file_stem) = entry.path().file_stem() {
                    if file_stem.to_string_lossy() == plugin_name {
                        return Ok(Some(
                            self.parse_plugin_file(entry.path(), is_engine_plugin)?
                        ));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Parse a .uplugin file and its modules
    fn parse_plugin_file(&self, uplugin_path: &Path, is_engine_plugin: bool) -> Result<UEPlugin> {
        let uplugin_file = parse_uplugin(uplugin_path)?;
        let plugin_root = uplugin_path.parent().unwrap().to_path_buf();

        let mut modules = Vec::new();
        for module_desc in &uplugin_file.modules {
            if let Ok(module) = self.parse_module_from_plugin(
                &module_desc.name,
                &module_desc.module_type,
                &plugin_root,
            ) {
                modules.push(module);
            }
        }

        Ok(UEPlugin {
            name: uplugin_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            uplugin_path: uplugin_path.to_path_buf(),
            plugin_root: plugin_root.clone(),
            version: uplugin_file.file_version,
            friendly_name: None,
            modules,
            plugin_dependencies: uplugin_file
                .plugins
                .iter()
                .map(|p| p.name.clone())
                .collect(),
            is_engine_plugin,
            can_contain_content: false,
        })
    }

    /// Parse project modules from .uproject
    fn parse_project_modules(&self, uproject_file: &UProjectFile) -> Result<Vec<UEModule>> {
        let mut modules = Vec::new();

        for module_desc in &uproject_file.modules {
            let source_path = self.project_root.join("Source").join(&module_desc.name);
            let build_cs_path = source_path.join(format!("{}.Build.cs", module_desc.name));

            if !build_cs_path.exists() {
                log::warn!(
                    "Build.cs not found for module {}: {}",
                    module_desc.name,
                    build_cs_path.display()
                );
                continue;
            }

            let (dependencies, include_paths, defines) = parse_build_cs(&build_cs_path)
                .with_context(|| format!("Failed to parse {}", build_cs_path.display()))?;

            let mut inferred_includes = infer_module_include_paths(&source_path);
            inferred_includes.extend(include_paths.inferred_paths.clone());

            modules.push(UEModule {
                name: module_desc.name.clone(),
                module_type: ModuleType::from_str(&module_desc.module_type),
                loading_phase: module_desc.loading_phase.clone(),
                source_path,
                build_cs_path,
                dependencies,
                include_paths: super::project_model::IncludePaths {
                    public_include_paths: include_paths.public_include_paths,
                    private_include_paths: include_paths.private_include_paths,
                    inferred_paths: inferred_includes,
                },
                defines,
            });
        }

        Ok(modules)
    }

    /// Parse a module from a plugin
    fn parse_module_from_plugin(
        &self,
        module_name: &str,
        module_type: &str,
        plugin_root: &Path,
    ) -> Result<UEModule> {
        let source_path = plugin_root.join("Source").join(module_name);
        let build_cs_path = source_path.join(format!("{}.Build.cs", module_name));

        if !build_cs_path.exists() {
            anyhow::bail!(
                "Build.cs not found for plugin module {}: {}",
                module_name,
                build_cs_path.display()
            );
        }

        let (dependencies, include_paths, defines) = parse_build_cs(&build_cs_path)?;

        let mut inferred_includes = infer_module_include_paths(&source_path);
        inferred_includes.extend(include_paths.inferred_paths.clone());

        Ok(UEModule {
            name: module_name.to_string(),
            module_type: ModuleType::from_str(module_type),
            loading_phase: "Default".to_string(),
            source_path,
            build_cs_path,
            dependencies,
            include_paths: super::project_model::IncludePaths {
                public_include_paths: include_paths.public_include_paths,
                private_include_paths: include_paths.private_include_paths,
                inferred_paths: inferred_includes,
            },
            defines,
        })
    }
}
