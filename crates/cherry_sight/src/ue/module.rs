// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UE5 module representation

use std::path::{Path, PathBuf};

/// Type of UE module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleType {
    Runtime,
    RuntimeNoCommandlet,
    RuntimeAndProgram,
    CookedOnly,
    UncookedOnly,
    Developer,
    DeveloperTool,
    Editor,
    EditorNoCommandlet,
    EditorAndProgram,
    Program,
    ServerOnly,
    ClientOnly,
}

impl ModuleType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Runtime" => Some(Self::Runtime),
            "RuntimeNoCommandlet" => Some(Self::RuntimeNoCommandlet),
            "RuntimeAndProgram" => Some(Self::RuntimeAndProgram),
            "CookedOnly" => Some(Self::CookedOnly),
            "UncookedOnly" => Some(Self::UncookedOnly),
            "Developer" => Some(Self::Developer),
            "DeveloperTool" => Some(Self::DeveloperTool),
            "Editor" => Some(Self::Editor),
            "EditorNoCommandlet" => Some(Self::EditorNoCommandlet),
            "EditorAndProgram" => Some(Self::EditorAndProgram),
            "Program" => Some(Self::Program),
            "ServerOnly" => Some(Self::ServerOnly),
            "ClientOnly" => Some(Self::ClientOnly),
            _ => None,
        }
    }
}

/// Represents a UE5 module
#[derive(Debug, Clone)]
pub struct UEModule {
    pub name: String,
    pub path: PathBuf,
    pub module_type: ModuleType,
    pub dependencies: Vec<String>,
    pub public_include_paths: Vec<PathBuf>,
    pub private_include_paths: Vec<PathBuf>,
    pub public_defines: Vec<String>,
}

impl UEModule {
    /// Create a module from a .uproject module descriptor
    pub fn from_descriptor(
        name: &str,
        module_type: &str,
        path: &Path,
        _project_root: &Path,
    ) -> Option<Self> {
        let module_type = ModuleType::from_str(module_type)?;

        Some(Self {
            name: name.to_string(),
            path: path.to_path_buf(),
            module_type,
            dependencies: Vec::new(),
            public_include_paths: vec![
                path.join("Public"),
                path.join("Classes"),
            ],
            private_include_paths: vec![
                path.join("Private"),
            ],
            public_defines: vec![
                format!("{}_API=", name.to_uppercase()),
            ],
        })
    }

    /// Load module from Build.cs file
    pub fn from_build_cs(path: &Path) -> anyhow::Result<Self> {
        // To be implemented - parse Build.cs
        // For now, return a basic module
        let name = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        Ok(Self {
            name,
            path: path.to_path_buf(),
            module_type: ModuleType::Runtime,
            dependencies: Vec::new(),
            public_include_paths: Vec::new(),
            private_include_paths: Vec::new(),
            public_defines: Vec::new(),
        })
    }

    /// Get all include paths for this module
    pub fn include_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        paths.extend(self.public_include_paths.clone());
        paths.extend(self.private_include_paths.clone());
        paths
    }

    /// Get all defines for this module
    pub fn defines(&self) -> Vec<String> {
        self.public_defines.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_type_from_str() {
        assert_eq!(
            ModuleType::from_str("Runtime"),
            Some(ModuleType::Runtime)
        );
        assert_eq!(
            ModuleType::from_str("Editor"),
            Some(ModuleType::Editor)
        );
        assert_eq!(ModuleType::from_str("Invalid"), None);
    }

    #[test]
    fn test_module_creation() {
        let path = PathBuf::from("/project/Source/MyModule");
        let project_root = PathBuf::from("/project");

        let module = UEModule::from_descriptor(
            "MyModule",
            "Runtime",
            &path,
            &project_root,
        )
        .unwrap();

        assert_eq!(module.name, "MyModule");
        assert_eq!(module.module_type, ModuleType::Runtime);
        assert!(module.include_paths().len() > 0);
    }
}
