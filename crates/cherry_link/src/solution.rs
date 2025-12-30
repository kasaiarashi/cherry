use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};

/// GUID-based project type identifiers from Visual Studio
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VsProjectType {
    /// Solution Folder - virtual folder for organizing projects
    SolutionFolder,
    /// C++ Project (.vcxproj)
    CppProject,
    /// C# Project (.csproj)
    CSharpProject,
    /// Unknown project type
    Unknown,
}

impl VsProjectType {
    fn from_guid(guid: &str) -> Self {
        match guid.to_uppercase().as_str() {
            "2150E333-8FDC-42A3-9474-1A3956D46DE8" => Self::SolutionFolder,
            "8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942" => Self::CppProject,
            "FAE04EC0-301F-11D3-BF4B-00C04F79EFBC" => Self::CSharpProject,
            _ => Self::Unknown,
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, Self::SolutionFolder)
    }

    pub fn is_project(&self) -> bool {
        matches!(self, Self::CppProject | Self::CSharpProject)
    }
}

/// Represents a project entry in a .sln file
#[derive(Debug, Clone)]
pub struct SlnProject {
    pub project_type: VsProjectType,
    pub name: String,
    pub relative_path: PathBuf,
    pub guid: String,
    pub dependencies: Vec<String>,
}

/// Represents the nested structure from GlobalSection(NestedProjects)
#[derive(Debug, Clone)]
pub struct SlnNestedProject {
    pub child_guid: String,
    pub parent_guid: String,
}

/// Build configuration extracted from solution
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SlnBuildConfiguration {
    pub configuration: String,
    pub platform: String,
}

impl SlnBuildConfiguration {
    pub fn display_name(&self) -> String {
        format!("{}|{}", self.configuration, self.platform)
    }
}

/// Complete parsed solution structure
#[derive(Debug, Clone)]
pub struct ParsedSolution {
    pub path: PathBuf,
    pub name: String,
    pub projects: Vec<SlnProject>,
    pub nested_projects: Vec<SlnNestedProject>,
    pub configurations: Vec<SlnBuildConfiguration>,
}

impl ParsedSolution {
    /// Get all root-level projects (those not nested under another project/folder)
    pub fn root_projects(&self) -> impl Iterator<Item = &SlnProject> {
        let nested_guids: std::collections::HashSet<_> = self
            .nested_projects
            .iter()
            .map(|np| np.child_guid.as_str())
            .collect();

        self.projects
            .iter()
            .filter(move |p| !nested_guids.contains(p.guid.as_str()))
    }

    /// Get children of a given parent guid
    pub fn children_of(&self, parent_guid: &str) -> impl Iterator<Item = &SlnProject> {
        let child_guids: std::collections::HashSet<_> = self
            .nested_projects
            .iter()
            .filter(|np| np.parent_guid == parent_guid)
            .map(|np| np.child_guid.as_str())
            .collect();

        self.projects
            .iter()
            .filter(move |p| child_guids.contains(p.guid.as_str()))
    }

    /// Find a project by GUID
    pub fn find_project(&self, guid: &str) -> Option<&SlnProject> {
        self.projects.iter().find(|p| p.guid == guid)
    }

    /// Get unique configurations (without duplicates from different platforms)
    pub fn unique_configurations(&self) -> Vec<String> {
        let mut configs: Vec<_> = self
            .configurations
            .iter()
            .map(|c| c.configuration.clone())
            .collect();
        configs.sort();
        configs.dedup();
        configs
    }

    /// Get unique platforms
    pub fn unique_platforms(&self) -> Vec<String> {
        let mut platforms: Vec<_> = self
            .configurations
            .iter()
            .map(|c| c.platform.clone())
            .collect();
        platforms.sort();
        platforms.dedup();
        platforms
    }

    /// Detect the Unreal Engine path from project references
    pub fn detect_engine_path(&self) -> Option<PathBuf> {
        for project in &self.projects {
            let path_str = project.relative_path.to_string_lossy();
            // Look for Engine paths like "W:\Softwares\UE_5.6\Engine\..."
            if let Some(idx) = path_str.find("Engine") {
                if idx > 0 {
                    let engine_root = &path_str[..idx + 6]; // Include "Engine"
                    return Some(PathBuf::from(engine_root));
                }
            }
        }
        None
    }
}

/// Parse a Visual Studio solution file
pub fn parse_solution(path: &Path) -> Result<ParsedSolution> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read solution file: {}", path.display()))?;

    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Solution".to_string());

    let projects = parse_projects(&content)?;
    let nested_projects = parse_nested_projects(&content);
    let configurations = parse_configurations(&content);

    Ok(ParsedSolution {
        path: path.to_path_buf(),
        name,
        projects,
        nested_projects,
        configurations,
    })
}

fn parse_projects(content: &str) -> Result<Vec<SlnProject>> {
    let mut projects = Vec::new();

    // Pattern: Project("{TYPE_GUID}") = "NAME", "PATH", "{GUID}"
    let project_regex = Regex::new(
        r#"Project\("\{([^}]+)\}"\)\s*=\s*"([^"]+)",\s*"([^"]+)",\s*"\{([^}]+)\}""#,
    )?;

    // Pattern for ProjectSection(ProjectDependencies)
    let deps_section_regex =
        Regex::new(r#"ProjectSection\(ProjectDependencies\).*?EndProjectSection"#)?;
    let dep_guid_regex = Regex::new(r#"\{([^}]+)\}\s*=\s*\{[^}]+\}"#)?;

    for cap in project_regex.captures_iter(content) {
        let type_guid = &cap[1];
        let name = cap[2].to_string();
        let relative_path = PathBuf::from(&cap[3]);
        let guid = cap[4].to_string();

        // Find dependencies if any
        let mut dependencies = Vec::new();
        let project_start = cap.get(0).map(|m| m.end()).unwrap_or(0);
        if let Some(end_project) = content[project_start..].find("EndProject") {
            let project_section = &content[project_start..project_start + end_project];
            if let Some(deps_match) = deps_section_regex.find(project_section) {
                for dep_cap in dep_guid_regex.captures_iter(deps_match.as_str()) {
                    dependencies.push(dep_cap[1].to_string());
                }
            }
        }

        projects.push(SlnProject {
            project_type: VsProjectType::from_guid(type_guid),
            name,
            relative_path,
            guid,
            dependencies,
        });
    }

    Ok(projects)
}

fn parse_nested_projects(content: &str) -> Vec<SlnNestedProject> {
    let mut nested = Vec::new();

    // Find GlobalSection(NestedProjects)
    if let Some(section) = extract_global_section(content, "NestedProjects") {
        // Pattern: {CHILD_GUID} = {PARENT_GUID}
        let nested_regex = Regex::new(r#"\{([^}]+)\}\s*=\s*\{([^}]+)\}"#).ok();
        if let Some(regex) = nested_regex {
            for cap in regex.captures_iter(&section) {
                nested.push(SlnNestedProject {
                    child_guid: cap[1].to_string(),
                    parent_guid: cap[2].to_string(),
                });
            }
        }
    }

    nested
}

fn parse_configurations(content: &str) -> Vec<SlnBuildConfiguration> {
    let mut configs = Vec::new();

    // Find GlobalSection(SolutionConfigurationPlatforms)
    if let Some(section) = extract_global_section(content, "SolutionConfigurationPlatforms") {
        // Pattern: Config|Platform = Config|Platform
        let config_regex = Regex::new(r#"^\s*([^|=\r\n]+)\|([^=\r\n]+)\s*="#).ok();
        if let Some(regex) = config_regex {
            for line in section.lines() {
                if let Some(cap) = regex.captures(line) {
                    let configuration = cap[1].trim().to_string();
                    let platform = cap[2].trim().to_string();

                    let config = SlnBuildConfiguration {
                        configuration,
                        platform,
                    };

                    // Avoid duplicates
                    if !configs.contains(&config) {
                        configs.push(config);
                    }
                }
            }
        }
    }

    configs
}

fn extract_global_section(content: &str, section_name: &str) -> Option<String> {
    let pattern = format!(
        r"GlobalSection\({}\).*?=.*?\n([\s\S]*?)EndGlobalSection",
        regex::escape(section_name)
    );
    let regex = Regex::new(&pattern).ok()?;
    regex
        .captures(content)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

/// Find a .sln file in the given directory
pub fn find_solution_file(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }

    std::fs::read_dir(dir).ok()?.find_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.extension().map(|e| e == "sln").unwrap_or(false) {
            Some(path)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_type_from_guid() {
        assert_eq!(
            VsProjectType::from_guid("2150E333-8FDC-42A3-9474-1A3956D46DE8"),
            VsProjectType::SolutionFolder
        );
        assert_eq!(
            VsProjectType::from_guid("8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942"),
            VsProjectType::CppProject
        );
        assert_eq!(
            VsProjectType::from_guid("FAE04EC0-301F-11D3-BF4B-00C04F79EFBC"),
            VsProjectType::CSharpProject
        );
        assert_eq!(
            VsProjectType::from_guid("unknown-guid"),
            VsProjectType::Unknown
        );
    }

    #[test]
    fn test_parse_simple_solution() {
        let content = r#"
Microsoft Visual Studio Solution File, Format Version 12.00
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "MyGame", "MyGame.vcxproj", "{12345678-1234-1234-1234-123456789ABC}"
EndProject
Global
    GlobalSection(SolutionConfigurationPlatforms) = preSolution
        Debug|Win64 = Debug|Win64
        Release|Win64 = Release|Win64
    EndGlobalSection
EndGlobal
"#;

        let projects = parse_projects(content).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "MyGame");
        assert_eq!(projects[0].project_type, VsProjectType::CppProject);

        let configs = parse_configurations(content);
        assert_eq!(configs.len(), 2);
        assert!(configs.iter().any(|c| c.configuration == "Debug"));
        assert!(configs.iter().any(|c| c.configuration == "Release"));
    }
}
