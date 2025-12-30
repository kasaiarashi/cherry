use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Type of file entry in a .vcxproj
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VcxprojFileType {
    /// C++ source file (.cpp, .c, .cc, etc.)
    ClCompile,
    /// Header file (.h, .hpp, etc.)
    ClInclude,
    /// Other file type (resources, etc.)
    None,
}

impl VcxprojFileType {
    pub fn from_element_name(name: &str) -> Self {
        match name {
            "ClCompile" => Self::ClCompile,
            "ClInclude" => Self::ClInclude,
            _ => Self::None,
        }
    }

    pub fn is_source(&self) -> bool {
        matches!(self, Self::ClCompile)
    }

    pub fn is_header(&self) -> bool {
        matches!(self, Self::ClInclude)
    }
}

/// Represents a file entry from .vcxproj
#[derive(Debug, Clone)]
pub struct VcxprojFile {
    pub relative_path: PathBuf,
    pub file_type: VcxprojFileType,
}

impl VcxprojFile {
    pub fn file_name(&self) -> Option<&str> {
        self.relative_path.file_name().and_then(|n| n.to_str())
    }

    pub fn extension(&self) -> Option<&str> {
        self.relative_path.extension().and_then(|e| e.to_str())
    }
}

/// Parsed project file
#[derive(Debug, Clone)]
pub struct ParsedVcxproj {
    pub path: PathBuf,
    pub name: String,
    pub files: Vec<VcxprojFile>,
}

impl ParsedVcxproj {
    /// Get all source files (.cpp, .c, etc.)
    pub fn source_files(&self) -> impl Iterator<Item = &VcxprojFile> {
        self.files.iter().filter(|f| f.file_type.is_source())
    }

    /// Get all header files (.h, .hpp, etc.)
    pub fn header_files(&self) -> impl Iterator<Item = &VcxprojFile> {
        self.files.iter().filter(|f| f.file_type.is_header())
    }

    /// Get files organized by directory
    pub fn files_by_directory(&self) -> std::collections::HashMap<PathBuf, Vec<&VcxprojFile>> {
        let mut map: std::collections::HashMap<PathBuf, Vec<&VcxprojFile>> =
            std::collections::HashMap::new();

        for file in &self.files {
            let dir = file
                .relative_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default();
            map.entry(dir).or_default().push(file);
        }

        map
    }

    /// Build a directory tree structure from the files
    pub fn directory_tree(&self) -> DirectoryNode {
        let mut root = DirectoryNode::new(PathBuf::new(), String::new());

        for file in &self.files {
            root.add_file(file);
        }

        root
    }
}

/// A node in the directory tree
#[derive(Debug, Clone)]
pub struct DirectoryNode {
    pub path: PathBuf,
    pub name: String,
    pub files: Vec<VcxprojFile>,
    pub children: Vec<DirectoryNode>,
}

impl DirectoryNode {
    pub fn new(path: PathBuf, name: String) -> Self {
        Self {
            path,
            name,
            files: Vec::new(),
            children: Vec::new(),
        }
    }

    fn add_file(&mut self, file: &VcxprojFile) {
        let components: Vec<_> = file.relative_path.components().collect();

        if components.len() <= 1 {
            // File is in root
            self.files.push(file.clone());
        } else {
            // Navigate/create subdirectories
            let mut current = self;
            for (i, component) in components.iter().enumerate() {
                if i == components.len() - 1 {
                    // Last component is the file
                    current.files.push(file.clone());
                } else {
                    // Directory component
                    let dir_name = component.as_os_str().to_string_lossy().to_string();

                    // Find or create child directory
                    let child_idx = current
                        .children
                        .iter()
                        .position(|c| c.name == dir_name)
                        .unwrap_or_else(|| {
                            let child_path = if current.path.as_os_str().is_empty() {
                                PathBuf::from(&dir_name)
                            } else {
                                current.path.join(&dir_name)
                            };
                            current
                                .children
                                .push(DirectoryNode::new(child_path, dir_name));
                            current.children.len() - 1
                        });

                    current = &mut current.children[child_idx];
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.children.is_empty()
    }

    pub fn file_count(&self) -> usize {
        self.files.len() + self.children.iter().map(|c| c.file_count()).sum::<usize>()
    }
}

/// Parse a Visual Studio C++ project file
pub fn parse_vcxproj(path: &Path) -> Result<ParsedVcxproj> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read vcxproj file: {}", path.display()))?;

    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Project".to_string());

    let files = parse_files(&content);

    Ok(ParsedVcxproj {
        path: path.to_path_buf(),
        name,
        files,
    })
}

fn parse_files(content: &str) -> Vec<VcxprojFile> {
    let mut files = Vec::new();

    // Parse ClCompile elements: <ClCompile Include="path/to/file.cpp" />
    if let Ok(compile_regex) = Regex::new(r#"<ClCompile\s+Include="([^"]+)""#) {
        for cap in compile_regex.captures_iter(content) {
            let path_str = normalize_path(&cap[1]);
            files.push(VcxprojFile {
                relative_path: PathBuf::from(path_str),
                file_type: VcxprojFileType::ClCompile,
            });
        }
    }

    // Parse ClInclude elements: <ClInclude Include="path/to/file.h" />
    if let Ok(include_regex) = Regex::new(r#"<ClInclude\s+Include="([^"]+)""#) {
        for cap in include_regex.captures_iter(content) {
            let path_str = normalize_path(&cap[1]);
            files.push(VcxprojFile {
                relative_path: PathBuf::from(path_str),
                file_type: VcxprojFileType::ClInclude,
            });
        }
    }

    // Parse None elements (other files): <None Include="path/to/file" />
    if let Ok(none_regex) = Regex::new(r#"<None\s+Include="([^"]+)""#) {
        for cap in none_regex.captures_iter(content) {
            let path_str = normalize_path(&cap[1]);
            files.push(VcxprojFile {
                relative_path: PathBuf::from(path_str),
                file_type: VcxprojFileType::None,
            });
        }
    }

    files
}

/// Normalize Windows-style paths to platform-appropriate paths
fn normalize_path(path: &str) -> String {
    // Convert backslashes to forward slashes for cross-platform compatibility
    path.replace('\\', "/")
}

/// Parse a C# project file (.csproj) - simpler format
pub fn parse_csproj(path: &Path) -> Result<ParsedVcxproj> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read csproj file: {}", path.display()))?;

    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Project".to_string());

    let mut files = Vec::new();

    // Parse Compile elements: <Compile Include="path/to/file.cs" />
    if let Ok(compile_regex) = Regex::new(r#"<Compile\s+Include="([^"]+)""#) {
        for cap in compile_regex.captures_iter(&content) {
            let path_str = normalize_path(&cap[1]);
            files.push(VcxprojFile {
                relative_path: PathBuf::from(path_str),
                file_type: VcxprojFileType::ClCompile,
            });
        }
    }

    Ok(ParsedVcxproj {
        path: path.to_path_buf(),
        name,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_vcxproj() {
        let content = r#"
<?xml version="1.0" encoding="utf-8"?>
<Project>
    <ItemGroup>
        <ClCompile Include="Source\MyGame\MyActor.cpp" />
        <ClCompile Include="Source\MyGame\MyGameMode.cpp" />
        <ClInclude Include="Source\MyGame\MyActor.h" />
        <ClInclude Include="Source\MyGame\MyGameMode.h" />
    </ItemGroup>
</Project>
"#;

        let files = parse_files(content);
        assert_eq!(files.len(), 4);

        let sources: Vec<_> = files
            .iter()
            .filter(|f| f.file_type == VcxprojFileType::ClCompile)
            .collect();
        assert_eq!(sources.len(), 2);

        let headers: Vec<_> = files
            .iter()
            .filter(|f| f.file_type == VcxprojFileType::ClInclude)
            .collect();
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(r"Source\MyGame\File.cpp"), "Source/MyGame/File.cpp");
        assert_eq!(normalize_path("Source/MyGame/File.cpp"), "Source/MyGame/File.cpp");
    }

    #[test]
    fn test_directory_tree() {
        let vcxproj = ParsedVcxproj {
            path: PathBuf::from("test.vcxproj"),
            name: "test".to_string(),
            files: vec![
                VcxprojFile {
                    relative_path: PathBuf::from("Source/Game/Actor.cpp"),
                    file_type: VcxprojFileType::ClCompile,
                },
                VcxprojFile {
                    relative_path: PathBuf::from("Source/Game/Actor.h"),
                    file_type: VcxprojFileType::ClInclude,
                },
                VcxprojFile {
                    relative_path: PathBuf::from("Source/Game/Mode.cpp"),
                    file_type: VcxprojFileType::ClCompile,
                },
            ],
        };

        let tree = vcxproj.directory_tree();
        assert_eq!(tree.file_count(), 3);
        assert_eq!(tree.children.len(), 1); // "Source"
        assert_eq!(tree.children[0].children.len(), 1); // "Game"
        assert_eq!(tree.children[0].children[0].files.len(), 3);
    }
}
