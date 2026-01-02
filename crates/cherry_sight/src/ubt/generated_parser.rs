// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Generated header file (.generated.h) parser

use std::path::{Path, PathBuf};
use regex::Regex;
use anyhow::{Context, Result};

/// Generated reflection data
#[derive(Debug, Clone)]
pub struct ReflectionData {
    /// Property metadata
    pub properties: Vec<PropertyMetadata>,
    /// Function metadata
    pub functions: Vec<FunctionMetadata>,
    /// Generated body macro location
    pub generated_body_line: Option<usize>,
}

/// Property metadata from generated code
#[derive(Debug, Clone)]
pub struct PropertyMetadata {
    pub name: String,
    pub property_type: String,
    pub offset: usize,
    pub flags: Vec<String>,
}

/// Function metadata from generated code
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<String>,
    pub flags: Vec<String>,
}

/// Information extracted from .generated.h file
#[derive(Debug, Clone)]
pub struct GeneratedInfo {
    pub file_path: PathBuf,
    pub class_name: String,
    pub reflection_data: ReflectionData,
    pub include_dependencies: Vec<PathBuf>,
}

/// Parses UE5 generated header files
pub struct GeneratedHeaderParser {
    property_regex: Regex,
    function_regex: Regex,
    include_regex: Regex,
}

impl GeneratedHeaderParser {
    pub fn new() -> Self {
        Self {
            // Match UPROPERTY declarations in generated code
            property_regex: Regex::new(
                r"UPROPERTY\(.*\)\s+(?P<type>[\w:]+)\s+(?P<name>\w+);"
            ).unwrap(),
            // Match UFUNCTION declarations
            function_regex: Regex::new(
                r"UFUNCTION\(.*\)\s+(?:static\s+)?(?P<return>[\w:]+)\s+(?P<name>\w+)\s*\("
            ).unwrap(),
            // Match includes
            include_regex: Regex::new(
                r#"#include\s+"(?P<file>[^"]+)""#
            ).unwrap(),
        }
    }

    /// Parse a .generated.h file
    pub fn parse_file(&self, path: &Path) -> Result<GeneratedInfo> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read generated header: {:?}", path))?;

        let class_name = self.extract_class_name(path)?;
        let reflection_data = self.parse_reflection_data(&content);
        let include_dependencies = self.parse_includes(&content);

        Ok(GeneratedInfo {
            file_path: path.to_path_buf(),
            class_name,
            reflection_data,
            include_dependencies,
        })
    }

    /// Extract class name from file path
    fn extract_class_name(&self, path: &Path) -> Result<String> {
        // File format: ClassName.generated.h
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("Invalid generated header file name")?;

        // Remove .generated suffix
        let class_name = file_name
            .strip_suffix(".generated")
            .unwrap_or(file_name)
            .to_string();

        Ok(class_name)
    }

    /// Parse reflection data from generated code
    fn parse_reflection_data(&self, content: &str) -> ReflectionData {
        let mut properties = Vec::new();
        let mut functions = Vec::new();
        let mut generated_body_line = None;

        let lines: Vec<&str> = content.lines().collect();

        // Create regexes outside the loop
        let decl_regex = Regex::new(r"(?P<type>[\w:]+)\s+(?P<name>\w+);").unwrap();
        let func_decl_regex = Regex::new(r"(?:static\s+)?(?P<return>[\w:]+)\s+(?P<name>\w+)\s*\(").unwrap();

        for (line_num, line) in lines.iter().enumerate() {
            // Parse properties (UPROPERTY might be on previous line)
            if let Some(caps) = self.property_regex.captures(line) {
                properties.push(PropertyMetadata {
                    name: caps["name"].to_string(),
                    property_type: caps["type"].to_string(),
                    offset: 0, // Would need actual offset calculation
                    flags: Vec::new(),
                });
            } else if line.trim().starts_with("UPROPERTY") && line_num + 1 < lines.len() {
                // UPROPERTY is on its own line, property declaration follows
                let next_line = lines[line_num + 1];
                if let Some(decl_match) = decl_regex.captures(next_line) {
                    properties.push(PropertyMetadata {
                        name: decl_match["name"].to_string(),
                        property_type: decl_match["type"].to_string(),
                        offset: 0,
                        flags: Vec::new(),
                    });
                }
            }

            // Parse functions (UFUNCTION might be on previous line)
            if let Some(caps) = self.function_regex.captures(line) {
                functions.push(FunctionMetadata {
                    name: caps["name"].to_string(),
                    return_type: caps["return"].to_string(),
                    parameters: Vec::new(), // Would need parameter parsing
                    flags: Vec::new(),
                });
            } else if line.trim().starts_with("UFUNCTION") && line_num + 1 < lines.len() {
                // UFUNCTION is on its own line, function declaration follows
                let next_line = lines[line_num + 1];
                if let Some(func_match) = func_decl_regex.captures(next_line) {
                    functions.push(FunctionMetadata {
                        name: func_match["name"].to_string(),
                        return_type: func_match["return"].to_string(),
                        parameters: Vec::new(),
                        flags: Vec::new(),
                    });
                }
            }

            // Find GENERATED_BODY macro
            if line.contains("GENERATED_BODY()") || line.contains("GENERATED_UCLASS_BODY()") {
                generated_body_line = Some(line_num + 1);
            }
        }

        ReflectionData {
            properties,
            functions,
            generated_body_line,
        }
    }

    /// Parse include directives
    fn parse_includes(&self, content: &str) -> Vec<PathBuf> {
        let mut includes = Vec::new();

        for line in content.lines() {
            if let Some(caps) = self.include_regex.captures(line) {
                includes.push(PathBuf::from(&caps["file"]));
            }
        }

        includes
    }

    /// Find all .generated.h files in a directory
    pub fn find_generated_headers(&self, directory: &Path) -> Result<Vec<PathBuf>> {
        let mut generated_files = Vec::new();

        if !directory.exists() {
            return Ok(generated_files);
        }

        for entry in std::fs::read_dir(directory)
            .with_context(|| format!("Failed to read directory: {:?}", directory))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if file_name.ends_with(".generated.h") {
                        generated_files.push(path);
                    }
                }
            }
        }

        Ok(generated_files)
    }

    /// Check if a file is a generated header
    pub fn is_generated_header(path: &Path) -> bool {
        path.file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.ends_with(".generated.h"))
            .unwrap_or(false)
    }
}

impl Default for GeneratedHeaderParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_class_name() {
        let parser = GeneratedHeaderParser::new();

        let path = PathBuf::from("MyClass.generated.h");
        let class_name = parser.extract_class_name(&path).unwrap();
        assert_eq!(class_name, "MyClass");
    }

    #[test]
    fn test_parse_reflection_data() {
        let parser = GeneratedHeaderParser::new();

        let content = r#"
UPROPERTY(EditAnywhere, BlueprintReadWrite)
float Health;

UFUNCTION(BlueprintCallable)
void TakeDamage();

GENERATED_BODY()
"#;

        let data = parser.parse_reflection_data(content);

        assert_eq!(data.properties.len(), 1);
        assert_eq!(data.properties[0].name, "Health");
        assert_eq!(data.properties[0].property_type, "float");

        assert_eq!(data.functions.len(), 1);
        assert_eq!(data.functions[0].name, "TakeDamage");

        assert!(data.generated_body_line.is_some());
    }

    #[test]
    fn test_parse_includes() {
        let parser = GeneratedHeaderParser::new();

        let content = r#"
#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "MyClass.generated.h"
"#;

        let includes = parser.parse_includes(content);

        assert_eq!(includes.len(), 3);
        assert!(includes.iter().any(|p| p.to_string_lossy().contains("CoreMinimal.h")));
    }

    #[test]
    fn test_find_generated_headers() {
        let temp_dir = TempDir::new().unwrap();
        let parser = GeneratedHeaderParser::new();

        // Create test files
        fs::write(temp_dir.path().join("Class1.generated.h"), "").unwrap();
        fs::write(temp_dir.path().join("Class2.generated.h"), "").unwrap();
        fs::write(temp_dir.path().join("RegularHeader.h"), "").unwrap();

        let headers = parser.find_generated_headers(temp_dir.path()).unwrap();

        assert_eq!(headers.len(), 2);
        assert!(headers.iter().all(|p| GeneratedHeaderParser::is_generated_header(p)));
    }

    #[test]
    fn test_is_generated_header() {
        assert!(GeneratedHeaderParser::is_generated_header(Path::new("MyClass.generated.h")));
        assert!(!GeneratedHeaderParser::is_generated_header(Path::new("MyClass.h")));
    }
}
