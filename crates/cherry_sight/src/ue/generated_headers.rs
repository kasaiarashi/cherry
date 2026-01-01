// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! UE5 generated header handling
//!
//! Handles .generated.h files created by UnrealHeaderTool

use std::path::{Path, PathBuf};

/// Information about a generated header file
#[derive(Debug, Clone)]
pub struct GeneratedHeader {
    pub source_file: PathBuf,
    pub generated_file: PathBuf,
}

impl GeneratedHeader {
    /// Get the expected generated header path for a source file
    pub fn for_source_file(source_path: &Path) -> Option<PathBuf> {
        let file_stem = source_path.file_stem()?.to_str()?;
        let parent = source_path.parent()?;

        // .generated.h files are typically in an Intermediate directory
        // For now, just return a simple transformation
        Some(parent.join(format!("{}.generated.h", file_stem)))
    }

    /// Check if a file has a corresponding generated header
    pub fn exists_for(source_path: &Path) -> bool {
        Self::for_source_file(source_path)
            .map(|p| p.exists())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generated_header_path() {
        let source = PathBuf::from("/project/Source/MyModule/MyClass.h");
        let generated = GeneratedHeader::for_source_file(&source).unwrap();

        assert_eq!(
            generated,
            PathBuf::from("/project/Source/MyModule/MyClass.generated.h")
        );
    }
}
